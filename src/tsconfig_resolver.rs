use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    CachedPath, Ctx, ResolveError, ResolveOptions, ResolveResult, ResolverImpl, Specifier,
    SpecifierError, TsConfig, TsconfigDiscovery, TsconfigOptions, TsconfigReferences,
    path::{PathUtil, is_path_relative},
};

/// A non-fatal problem encountered while loading a config from `extends`.
#[derive(Debug, Clone, PartialEq)]
pub struct TsconfigDiagnostic {
    /// The config containing the failing `extends` entry.
    pub config_path: PathBuf,

    /// The error raised while resolving or loading the extended config.
    pub error: ResolveError,
}

/// A loaded tsconfig together with diagnostics and files needed to invalidate it.
#[derive(Debug, Clone)]
pub struct TsconfigLoad {
    /// The usable config, including every base config that loaded successfully.
    pub config: Arc<TsConfig>,

    /// Non-fatal errors from missing or malformed extended configs.
    pub diagnostics: Arc<[TsconfigDiagnostic]>,

    /// Config files read while producing [`Self::config`].
    pub file_dependencies: Arc<[PathBuf]>,

    /// Config files or resolution candidates that were not found.
    pub missing_dependencies: Arc<[PathBuf]>,
}

impl TsconfigLoad {
    pub(crate) fn from_context(config: Arc<TsConfig>, context: TsconfigLoadContext) -> Self {
        Self {
            config,
            diagnostics: context.diagnostics.into(),
            file_dependencies: context.file_dependencies.into(),
            missing_dependencies: context.missing_dependencies.into(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TsconfigLoadContext {
    diagnostics: Vec<TsconfigDiagnostic>,
    file_dependencies: Vec<PathBuf>,
    missing_dependencies: Vec<PathBuf>,
}

impl TsconfigLoadContext {
    pub fn add_file_dependency(&mut self, path: &Path) {
        push_unique(&mut self.file_dependencies, path);
    }

    fn add_missing_dependency(&mut self, path: &Path) {
        push_unique(&mut self.missing_dependencies, path);
    }

    fn add_diagnostic(&mut self, config_path: &Path, error: ResolveError) {
        let diagnostic = TsconfigDiagnostic { config_path: config_path.to_path_buf(), error };
        if !self.diagnostics.contains(&diagnostic) {
            self.diagnostics.push(diagnostic);
        }
    }

    fn extend_load(&mut self, load: &TsconfigLoad) {
        for diagnostic in load.diagnostics.iter().cloned() {
            if !self.diagnostics.contains(&diagnostic) {
                self.diagnostics.push(diagnostic);
            }
        }
        for dependency in load.file_dependencies.iter() {
            self.add_file_dependency(dependency);
        }
        for dependency in load.missing_dependencies.iter() {
            self.add_missing_dependency(dependency);
        }
    }

    fn extend_resolve_context(&mut self, context: &Ctx) {
        if let Some(dependencies) = &context.file_dependencies {
            for dependency in dependencies {
                self.add_file_dependency(dependency);
            }
        }
        if let Some(dependencies) = &context.missing_dependencies {
            for dependency in dependencies {
                self.add_missing_dependency(dependency);
            }
        }
    }
}

fn push_unique(paths: &mut Vec<PathBuf>, path: &Path) {
    if !paths.iter().any(|candidate| candidate == path) {
        paths.push(path.to_path_buf());
    }
}

#[derive(Default)]
pub struct TsconfigResolveContext {
    extended_configs: Vec<PathBuf>,
    referenced_configs: Vec<PathBuf>,
}

impl TsconfigResolveContext {
    pub fn with_extended_file<R, T: FnOnce(&mut Self) -> R>(&mut self, path: PathBuf, cb: T) -> R {
        self.extended_configs.push(path);
        let result = cb(self);
        self.extended_configs.pop();
        result
    }

    pub fn is_already_extended(&self, path: &Path) -> bool {
        self.extended_configs.iter().any(|config| config == path)
    }

    pub fn get_extended_configs_with(&self, path: PathBuf) -> Vec<PathBuf> {
        let mut new_vec = Vec::with_capacity(self.extended_configs.len() + 1);
        new_vec.extend_from_slice(&self.extended_configs);
        new_vec.push(path);
        new_vec
    }

    pub fn with_referenced_file<R, T: FnOnce(&mut Self) -> R>(
        &mut self,
        path: PathBuf,
        cb: T,
    ) -> R {
        self.referenced_configs.push(path);
        let result = cb(self);
        self.referenced_configs.pop();
        result
    }

    pub fn is_already_referenced(&self, path: &Path) -> bool {
        self.referenced_configs.iter().any(|config| config == path)
    }

    pub fn is_direct_self_reference(&self, path: &Path) -> bool {
        self.referenced_configs.last().is_some_and(|config| config == path)
    }

    pub fn get_referenced_configs_with(&self, path: PathBuf) -> Vec<PathBuf> {
        let mut configs = Vec::with_capacity(self.referenced_configs.len() + 1);
        configs.extend_from_slice(&self.referenced_configs);
        configs.push(path);
        configs
    }
}

impl ResolverImpl {
    /// Finds the `tsconfig` to which this `path` belongs.
    ///
    /// If the `path` is inside `node_modules`, this function always returns `None`.
    ///
    /// Algorithm:
    ///
    /// 1. Search for `tsconfig.json` in ancestor directories.
    /// 2. Search its project-reference graph breadth-first for a config that owns the path.
    /// 3. If neither the config nor its graph owns the path, continue with the next ancestor.
    /// 4. [`TsconfigDiscovery::AutoNearest`] instead returns the first config as a compatibility
    ///    fallback, bounding discovery at the nearest project root.
    ///
    /// # Errors
    ///
    /// * Returns an error if the tsconfig is invalid, including any extended or referenced tsconfigs.
    pub fn find_tsconfig<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Option<Arc<TsConfig>>, ResolveError> {
        let path = path.as_ref().to_string_lossy();
        let specifier = Specifier::parse(path.as_ref()).map_err(ResolveError::Specifier)?;
        let path = Path::new(specifier.path());
        let cached_path = self.cache.value(path);
        self.find_tsconfig_tracing(&cached_path)
    }

    fn find_tsconfig_tracing(
        &self,
        cached_path: &CachedPath,
    ) -> Result<Option<Arc<TsConfig>>, ResolveError> {
        // Don't discover tsconfig for paths inside node_modules
        if cached_path.inside_node_modules() {
            return Ok(None);
        }
        // Skip non-absolute paths (e.g. virtual modules)
        if !cached_path.path.is_absolute() {
            return Ok(None);
        }
        let span = tracing::debug_span!("find_tsconfig", path = %cached_path);
        let _enter = span.enter();
        cached_path
            .resolved_tsconfig
            .get_or_try_init(|| {
                self.find_tsconfig_impl(cached_path).map(|option_tsconfig| {
                    option_tsconfig.map(|tsconfig| {
                        let r = TsConfig::resolve_tsconfig_solution(tsconfig, cached_path.path());
                        tracing::debug!(path = %cached_path, ret = ?r);
                        r
                    })
                })
            })
            .cloned()
    }

    /// Find tsconfig.json of a path by traversing parent directories.
    ///
    /// # Errors
    ///
    /// * [ResolveError::TsconfigLoadFailed]
    fn find_tsconfig_impl(
        &self,
        cached_path: &CachedPath,
    ) -> Result<Option<Arc<TsConfig>>, ResolveError> {
        match &self.options.tsconfig {
            None => Ok(None),
            Some(TsconfigDiscovery::Auto) => self.find_tsconfig_auto(cached_path, false),
            Some(TsconfigDiscovery::AutoNearest) => self.find_tsconfig_auto(cached_path, true),
            Some(TsconfigDiscovery::Manual(o)) => self.find_tsconfig_manual(o),
        }
    }

    fn find_tsconfig_auto(
        &self,
        cached_path: &CachedPath,
        nearest_fallback: bool,
    ) -> Result<Option<Arc<TsConfig>>, ResolveError> {
        let mut ctx = Ctx::default();
        let mut cache_value = Some(cached_path.clone());
        while let Some(cv) = cache_value {
            if let Some(tsconfig) = cv.tsconfig.get_or_try_init(|| {
                let tsconfig_path = cv.path.join("tsconfig.json");
                let tsconfig_path = self.cache.value(&tsconfig_path);
                if self.is_file(&tsconfig_path, &mut ctx) {
                    match self.resolve_tsconfig(tsconfig_path.path()) {
                        Ok(tsconfig) => Ok(Some(tsconfig)),
                        // Skip unreadable tsconfig files (e.g. permission denied)
                        // and continue walking parent directories
                        Err(ResolveError::TsconfigLoadFailed { ref source, .. })
                            if matches!(source.as_ref(), ResolveError::IOError(_)) =>
                        {
                            Ok(None)
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    Ok(None)
                }
            })? {
                // Return the nearest tsconfig that owns the file (directly via
                // `files`/`include`/`exclude`, or via a matching reference);
                // otherwise keep walking up to an ancestor that does.
                if tsconfig.claims_ownership_of(cached_path.path()) {
                    return Ok(Some(Arc::clone(tsconfig)));
                }
                if nearest_fallback {
                    return Ok(Some(Arc::clone(tsconfig)));
                }
            }
            cache_value = cv.parent(&self.cache);
        }
        // No tsconfig owns the file. Both `tsserver` and `typescript-go` leave
        // such a file in an inferred project (no `paths`/`baseUrl`), rather than
        // applying an unrelated ancestor's `compilerOptions`, so return `None`.
        Ok(None)
    }

    /// The manually configured tsconfig ([`TsconfigDiscovery::Manual`]); `Auto` discovery is
    /// deliberately skipped.
    pub(crate) fn manual_tsconfig(&self) -> Result<Option<Arc<TsConfig>>, ResolveError> {
        match &self.options.tsconfig {
            Some(TsconfigDiscovery::Manual(o)) => self.find_tsconfig_manual(o),
            _ => Ok(None),
        }
    }

    fn find_tsconfig_manual(
        &self,
        tsconfig_options: &TsconfigOptions,
    ) -> Result<Option<Arc<TsConfig>>, ResolveError> {
        // Cache the loaded tsconfig in /
        self.cache
            .value(Path::new("/"))
            .tsconfig
            .get_or_try_init(|| {
                let mut ctx = TsconfigResolveContext::default();
                self.load_tsconfig(
                    true,
                    &tsconfig_options.config_file,
                    tsconfig_options.references,
                    &mut ctx,
                )
                .map(|load| Some(Arc::clone(&load.config)))
            })
            .cloned()
    }

    /// Resolve `tsconfig`.
    ///
    /// The path can be:
    ///
    /// * Path to a file with `.json` extension.
    /// * Path to a file without `.json` extension, `.json` will be appended to filename.
    /// * Path to a directory, where the filename is defaulted to `tsconfig.json`
    ///
    /// # Errors
    ///
    /// * See [ResolveError]
    pub fn resolve_tsconfig<P: AsRef<Path>>(&self, path: P) -> Result<Arc<TsConfig>, ResolveError> {
        self.resolve_tsconfig_with_context(path).map(|load| load.config)
    }

    /// Resolve `tsconfig`, retaining non-fatal `extends` diagnostics and watch dependencies.
    ///
    /// A missing or malformed root config remains an error. Missing and malformed configs loaded
    /// through `extends` are reported in [`TsconfigLoad::diagnostics`], while the successfully
    /// parsed local options and bases remain available in [`TsconfigLoad::config`].
    ///
    /// # Errors
    ///
    /// * See [ResolveError]
    pub fn resolve_tsconfig_with_context<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<TsconfigLoad, ResolveError> {
        let path = path.as_ref();
        let references = match &self.options.tsconfig {
            Some(TsconfigDiscovery::Manual(o)) => o.references,
            Some(TsconfigDiscovery::Auto | TsconfigDiscovery::AutoNearest) => {
                TsconfigReferences::Auto
            }
            None => TsconfigReferences::Disabled,
        };
        self.load_tsconfig(true, path, references, &mut TsconfigResolveContext::default())
            .map(|load| load.as_ref().clone())
    }

    fn load_tsconfig(
        &self,
        root: bool,
        path: &Path,
        references: TsconfigReferences,
        ctx: &mut TsconfigResolveContext,
    ) -> Result<Arc<TsconfigLoad>, ResolveError> {
        self.cache.get_tsconfig(root, path, |tsconfig, load_context| {
            let directory = self.cache.value(tsconfig.directory());
            tracing::trace!(tsconfig = ?tsconfig, "load_tsconfig");

            if ctx.is_already_extended(tsconfig.path()) {
                return Err(ResolveError::TsconfigCircularExtend(
                    ctx.get_extended_configs_with(tsconfig.path().to_path_buf()).into(),
                ));
            }
            if root && ctx.is_already_referenced(tsconfig.path()) {
                if ctx.is_direct_self_reference(tsconfig.path()) {
                    return Err(ResolveError::TsconfigSelfReference(tsconfig.path().to_path_buf()));
                }
                return Err(ResolveError::TsconfigCircularReference(
                    ctx.get_referenced_configs_with(tsconfig.path().to_path_buf()).into(),
                ));
            }

            self.extend_tsconfig(&directory, tsconfig, ctx, load_context)?;

            if tsconfig.load_references(references) {
                let path = tsconfig.path().to_path_buf();
                let directory = tsconfig.directory().to_path_buf();
                let reference_paths = tsconfig
                    .references
                    .iter()
                    .map(|reference| directory.normalize_with(&reference.path))
                    .collect::<Vec<_>>();
                ctx.with_referenced_file(path, |ctx| {
                    for reference_path in reference_paths {
                        let referenced_tsconfig = self.load_tsconfig(
                            /* root */ true,
                            &reference_path,
                            TsconfigReferences::Auto,
                            ctx,
                        )?;
                        load_context.extend_load(&referenced_tsconfig);
                        tsconfig.references_resolved.push(Arc::clone(&referenced_tsconfig.config));
                    }
                    Ok::<(), ResolveError>(())
                })?;
            }
            Ok(())
        })
    }

    fn extend_tsconfig(
        &self,
        directory: &CachedPath,
        tsconfig: &mut TsConfig,
        ctx: &mut TsconfigResolveContext,
        load_context: &mut TsconfigLoadContext,
    ) -> Result<(), ResolveError> {
        let mut extended_tsconfig_paths = Vec::new();
        for specifier in tsconfig.extends() {
            let mut resolve_context = Ctx::default();
            resolve_context.init_file_dependencies();
            let result = self.get_extended_tsconfig_path(
                directory,
                tsconfig,
                specifier,
                &mut resolve_context,
            );
            load_context.extend_resolve_context(&resolve_context);
            match result {
                Ok(path) => extended_tsconfig_paths.push(path),
                Err(error)
                    if Self::recover_extended_tsconfig_error(tsconfig, load_context, &error) => {}
                Err(error) => return Err(error),
            }
        }

        // Iterate in reverse so that later `extends` entries take precedence —
        // see comment in `load_tsconfig`.
        ctx.with_extended_file(tsconfig.path().to_owned(), |ctx| {
            for extended_tsconfig_path in extended_tsconfig_paths.into_iter().rev() {
                match self.load_tsconfig(
                    /* root */ false,
                    &extended_tsconfig_path,
                    TsconfigReferences::Disabled,
                    ctx,
                ) {
                    Ok(extended_tsconfig) => {
                        load_context.extend_load(&extended_tsconfig);
                        tsconfig.extend_tsconfig(&extended_tsconfig.config);
                    }
                    Err(error)
                        if Self::recover_extended_tsconfig_error(
                            tsconfig,
                            load_context,
                            &error,
                        ) => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(())
        })
    }

    fn recover_extended_tsconfig_error(
        tsconfig: &TsConfig,
        load_context: &mut TsconfigLoadContext,
        error: &ResolveError,
    ) -> bool {
        match error {
            ResolveError::TsconfigNotFound(path) => {
                load_context.add_missing_dependency(path);
            }
            ResolveError::TsconfigLoadFailed { path, source }
                if matches!(source.as_ref(), ResolveError::Json(_)) =>
            {
                load_context.add_file_dependency(path);
            }
            _ => return false,
        }
        load_context.add_diagnostic(tsconfig.path(), error.clone());
        true
    }

    /// Resolves
    /// * `compilerOptions.paths`
    /// * `compilerOptions.rootDirs` (if specifier is relative)
    /// * `compilerOptions.baseUrl` (if specifier is non-relative)
    // <https://github.com/microsoft/TypeScript/blob/v5.9.3/src/compiler/moduleNameResolver.ts#L1550>
    pub(crate) fn resolve_tsconfig_compiler_options(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        tsconfig: Option<&TsConfig>,
        ctx: &mut Ctx,
    ) -> ResolveResult {
        if cached_path.inside_node_modules() {
            return Ok(None);
        }
        let Some(tsconfig) = tsconfig else { return Ok(None) };
        let paths = match &self.options.tsconfig {
            // Do not resolve against project references because its already resolved during
            // initialization phase.
            Some(TsconfigDiscovery::Auto | TsconfigDiscovery::AutoNearest) => {
                tsconfig.resolve_path_alias(specifier)
            }
            Some(TsconfigDiscovery::Manual(o))
                if matches!(o.references, TsconfigReferences::Disabled) =>
            {
                tsconfig.resolve_path_alias(specifier)
            }
            Some(TsconfigDiscovery::Manual(o))
                if matches!(o.references, TsconfigReferences::Auto) =>
            {
                if ctx.resolve_file {
                    // This is the solution tsconfig, resolve directly.
                    tsconfig.resolve_path_alias(specifier)
                } else {
                    // This is the manually provided tsconfig, resolve against project references..
                    tsconfig.resolve_references_then_self_paths(cached_path.path(), specifier)
                }
            }
            None | Some(TsconfigDiscovery::Manual(_)) => return Ok(None),
        };
        for path in paths {
            let resolved_path = self.cache.value(&path);
            if let Some(resolution) =
                self.load_as_file_or_directory(&resolved_path, ".", Some(tsconfig), ctx)?
            {
                return Ok(Some(resolution));
            }
        }
        if is_path_relative(specifier) {
            if let Some(path) =
                self.load_tsconfig_root_dirs(cached_path, specifier, tsconfig, ctx)?
            {
                return Ok(Some(path));
            }
        } else if let Some(path) = tsconfig.resolve_base_url(specifier) {
            let resolved_path = self.cache.value(&path);
            if let Some(resolution) =
                self.load_as_file_or_directory(&resolved_path, ".", Some(tsconfig), ctx)?
            {
                return Ok(Some(resolution));
            }
        }
        Ok(None)
    }

    pub(crate) fn load_tsconfig_root_dirs(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        tsconfig: &TsConfig,
        ctx: &mut Ctx,
    ) -> ResolveResult {
        debug_assert!(is_path_relative(specifier));
        debug_assert!(!cached_path.inside_node_modules());
        let Some(root_dirs) = &tsconfig.compiler_options.root_dirs else { return Ok(None) };

        // Use the containing directory, not the file itself
        let containing_directory = if self.is_dir(cached_path, ctx) {
            cached_path.clone()
        } else {
            cached_path.parent(&self.cache).unwrap_or_else(|| cached_path.clone())
        };
        let candidate = containing_directory.normalize_with(specifier, &self.cache);
        let mut matched_root_dir: Option<PathBuf> = None;
        for root_dir in root_dirs {
            let is_longest_matching_prefix = candidate.path().starts_with(root_dir)
                && matched_root_dir
                    .as_ref()
                    .is_none_or(|prefix| prefix.as_os_str().len() < root_dir.as_os_str().len());
            if is_longest_matching_prefix {
                matched_root_dir.replace(root_dir.clone());
            }
        }
        let Some(matched_root_dir) = matched_root_dir else {
            return Ok(None);
        };
        if let Some(p) = self.load_as_file_or_directory(&candidate, ".", Some(tsconfig), ctx)? {
            return Ok(Some(p));
        }
        // Defensive: This should never fail because we already verified the prefix match,
        // but we handle it gracefully in case of unexpected path normalization edge cases.
        let Ok(suffix) = candidate.path().strip_prefix(&matched_root_dir) else {
            return Ok(None);
        };
        for root_dir in root_dirs {
            if *root_dir == matched_root_dir {
                continue;
            }
            let candidate = root_dir.normalize_with(suffix);
            let cached_candidate = self.cache.value(&candidate);
            if let Some(resolved) =
                self.load_as_file_or_directory(&cached_candidate, ".", Some(tsconfig), ctx)?
            {
                return Ok(Some(resolved));
            }
        }
        Ok(None)
    }

    /// Lookup configuration shared by tsconfig `extends` targets that resolve
    /// through `node_modules`: bare package specifiers (via the package
    /// `exports` field) and `#`-prefixed subpath imports (via the package
    /// `imports` field). Both use the same conditions so that, for example,
    /// `extends: "pkg"` and `extends: "#pkg"` agree on which condition wins.
    fn tsconfig_extends_resolver(&self) -> Self {
        let options = ResolveOptions {
            tsconfig: None,
            condition_names: vec!["node".into(), "import".into()],
            extensions: vec![".json".into()],
            main_files: vec!["tsconfig".into()],
            #[cfg(feature = "yarn_pnp")]
            yarn_pnp: self.options.yarn_pnp,
            #[cfg(feature = "yarn_pnp")]
            cwd: self.options.cwd.clone(),
            ..ResolveOptions::default()
        }
        .sanitize();
        let alias = crate::alias::compile_alias(&options.alias);
        let fallback = crate::alias::compile_alias(&options.fallback);
        // Extends-resolution never toggles `yarn_pnp`, so reuse the same cache (and thus the
        // same underlying filesystem) rather than rebuilding it.
        Self { options, cache: Arc::clone(&self.cache), alias, fallback }
    }

    fn get_extended_tsconfig_path(
        &self,
        directory: &CachedPath,
        tsconfig: &TsConfig,
        specifier: &str,
        ctx: &mut Ctx,
    ) -> Result<PathBuf, ResolveError> {
        match specifier.as_bytes().first() {
            None => Err(ResolveError::Specifier(SpecifierError::Empty(specifier.to_string()))),
            Some(b'/') => Ok(PathBuf::from(specifier)),
            Some(b'.') => Ok(tsconfig.directory().normalize_with(specifier)),
            // Node.js subpath imports, e.g. `extends: "#config"`, resolved
            // through the nearest `package.json` `imports` field — the same
            // path the resolver takes for `require("#config")`.
            Some(b'#') => {
                let resolved = self
                    .tsconfig_extends_resolver()
                    .load_package_imports(directory, specifier, Some(tsconfig), ctx)
                    .map_err(|err| match err {
                        ResolveError::PackageImportNotDefined(..) | ResolveError::NotFound(..) => {
                            ResolveError::TsconfigNotFound(PathBuf::from(specifier))
                        }
                        _ => err,
                    })?;
                resolved
                    .map(|p| p.path().to_path_buf())
                    .ok_or_else(|| ResolveError::TsconfigNotFound(PathBuf::from(specifier)))
            }
            _ => self
                .tsconfig_extends_resolver()
                .load_bare_package(directory, specifier, None, ctx)
                .map(|p| p.to_path_buf())
                .map_err(|err| match err {
                    ResolveError::NotFound(_) => {
                        ResolveError::TsconfigNotFound(PathBuf::from(specifier))
                    }
                    _ => err,
                }),
        }
    }
}
