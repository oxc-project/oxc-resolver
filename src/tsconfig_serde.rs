use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::Deserialize;

use crate::{
    CompilerOptions, CompilerOptionsPathsMap, PathUtil, ProjectReference, TsConfig,
    TsconfigReferences,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TsConfigSerde {
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
    pub compiler_options: CompilerOptionsSerde,

    /// Bubbled up project references with a reference to their tsconfig.
    #[serde(default)]
    pub references: Vec<ProjectReferenceSerde>,
}

impl TsConfig for TsConfigSerde {
    type Co = CompilerOptionsSerde;

    fn root(&self) -> bool {
        self.root
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn directory(&self) -> &Path {
        debug_assert!(self.path.file_name().is_some());
        self.path.parent().unwrap()
    }

    fn compiler_options(&self) -> &Self::Co {
        &self.compiler_options
    }

    fn compiler_options_mut(&mut self) -> &mut Self::Co {
        &mut self.compiler_options
    }

    fn extends(&self) -> impl Iterator<Item = &str> {
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

    fn load_references(&mut self, references: &TsconfigReferences) -> bool {
        match references {
            TsconfigReferences::Disabled => {
                self.references.drain(..);
            }
            TsconfigReferences::Auto => {}
            TsconfigReferences::Paths(paths) => {
                self.references = paths
                    .iter()
                    .map(|path| ProjectReferenceSerde { path: path.clone(), tsconfig: None })
                    .collect();
            }
        }

        !self.references.is_empty()
    }

    fn references(&self) -> impl Iterator<Item = &impl ProjectReference<Tc = Self>> {
        self.references.iter()
    }

    fn references_mut(&mut self) -> impl Iterator<Item = &mut impl ProjectReference<Tc = Self>> {
        self.references.iter_mut()
    }
}

/// Compiler Options
///
/// <https://www.typescriptlang.org/tsconfig#compilerOptions>
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerOptionsSerde {
    pub base_url: Option<PathBuf>,

    /// Path aliases.
    pub paths: Option<CompilerOptionsPathsMap>,

    /// The actual base from where path aliases are resolved.
    #[serde(skip)]
    paths_base: PathBuf,

    /// <https://www.typescriptlang.org/tsconfig/#experimentalDecorators>
    pub experimental_decorators: Option<bool>,

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
}

impl CompilerOptions for CompilerOptionsSerde {
    fn base_url(&self) -> Option<&Path> {
        self.base_url.as_deref()
    }

    fn set_base_url(&mut self, base_url: PathBuf) {
        self.base_url = Some(base_url);
    }

    fn paths(&self) -> Option<&CompilerOptionsPathsMap> {
        self.paths.as_ref()
    }

    fn paths_mut(&mut self) -> Option<&mut CompilerOptionsPathsMap> {
        self.paths.as_mut()
    }

    fn set_paths(&mut self, paths: Option<CompilerOptionsPathsMap>) {
        self.paths = paths;
    }

    fn paths_base(&self) -> &Path {
        &self.paths_base
    }

    fn set_paths_base(&mut self, paths_base: PathBuf) {
        self.paths_base = paths_base;
    }

    fn experimental_decorators(&self) -> Option<&bool> {
        self.experimental_decorators.as_ref()
    }

    fn set_experimental_decorators(&mut self, experimental_decorators: bool) {
        self.experimental_decorators = Some(experimental_decorators);
    }

    fn jsx(&self) -> Option<&str> {
        self.jsx.as_deref()
    }

    fn set_jsx(&mut self, jsx: String) {
        self.jsx = Some(jsx);
    }

    fn jsx_factory(&self) -> Option<&str> {
        self.jsx_factory.as_deref()
    }

    fn set_jsx_factory(&mut self, jsx_factory: String) {
        self.jsx_factory = Some(jsx_factory);
    }

    fn jsx_fragment_factory(&self) -> Option<&str> {
        self.jsx_fragment_factory.as_deref()
    }

    fn set_jsx_fragment_factory(&mut self, jsx_fragment_factory: String) {
        self.jsx_fragment_factory = Some(jsx_fragment_factory);
    }

    fn jsx_import_source(&self) -> Option<&str> {
        self.jsx_import_source.as_deref()
    }

    fn set_jsx_import_source(&mut self, jsx_import_source: String) {
        self.jsx_import_source = Some(jsx_import_source);
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
pub struct ProjectReferenceSerde {
    pub path: PathBuf,

    #[serde(skip)]
    pub tsconfig: Option<Arc<TsConfigSerde>>,
}

impl ProjectReference for ProjectReferenceSerde {
    type Tc = TsConfigSerde;

    fn path(&self) -> &Path {
        &self.path
    }

    fn tsconfig(&self) -> Option<Arc<Self::Tc>> {
        self.tsconfig.clone()
    }

    fn set_tsconfig(&mut self, tsconfig: Arc<Self::Tc>) {
        self.tsconfig.replace(tsconfig);
    }
}

impl TsConfigSerde {
    /// Parses the tsconfig from a JSON string.
    ///
    /// # Errors
    ///
    /// * Any error that can be returned by `serde_json::from_str()`.
    pub fn parse(root: bool, path: &Path, json: &mut str) -> Result<Self, serde_json::Error> {
        _ = json_strip_comments::strip(json);
        if json.trim().is_empty() {
            let mut tsconfig: Self = serde_json::from_str("{}")?;
            tsconfig.root = root;
            tsconfig.path = path.to_path_buf();
            return Ok(tsconfig);
        }
        let mut tsconfig: Self = serde_json::from_str(json)?;
        tsconfig.root = root;
        tsconfig.path = path.to_path_buf();
        let directory = tsconfig.directory().to_path_buf();
        if let Some(base_url) = tsconfig.compiler_options.base_url {
            tsconfig.compiler_options.base_url = Some(directory.normalize_with(base_url));
        }
        if tsconfig.compiler_options.paths.is_some() {
            tsconfig.compiler_options.paths_base =
                tsconfig.compiler_options.base_url.as_ref().map_or(directory, Clone::clone);
        }
        Ok(tsconfig)
    }
}
