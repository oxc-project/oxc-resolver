//! package.json definitions
//!
//! Code related to export field are copied from [Parcel's resolver](https://github.com/parcel-bundler/parcel/blob/v2/packages/utils/node-resolver-rs/src/package_json.rs)
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use nodejs_package_json::{
    BrowserField, ImportExportField, ImportExportMap, PackageJson as BasePackageJson,
};
use serde::Deserialize;

use crate::{path::PathUtil, ResolveError, ResolveOptions};

/// Deserialized package.json
#[derive(Debug, Deserialize)]
pub struct PackageJson {
    /// Path to `package.json`. Contains the `package.json` filename.
    #[serde(skip)]
    pub(crate) path: PathBuf,

    /// Realpath to `package.json`. Contains the `package.json` filename.
    #[serde(skip)]
    pub(crate) realpath: PathBuf,

    #[serde(skip)]
    pub(crate) raw_json: Arc<serde_json::Value>,

    /// The "name" field defines your package's name.
    /// The "name" field can be used in addition to the "exports" field to self-reference a package using its name.
    ///
    /// <https://nodejs.org/api/packages.html#name>
    pub(crate) name: Option<String>,

    /// The "main" field defines the entry point of a package when imported by name via a node_modules lookup. Its value is a path.
    /// When a package has an "exports" field, this will take precedence over the "main" field when importing the package by name.
    ///
    /// Values are dynamically added from [ResolveOptions::main_fields].
    ///
    /// <https://nodejs.org/api/packages.html#main>
    #[serde(skip)]
    pub(crate) main_fields: Vec<String>,

    /// The "exports" field allows defining the entry points of a package when imported by name loaded either via a node_modules lookup or a self-reference to its own name.
    ///
    /// <https://nodejs.org/api/packages.html#exports>
    #[serde(skip)]
    pub(crate) exports: Vec<ImportExportField>,

    /// In addition to the "exports" field, there is a package "imports" field to create private mappings that only apply to import specifiers from within the package itself.
    ///
    /// <https://nodejs.org/api/packages.html#subpath-imports>
    #[serde(default)]
    pub(crate) imports: Box<ImportExportMap>,

    /// The "browser" field is provided by a module author as a hint to javascript bundlers or component tools when packaging modules for client side use.
    /// Multiple values are configured by [ResolveOptions::alias_fields].
    ///
    /// <https://github.com/defunctzombie/package-browser-field-spec>
    #[serde(skip)]
    pub(crate) browser_fields: Vec<BrowserField>,
}

impl PackageJson {
    /// # Panics
    /// # Errors
    pub(crate) fn parse(
        path: PathBuf,
        realpath: PathBuf,
        json: &str,
        options: &ResolveOptions,
    ) -> Result<Self, serde_json::Error> {
        // let data: BasePackageJson = serde_json::from_str(json)?;

        let mut package_json_value: serde_json::Value = serde_json::from_str(json)?;
        let mut package_json: Self = Self::deserialize(&package_json_value)?;
        package_json.main_fields.reserve_exact(options.main_fields.len());
        package_json.browser_fields.reserve_exact(options.alias_fields.len());
        package_json.exports.reserve_exact(options.exports_fields.len());

        if let Some(package_json_value) = package_json_value.as_object_mut() {
            // Dynamically create `main_fields`.
            for main_field_key in &options.main_fields {
                // Using `get` + `clone` instead of remove here
                // because `main_fields` may contain `browser`, which is also used in `browser_fields.
                if let Some(serde_json::Value::String(value)) =
                    package_json_value.get(main_field_key)
                {
                    package_json.main_fields.push(value.clone());
                }
            }
        }

        // Dynamically create `browser_fields`.
        let dir = path.parent().unwrap();
        for object_path in &options.alias_fields {
            if let Some(browser_field) = Self::get_value_by_path(&package_json_value, object_path) {
                let mut browser_field = BrowserField::deserialize(browser_field)?;
                // Normalize all relative paths to make browser_field a constant value lookup
                if let BrowserField::Map(map) = &mut browser_field {
                    let keys = map.keys().cloned().collect::<Vec<_>>();
                    for key in keys {
                        // Normalize the key if it looks like a file "foo.js"
                        if key.extension().is_some() {
                            map.insert(dir.normalize_with(&key), map[&key].clone());
                        }
                        // Normalize the key if it is relative path "./relative"
                        if key.starts_with(".") {
                            if let Some(value) = map.remove(&key) {
                                map.insert(dir.normalize_with(&key), value);
                            }
                        }
                    }
                }
                package_json.browser_fields.push(browser_field);
            }
        }

        // Dynamically create `exports`.
        for object_path in &options.exports_fields {
            if let Some(exports) = Self::get_value_by_path(&package_json_value, object_path) {
                let exports = ImportExportField::deserialize(exports)?;
                package_json.exports.push(exports);
            }
        }

        package_json.path = path;
        package_json.realpath = realpath;

        // Remove large fields that useless for pragmatic use, see [raw_json] API
        if let Some(package_json_value) = package_json_value.as_object_mut() {
            package_json_value.remove("description");
            package_json_value.remove("keywords");
            package_json_value.remove("scripts");
            package_json_value.remove("dependencies");
            package_json_value.remove("devDependencies");
            package_json_value.remove("peerDependencies");
            package_json_value.remove("optionalDependencies");
        }

        package_json.raw_json = Arc::new(package_json_value);
        Ok(package_json)
    }

    fn get_value_by_path<'a>(
        package_json: &'a serde_json::Value,
        path: &[String],
    ) -> Option<&'a serde_json::Value> {
        if path.is_empty() {
            return None;
        }
        let mut value = package_json;
        for key in path {
            if let Some(inner_value) = value.as_object().and_then(|o| o.get(key)) {
                value = inner_value;
            } else {
                return None;
            }
        }
        Some(value)
    }

    /// Raw serde json value of `package.json`.
    ///
    /// This is currently used in Rspack for:
    /// * getting the `sideEffects` field
    /// * query in <https://www.rspack.dev/config/module.html#ruledescriptiondata> - search on GitHub indicates query on the `type` field.
    ///
    /// To reduce overall memory consumption, large fields that useless for pragmatic use are removed.
    /// They are: `description`, `keywords`, `scripts`,
    /// `dependencies` and `devDependencies`, `peerDependencies`, `optionalDependencies`.
    pub fn raw_json(&self) -> &Arc<serde_json::Value> {
        &self.raw_json
    }

    /// Directory to `package.json`
    ///
    /// # Panics
    ///
    /// * When the package.json path is misconfigured.
    pub fn directory(&self) -> &Path {
        debug_assert!(self.realpath.file_name().is_some_and(|x| x == "package.json"));
        self.realpath.parent().unwrap()
    }

    /// Resolve the request string for this package.json by looking at the `browser` field.
    ///
    /// # Errors
    ///
    /// * Returns [ResolveError::Ignored] for `"path": false` in `browser` field.
    pub(crate) fn resolve_browser_field(
        &self,
        path: &Path,
        request: Option<&str>,
    ) -> Result<Option<&str>, ResolveError> {
        if self.browser_fields.is_empty() {
            return Ok(None);
        }
        let request = request.map_or(path, Path::new);
        for browser in &self.browser_fields {
            // Only object is valid, all other types are invalid
            // https://github.com/webpack/enhanced-resolve/blob/3a28f47788de794d9da4d1702a3a583d8422cd48/lib/AliasFieldPlugin.js#L44-L52
            if let BrowserField::Map(field_data) = browser {
                if let Some(value) = field_data.get(request) {
                    return Self::alias_value(path, value);
                }
            }
        }
        Ok(None)
    }

    fn alias_value<'a>(
        key: &Path,
        value: &'a serde_json::Value,
    ) -> Result<Option<&'a str>, ResolveError> {
        match value {
            serde_json::Value::String(value) => Ok(Some(value.as_str())),
            serde_json::Value::Bool(b) if !b => Err(ResolveError::Ignored(key.to_path_buf())),
            _ => Ok(None),
        }
    }
}
