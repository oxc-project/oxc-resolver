use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::PackageJson;

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
    pub(crate) path: PathBuf,

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

    /// Whether the resolution succeeded by matching a TypeScript extension
    /// that was explicitly written in the specifier.
    ///
    /// # Examples
    ///
    /// ```
    /// // Specifier: "./foo.ts" → Resolved: "/project/foo.ts"
    /// // resolved_using_ts_extension = true
    ///
    /// // Specifier: "./foo" → Resolved: "/project/foo.ts"
    /// // resolved_using_ts_extension = false
    ///
    /// // Specifier: "./foo.js" → Resolved: "/project/foo.ts"
    /// // resolved_using_ts_extension = false (specifier had .js, not .ts)
    /// ```
    pub(crate) resolved_using_ts_extension: bool,
}

impl Clone for Resolution {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            query: self.query.clone(),
            fragment: self.fragment.clone(),
            package_json: self.package_json.clone(),
            module_type: self.module_type,
            resolved_using_ts_extension: self.resolved_using_ts_extension,
        }
    }
}

impl fmt::Debug for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resolution")
            .field("path", &self.path)
            .field("query", &self.query)
            .field("fragment", &self.fragment)
            .field("module_type", &self.module_type)
            .field("package_json", &self.package_json.as_ref().map(|p| p.path()))
            .field("resolved_using_ts_extension", &self.resolved_using_ts_extension)
            .finish()
    }
}

impl PartialEq for Resolution {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.query == other.query && self.fragment == other.fragment
    }
}
impl Eq for Resolution {}

impl Resolution {
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
    pub fn package_json(&self) -> Option<&Arc<PackageJson>> {
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

    /// Returns the module type of this path.
    #[must_use]
    pub fn module_type(&self) -> Option<ModuleType> {
        self.module_type
    }

    /// Returns whether the resolution succeeded by matching a TypeScript extension
    /// that was explicitly written in the specifier.
    ///
    /// This is `true` when:
    /// - The specifier contains a TypeScript extension (`.ts`, `.tsx`, `.mts`, `.cts`, `.d.ts`, `.d.mts`, `.d.cts`)
    /// - The resolved file has the same extension as the specifier
    ///
    /// # Examples
    ///
    /// ```
    /// // Specifier: "./foo.ts" → Resolved: "/project/foo.ts"
    /// // returns true
    ///
    /// // Specifier: "./foo" → Resolved: "/project/foo.ts"
    /// // returns false
    ///
    /// // Specifier: "./foo.js" → Resolved: "/project/foo.ts" (via extensionAlias)
    /// // returns false
    /// ```
    #[must_use]
    pub const fn resolved_using_ts_extension(&self) -> bool {
        self.resolved_using_ts_extension
    }
}
