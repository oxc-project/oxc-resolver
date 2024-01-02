use std::ops::{Deref, DerefMut};

use crate::error::ResolveError;

#[derive(Debug, Default, Clone)]
pub struct ResolveContext(ResolveContextImpl);

#[derive(Debug, Default, Clone)]
pub struct ResolveContextImpl {
    pub fully_specified: bool,
    pub query: Option<String>,
    pub fragment: Option<String>,
    /// The current resolving alias for bailing recursion alias.
    pub resolving_alias: Option<String>,
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

    pub fn with_resolving_alias(&mut self, alias: String) {
        self.resolving_alias = Some(alias);
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
