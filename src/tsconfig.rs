use std::{
    borrow::Cow,
    fmt::Debug,
    hash::BuildHasherDefault,
    path::{Path, PathBuf},
    sync::Arc,
};

use indexmap::IndexMap;
use rustc_hash::FxHasher;
use serde::Deserialize;

use crate::{TsconfigReferences, path::PathUtil, replace_bom_with_whitespace};

/// Template variable `${configDir}` for substitution of config files
/// directory path.
///
/// NOTE: All tests cases are just a head replacement of `${configDir}`, so
///       we are constrained as such.
///
/// See <https://github.com/microsoft/TypeScript/pull/58042>.
/// Allow list: <https://github.com/microsoft/TypeScript/issues/57485#issuecomment-2027787456>
const TEMPLATE_VARIABLE: &str = "${configDir}";

const GLOB_ALL_PATTERN: &str = "**/*";

pub type CompilerOptionsPathsMap = IndexMap<String, Vec<PathBuf>, BuildHasherDefault<FxHasher>>;

/// Project Reference
///
/// <https://www.typescriptlang.org/docs/handbook/project-references.html>
#[derive(Debug, Deserialize)]
pub struct ProjectReference {
    pub path: PathBuf,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TsConfig {
    /// Whether this is the caller tsconfig.
    /// Used for final template variable substitution when all configs are extended and merged.
    #[serde(skip)]
    pub root: bool,

    /// Path to `tsconfig.json`. Contains the `tsconfig.json` filename.
    #[serde(skip)]
    pub path: PathBuf,

    #[serde(default)]
    pub files: Option<Vec<PathBuf>>,

    #[serde(default)]
    pub include: Option<Vec<PathBuf>>,

    #[serde(default)]
    pub exclude: Option<Vec<PathBuf>>,

    #[serde(default)]
    pub extends: Option<ExtendsField>,

    #[serde(default)]
    pub compiler_options: CompilerOptions,

    #[serde(default)]
    pub references: Vec<ProjectReference>,

    /// Resolved project references.
    ///
    /// Corresponds to each item in [TsConfig::references].
    #[serde(skip)]
    pub references_resolved: Vec<Arc<Self>>,
}

impl TsConfig {
    /// Parses the tsconfig from a JSON string.
    ///
    /// # Errors
    ///
    /// * Any error that can be returned by `serde_json::from_str()`.
    pub fn parse(root: bool, path: &Path, json: String) -> Result<Self, serde_json::Error> {
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
        tsconfig.compiler_options.paths_base =
            tsconfig.compiler_options.base_url.as_ref().map_or_else(
                || tsconfig.directory().to_path_buf(),
                |base_url| {
                    if base_url.to_string_lossy().starts_with(TEMPLATE_VARIABLE) {
                        base_url.clone()
                    } else {
                        tsconfig.directory().normalize_with(base_url)
                    }
                },
            );
        Ok(tsconfig)
    }

    /// Whether this is the caller tsconfig.
    /// Used for final template variable substitution when all configs are extended and merged.
    #[must_use]
    pub fn root(&self) -> bool {
        self.root
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
    pub(crate) fn load_references(&mut self, references: &TsconfigReferences) -> bool {
        match references {
            TsconfigReferences::Disabled => {
                self.references.drain(..);
            }
            TsconfigReferences::Auto => {}
            TsconfigReferences::Paths(paths) => {
                self.references =
                    paths.iter().map(|path| ProjectReference { path: path.clone() }).collect();
            }
        }

        !self.references.is_empty()
    }

    /// Inherits settings from the given tsconfig into `self`.
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
    pub(crate) fn extend_tsconfig(&mut self, tsconfig: &Self) {
        if self.files.is_none()
            && let Some(files) = &tsconfig.files
        {
            self.files = Some(files.clone());
        }

        if self.include.is_none()
            && let Some(include) = &tsconfig.include
        {
            self.include = Some(include.clone());
        }

        if self.exclude.is_none()
            && let Some(exclude) = &tsconfig.exclude
        {
            self.exclude = Some(exclude.clone());
        }

        let compiler_options = &mut self.compiler_options;

        if compiler_options.base_url.is_none() {
            compiler_options.base_url.clone_from(&tsconfig.compiler_options.base_url);
            if tsconfig.compiler_options.base_url.is_some() {
                compiler_options.paths_base.clone_from(&tsconfig.compiler_options.paths_base);
            }
        }
        if compiler_options.paths.is_none() {
            if compiler_options.base_url.is_none() && tsconfig.compiler_options.base_url.is_none() {
                compiler_options.paths_base.clone_from(&tsconfig.compiler_options.paths_base);
            }
            compiler_options.paths.clone_from(&tsconfig.compiler_options.paths);
        }

        if compiler_options.experimental_decorators.is_none()
            && let Some(experimental_decorators) =
                &tsconfig.compiler_options.experimental_decorators
        {
            compiler_options.experimental_decorators = Some(*experimental_decorators);
        }

        if compiler_options.emit_decorator_metadata.is_none()
            && let Some(emit_decorator_metadata) =
                &tsconfig.compiler_options.emit_decorator_metadata
        {
            compiler_options.emit_decorator_metadata = Some(*emit_decorator_metadata);
        }

        if compiler_options.use_define_for_class_fields.is_none()
            && let Some(use_define_for_class_fields) =
                &tsconfig.compiler_options.use_define_for_class_fields
        {
            compiler_options.use_define_for_class_fields = Some(*use_define_for_class_fields);
        }

        if compiler_options.rewrite_relative_import_extensions.is_none()
            && let Some(rewrite_relative_import_extensions) =
                &tsconfig.compiler_options.rewrite_relative_import_extensions
        {
            compiler_options.rewrite_relative_import_extensions =
                Some(*rewrite_relative_import_extensions);
        }

        if compiler_options.jsx.is_none()
            && let Some(jsx) = &tsconfig.compiler_options.jsx
        {
            compiler_options.jsx = Some(jsx.clone());
        }

        if compiler_options.jsx_factory.is_none()
            && let Some(jsx_factory) = &tsconfig.compiler_options.jsx_factory
        {
            compiler_options.jsx_factory = Some(jsx_factory.clone());
        }

        if compiler_options.jsx_fragment_factory.is_none()
            && let Some(jsx_fragment_factory) = &tsconfig.compiler_options.jsx_fragment_factory
        {
            compiler_options.jsx_fragment_factory = Some(jsx_fragment_factory.clone());
        }

        if compiler_options.jsx_import_source.is_none()
            && let Some(jsx_import_source) = &tsconfig.compiler_options.jsx_import_source
        {
            compiler_options.jsx_import_source = Some(jsx_import_source.clone());
        }

        if compiler_options.verbatim_module_syntax.is_none()
            && let Some(verbatim_module_syntax) = &tsconfig.compiler_options.verbatim_module_syntax
        {
            compiler_options.verbatim_module_syntax = Some(*verbatim_module_syntax);
        }

        if compiler_options.preserve_value_imports.is_none()
            && let Some(preserve_value_imports) = &tsconfig.compiler_options.preserve_value_imports
        {
            compiler_options.preserve_value_imports = Some(*preserve_value_imports);
        }

        if compiler_options.imports_not_used_as_values.is_none()
            && let Some(imports_not_used_as_values) =
                &tsconfig.compiler_options.imports_not_used_as_values
        {
            compiler_options.imports_not_used_as_values = Some(imports_not_used_as_values.clone());
        }

        if compiler_options.target.is_none()
            && let Some(target) = &tsconfig.compiler_options.target
        {
            compiler_options.target = Some(target.clone());
        }

        if compiler_options.module.is_none()
            && let Some(module) = &tsconfig.compiler_options.module
        {
            compiler_options.module = Some(module.clone());
        }

        if compiler_options.allow_js.is_none()
            && let Some(allow_js) = &tsconfig.compiler_options.allow_js
        {
            compiler_options.allow_js = Some(*allow_js);
        }
    }
    /// "Build" the root tsconfig, resolve:
    ///
    /// * `{configDir}` template variable
    /// * `paths_base` for resolving paths alias
    /// * `baseUrl` to absolute path
    #[must_use]
    pub(crate) fn build(mut self) -> Self {
        // Only the root tsconfig requires paths resolution.
        if !self.root() {
            return self;
        }

        let config_dir = self.directory().to_path_buf();

        // Substitute template variable in `tsconfig.files`.
        if let Some(files) = self.files.take() {
            self.files = Some(files.into_iter().map(|p| self.adjust_path(p)).collect());
        }

        // Substitute template variable in `tsconfig.include`.
        if let Some(includes) = self.include.take() {
            self.include = Some(includes.into_iter().map(|p| self.adjust_path(p)).collect());
        }

        // Substitute template variable in `tsconfig.exclude`.
        if let Some(excludes) = self.exclude.take() {
            self.exclude = Some(excludes.into_iter().map(|p| self.adjust_path(p)).collect());
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

        if self.compiler_options.paths.is_some() {
            // Substitute template variable in `tsconfig.compilerOptions.paths`.
            for paths in self.compiler_options.paths.as_mut().unwrap().values_mut() {
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
        }

        self
    }

    #[expect(clippy::option_if_let_else)]
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
        for tsconfig in &self.references_resolved {
            if path.starts_with(&tsconfig.compiler_options.paths_base) {
                return tsconfig.resolve_path_alias(specifier);
            }
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
        if specifier.starts_with('.') {
            return Vec::new();
        }

        let compiler_options = &self.compiler_options;
        let base_url_iter = vec![compiler_options.paths_base.normalize_with(specifier)];

        let Some(paths_map) = &compiler_options.paths else {
            return base_url_iter;
        };

        let paths = paths_map.get(specifier).map_or_else(
            || {
                let mut longest_prefix_length = 0;
                let mut longest_suffix_length = 0;
                let mut best_key: Option<&String> = None;

                for key in paths_map.keys() {
                    if let Some((prefix, suffix)) = key.split_once('*')
                        && (best_key.is_none() || prefix.len() > longest_prefix_length)
                        && specifier.starts_with(prefix)
                        && specifier.ends_with(suffix)
                    {
                        longest_prefix_length = prefix.len();
                        longest_suffix_length = suffix.len();
                        best_key.replace(key);
                    }
                }

                best_key.and_then(|key| paths_map.get(key)).map_or_else(Vec::new, |paths| {
                    paths
                        .iter()
                        .map(|path| {
                            PathBuf::from(path.to_string_lossy().replace(
                                '*',
                                &specifier[longest_prefix_length
                                    ..specifier.len() - longest_suffix_length],
                            ))
                        })
                        .collect::<Vec<_>>()
                })
            },
            Clone::clone,
        );

        paths.into_iter().chain(base_url_iter).collect()
    }
}

/// Compiler Options
///
/// <https://www.typescriptlang.org/tsconfig#compilerOptions>
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerOptions {
    pub base_url: Option<PathBuf>,

    /// Path aliases.
    pub paths: Option<CompilerOptionsPathsMap>,

    /// The "base_url" at which this tsconfig is defined.
    #[serde(skip)]
    pub(crate) paths_base: PathBuf,

    /// <https://www.typescriptlang.org/tsconfig/#experimentalDecorators>
    pub experimental_decorators: Option<bool>,

    /// <https://www.typescriptlang.org/tsconfig/#emitDecoratorMetadata>
    pub emit_decorator_metadata: Option<bool>,

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

/// Tsconfig resolver
impl TsConfig {
    pub(crate) fn resolve_tsconfig_solution(tsconfig: Arc<Self>, path: &Path) -> Arc<Self> {
        if !tsconfig.references_resolved.is_empty()
            && tsconfig.is_file_extension_allowed_in_tsconfig(path)
            && !tsconfig.is_file_included_in_tsconfig(path)
            && let Some(solution_tsconfig) = tsconfig
                .references_resolved
                .iter()
                .find(|referenced| referenced.is_file_included_in_tsconfig(path))
                .map(Arc::clone)
        {
            return solution_tsconfig;
        }
        tsconfig
    }

    fn is_file_included_in_tsconfig(&self, path: &Path) -> bool {
        // 1. Check files array (highest priority - overrides exclude)
        if self.files.as_ref().is_some_and(|files| files.iter().any(|file| Path::new(file) == path))
        {
            return true;
        }
        // 2. Check include patterns
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
        // 3. Check exclude patterns
        if is_included {
            return self.exclude.as_ref().is_none_or(|exclude_patterns| {
                !self.is_glob_matches(path, GlobPattern::Pattern(exclude_patterns))
            });
        }
        false
    }

    fn is_glob_matches(&self, path: &Path, pattern: GlobPattern) -> bool {
        let path_str = path.to_string_lossy().replace('\\', "/");
        match pattern {
            GlobPattern::All => self.is_glob_match(GLOB_ALL_PATTERN, path, &path_str),
            GlobPattern::Pattern(patterns) => patterns.iter().any(|pattern| {
                let pattern = pattern.to_string_lossy().replace('\\', "/");
                self.is_glob_match(pattern.as_ref(), path, &path_str)
            }),
        }
    }

    fn is_glob_match(&self, pattern: &str, path: &Path, path_str: &str) -> bool {
        if pattern == path_str {
            return true;
        }
        // Special case: **/* matches everything
        if pattern == GLOB_ALL_PATTERN {
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
        // Fast check: if pattern ends with *, filename must have valid extension
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
}
