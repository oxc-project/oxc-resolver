use std::{
    fmt::Debug,
    hash::BuildHasherDefault,
    path::{Path, PathBuf},
    sync::Arc,
};

use indexmap::IndexMap;
use rustc_hash::FxHasher;
use serde::Deserialize;

use crate::{TsconfigReferences, path::PathUtil};

const TEMPLATE_VARIABLE: &str = "${configDir}";

pub type CompilerOptionsPathsMap = IndexMap<String, Vec<String>, BuildHasherDefault<FxHasher>>;

#[derive(Debug, Deserialize)]
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
    pub extends: Option<ExtendsField>,

    #[serde(default)]
    pub compiler_options: CompilerOptions,

    /// Bubbled up project references with a reference to their tsconfig.
    #[serde(default)]
    pub references: Vec<ProjectReference>,
}

impl TsConfig {
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

    /// Returns the compiler options configured in this tsconfig.
    #[must_use]
    pub fn compiler_options(&self) -> &CompilerOptions {
        &self.compiler_options
    }

    /// Returns a mutable reference to the compiler options configured in this
    /// tsconfig.
    #[must_use]
    pub fn compiler_options_mut(&mut self) -> &mut CompilerOptions {
        &mut self.compiler_options
    }

    /// Returns any paths to tsconfigs that should be extended by this tsconfig.
    pub fn extends(&self) -> impl Iterator<Item = &str> {
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
                self.references = paths
                    .iter()
                    .map(|path| ProjectReference { path: path.clone(), tsconfig: None })
                    .collect();
            }
        }

        !self.references.is_empty()
    }

    /// Returns references to other tsconfig files.
    pub(crate) fn references(&self) -> impl Iterator<Item = &ProjectReference> {
        self.references.iter()
    }

    /// Returns mutable references to other tsconfig files.
    pub(crate) fn references_mut(&mut self) -> impl Iterator<Item = &mut ProjectReference> {
        self.references.iter_mut()
    }

    /// Returns the base path from which to resolve aliases.
    ///
    /// The base path can be configured by the user as part of the
    /// [CompilerOptions]. If not configured, it returns the directory in which
    /// the tsconfig itself is found.
    #[must_use]
    pub(crate) fn base_path(&self) -> &Path {
        self.compiler_options().base_url().unwrap_or_else(|| self.directory())
    }

    /// Inherits settings from the given tsconfig into `self`.
    pub(crate) fn extend_tsconfig(&mut self, tsconfig: &Self) {
        let compiler_options = self.compiler_options_mut();

        if compiler_options.base_url().is_none() {
            if let Some(base_url) = tsconfig.compiler_options().base_url() {
                compiler_options.set_base_url(base_url.to_path_buf());
            }
        }

        if compiler_options.paths().is_none() {
            let paths_base = compiler_options
                .base_url()
                .map_or_else(|| tsconfig.directory().to_path_buf(), Path::to_path_buf);
            compiler_options.set_paths_base(paths_base);
            compiler_options.set_paths(tsconfig.compiler_options().paths().cloned());
        }

        if compiler_options.experimental_decorators().is_none() {
            if let Some(experimental_decorators) =
                tsconfig.compiler_options().experimental_decorators()
            {
                compiler_options.set_experimental_decorators(*experimental_decorators);
            }
        }

        if compiler_options.jsx().is_none() {
            if let Some(jsx) = tsconfig.compiler_options().jsx() {
                compiler_options.set_jsx(jsx.to_string());
            }
        }

        if compiler_options.jsx_factory().is_none() {
            if let Some(jsx_factory) = tsconfig.compiler_options().jsx_factory() {
                compiler_options.set_jsx_factory(jsx_factory.to_string());
            }
        }

        if compiler_options.jsx_fragment_factory().is_none() {
            if let Some(jsx_fragment_factory) = tsconfig.compiler_options().jsx_fragment_factory() {
                compiler_options.set_jsx_fragment_factory(jsx_fragment_factory.to_string());
            }
        }

        if compiler_options.jsx_import_source().is_none() {
            if let Some(jsx_import_source) = tsconfig.compiler_options().jsx_import_source() {
                compiler_options.set_jsx_import_source(jsx_import_source.to_string());
            }
        }

        if compiler_options.verbatim_module_syntax().is_none() {
            if let Some(verbatim_module_syntax) =
                tsconfig.compiler_options().verbatim_module_syntax()
            {
                compiler_options.set_verbatim_module_syntax(*verbatim_module_syntax);
            }
        }

        if compiler_options.preserve_value_imports().is_none() {
            if let Some(preserve_value_imports) =
                tsconfig.compiler_options().preserve_value_imports()
            {
                compiler_options.set_preserve_value_imports(*preserve_value_imports);
            }
        }

        if compiler_options.imports_not_used_as_values().is_none() {
            if let Some(imports_not_used_as_values) =
                tsconfig.compiler_options().imports_not_used_as_values()
            {
                compiler_options
                    .set_imports_not_used_as_values(imports_not_used_as_values.to_string());
            }
        }

        if compiler_options.target().is_none() {
            if let Some(target) = tsconfig.compiler_options().target() {
                compiler_options.set_target(target.to_string());
            }
        }

        if compiler_options.module().is_none() {
            if let Some(module) = tsconfig.compiler_options().module() {
                compiler_options.set_module(module.to_string());
            }
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

        if let Some(base_url) = self.compiler_options().base_url() {
            // Substitute template variable in `tsconfig.compilerOptions.baseUrl`.
            let base_url = base_url.to_string_lossy().strip_prefix(TEMPLATE_VARIABLE).map_or_else(
                || config_dir.normalize_with(base_url),
                |stripped_path| config_dir.join(stripped_path.trim_start_matches('/')),
            );
            self.compiler_options_mut().set_base_url(base_url);
        }

        if self.compiler_options().paths().is_some() {
            // `bases_base` should use config dir if it is not resolved with base url nor extended
            // with another tsconfig.
            if let Some(base_url) = self.compiler_options().base_url().map(Path::to_path_buf) {
                self.compiler_options_mut().set_paths_base(base_url);
            }

            if self.compiler_options().paths_base().as_os_str().is_empty() {
                self.compiler_options_mut().set_paths_base(config_dir.clone());
            }

            // Substitute template variable in `tsconfig.compilerOptions.paths`.
            for paths in self.compiler_options_mut().paths_mut().unwrap().values_mut() {
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
        if path.starts_with(self.base_path()) {
            let paths = self.resolve_path_alias(specifier);
            if !paths.is_empty() {
                return paths;
            }
        }
        for tsconfig in self.references().filter_map(ProjectReference::tsconfig) {
            if path.starts_with(tsconfig.base_path()) {
                return tsconfig.resolve_path_alias(specifier);
            }
        }
        Vec::new()
    }

    /// Resolves the given `specifier` within the project configured by this
    /// tsconfig.
    ///
    /// `specifier` is expected to be a path alias.
    // Copied from parcel
    // <https://github.com/parcel-bundler/parcel/blob/b6224fd519f95e68d8b93ba90376fd94c8b76e69/packages/utils/node-resolver-rs/src/tsconfig.rs#L93>
    #[must_use]
    pub(crate) fn resolve_path_alias(&self, specifier: &str) -> Vec<PathBuf> {
        if specifier.starts_with(['/', '.']) {
            return Vec::new();
        }

        let compiler_options = self.compiler_options();
        let base_url_iter = compiler_options
            .base_url()
            .map_or_else(Vec::new, |base_url| vec![base_url.normalize_with(specifier)]);

        let Some(paths_map) = compiler_options.paths() else {
            return base_url_iter;
        };

        let paths = paths_map.get(specifier).map_or_else(
            || {
                let mut longest_prefix_length = 0;
                let mut longest_suffix_length = 0;
                let mut best_key: Option<&String> = None;

                for key in paths_map.keys() {
                    if let Some((prefix, suffix)) = key.split_once('*') {
                        if (best_key.is_none() || prefix.len() > longest_prefix_length)
                            && specifier.starts_with(prefix)
                            && specifier.ends_with(suffix)
                        {
                            longest_prefix_length = prefix.len();
                            longest_suffix_length = suffix.len();
                            best_key.replace(key);
                        }
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
            .map(|p| compiler_options.paths_base().normalize_with(p))
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
}

impl CompilerOptions {
    /// Explicit base URL configured by the user.
    #[must_use]
    fn base_url(&self) -> Option<&Path> {
        self.base_url.as_deref()
    }

    /// Sets the base URL.
    fn set_base_url(&mut self, base_url: PathBuf) {
        self.base_url = Some(base_url);
    }

    /// Path aliases.
    #[must_use]
    fn paths(&self) -> Option<&CompilerOptionsPathsMap> {
        self.paths.as_ref()
    }

    /// Returns a mutable reference to the path aliases.
    #[must_use]
    fn paths_mut(&mut self) -> Option<&mut CompilerOptionsPathsMap> {
        self.paths.as_mut()
    }

    /// Sets the path aliases.
    fn set_paths(&mut self, paths: Option<CompilerOptionsPathsMap>) {
        self.paths = paths;
    }

    /// The actual base from where path aliases are resolved.
    #[must_use]
    fn paths_base(&self) -> &Path {
        &self.paths_base
    }

    /// Sets the path base.
    fn set_paths_base(&mut self, paths_base: PathBuf) {
        self.paths_base = paths_base;
    }

    /// Whether to enable experimental decorators.
    fn experimental_decorators(&self) -> Option<&bool> {
        self.experimental_decorators.as_ref()
    }

    /// Sets whether to enable experimental decorators.
    fn set_experimental_decorators(&mut self, experimental_decorators: bool) {
        self.experimental_decorators = Some(experimental_decorators);
    }

    // /// Whether to emit decorator metadata.
    // fn emit_decorator_metadata(&self) -> Option<&bool> {
    //     self.emit_decorator_metadata.as_ref()
    // }

    // /// Sets whether to emit decorator metadata.
    // fn set_emit_decorator_metadata(&mut self, emit_decorator_metadata: bool) {
    //     self.emit_decorator_metadata = Some(emit_decorator_metadata);
    // }

    /// JSX.
    fn jsx(&self) -> Option<&str> {
        self.jsx.as_deref()
    }

    /// Sets JSX.
    fn set_jsx(&mut self, jsx: String) {
        self.jsx = Some(jsx);
    }

    /// JSX factory.
    fn jsx_factory(&self) -> Option<&str> {
        self.jsx_factory.as_deref()
    }

    /// Sets JSX factory.
    fn set_jsx_factory(&mut self, jsx_factory: String) {
        self.jsx_factory = Some(jsx_factory);
    }

    /// JSX fragment factory.
    fn jsx_fragment_factory(&self) -> Option<&str> {
        self.jsx_fragment_factory.as_deref()
    }

    /// Sets JSX fragment factory.
    fn set_jsx_fragment_factory(&mut self, jsx_fragment_factory: String) {
        self.jsx_fragment_factory = Some(jsx_fragment_factory);
    }

    /// JSX import source.
    fn jsx_import_source(&self) -> Option<&str> {
        self.jsx_import_source.as_deref()
    }

    /// Sets JSX import source.
    fn set_jsx_import_source(&mut self, jsx_import_source: String) {
        self.jsx_import_source = Some(jsx_import_source);
    }

    /// Whether to use verbatim module syntax.
    fn verbatim_module_syntax(&self) -> Option<&bool> {
        self.verbatim_module_syntax.as_ref()
    }

    /// Sets whether to use verbatim module syntax.
    fn set_verbatim_module_syntax(&mut self, verbatim_module_syntax: bool) {
        self.verbatim_module_syntax = Some(verbatim_module_syntax);
    }

    /// Whether to preserve value imports.
    fn preserve_value_imports(&self) -> Option<&bool> {
        self.preserve_value_imports.as_ref()
    }

    /// Sets whether to preserve value imports.
    fn set_preserve_value_imports(&mut self, preserve_value_imports: bool) {
        self.preserve_value_imports = Some(preserve_value_imports);
    }

    /// Whether to use imports not used as values.
    fn imports_not_used_as_values(&self) -> Option<&str> {
        self.imports_not_used_as_values.as_deref()
    }

    /// Sets whether to use imports not used as values.
    fn set_imports_not_used_as_values(&mut self, imports_not_used_as_values: String) {
        self.imports_not_used_as_values = Some(imports_not_used_as_values);
    }

    /// Target.
    fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }

    /// Sets the target.
    fn set_target(&mut self, target: String) {
        self.target = Some(target);
    }

    /// Module.
    fn module(&self) -> Option<&str> {
        self.module.as_deref()
    }

    /// Sets the module.
    fn set_module(&mut self, module: String) {
        self.module = Some(module);
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

/// Project Reference
///
/// <https://www.typescriptlang.org/docs/handbook/project-references.html>
#[derive(Debug, Deserialize)]
pub struct ProjectReference {
    pub path: PathBuf,

    #[serde(skip)]
    pub tsconfig: Option<Arc<TsConfig>>,
}

impl ProjectReference {
    /// Returns the path to a directory containing a `tsconfig.json` file, or to
    /// the config file itself (which may have any name).
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Returns the resolved tsconfig, if one has been set.
    #[must_use]
    pub fn tsconfig(&self) -> Option<Arc<TsConfig>> {
        self.tsconfig.clone()
    }

    /// Sets the resolved tsconfig.
    pub fn set_tsconfig(&mut self, tsconfig: Arc<TsConfig>) {
        self.tsconfig.replace(tsconfig);
    }
}

impl TsConfig {
    /// Parses the tsconfig from a JSON string.
    ///
    /// # Errors
    ///
    /// * Any error that can be returned by `serde_json::from_str()`.
    pub fn parse(root: bool, path: &Path, json: &mut str) -> Result<Self, serde_json::Error> {
        let json = trim_start_matches_mut(json, '\u{feff}'); // strip bom
        _ = json_strip_comments::strip(json);
        let mut tsconfig: Self = serde_json::from_str(json)?;
        tsconfig.root = root;
        tsconfig.path = path.to_path_buf();
        Ok(tsconfig)
    }
}

fn trim_start_matches_mut(s: &mut str, pat: char) -> &mut str {
    if s.starts_with(pat) {
        // trim the prefix
        &mut s[pat.len_utf8()..]
    } else {
        s
    }
}
