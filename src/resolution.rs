use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{Cache, PackageJson};

/// The final path resolution with optional `?query` and `#fragment`
pub struct Resolution<C: Cache> {
    pub(crate) path: PathBuf,

    /// path query `?query`, contains `?`.
    pub(crate) query: Option<String>,

    /// path fragment `#query`, contains `#`.
    pub(crate) fragment: Option<String>,

    pub(crate) package_json: Option<Arc<C::Pj>>,
}

impl<C: Cache> Clone for Resolution<C> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            query: self.query.clone(),
            fragment: self.fragment.clone(),
            package_json: self.package_json.clone(),
        }
    }
}

impl<C: Cache> fmt::Debug for Resolution<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resolution")
            .field("path", &self.path)
            .field("query", &self.query)
            .field("fragment", &self.fragment)
            .field("package_json", &self.package_json.as_ref().map(|p| p.path()))
            .finish()
    }
}

impl<C: Cache> PartialEq for Resolution<C> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.query == other.query && self.fragment == other.fragment
    }
}
impl<C: Cache> Eq for Resolution<C> {}

impl<C: Cache> Resolution<C> {
    /// Returns the path without query and fragment
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the path without query and fragment
    #[must_use]
    pub fn into_path_buf(self) -> PathBuf {
        self.path
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
    pub const fn package_json(&self) -> Option<&Arc<C::Pj>> {
        self.package_json.as_ref()
    }

    /// Returns the full path with query and fragment
    #[must_use]
    pub fn full_path(&self) -> PathBuf {
        let mut path = self.path.clone().into_os_string();
        if let Some(query) = &self.query {
            path.push(query);
        }
        if let Some(fragment) = &self.fragment {
            path.push(fragment);
        }
        PathBuf::from(path)
    }
}
