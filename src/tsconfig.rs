use std::{
    borrow::Cow,
    collections::VecDeque,
    fmt::Debug,
    hash::BuildHasherDefault,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use compact_str::CompactString;
use indexmap::IndexMap;
use rustc_hash::{FxHashSet, FxHasher};
use serde::Deserialize;

use crate::{
    TsconfigReferences,
    path::{PathUtil, is_path_relative},
    replace_bom_with_whitespace,
};

/// Template variable `${configDir}` for substitution of config files
/// directory path.
///
/// NOTE: All tests cases are just a head replacement of `${configDir}`, so
///       we are constrained as such.
///
/// See <https://github.com/microsoft/TypeScript/pull/58042>.
/// Allow list: <https://github.com/microsoft/TypeScript/issues/57485#issuecomment-2027787456>
const TEMPLATE_VARIABLE: &str = "${configDir}";

pub type CompilerOptionsPathsMap = IndexMap<String, Vec<PathBuf>, BuildHasherDefault<FxHasher>>;

#[derive(Clone, Debug)]
struct ConfigField<T> {
    present: bool,
    value: Option<T>,
}

impl<T> Default for ConfigField<T> {
    fn default() -> Self {
        Self { present: false, value: None }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for ConfigField<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self { present: true, value: Option::<T>::deserialize(deserializer)? })
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct TsConfigPresence {
    files: bool,
    include: bool,
    exclude: bool,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTsConfig {
    #[serde(default)]
    files: ConfigField<Vec<PathBuf>>,
    #[serde(default)]
    include: ConfigField<Vec<PathBuf>>,
    #[serde(default)]
    exclude: ConfigField<Vec<PathBuf>>,
    #[serde(default)]
    extends: Option<ExtendsField>,
    #[serde(default)]
    compiler_options: CompilerOptions,
    #[serde(default)]
    references: Vec<ProjectReference>,
}

/// Project Reference
///
/// <https://www.typescriptlang.org/docs/handbook/project-references.html>
#[derive(Clone, Debug, Deserialize)]
pub struct ProjectReference {
    pub path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct TsConfig {
    /// Whether this is the caller tsconfig.
    /// `false` for configs loaded through `extends`.
    pub root: bool,

    /// Whether `build()` should normalize paths.
    /// Set to true when caching to ensure paths are always normalized regardless of `root`.
    should_build: bool,

    /// Path to `tsconfig.json`. Contains the `tsconfig.json` filename.
    pub path: PathBuf,

    pub files: Option<Vec<PathBuf>>,

    pub include: Option<Vec<PathBuf>>,

    pub exclude: Option<Vec<PathBuf>>,

    pub extends: Option<ExtendsField>,

    pub compiler_options: CompilerOptions,

    pub references: Vec<ProjectReference>,

    presence: TsConfigPresence,

    /// Resolved project references.
    ///
    /// Corresponds to each item in [TsConfig::references].
    pub references_resolved: Vec<Arc<Self>>,
}

impl<'de> Deserialize<'de> for TsConfig {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawTsConfig::deserialize(deserializer)?;
        Ok(Self {
            files: raw.files.value,
            include: raw.include.value,
            exclude: raw.exclude.value,
            extends: raw.extends,
            compiler_options: raw.compiler_options,
            references: raw.references,
            presence: TsConfigPresence {
                files: raw.files.present,
                include: raw.include.present,
                exclude: raw.exclude.present,
            },
            ..Self::default()
        })
    }
}

impl TsConfig {
    /// Parses the tsconfig from a JSON string.
    ///
    /// `path` is the requested path (used as identity for cache lookup and error
    /// reporting). `canonical_path` anchors `baseUrl` / `paths` at the real
    /// tsconfig directory when `extends` traverses a symlink. Callers that don't
    /// care about symlinks may pass the same value for both.
    ///
    /// # Errors
    ///
    /// * Any error that can be returned by `serde_json::from_str()`.
    ///
    /// # Panics
    ///
    /// * When `canonical_path` has no parent directory.
    pub fn parse(
        root: bool,
        path: &Path,
        canonical_path: &Path,
        json: String,
    ) -> Result<Self, serde_json::Error> {
        let mut json = json.into_bytes();
        replace_bom_with_whitespace(&mut json);
        _ = json_strip_comments::strip_slice(&mut json);
        let mut tsconfig: Self = if json.iter().all(u8::is_ascii_whitespace) {
            Self::default()
        } else {
            serde_json::from_slice(&json)?
        };
        tsconfig.root = root;
        tsconfig.path = path.to_path_buf();
        let canonical_directory = canonical_path.parent().unwrap();
        tsconfig.compiler_options.paths_base =
            tsconfig.compiler_options.base_url.as_ref().map_or_else(
                || canonical_directory.to_path_buf(),
                |base_url| {
                    if base_url.to_string_lossy().starts_with(TEMPLATE_VARIABLE) {
                        base_url.clone()
                    } else {
                        canonical_directory.normalize_with(base_url)
                    }
                },
            );
        if let Some(root_dirs) = &mut tsconfig.compiler_options.root_dirs {
            for root_dir in root_dirs.iter_mut() {
                if !root_dir.to_string_lossy().starts_with(TEMPLATE_VARIABLE) {
                    *root_dir = canonical_directory.normalize_with(&root_dir);
                }
            }
        }
        if let Some(out_dir) = &mut tsconfig.compiler_options.out_dir
            && !out_dir.to_string_lossy().starts_with(TEMPLATE_VARIABLE)
        {
            *out_dir = canonical_directory.normalize_with(&out_dir);
        }
        if let Some(declaration_dir) = &mut tsconfig.compiler_options.declaration_dir
            && !declaration_dir.to_string_lossy().starts_with(TEMPLATE_VARIABLE)
        {
            *declaration_dir = canonical_directory.normalize_with(&declaration_dir);
        }
        Ok(tsconfig)
    }

    /// Whether this is the caller tsconfig.
    /// `false` for configs loaded through `extends`.
    #[must_use]
    pub fn root(&self) -> bool {
        self.root
    }

    /// Whether `build()` should normalize paths.
    #[must_use]
    pub fn should_build(&self) -> bool {
        self.should_build
    }

    /// Set whether `build()` should normalize paths.
    pub fn set_should_build(&mut self, should_build: bool) {
        self.should_build = should_build;
    }

    /// Returns the path where the `tsconfig.json` was found.
    ///
    /// Contains the `tsconfig.json` filename.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Directory to `tsconfig.json`.
    ///
    /// # Panics
    ///
    /// * When the `tsconfig.json` path is misconfigured.
    #[must_use]
    pub fn directory(&self) -> &Path {
        debug_assert!(self.path.file_name().is_some());
        self.path.parent().unwrap()
    }

    /// Returns any paths to tsconfigs that should be extended by this tsconfig.
    pub(crate) fn extends(&self) -> impl Iterator<Item = &str> {
        let specifiers = match &self.extends {
            Some(ExtendsField::Single(specifier)) => {
                vec![specifier.as_str()]
            }
            Some(ExtendsField::Multiple(specifiers)) => {
                specifiers.iter().map(String::as_str).collect()
            }
            None => Vec::new(),
        };
        specifiers.into_iter()
    }

    /// Loads the given references into this tsconfig.
    ///
    /// Returns whether any references are defined in the tsconfig.
    pub(crate) fn load_references(&mut self, references: TsconfigReferences) -> bool {
        match references {
            TsconfigReferences::Disabled => {
                self.references.clear();
            }
            TsconfigReferences::Auto => {}
        }
        !self.references.is_empty()
    }

    /// Inherits fields that were not declared in `self` from `tsconfig`.
    ///
    /// Presence is tracked independently from the public `Option` values so an explicit `null`
    /// clears an inherited option. Callers apply multiple bases in reverse order, making the last
    /// entry in an `extends` array win exactly as it does in TypeScript.
    pub(crate) fn extend_tsconfig(&mut self, tsconfig: &Self) {
        self.inherit_file_patterns(tsconfig);

        let compiler_options = &mut self.compiler_options;
        let inherited_options = &tsconfig.compiler_options;

        macro_rules! inherit {
            ($field:ident, $flag:ident) => {
                if !compiler_options.presence.contains($flag)
                    && inherited_options.presence.contains($flag)
                {
                    compiler_options.$field.clone_from(&inherited_options.$field);
                    compiler_options.presence.insert($flag);
                }
            };
        }

        let had_base_url = compiler_options.presence.contains(BASE_URL);
        let had_paths = compiler_options.presence.contains(PATHS);
        inherit!(base_url, BASE_URL);
        if !had_base_url
            && inherited_options.presence.contains(BASE_URL)
            && inherited_options.base_url.is_some()
        {
            compiler_options.paths_base.clone_from(&inherited_options.paths_base);
        }
        inherit!(paths, PATHS);
        if !had_paths
            && inherited_options.presence.contains(PATHS)
            && compiler_options.base_url.is_none()
            && inherited_options.base_url.is_none()
        {
            compiler_options.paths_base.clone_from(&inherited_options.paths_base);
        }

        inherit!(experimental_decorators, EXPERIMENTAL_DECORATORS);
        inherit!(emit_decorator_metadata, EMIT_DECORATOR_METADATA);
        inherit!(strict, STRICT);
        inherit!(strict_null_checks, STRICT_NULL_CHECKS);
        inherit!(use_define_for_class_fields, USE_DEFINE_FOR_CLASS_FIELDS);
        inherit!(rewrite_relative_import_extensions, REWRITE_RELATIVE_IMPORT_EXTENSIONS);
        inherit!(jsx, JSX);
        inherit!(jsx_factory, JSX_FACTORY);
        inherit!(jsx_fragment_factory, JSX_FRAGMENT_FACTORY);
        inherit!(jsx_import_source, JSX_IMPORT_SOURCE);
        inherit!(verbatim_module_syntax, VERBATIM_MODULE_SYNTAX);
        inherit!(preserve_value_imports, PRESERVE_VALUE_IMPORTS);
        inherit!(imports_not_used_as_values, IMPORTS_NOT_USED_AS_VALUES);
        inherit!(target, TARGET);
        inherit!(module, MODULE);
        inherit!(allow_js, ALLOW_JS);
        inherit!(root_dirs, ROOT_DIRS);
        inherit!(out_dir, OUT_DIR);
        inherit!(declaration_dir, DECLARATION_DIR);
        inherit!(resolve_json_module, RESOLVE_JSON_MODULE);
        inherit!(check_js, CHECK_JS);
    }

    fn inherit_file_patterns(&mut self, tsconfig: &Self) {
        let inherited_directory = tsconfig.directory().to_path_buf();
        let directory = self.directory().to_path_buf();

        macro_rules! inherit {
            ($field:ident) => {
                if !self.presence.$field && tsconfig.presence.$field {
                    self.$field = tsconfig.$field.as_ref().map(|patterns| {
                        rebase_patterns(patterns, &inherited_directory, &directory)
                    });
                    self.presence.$field = true;
                }
            };
        }

        inherit!(files);
        inherit!(include);
        inherit!(exclude);
    }

    /// "Build" the root tsconfig, resolve:
    ///
    /// * `{configDir}` template variable
    /// * `paths_base` for resolving paths alias
    /// * `baseUrl` to absolute path
    #[must_use]
    pub(crate) fn build(mut self) -> Self {
        // Only build if should_build is true.
        // This is controlled separately from `root` to avoid cache pollution.
        if !self.should_build {
            return self;
        }

        let config_dir = self.directory().to_path_buf();

        // Inherited file patterns were rebased lexically to this config's directory while merging.
        if let Some(files) = self.files.take() {
            self.files = Some(files.into_iter().map(|path| self.adjust_path(path)).collect());
        }
        if let Some(include) = self.include.take() {
            self.include = Some(include.into_iter().map(|path| self.adjust_path(path)).collect());
        }
        if let Some(exclude) = self.exclude.take() {
            self.exclude = Some(exclude.into_iter().map(|path| self.adjust_path(path)).collect());
        }

        if let Some(base_url) = &self.compiler_options.base_url {
            self.compiler_options.base_url = Some(self.adjust_path(base_url.clone()));
        }

        if let Some(stripped_path) =
            self.compiler_options.paths_base.to_string_lossy().strip_prefix(TEMPLATE_VARIABLE)
        {
            self.compiler_options.paths_base =
                config_dir.join(stripped_path.trim_start_matches('/'));
        }

        if let Some(root_dirs) = &mut self.compiler_options.root_dirs {
            for root_dir in root_dirs.iter_mut() {
                if let Some(stripped_path) =
                    root_dir.to_string_lossy().strip_prefix(TEMPLATE_VARIABLE)
                {
                    *root_dir = config_dir.join(stripped_path.trim_start_matches('/'));
                }
            }
        }

        if let Some(out_dir) = &mut self.compiler_options.out_dir
            && let Some(stripped_path) = out_dir.to_string_lossy().strip_prefix(TEMPLATE_VARIABLE)
        {
            *out_dir = config_dir.join(stripped_path.trim_start_matches('/'));
        }
        if let Some(declaration_dir) = &mut self.compiler_options.declaration_dir
            && let Some(stripped_path) =
                declaration_dir.to_string_lossy().strip_prefix(TEMPLATE_VARIABLE)
        {
            *declaration_dir = config_dir.join(stripped_path.trim_start_matches('/'));
        }

        if let Some(paths_map) = &mut self.compiler_options.paths {
            // Substitute template variable in `tsconfig.compilerOptions.paths`.
            for paths in paths_map.values_mut() {
                for path in paths {
                    *path = if let Some(stripped_path) =
                        path.to_string_lossy().strip_prefix(TEMPLATE_VARIABLE)
                    {
                        config_dir.join(stripped_path.trim_start_matches('/'))
                    } else {
                        self.compiler_options.paths_base.normalize_with(&path)
                    };
                }
            }
            self.compiler_options.compiled_paths =
                Some(Arc::new(CompiledTsconfigPaths::new(paths_map)));
        } else {
            self.compiler_options.compiled_paths = None;
        }

        self
    }

    #[expect(clippy::option_if_let_else, reason = "the if/else reads clearer than `map_or_else`")]
    fn adjust_path(&self, path: PathBuf) -> PathBuf {
        if let Some(stripped) = path.to_string_lossy().strip_prefix(TEMPLATE_VARIABLE) {
            self.directory().join(stripped.trim_start_matches('/'))
        } else {
            self.directory().normalize_with(path)
        }
    }

    /// Resolves the given `specifier` within project references and then [CompilerOptions::paths].
    ///
    /// `specifier` can be either a real path or an alias.
    #[must_use]
    pub(crate) fn resolve_references_then_self_paths(
        &self,
        path: &Path,
        specifier: &str,
    ) -> Vec<PathBuf> {
        let mut queue = self.references_resolved.iter().cloned().collect::<VecDeque<_>>();
        let mut visited = FxHashSet::default();
        while let Some(config) = queue.pop_front() {
            if !visited.insert(config.path.clone()) {
                continue;
            }
            if path.starts_with(&config.compiler_options.paths_base) {
                return config.resolve_path_alias(specifier);
            }
            queue.extend(config.references_resolved.iter().cloned());
        }
        self.resolve_path_alias(specifier)
    }

    /// Resolves the given `specifier` within the project configured by this
    /// tsconfig.
    ///
    /// `specifier` is expected to be a path alias.
    // Copied from parcel
    // <https://github.com/parcel-bundler/parcel/blob/b6224fd519f95e68d8b93ba90376fd94c8b76e69/packages/utils/node-resolver-rs/src/tsconfig.rs#L93>
    #[must_use]
    pub(crate) fn resolve_path_alias(&self, specifier: &str) -> Vec<PathBuf> {
        if is_path_relative(specifier) {
            return Vec::new();
        }

        let compiler_options = &self.compiler_options;

        let Some(paths_map) = &compiler_options.paths else {
            return vec![];
        };

        if let Some(paths) = paths_map.get(specifier) {
            return paths.clone();
        }

        if let Some(compiled_paths) = &compiler_options.compiled_paths
            && let Some(paths) = compiled_paths.resolve(specifier)
        {
            return paths;
        }

        Vec::new()
    }

    pub(crate) fn resolve_base_url(&self, specifier: &str) -> Option<PathBuf> {
        self.compiler_options
            .base_url
            .is_some()
            .then(|| self.compiler_options.paths_base.normalize_with(specifier))
    }

    /// Maps a non-relative module `specifier` through this tsconfig's `compilerOptions.paths`
    /// and `baseUrl` configuration, returning the mapped absolute path candidates without
    /// checking whether they exist on disk.
    ///
    /// Unlike normal resolution (e.g. [`crate::ResolverImpl::resolve_file`]), which discards
    /// a `paths`/`baseUrl` mapping when the mapped path does not point at a real file, this
    /// keeps the mapping. It is intended for callers that need the alias mapping for a specifier
    /// that is not expected to resolve to a real file, such as glob patterns used by
    /// `import.meta.glob` (e.g. `@/foo/**/*`).
    ///
    /// Discover the tsconfig for an importing file with [`crate::ResolverImpl::find_tsconfig`],
    /// which resolves project references so the returned config is the one that owns the file.
    #[must_use]
    pub fn resolve_path_alias_or_base_url(&self, specifier: &str) -> Vec<PathBuf> {
        let mut paths = self.resolve_path_alias(specifier);
        if paths.is_empty()
            && let Some(base_url_path) = self.resolve_base_url(specifier)
        {
            paths.push(base_url_path);
        }
        paths
    }
}

fn rebase_patterns(patterns: &[PathBuf], from: &Path, to: &Path) -> Vec<PathBuf> {
    patterns
        .iter()
        .map(|pattern| {
            if pattern.is_absolute() || pattern.to_string_lossy().starts_with(TEMPLATE_VARIABLE) {
                return pattern.clone();
            }
            let target = from.normalize_with(pattern);
            relative_path(to, &target).unwrap_or(target)
        })
        .collect()
}

/// Returns `target` relative to `base` without resolving symlinks.
fn relative_path(base: &Path, target: &Path) -> Option<PathBuf> {
    let base = base.components().collect::<Vec<_>>();
    let target = target.components().collect::<Vec<_>>();
    let common = base.iter().zip(&target).take_while(|(left, right)| left == right).count();

    // Different roots or Windows drive prefixes cannot be expressed as a relative path.
    if common == 0
        || base[common..]
            .iter()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return None;
    }

    let mut relative = PathBuf::new();
    for component in &base[common..] {
        if matches!(component, Component::Normal(_)) {
            relative.push(Component::ParentDir);
        }
    }
    for component in &target[common..] {
        relative.push(component.as_os_str());
    }
    if relative.as_os_str().is_empty() {
        relative.push(Component::CurDir);
    }
    Some(relative)
}

#[derive(Clone, Copy, Debug, Default)]
struct CompilerOptionsPresence(u32);

impl CompilerOptionsPresence {
    fn contains(self, field: u32) -> bool {
        self.0 & field != 0
    }

    fn insert(&mut self, field: u32) {
        self.0 |= field;
    }
}

const BASE_URL: u32 = 1 << 0;
const PATHS: u32 = 1 << 1;
const EXPERIMENTAL_DECORATORS: u32 = 1 << 2;
const EMIT_DECORATOR_METADATA: u32 = 1 << 3;
const STRICT: u32 = 1 << 4;
const STRICT_NULL_CHECKS: u32 = 1 << 5;
const USE_DEFINE_FOR_CLASS_FIELDS: u32 = 1 << 6;
const REWRITE_RELATIVE_IMPORT_EXTENSIONS: u32 = 1 << 7;
const JSX: u32 = 1 << 8;
const JSX_FACTORY: u32 = 1 << 9;
const JSX_FRAGMENT_FACTORY: u32 = 1 << 10;
const JSX_IMPORT_SOURCE: u32 = 1 << 11;
const VERBATIM_MODULE_SYNTAX: u32 = 1 << 12;
const PRESERVE_VALUE_IMPORTS: u32 = 1 << 13;
const IMPORTS_NOT_USED_AS_VALUES: u32 = 1 << 14;
const TARGET: u32 = 1 << 15;
const MODULE: u32 = 1 << 16;
const ALLOW_JS: u32 = 1 << 17;
const ROOT_DIRS: u32 = 1 << 18;
const OUT_DIR: u32 = 1 << 19;
const DECLARATION_DIR: u32 = 1 << 20;
const RESOLVE_JSON_MODULE: u32 = 1 << 21;
const CHECK_JS: u32 = 1 << 22;

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCompilerOptions {
    #[serde(default)]
    base_url: ConfigField<PathBuf>,
    #[serde(default)]
    paths: ConfigField<CompilerOptionsPathsMap>,
    #[serde(default)]
    experimental_decorators: ConfigField<bool>,
    #[serde(default)]
    emit_decorator_metadata: ConfigField<bool>,
    #[serde(default)]
    strict: ConfigField<bool>,
    #[serde(default)]
    strict_null_checks: ConfigField<bool>,
    #[serde(default)]
    use_define_for_class_fields: ConfigField<bool>,
    #[serde(default)]
    rewrite_relative_import_extensions: ConfigField<bool>,
    #[serde(default)]
    jsx: ConfigField<String>,
    #[serde(default)]
    jsx_factory: ConfigField<String>,
    #[serde(default)]
    jsx_fragment_factory: ConfigField<String>,
    #[serde(default)]
    jsx_import_source: ConfigField<String>,
    #[serde(default)]
    verbatim_module_syntax: ConfigField<bool>,
    #[serde(default)]
    preserve_value_imports: ConfigField<bool>,
    #[serde(default)]
    imports_not_used_as_values: ConfigField<String>,
    #[serde(default)]
    target: ConfigField<String>,
    #[serde(default)]
    module: ConfigField<String>,
    #[serde(default)]
    allow_js: ConfigField<bool>,
    #[serde(default)]
    root_dirs: ConfigField<Vec<PathBuf>>,
    #[serde(default)]
    out_dir: ConfigField<PathBuf>,
    #[serde(default)]
    declaration_dir: ConfigField<PathBuf>,
    #[serde(default)]
    resolve_json_module: ConfigField<bool>,
    #[serde(default)]
    check_js: ConfigField<bool>,
}

/// Compiler Options
///
/// <https://www.typescriptlang.org/tsconfig#compilerOptions>
#[derive(Clone, Debug, Default)]
pub struct CompilerOptions {
    pub base_url: Option<PathBuf>,

    /// Path aliases.
    pub paths: Option<CompilerOptionsPathsMap>,

    /// Pre-compiled wildcard path aliases for faster runtime matching.
    compiled_paths: Option<Arc<CompiledTsconfigPaths>>,

    /// The "base_url" at which this tsconfig is defined.
    pub(crate) paths_base: PathBuf,

    presence: CompilerOptionsPresence,

    /// <https://www.typescriptlang.org/tsconfig/#experimentalDecorators>
    pub experimental_decorators: Option<bool>,

    /// <https://www.typescriptlang.org/tsconfig/#emitDecoratorMetadata>
    pub emit_decorator_metadata: Option<bool>,

    /// <https://www.typescriptlang.org/tsconfig/#strict>
    pub strict: Option<bool>,

    /// <https://www.typescriptlang.org/tsconfig/#strictNullChecks>
    pub strict_null_checks: Option<bool>,

    /// <https://www.typescriptlang.org/tsconfig/#useDefineForClassFields>
    pub use_define_for_class_fields: Option<bool>,

    /// <https://www.typescriptlang.org/tsconfig/#rewriteRelativeImportExtensions>
    pub rewrite_relative_import_extensions: Option<bool>,

    /// <https://www.typescriptlang.org/tsconfig/#jsx>
    pub jsx: Option<String>,

    /// <https://www.typescriptlang.org/tsconfig/#jsxFactory>
    pub jsx_factory: Option<String>,

    /// <https://www.typescriptlang.org/tsconfig/#jsxFragmentFactory>
    pub jsx_fragment_factory: Option<String>,

    /// <https://www.typescriptlang.org/tsconfig/#jsxImportSource>
    pub jsx_import_source: Option<String>,

    /// <https://www.typescriptlang.org/tsconfig/#verbatimModuleSyntax>
    pub verbatim_module_syntax: Option<bool>,

    /// <https://www.typescriptlang.org/tsconfig/#preserveValueImports>
    pub preserve_value_imports: Option<bool>,

    /// <https://www.typescriptlang.org/tsconfig/#importsNotUsedAsValues>
    pub imports_not_used_as_values: Option<String>,

    /// <https://www.typescriptlang.org/tsconfig/#target>
    pub target: Option<String>,

    /// <https://www.typescriptlang.org/tsconfig/#module>
    pub module: Option<String>,

    /// <https://www.typescriptlang.org/tsconfig/#allowJs>
    pub allow_js: Option<bool>,

    /// <https://www.typescriptlang.org/tsconfig/#rootDirs>
    pub root_dirs: Option<Vec<PathBuf>>,

    /// <https://www.typescriptlang.org/tsconfig/#outDir>
    pub out_dir: Option<PathBuf>,

    /// <https://www.typescriptlang.org/tsconfig/#declarationDir>
    pub declaration_dir: Option<PathBuf>,

    /// <https://www.typescriptlang.org/tsconfig/#resolveJsonModule>
    pub resolve_json_module: Option<bool>,

    /// <https://www.typescriptlang.org/tsconfig/#checkJs>
    pub check_js: Option<bool>,
}

impl<'de> Deserialize<'de> for CompilerOptions {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawCompilerOptions::deserialize(deserializer)?;
        let mut presence = CompilerOptionsPresence::default();

        macro_rules! field {
            ($name:ident, $flag:ident) => {{
                if raw.$name.present {
                    presence.insert($flag);
                }
                raw.$name.value
            }};
        }

        Ok(Self {
            base_url: field!(base_url, BASE_URL),
            paths: field!(paths, PATHS),
            experimental_decorators: field!(experimental_decorators, EXPERIMENTAL_DECORATORS),
            emit_decorator_metadata: field!(emit_decorator_metadata, EMIT_DECORATOR_METADATA),
            strict: field!(strict, STRICT),
            strict_null_checks: field!(strict_null_checks, STRICT_NULL_CHECKS),
            use_define_for_class_fields: field!(
                use_define_for_class_fields,
                USE_DEFINE_FOR_CLASS_FIELDS
            ),
            rewrite_relative_import_extensions: field!(
                rewrite_relative_import_extensions,
                REWRITE_RELATIVE_IMPORT_EXTENSIONS
            ),
            jsx: field!(jsx, JSX),
            jsx_factory: field!(jsx_factory, JSX_FACTORY),
            jsx_fragment_factory: field!(jsx_fragment_factory, JSX_FRAGMENT_FACTORY),
            jsx_import_source: field!(jsx_import_source, JSX_IMPORT_SOURCE),
            verbatim_module_syntax: field!(verbatim_module_syntax, VERBATIM_MODULE_SYNTAX),
            preserve_value_imports: field!(preserve_value_imports, PRESERVE_VALUE_IMPORTS),
            imports_not_used_as_values: field!(
                imports_not_used_as_values,
                IMPORTS_NOT_USED_AS_VALUES
            ),
            target: field!(target, TARGET),
            module: field!(module, MODULE),
            allow_js: field!(allow_js, ALLOW_JS),
            root_dirs: field!(root_dirs, ROOT_DIRS),
            out_dir: field!(out_dir, OUT_DIR),
            declaration_dir: field!(declaration_dir, DECLARATION_DIR),
            resolve_json_module: field!(resolve_json_module, RESOLVE_JSON_MODULE),
            check_js: field!(check_js, CHECK_JS),
            presence,
            ..Self::default()
        })
    }
}

/// Value for the "extends" field.
///
/// <https://www.typescriptlang.org/tsconfig/#extends>
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum ExtendsField {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Clone, Copy)]
enum GlobPattern<'a> {
    Pattern(&'a [PathBuf]),
    All,
}

/// Normalize Windows `\` to `/` for glob matching, without allocating when there's nothing to replace.
fn to_forward_slashes(s: Cow<'_, str>) -> Cow<'_, str> {
    if s.contains('\\') { Cow::Owned(s.replace('\\', "/")) } else { s }
}

/// Tsconfig resolver
impl TsConfig {
    pub(crate) fn resolve_tsconfig_solution(tsconfig: Arc<Self>, path: &Path) -> Arc<Self> {
        if let Some(solution_tsconfig) = tsconfig.find_referenced_config(path) {
            return solution_tsconfig;
        }
        tsconfig
    }

    fn find_referenced_config(&self, path: &Path) -> Option<Arc<Self>> {
        let mut queue = self.references_resolved.iter().cloned().collect::<VecDeque<_>>();
        let mut visited = FxHashSet::default();
        while let Some(config) = queue.pop_front() {
            if !visited.insert(config.path.clone()) {
                continue;
            }
            if config.is_file_included_in_tsconfig(path) {
                return Some(config);
            }
            queue.extend(config.references_resolved.iter().cloned());
        }
        None
    }

    /// Whether this tsconfig (directly or via a referenced sub-project) claims
    /// ownership of `path`. Used by tsconfig auto-discovery to decide whether
    /// to keep walking up to an ancestor `tsconfig.json` when the nearest one
    /// doesn't actually cover the file via its `files` / `include` / `exclude`
    /// or via a matching reference.
    pub(crate) fn claims_ownership_of(&self, path: &Path) -> bool {
        // Any matching reference claims ownership (consistent with
        // resolve_tsconfig_solution).
        if self.find_referenced_config(path).is_some() {
            return true;
        }
        // Solution-style configs (have `references` and explicit empty
        // `files` / `include`) never claim files themselves. Per the
        // TypeScript spec, an *omitted* `include` defaults to `**/*`, so it
        // must fall through to `is_file_included_in_tsconfig`; only an
        // explicit empty array means "own no files".
        let is_solution_style = !self.references_resolved.is_empty()
            && matches!(self.files.as_deref(), Some([]))
            && matches!(self.include.as_deref(), Some([]));
        if is_solution_style {
            return false;
        }
        self.is_file_included_in_tsconfig(path)
    }

    fn is_file_included_in_tsconfig(&self, path: &Path) -> bool {
        // 1. Check files array (highest priority - overrides exclude)
        if self.files.as_ref().is_some_and(|files| files.iter().any(|file| Path::new(file) == path))
        {
            return true;
        }
        // 2. Reject non-program inputs: a `.js`-family file with `allowJs` off
        //    (even if a glob names `.js`), or an extensionless path
        if self.is_extensionless_or_uncompiled_js(path) {
            return false;
        }
        // 3. Check include patterns
        let is_included = self.include.as_ref().map_or_else(
            || {
                if self.files.is_some() {
                    false
                } else {
                    self.is_glob_matches(path, GlobPattern::All)
                }
            },
            |include_patterns| self.is_glob_matches(path, GlobPattern::Pattern(include_patterns)),
        );
        // 4. Check exclude patterns
        if is_included {
            return self.exclude.as_ref().is_none_or(|exclude_patterns| {
                !self.is_glob_matches(path, GlobPattern::Pattern(exclude_patterns))
            });
        }
        false
    }

    fn is_glob_matches(&self, path: &Path, pattern: GlobPattern) -> bool {
        match pattern {
            // The default `**/*` is scoped to the tsconfig directory; as a
            // wildcard pattern it is restricted to the configured TS/JS
            // extensions, same as the gate in `is_glob_match`.
            GlobPattern::All => {
                path.starts_with(self.directory())
                    && self.is_file_extension_allowed_in_tsconfig(path)
            }
            GlobPattern::Pattern(patterns) => {
                let path_str = to_forward_slashes(path.to_string_lossy());
                patterns.iter().any(|pattern| {
                    let pattern = to_forward_slashes(pattern.to_string_lossy());
                    self.is_glob_match(pattern.as_ref(), path, path_str.as_ref())
                })
            }
        }
    }

    fn is_glob_match(&self, pattern: &str, path: &Path, path_str: &str) -> bool {
        if pattern == path_str {
            return true;
        }
        // Normalize pattern: add implicit /**/* for directory patterns
        // Find the part after the last '/' to check if it looks like a directory
        let after_last_slash = pattern.rsplit('/').next().unwrap_or(pattern);
        let needs_implicit_glob = !after_last_slash.contains(['.', '*', '?']);
        let pattern = if needs_implicit_glob {
            Cow::Owned(format!(
                "{pattern}{}",
                if pattern.ends_with('/') { "**/*" } else { "/**/*" }
            ))
        } else {
            Cow::Borrowed(pattern)
        };
        // Extension gate, applied only to wildcard-terminated patterns: a pattern
        // ending in `*` (e.g. the default `**/*`, or a bare directory like `src`
        // that expands to `src/**/*`) matches "any file", so — mirroring
        // TypeScript — it is restricted to the configured TS/JS extensions.
        // A pattern naming an explicit extension (e.g. `src/**/*.vue`) does not
        // end in `*` and is matched verbatim below; this is what lets
        // solution-style tsconfigs claim explicitly-included non-TS files such
        // as `.vue` / `.svelte` (e.g. the Vite Vue scaffold).
        if pattern.ends_with('*') && !self.is_file_extension_allowed_in_tsconfig(path) {
            return false;
        }
        fast_glob::glob_match(pattern.as_ref(), path_str)
    }

    fn is_file_extension_allowed_in_tsconfig(&self, path: &Path) -> bool {
        const TS_EXTENSIONS: [&str; 4] = ["ts", "tsx", "mts", "cts"];
        const JS_EXTENSIONS: [&str; 4] = ["js", "jsx", "mjs", "cjs"];
        let allow_js = self.compiler_options.allow_js.is_some_and(|b| b);
        path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| {
            TS_EXTENSIONS.contains(&ext)
                || if allow_js { JS_EXTENSIONS.contains(&ext) } else { false }
        })
    }

    /// Whether `path` has no extension (typically a directory) or a `.js`-family
    /// extension this tsconfig does not compile (`allowJs` off). Unlike
    /// `is_file_extension_allowed_in_tsconfig`, it excludes genuine non-TS files
    /// (e.g. `.vue`), which must instead be claimed through `include`.
    fn is_extensionless_or_uncompiled_js(&self, path: &Path) -> bool {
        let allow_js = self.compiler_options.allow_js.is_some_and(|b| b);
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_none_or(|ext| !allow_js && matches!(ext, "js" | "jsx" | "mjs" | "cjs"))
    }
}

#[derive(Clone, Debug, Default)]
struct CompiledTsconfigPaths {
    wildcard_patterns: Vec<CompiledTsconfigPathPattern>,
}

#[derive(Clone, Debug)]
struct CompiledTsconfigPathPattern {
    prefix: CompactString,
    suffix: CompactString,
    prefix_len: usize,
    suffix_len: usize,
    targets: Vec<CompiledTsconfigPathTarget>,
}

#[derive(Clone, Debug)]
enum CompiledTsconfigPathTarget {
    Static(PathBuf),
    Wildcard { prefix: CompactString, suffix: CompactString },
}

impl CompiledTsconfigPaths {
    fn new(paths_map: &CompilerOptionsPathsMap) -> Self {
        let mut wildcard_patterns =
            Vec::<CompiledTsconfigPathPattern>::with_capacity(paths_map.len());
        for (key, paths) in paths_map {
            let Some((prefix, suffix)) = key.split_once('*') else {
                continue;
            };
            let targets = paths
                .iter()
                .map(|path| {
                    let path_str = path.to_string_lossy();
                    path_str.split_once('*').map_or_else(
                        || CompiledTsconfigPathTarget::Static(path.clone()),
                        |(target_prefix, target_suffix)| CompiledTsconfigPathTarget::Wildcard {
                            prefix: CompactString::new(target_prefix),
                            suffix: CompactString::new(target_suffix),
                        },
                    )
                })
                .collect::<Vec<_>>();
            let pattern = CompiledTsconfigPathPattern {
                prefix: CompactString::new(prefix),
                suffix: CompactString::new(suffix),
                prefix_len: prefix.len(),
                suffix_len: suffix.len(),
                targets,
            };

            // Match longer prefixes first. Equal-length prefixes keep insertion order.
            let index = wildcard_patterns
                .iter()
                .position(|existing| existing.prefix_len < pattern.prefix_len)
                .unwrap_or(wildcard_patterns.len());
            wildcard_patterns.insert(index, pattern);
        }

        Self { wildcard_patterns }
    }

    fn resolve(&self, specifier: &str) -> Option<Vec<PathBuf>> {
        self.wildcard_patterns.iter().find_map(|pattern| {
            if !specifier.starts_with(pattern.prefix.as_str())
                || !specifier.ends_with(pattern.suffix.as_str())
                || specifier.len() < pattern.prefix_len + pattern.suffix_len
            {
                return None;
            }
            let wildcard = &specifier[pattern.prefix_len..specifier.len() - pattern.suffix_len];
            Some(pattern.targets.iter().map(|target| target.resolve(wildcard)).collect())
        })
    }
}

impl CompiledTsconfigPathTarget {
    fn resolve(&self, wildcard: &str) -> PathBuf {
        match self {
            Self::Static(path) => path.clone(),
            Self::Wildcard { prefix, suffix } => {
                let mut resolved =
                    String::with_capacity(prefix.len() + wildcard.len() + suffix.len());
                resolved.push_str(prefix.as_str());
                resolved.push_str(wildcard);
                resolved.push_str(suffix.as_str());
                PathBuf::from(resolved)
            }
        }
    }
}
