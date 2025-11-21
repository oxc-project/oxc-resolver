use std::{
    fmt::Debug,
    hash::BuildHasherDefault,
    path::{Path, PathBuf},
    sync::Arc,
};

use indexmap::IndexMap;
use rustc_hash::FxHasher;
use serde::Deserialize;

use crate::{TsconfigReferences, path::PathUtil, replace_bom_with_whitespace};

const TEMPLATE_VARIABLE: &str = "${configDir}";

pub type CompilerOptionsPathsMap = IndexMap<String, Vec<String>, BuildHasherDefault<FxHasher>>;

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
    pub files: Option<Vec<String>>,

    #[serde(default)]
    pub include: Option<Vec<String>>,

    #[serde(default)]
    pub exclude: Option<Vec<String>>,

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
    pub references_resolved: Vec<Arc<TsConfig>>,
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

    /// Returns the base path from which to resolve aliases.
    ///
    /// The base path can be configured by the user as part of the
    /// [CompilerOptions]. If not configured, it returns the directory in which
    /// the tsconfig itself is found.
    #[must_use]
    pub(crate) fn base_path(&self) -> &Path {
        self.compiler_options.base_url.as_ref().map_or_else(|| self.directory(), |p| p.as_path())
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

        let tsconfig_dir = tsconfig.directory();
        let compiler_options = &mut self.compiler_options;

        if compiler_options.base_url.is_none()
            && let Some(base_url) = &tsconfig.compiler_options.base_url
        {
            compiler_options.base_url = Some(if base_url.starts_with(TEMPLATE_VARIABLE) {
                base_url.clone()
            } else {
                tsconfig_dir.join(base_url).normalize()
            });
        }

        if compiler_options.paths.is_none() {
            let paths_base = compiler_options.base_url.as_ref().map_or_else(
                || tsconfig_dir.to_path_buf(),
                |path| {
                    if path.starts_with(TEMPLATE_VARIABLE) {
                        path.clone()
                    } else {
                        tsconfig_dir.join(path).normalize()
                    }
                },
            );
            compiler_options.paths_base = paths_base;
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

        if let Some(base_url) = &self.compiler_options.base_url {
            // Substitute template variable in `tsconfig.compilerOptions.baseUrl`.
            let base_url = base_url.to_string_lossy().strip_prefix(TEMPLATE_VARIABLE).map_or_else(
                || config_dir.normalize_with(base_url),
                |stripped_path| config_dir.join(stripped_path.trim_start_matches('/')),
            );
            self.compiler_options.base_url = Some(base_url);
        }

        if self.compiler_options.paths.is_some() {
            // `paths_base` should use config dir if it is not resolved with base url nor extended
            // with another tsconfig.
            if let Some(base_url) = self.compiler_options.base_url.clone() {
                self.compiler_options.paths_base = base_url;
            }

            if self.compiler_options.paths_base.as_os_str().is_empty() {
                self.compiler_options.paths_base.clone_from(&config_dir);
            }

            // Substitute template variable in `tsconfig.compilerOptions.paths`.
            for paths in self.compiler_options.paths.as_mut().unwrap().values_mut() {
                for path in paths {
                    Self::substitute_template_variable(&config_dir, path);
                }
            }
        }

        self
    }

    /// Template variable `${configDir}` for substitution of config files
    /// directory path.
    ///
    /// NOTE: All tests cases are just a head replacement of `${configDir}`, so
    ///       we are constrained as such.
    ///
    /// See <https://github.com/microsoft/TypeScript/pull/58042>.
    pub(crate) fn substitute_template_variable(directory: &Path, path: &mut String) {
        if let Some(stripped_path) = path.strip_prefix(TEMPLATE_VARIABLE) {
            *path =
                directory.join(stripped_path.trim_start_matches('/')).to_string_lossy().to_string();
        }
    }

    /// Resolves the given `specifier` within the project configured by this
    /// tsconfig, relative to the given `path`.
    ///
    /// `specifier` can be either a real path or an alias.
    #[must_use]
    pub(crate) fn resolve(&self, path: &Path, specifier: &str) -> Vec<PathBuf> {
        let paths = self.resolve_path_alias(specifier);
        for tsconfig in &self.references_resolved {
            if path.starts_with(tsconfig.base_path()) {
                return [tsconfig.resolve_path_alias(specifier), paths].concat();
            }
        }
        paths
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
        let base_url_iter = compiler_options
            .base_url
            .as_ref()
            .map_or_else(Vec::new, |base_url| vec![base_url.normalize_with(specifier)]);

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
                            path.replace(
                                '*',
                                &specifier[longest_prefix_length
                                    ..specifier.len() - longest_suffix_length],
                            )
                        })
                        .collect::<Vec<_>>()
                })
            },
            Clone::clone,
        );

        paths
            .into_iter()
            .map(|p| compiler_options.paths_base.normalize_with(p))
            .chain(base_url_iter)
            .collect()
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

    /// The actual base from where path aliases are resolved.
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
