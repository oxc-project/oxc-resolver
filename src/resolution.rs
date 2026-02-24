use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{CachedPath, PackageJson};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ModuleType {
    Module,
    CommonJs,
    Json,
    Wasm,
    Addon,
}

/// The final path resolution with optional `?query` and `#fragment`
pub struct Resolution {
    pub(crate) cached_path: CachedPath,

    /// Path query `?query`, contains `?`.
    pub(crate) query: Option<String>,

    /// Path fragment `#query`, contains `#`.
    pub(crate) fragment: Option<String>,

    /// `package.json` for the given module.
    pub(crate) package_json: Option<Arc<PackageJson>>,

    /// Module type for this path.
    ///
    /// Enable with [crate::ResolveOptions::module_type].
    ///
    /// The module type is computed `ESM_FILE_FORMAT` from the [ESM resolution algorithm specification](https://nodejs.org/docs/latest/api/esm.html#resolution-algorithm-specification).
    ///
    ///  The algorithm uses the file extension or finds the closest `package.json` with the `type` field.
    pub(crate) module_type: Option<ModuleType>,
}

impl Clone for Resolution {
    fn clone(&self) -> Self {
        Self {
            cached_path: self.cached_path.clone(),
            query: self.query.clone(),
            fragment: self.fragment.clone(),
            package_json: self.package_json.clone(),
            module_type: self.module_type,
        }
    }
}

impl fmt::Debug for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resolution")
            .field("path", &self.cached_path.path())
            .field("query", &self.query)
            .field("fragment", &self.fragment)
            .field("module_type", &self.module_type)
            .field("package_json", &self.package_json.as_ref().map(|p| p.path()))
            .finish()
    }
}

impl PartialEq for Resolution {
    fn eq(&self, other: &Self) -> bool {
        self.cached_path.path() == other.cached_path.path()
            && self.query == other.query
            && self.fragment == other.fragment
    }
}
impl Eq for Resolution {}

impl Resolution {
    /// Returns the path without query and fragment
    #[must_use]
    pub fn path(&self) -> &Path {
        self.cached_path.path()
    }

    /// Returns the path without query and fragment
    #[must_use]
    pub fn into_path_buf(self) -> PathBuf {
        self.cached_path.to_path_buf()
    }

    /// Returns the path query `?query`, contains the leading `?`
    #[must_use]
    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    /// Returns the path fragment `#fragment`, contains the leading `#`
    #[must_use]
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }

    /// Returns serialized package_json
    #[must_use]
    pub fn package_json(&self) -> Option<&Arc<PackageJson>> {
        self.package_json.as_ref()
    }

    /// Returns the full path with query and fragment
    #[must_use]
    pub fn full_path(&self) -> PathBuf {
        let path = self.cached_path.path();
        if self.query.is_none() && self.fragment.is_none() {
            return path.to_path_buf();
        }
        let mut os_path = path.as_os_str().to_os_string();
        if let Some(query) = &self.query {
            os_path.push(query);
        }
        if let Some(fragment) = &self.fragment {
            os_path.push(fragment);
        }
        PathBuf::from(os_path)
    }

    /// Returns the module type of this path.
    #[must_use]
    pub fn module_type(&self) -> Option<ModuleType> {
        self.module_type
    }
}
