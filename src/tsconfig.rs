use std::{
    fmt::Debug,
    hash::BuildHasherDefault,
    path::{Path, PathBuf},
    sync::Arc,
};

use indexmap::IndexMap;
use rustc_hash::FxHasher;

use crate::{TsconfigReferences, path::PathUtil};

const TEMPLATE_VARIABLE: &str = "${configDir}";

pub type CompilerOptionsPathsMap = IndexMap<String, Vec<String>, BuildHasherDefault<FxHasher>>;

/// Abstract representation for the contents of a `tsconfig.json` file, as well
/// as the location where it was found.
///
/// This representation makes no assumptions regarding how the file was
/// deserialized.
#[allow(clippy::missing_errors_doc)] // trait impls should be free to return any typesafe error
pub trait TsConfig: Sized + Debug {
    type Co: CompilerOptions + Debug;

    /// Whether this is the caller tsconfig.
    /// Used for final template variable substitution when all configs are extended and merged.
    fn root(&self) -> bool;

    /// Returns the path where the `tsconfig.json` was found.
    ///
    /// Contains the `tsconfig.json` filename.
    #[must_use]
    fn path(&self) -> &Path;

    /// Directory to `tsconfig.json`.
    ///
    /// # Panics
    ///
    /// * When the `tsconfig.json` path is misconfigured.
    #[must_use]
    fn directory(&self) -> &Path;

    /// Returns the compiler options configured in this tsconfig.
    #[must_use]
    fn compiler_options(&self) -> &Self::Co;

    /// Returns a mutable reference to the compiler options configured in this
    /// tsconfig.
    #[must_use]
    fn compiler_options_mut(&mut self) -> &mut Self::Co;

    /// Returns any paths to tsconfigs that should be extended by this tsconfig.
    #[must_use]
    fn extends(&self) -> impl Iterator<Item = &str>;

    /// Loads the given references into this tsconfig.
    ///
    /// Returns whether any references are defined in the tsconfig.
    fn load_references(&mut self, references: &TsconfigReferences) -> bool;

    /// Returns references to other tsconfig files.
    #[must_use]
    fn references(&self) -> impl Iterator<Item = &impl ProjectReference<Tc = Self>>;

    /// Returns mutable references to other tsconfig files.
    #[must_use]
    fn references_mut(&mut self) -> impl Iterator<Item = &mut impl ProjectReference<Tc = Self>>;

    /// Returns the base path from which to resolve aliases.
    ///
    /// The base path can be configured by the user as part of the
    /// [CompilerOptions]. If not configured, it returns the directory in which
    /// the tsconfig itself is found.
    #[must_use]
    fn base_path(&self) -> &Path {
        self.compiler_options().base_url().unwrap_or_else(|| self.directory())
    }

    /// Inherits settings from the given tsconfig into `self`.
    fn extend_tsconfig(&mut self, tsconfig: &Self) {
        let compiler_options = self.compiler_options_mut();

        let tsconfig_dir = tsconfig.directory();

        if compiler_options.base_url().is_none() {
            if let Some(base_url) = tsconfig.compiler_options().base_url() {
                compiler_options.set_base_url(if base_url.starts_with(TEMPLATE_VARIABLE) {
                    base_url.to_path_buf()
                } else {
                    tsconfig_dir.join(base_url).normalize()
                });
            }
        }

        if compiler_options.paths().is_none() {
            let paths_base = compiler_options.base_url().map_or_else(
                || tsconfig_dir.to_path_buf(),
                |path| {
                    if path.starts_with(TEMPLATE_VARIABLE) {
                        path.to_path_buf()
                    } else {
                        tsconfig_dir.join(path).normalize()
                    }
                },
            );
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
    }

    /// "Build" the root tsconfig, resolve:
    ///
    /// * `{configDir}` template variable
    /// * `paths_base` for resolving paths alias
    /// * `baseUrl` to absolute path
    #[must_use]
    fn build(mut self) -> Self {
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
            // `paths_base` should use config dir if it is not resolved with base url nor extended
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
    fn substitute_template_variable(directory: &Path, path: &mut String) {
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
    fn resolve(&self, path: &Path, specifier: &str) -> Vec<PathBuf> {
        let paths = self.resolve_path_alias(specifier);
        for tsconfig in self.references().filter_map(ProjectReference::tsconfig) {
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
    fn resolve_path_alias(&self, specifier: &str) -> Vec<PathBuf> {
        if specifier.starts_with('.') {
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

/// Compiler Options.
///
/// <https://www.typescriptlang.org/tsconfig#compilerOptions>
pub trait CompilerOptions {
    /// Explicit base URL configured by the user.
    #[must_use]
    fn base_url(&self) -> Option<&Path>;

    /// Sets the base URL.
    fn set_base_url(&mut self, base_url: PathBuf);

    /// Path aliases.
    #[must_use]
    fn paths(&self) -> Option<&CompilerOptionsPathsMap>;

    /// Returns a mutable reference to the path aliases.
    #[must_use]
    fn paths_mut(&mut self) -> Option<&mut CompilerOptionsPathsMap>;

    /// Sets the path aliases.
    fn set_paths(&mut self, paths: Option<CompilerOptionsPathsMap>);

    /// The actual base from where path aliases are resolved.
    #[must_use]
    fn paths_base(&self) -> &Path;

    /// Sets the path base.
    fn set_paths_base(&mut self, paths_base: PathBuf);

    /// Whether to enable experimental decorators.
    fn experimental_decorators(&self) -> Option<&bool> {
        None
    }

    /// Sets whether to enable experimental decorators.
    fn set_experimental_decorators(&mut self, _experimental_decorators: bool) {}

    /// Whether to emit decorator metadata.
    fn emit_decorator_metadata(&self) -> Option<&bool> {
        None
    }

    /// Sets whether to emit decorator metadata.
    fn set_emit_decorator_metadata(&mut self, _emit_decorator_metadata: bool) {}

    /// JSX.
    fn jsx(&self) -> Option<&str> {
        None
    }

    /// Sets JSX.
    fn set_jsx(&mut self, _jsx: String) {}

    /// JSX factory.
    fn jsx_factory(&self) -> Option<&str> {
        None
    }

    /// Sets JSX factory.
    fn set_jsx_factory(&mut self, _jsx_factory: String) {}

    /// JSX fragment factory.
    fn jsx_fragment_factory(&self) -> Option<&str> {
        None
    }

    /// Sets JSX fragment factory.
    fn set_jsx_fragment_factory(&mut self, _jsx_fragment_factory: String) {}

    /// JSX import source.
    fn jsx_import_source(&self) -> Option<&str> {
        None
    }

    /// Sets JSX import source.
    fn set_jsx_import_source(&mut self, _jsx_import_source: String) {}
}

/// Project Reference.
///
/// <https://www.typescriptlang.org/docs/handbook/project-references.html>
pub trait ProjectReference {
    type Tc: TsConfig;

    /// Returns the path to a directory containing a `tsconfig.json` file, or to
    /// the config file itself (which may have any name).
    #[must_use]
    fn path(&self) -> &Path;

    /// Returns the resolved tsconfig, if one has been set.
    #[must_use]
    fn tsconfig(&self) -> Option<Arc<Self::Tc>>;

    /// Sets the resolved tsconfig.
    fn set_tsconfig(&mut self, tsconfig: Arc<Self::Tc>);
}
