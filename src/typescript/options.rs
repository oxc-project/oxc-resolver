use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeResolutionMode {
    Declaration,
    Full,
    None,
}

impl Default for TypeResolutionMode {
    fn default() -> Self {
        Self::Full
    }
}

#[derive(Debug, Clone, Default)]
pub struct TypeScriptOptions {
    pub typescript_version: Option<String>,
    pub type_roots: Option<Vec<PathBuf>>,
    pub type_resolution_mode: TypeResolutionMode,
    pub resolve_type_references: bool,
}

impl TypeScriptOptions {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_typescript_version(mut self, version: String) -> Self {
        self.typescript_version = Some(version);
        self
    }

    #[must_use]
    pub fn with_type_roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.type_roots = Some(roots);
        self
    }

    #[must_use]
    pub fn with_type_resolution_mode(mut self, mode: TypeResolutionMode) -> Self {
        self.type_resolution_mode = mode;
        self
    }

    #[must_use]
    pub fn with_resolve_type_references(mut self, enabled: bool) -> Self {
        self.resolve_type_references = enabled;
        self
    }
}
