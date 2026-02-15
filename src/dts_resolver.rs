//! DTS resolution algorithm matching TypeScript's `ts.resolveModuleName`
//! with `moduleResolution: "bundler"`.
//!
//! This implements a separate resolution algorithm from the main enhanced-resolve
//! algorithm, following TypeScript's module resolution strategy for declaration files.
//!
//! Key differences from enhanced-resolve:
//! - Two-pass `node_modules` walk: all ancestors for TS/DTS+`@types` before JS
//! - `@types` scoped name mangling: `@babel/core` -> `@types/babel__core`
//! - TypeScript extension substitution: `.js` -> `.ts`, `.d.ts` (not extensionAlias)
//! - `typesVersions` package.json field support
//! - When `exports` exists, `types`/`typings`/`main` are ignored

use std::{borrow::Cow, path::Path};

use crate::{
    CachedPath, FileSystem, PackageJson, ResolveError, ResolverGeneric,
    context::ResolveContext as Ctx,
    resolution::{ModuleType, Resolution},
    specifier::Specifier,
};

type ResolveResult = Result<Option<CachedPath>, ResolveError>;

/// Extension categories for DTS resolution, using bitflag pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Extensions(u8);

impl Extensions {
    /// `.ts`, `.tsx`, `.mts`, `.cts`
    const TYPESCRIPT: Self = Self(0b0001);
    /// `.js`, `.jsx`, `.mjs`, `.cjs`
    const JAVASCRIPT: Self = Self(0b0010);
    /// `.d.ts`, `.d.mts`, `.d.cts`
    const DECLARATION: Self = Self(0b0100);

    const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    const fn is_empty(self) -> bool {
        self.0 == 0
    }

    const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }
}

impl std::ops::BitOr for Extensions {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

impl std::ops::BitAnd for Extensions {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl<Fs: FileSystem> ResolverGeneric<Fs> {
    /// Resolve a module specifier for TypeScript declaration files.
    ///
    /// Matches `ts.resolveModuleName` with `moduleResolution: "bundler"`.
    ///
    /// `containing_file` is the absolute path of the file containing the import.
    /// `specifier` is the module specifier string.
    ///
    /// The resolver uses the existing `ResolveOptions` for:
    /// - `condition_names` (used for `exports` resolution; `"types"` is always added)
    /// - `tsconfig` (for `paths` mapping)
    /// - `symlinks` (for realpath resolution)
    ///
    /// # Errors
    ///
    /// * See [`ResolveError`]
    pub fn resolve_dts<P: AsRef<Path>>(
        &self,
        containing_file: P,
        specifier: &str,
    ) -> Result<Resolution, ResolveError> {
        let containing_file = containing_file.as_ref();
        let span =
            tracing::debug_span!("resolve_dts", file = ?containing_file, specifier = specifier);
        let _enter = span.enter();
        let result = self.resolve_dts_impl(containing_file, specifier);
        match &result {
            Ok(r) => tracing::debug!(ret = ?r.path),
            Err(err) => tracing::debug!(?err),
        }
        result
    }

    fn resolve_dts_impl(
        &self,
        containing_file: &Path,
        specifier: &str,
    ) -> Result<Resolution, ResolveError> {
        let mut ctx = Ctx::default();

        let containing_dir = containing_file.parent().unwrap_or(containing_file);
        let cached_dir = self.cache.value(containing_dir);

        let extensions =
            Extensions::TYPESCRIPT.union(Extensions::JAVASCRIPT).union(Extensions::DECLARATION);

        // Parse query/fragment
        let parsed = Specifier::parse(specifier).map_err(ResolveError::Specifier)?;
        ctx.with_query_fragment(parsed.query, parsed.fragment);
        let specifier = parsed.path();

        // 1. tsconfig paths (non-relative only)
        if !specifier.starts_with('.')
            && !specifier.starts_with('/')
            && let Some(path) = self.dts_resolve_tsconfig_paths(&cached_dir, specifier, &mut ctx)?
        {
            return self.dts_finalize(&path, &mut ctx);
        }

        // 2. Route by specifier type
        let result = if specifier.starts_with('.') || specifier.starts_with('/') {
            // Relative or absolute
            let candidate = cached_dir.normalize_with(specifier, &self.cache);
            self.dts_resolve_relative(extensions, &candidate, &mut ctx)?
        } else if specifier.starts_with('#') {
            // Package imports
            self.dts_resolve_package_imports(&cached_dir, specifier, &mut ctx)?
        } else if specifier.contains(':') {
            // URI scheme (node:fs, etc.)
            None
        } else {
            // Bare specifier -> node_modules
            let (package_name, rest) = Self::parse_package_specifier(specifier);

            // Try self-referencing first
            if let Some(path) = self.dts_resolve_package_self(&cached_dir, specifier, &mut ctx)? {
                Some(path)
            } else {
                self.dts_resolve_node_modules(
                    extensions,
                    specifier,
                    package_name,
                    rest,
                    &cached_dir,
                    &mut ctx,
                )?
            }
        };

        result.map_or_else(
            || Err(ResolveError::NotFound(specifier.to_string())),
            |path| self.dts_finalize(&path, &mut ctx),
        )
    }

    fn dts_finalize(
        &self,
        cached_path: &CachedPath,
        ctx: &mut Ctx,
    ) -> Result<Resolution, ResolveError> {
        let path = self.load_realpath(cached_path)?;
        let package_json = self.find_package_json_for_a_package(cached_path, ctx)?;
        let module_type = Self::dts_module_type(cached_path);
        Ok(Resolution {
            path,
            query: ctx.query.take(),
            fragment: ctx.fragment.take(),
            package_json,
            module_type,
        })
    }

    fn dts_module_type(cached_path: &CachedPath) -> Option<ModuleType> {
        let path_str = cached_path.path().to_string_lossy();
        if path_str.ends_with(".d.mts") || path_str.ends_with(".mts") {
            Some(ModuleType::Module)
        } else if path_str.ends_with(".d.cts") || path_str.ends_with(".cts") {
            Some(ModuleType::CommonJs)
        } else if path_str.ends_with(".json") {
            Some(ModuleType::Json)
        } else {
            None
        }
    }

    // -------- Core resolution methods --------

    fn dts_resolve_relative(
        &self,
        extensions: Extensions,
        candidate: &CachedPath,
        ctx: &mut Ctx,
    ) -> ResolveResult {
        if let Some(path) = self.dts_resolve_as_file(extensions, candidate, ctx) {
            return Ok(Some(path));
        }
        self.dts_resolve_as_directory(extensions, candidate, ctx)
    }

    /// TS: `loadModuleFromFile`
    fn dts_resolve_as_file(
        &self,
        extensions: Extensions,
        candidate: &CachedPath,
        ctx: &mut Ctx,
    ) -> Option<CachedPath> {
        let path_str = candidate.path().to_string_lossy();

        // Phase 1: Extension replacement (./foo.js -> ./foo.ts)
        if let Some(original_ext) = Self::dts_get_known_extension(&path_str) {
            // Strip the extension to get the extensionless base
            let base_len = path_str.len() - original_ext.len();
            let base_str = &path_str[..base_len];
            let base = self.cache.value(Path::new(base_str));
            if let Some(path) = self.dts_try_extensions(&base, extensions, original_ext, ctx) {
                return Some(path);
            }
        }

        // Phase 2: Extension addition (./foo -> ./foo.ts)
        self.dts_try_extensions(candidate, extensions, "", ctx)
    }

    /// Get the known extension from a path, including compound extensions like `.d.ts`.
    fn dts_get_known_extension(path: &str) -> Option<&str> {
        // Check compound extensions first
        for ext in &[".d.ts", ".d.mts", ".d.cts"] {
            if path.ends_with(ext) {
                return Some(ext);
            }
        }
        // Check single extensions
        let dot_pos = path.rfind('.')?;
        let ext = &path[dot_pos..];
        match ext {
            ".ts" | ".tsx" | ".mts" | ".cts" | ".js" | ".jsx" | ".mjs" | ".cjs" | ".json" => {
                Some(ext)
            }
            _ => {
                // Unknown extensions like .vue, .svelte - return them for d.{ext}.ts handling
                if !ext.is_empty() && ext.len() < 10 { Some(ext) } else { None }
            }
        }
    }

    /// TS: `tryAddingExtensions`
    #[allow(clippy::too_many_lines)]
    fn dts_try_extensions(
        &self,
        base: &CachedPath,
        extensions: Extensions,
        original_ext: &str,
        ctx: &mut Ctx,
    ) -> Option<CachedPath> {
        match original_ext {
            ".mjs" | ".mts" | ".d.mts" => {
                if extensions.contains(Extensions::TYPESCRIPT)
                    && let Some(p) = self.dts_try_file(base, ".mts", ctx)
                {
                    return Some(p);
                }
                if extensions.contains(Extensions::DECLARATION)
                    && let Some(p) = self.dts_try_file(base, ".d.mts", ctx)
                {
                    return Some(p);
                }
                if extensions.contains(Extensions::JAVASCRIPT)
                    && let Some(p) = self.dts_try_file(base, ".mjs", ctx)
                {
                    return Some(p);
                }
            }
            ".cjs" | ".cts" | ".d.cts" => {
                if extensions.contains(Extensions::TYPESCRIPT)
                    && let Some(p) = self.dts_try_file(base, ".cts", ctx)
                {
                    return Some(p);
                }
                if extensions.contains(Extensions::DECLARATION)
                    && let Some(p) = self.dts_try_file(base, ".d.cts", ctx)
                {
                    return Some(p);
                }
                if extensions.contains(Extensions::JAVASCRIPT)
                    && let Some(p) = self.dts_try_file(base, ".cjs", ctx)
                {
                    return Some(p);
                }
            }
            ".json" => {
                if extensions.contains(Extensions::DECLARATION)
                    && let Some(p) = self.dts_try_file(base, ".d.json.ts", ctx)
                {
                    return Some(p);
                }
            }
            ".tsx" | ".jsx" => {
                if extensions.contains(Extensions::TYPESCRIPT) {
                    if let Some(p) = self.dts_try_file(base, ".tsx", ctx) {
                        return Some(p);
                    }
                    if let Some(p) = self.dts_try_file(base, ".ts", ctx) {
                        return Some(p);
                    }
                }
                if extensions.contains(Extensions::DECLARATION)
                    && let Some(p) = self.dts_try_file(base, ".d.ts", ctx)
                {
                    return Some(p);
                }
                if extensions.contains(Extensions::JAVASCRIPT) {
                    if let Some(p) = self.dts_try_file(base, ".jsx", ctx) {
                        return Some(p);
                    }
                    if let Some(p) = self.dts_try_file(base, ".js", ctx) {
                        return Some(p);
                    }
                }
            }
            ".ts" | ".d.ts" | ".js" | "" => {
                if extensions.contains(Extensions::TYPESCRIPT) {
                    if let Some(p) = self.dts_try_file(base, ".ts", ctx) {
                        return Some(p);
                    }
                    if let Some(p) = self.dts_try_file(base, ".tsx", ctx) {
                        return Some(p);
                    }
                }
                if extensions.contains(Extensions::DECLARATION)
                    && let Some(p) = self.dts_try_file(base, ".d.ts", ctx)
                {
                    return Some(p);
                }
                if extensions.contains(Extensions::JAVASCRIPT) {
                    if let Some(p) = self.dts_try_file(base, ".js", ctx) {
                        return Some(p);
                    }
                    if let Some(p) = self.dts_try_file(base, ".jsx", ctx) {
                        return Some(p);
                    }
                }
            }
            // Unknown extensions like ".vue", ".svelte" -> try .d{ext}.ts
            other => {
                if extensions.contains(Extensions::DECLARATION) {
                    let d_ext = format!(".d{other}.ts");
                    if let Some(p) = self.dts_try_file(base, &d_ext, ctx) {
                        return Some(p);
                    }
                }
            }
        }
        None
    }

    fn dts_try_file(&self, base: &CachedPath, ext: &str, ctx: &mut Ctx) -> Option<CachedPath> {
        let candidate = base.add_extension(ext, &self.cache);
        if self.cache.is_file(&candidate, ctx) { Some(candidate) } else { None }
    }

    /// TS: `loadNodeModuleFromDirectoryWorker`
    fn dts_resolve_as_directory(
        &self,
        extensions: Extensions,
        candidate: &CachedPath,
        ctx: &mut Ctx,
    ) -> ResolveResult {
        if !self.cache.is_dir(candidate, ctx) {
            return Ok(None);
        }

        let pkg = self.cache.get_package_json(candidate, &self.options, ctx)?;
        let main_fields = ["main".to_string()];

        // Try typesVersions paths
        if let Some(ref pkg) = pkg
            && let Some(version_paths) = Self::dts_get_matching_version_paths(pkg)
        {
            let mut entry = if extensions.contains(Extensions::DECLARATION) {
                pkg.typings().or_else(|| pkg.types())
            } else {
                None
            };
            if entry.is_none()
                && extensions.intersects(
                    Extensions::TYPESCRIPT
                        .union(Extensions::JAVASCRIPT)
                        .union(Extensions::DECLARATION),
                )
            {
                entry = pkg.main_fields(&main_fields).next();
            }

            let vp_specifier = entry.unwrap_or("index");
            if let Some(path) = self.dts_resolve_via_version_paths(
                extensions,
                vp_specifier,
                candidate,
                &version_paths,
                ctx,
            )? {
                return Ok(Some(path));
            }
        }

        // Determine entry file (types/typings/main)
        if let Some(ref pkg) = pkg {
            let mut entry = if extensions.contains(Extensions::DECLARATION) {
                pkg.typings().or_else(|| pkg.types())
            } else {
                None
            };
            if entry.is_none()
                && extensions.intersects(
                    Extensions::TYPESCRIPT
                        .union(Extensions::JAVASCRIPT)
                        .union(Extensions::DECLARATION),
                )
            {
                entry = pkg.main_fields(&main_fields).next();
            }
            if let Some(entry_str) = entry {
                let entry_path = candidate.normalize_with(entry_str, &self.cache);
                if let Some(path) = self.dts_resolve_as_file(extensions, &entry_path, ctx) {
                    return Ok(Some(path));
                }
                if self.cache.is_dir(&entry_path, ctx) {
                    let index = entry_path.push("index", &self.cache);
                    if let Some(path) = self.dts_resolve_as_file(extensions, &index, ctx) {
                        return Ok(Some(path));
                    }
                }
            }
        }

        // Fallback: index
        let index = candidate.push("index", &self.cache);
        Ok(self.dts_resolve_as_file(extensions, &index, ctx))
    }

    // -------- node_modules resolution (TWO-PASS) --------

    fn dts_resolve_node_modules(
        &self,
        extensions: Extensions,
        specifier: &str,
        package_name: &str,
        rest: &str,
        directory: &CachedPath,
        ctx: &mut Ctx,
    ) -> ResolveResult {
        let priority_exts = extensions & Extensions::TYPESCRIPT.union(Extensions::DECLARATION);
        let secondary_exts =
            extensions.difference(Extensions::TYPESCRIPT.union(Extensions::DECLARATION));

        // PASS 1: Walk ALL ancestors for TS/DTS + @types
        if !priority_exts.is_empty() {
            for ancestor in
                std::iter::successors(Some(directory.clone()), |cp| cp.parent(&self.cache))
            {
                let nm = ancestor.push("node_modules", &self.cache);
                if !self.cache.is_dir(&nm, ctx) {
                    continue;
                }

                // Try implementation package
                if let Some(path) =
                    self.dts_resolve_in_node_modules_dir(priority_exts, specifier, &nm, ctx)?
                {
                    return Ok(Some(path));
                }

                // Try @types
                if priority_exts.contains(Extensions::DECLARATION) {
                    let mangled = Self::dts_mangle_scoped_name(package_name);
                    let at_types_dir = nm.push("@types", &self.cache);
                    if self.cache.is_dir(&at_types_dir, ctx) {
                        let at_types_specifier = if rest.is_empty() {
                            mangled.clone()
                        } else {
                            format!("{mangled}{rest}")
                        };
                        if let Some(path) = self.dts_resolve_in_node_modules_dir(
                            Extensions::DECLARATION,
                            &at_types_specifier,
                            &at_types_dir,
                            ctx,
                        )? {
                            return Ok(Some(path));
                        }
                    }
                }
            }
        }

        // PASS 2: Walk ALL ancestors for JS only (no @types)
        if !secondary_exts.is_empty() {
            for ancestor in
                std::iter::successors(Some(directory.clone()), |cp| cp.parent(&self.cache))
            {
                let nm = ancestor.push("node_modules", &self.cache);
                if !self.cache.is_dir(&nm, ctx) {
                    continue;
                }
                if let Some(path) =
                    self.dts_resolve_in_node_modules_dir(secondary_exts, specifier, &nm, ctx)?
                {
                    return Ok(Some(path));
                }
            }
        }

        Ok(None)
    }

    fn dts_resolve_in_node_modules_dir(
        &self,
        extensions: Extensions,
        specifier: &str,
        nm_dir: &CachedPath,
        ctx: &mut Ctx,
    ) -> ResolveResult {
        let (package_name, rest) = Self::parse_package_specifier(specifier);
        let pkg_dir = nm_dir.normalize_with(package_name, &self.cache);

        if !self.cache.is_dir(&pkg_dir, ctx) {
            return Ok(None);
        }

        let pkg = self.cache.get_package_json(&pkg_dir, &self.options, ctx)?;

        // PRIORITY 1: exports (blocks everything else when present)
        if let Some(ref pkg) = pkg
            && pkg.exports().is_some()
        {
            let subpath = if rest.is_empty() { ".".to_string() } else { format!(".{rest}") };

            for exports in pkg.exports_fields(&self.options.exports_fields) {
                if let Ok(Some(path)) =
                    self.package_exports_resolve(&pkg_dir, &subpath, &exports, None, ctx)
                {
                    // Try to resolve the ESM match (file may need extension)
                    if let Some(resolved) = self.dts_resolve_esm_match(&path, ctx) {
                        return Ok(Some(resolved));
                    }
                    return Ok(Some(path));
                }
            }
            // exports blocks types/typings/main
            return Ok(None);
        }

        // PRIORITY 2: typesVersions (for subpath imports: rest != "")
        if !rest.is_empty()
            && let Some(ref pkg) = pkg
            && let Some(version_paths) = Self::dts_get_matching_version_paths(pkg)
        {
            let rest_without_slash = rest.strip_prefix('/').unwrap_or(rest);
            if let Some(path) = self.dts_resolve_via_version_paths(
                extensions,
                rest_without_slash,
                &pkg_dir,
                &version_paths,
                ctx,
            )? {
                return Ok(Some(path));
            }
        }

        // PRIORITY 3: standard file + directory
        if !rest.is_empty() {
            let candidate = nm_dir.normalize_with(specifier, &self.cache);
            if let Some(path) = self.dts_resolve_as_file(extensions, &candidate, ctx) {
                return Ok(Some(path));
            }
            if self.cache.is_dir(&candidate, ctx) {
                return self.dts_resolve_as_directory(extensions, &candidate, ctx);
            }
        }

        self.dts_resolve_as_directory(extensions, &pkg_dir, ctx)
    }

    fn dts_resolve_esm_match(&self, cached_path: &CachedPath, ctx: &mut Ctx) -> Option<CachedPath> {
        if self.cache.is_file(cached_path, ctx) {
            return Some(cached_path.clone());
        }
        // Try as file with TS extensions
        let extensions =
            Extensions::TYPESCRIPT.union(Extensions::DECLARATION).union(Extensions::JAVASCRIPT);
        self.dts_resolve_as_file(extensions, cached_path, ctx)
    }

    // -------- @types name mangling --------

    pub(crate) fn dts_mangle_scoped_name(name: &str) -> String {
        name.strip_prefix('@').map_or_else(|| name.to_string(), |rest| rest.replacen('/', "__", 1))
    }

    // -------- typesVersions --------

    /// Get the first matching version path from typesVersions.
    ///
    /// TypeScript matches `*` (wildcard) as "any version", which is the common case.
    /// For simplicity, we match all version ranges.
    fn dts_get_matching_version_paths(pkg: &PackageJson) -> Option<Vec<(String, Vec<String>)>> {
        let types_versions = pkg.types_versions()?;

        // TypeScript iterates versions and picks the first matching one.
        // The `*` key matches all versions, which is the overwhelmingly common case.
        for (_version_range, paths_value) in types_versions.iter() {
            if let Some(map) = paths_value.as_map() {
                let mut result = Vec::new();
                for (pattern, targets_entry) in map.iter() {
                    let targets: Vec<String> = if let Some(arr) = targets_entry.as_array() {
                        arr.iter().filter_map(|v| v.as_string().map(String::from)).collect()
                    } else if let Some(s) = targets_entry.as_string() {
                        vec![s.to_string()]
                    } else {
                        continue;
                    };
                    result.push((pattern.to_string(), targets));
                }
                if !result.is_empty() {
                    return Some(result);
                }
            }
        }
        None
    }

    /// Resolve a specifier against typesVersions path mappings.
    fn dts_resolve_via_version_paths(
        &self,
        extensions: Extensions,
        specifier: &str,
        base_dir: &CachedPath,
        version_paths: &[(String, Vec<String>)],
        ctx: &mut Ctx,
    ) -> ResolveResult {
        for (pattern, targets) in version_paths {
            if let Some(matched) = Self::dts_match_pattern(pattern, specifier) {
                for target in targets {
                    let resolved_target = target.replace('*', &matched);
                    let candidate = base_dir.normalize_with(&resolved_target, &self.cache);
                    if let Some(path) = self.dts_resolve_as_file(extensions, &candidate, ctx) {
                        return Ok(Some(path));
                    }
                    if self.cache.is_dir(&candidate, ctx)
                        && let Some(path) =
                            self.dts_resolve_as_directory(extensions, &candidate, ctx)?
                    {
                        return Ok(Some(path));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Match a specifier against a pattern with optional `*` wildcard.
    fn dts_match_pattern<'a>(pattern: &str, specifier: &'a str) -> Option<Cow<'a, str>> {
        if let Some((prefix, suffix)) = pattern.split_once('*') {
            if specifier.starts_with(prefix)
                && (suffix.is_empty() || specifier.ends_with(suffix))
                && specifier.len() >= prefix.len() + suffix.len()
            {
                let matched = &specifier[prefix.len()..specifier.len() - suffix.len()];
                Some(Cow::Borrowed(matched))
            } else {
                None
            }
        } else if pattern == specifier {
            Some(Cow::Borrowed(""))
        } else {
            None
        }
    }

    // -------- tsconfig paths --------

    fn dts_resolve_tsconfig_paths(
        &self,
        _cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut Ctx,
    ) -> ResolveResult {
        // Reuse the existing tsconfig resolution
        let tsconfig = match &self.options.tsconfig {
            Some(crate::TsconfigDiscovery::Manual(o)) => self.find_tsconfig_manual(o)?,
            _ => None,
        };

        let Some(tsconfig) = tsconfig.as_deref() else {
            return Ok(None);
        };

        // Resolve path aliases
        let paths = tsconfig.resolve_path_alias(specifier);
        let extensions =
            Extensions::TYPESCRIPT.union(Extensions::DECLARATION).union(Extensions::JAVASCRIPT);
        for path in paths {
            let resolved_path = self.cache.value(&path);
            if let Some(result) = self.dts_resolve_relative(extensions, &resolved_path, ctx)? {
                return Ok(Some(result));
            }
        }

        // Try baseUrl
        if let Some(path) = tsconfig.resolve_base_url(specifier) {
            let resolved_path = self.cache.value(&path);
            if let Some(result) = self.dts_resolve_relative(extensions, &resolved_path, ctx)? {
                return Ok(Some(result));
            }
        }

        Ok(None)
    }

    // -------- Package imports (#) --------

    fn dts_resolve_package_imports(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut Ctx,
    ) -> ResolveResult {
        self.load_package_imports(cached_path, specifier, None, ctx)
    }

    // -------- Package self --------

    fn dts_resolve_package_self(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut Ctx,
    ) -> ResolveResult {
        self.load_package_self(cached_path, specifier, None, ctx)
    }
}
