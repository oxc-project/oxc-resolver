use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use crate::error::ResolveError;

#[derive(Debug, Default, Clone)]
pub struct ResolveContext(ResolveContextImpl);

#[derive(Debug, Default, Clone)]
pub struct ResolveContextImpl {
    pub fully_specified: bool,

    pub query: Option<String>,

    pub fragment: Option<String>,

    /// Files that was found on file system
    pub file_dependencies: Option<Vec<PathBuf>>,

    /// Files that was found on file system
    pub missing_dependencies: Option<Vec<PathBuf>>,

    /// The current resolving alias for bailing recursion alias.
    pub resolving_alias: Option<String>,

    /// Resolve files in node_modules.
    /// If extension alias is enabled and all of the aliased extension are not found:
    ///   1. if in node_modules, we can fallback to the original extension.
    ///   2. if not in node_modules, should not allow to fallback to the original extension or add extensions.
    pub resolve_in_node_modules: bool,

    /// For avoiding infinite recursion, which will cause stack overflow.
    depth: u8,
}

impl Deref for ResolveContext {
    type Target = ResolveContextImpl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ResolveContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ResolveContext {
    pub fn with_fully_specified(&mut self, yes: bool) {
        self.fully_specified = yes;
    }

    pub fn with_query_fragment(&mut self, query: Option<&str>, fragment: Option<&str>) {
        if let Some(query) = query {
            self.query.replace(query.to_string());
        }
        if let Some(fragment) = fragment {
            self.fragment.replace(fragment.to_string());
        }
    }

    pub fn init_file_dependencies(&mut self) {
        self.file_dependencies.replace(vec![]);
        self.missing_dependencies.replace(vec![]);
    }

    pub fn add_file_dependency(&mut self, dep: &Path) {
        if let Some(deps) = &mut self.file_dependencies {
            deps.push(dep.to_path_buf());
        }
    }

    pub fn add_missing_dependency(&mut self, dep: &Path) {
        if let Some(deps) = &mut self.missing_dependencies {
            deps.push(dep.to_path_buf());
        }
    }

    pub fn with_resolving_alias(&mut self, alias: String) {
        self.resolving_alias = Some(alias);
    }

    pub fn with_resolve_in_node_modules(&mut self, yes: bool) {
        self.resolve_in_node_modules = yes;
    }

    pub fn test_for_infinite_recursion(&mut self) -> Result<(), ResolveError> {
        self.depth += 1;
        // 64 should be more than enough for detecting infinite recursion.
        if self.depth > 64 {
            return Err(ResolveError::Recursion);
        }
        Ok(())
    }
}
