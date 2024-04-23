//! package.json definitions
//!
//! Code related to export field are copied from [Parcel's resolver](https://github.com/parcel-bundler/parcel/blob/v2/packages/utils/node-resolver-rs/src/package_json.rs)
use std::path::{Path, PathBuf};

use nodejs_package_json::BrowserField;
use serde::Deserialize;
use serde_json::Value as JSONValue;

use crate::{path::PathUtil, ResolveError, ResolveOptions};

pub type ImportExportMap = serde_json::Map<String, JSONValue>;

/// Deserialized package.json
#[derive(Debug, Default)]
pub struct PackageJson {
    /// Path to `package.json`. Contains the `package.json` filename.
    pub path: PathBuf,

    /// Realpath to `package.json`. Contains the `package.json` filename.
    pub realpath: PathBuf,

    raw_json: std::sync::Arc<JSONValue>,

    /// The "name" field defines your package's name.
    /// The "name" field can be used in addition to the "exports" field to self-reference a package using its name.
    ///
    /// <https://nodejs.org/api/packages.html#name>
    pub name: Option<String>,

    /// The "browser" field is provided by a module author as a hint to javascript bundlers or component tools when packaging modules for client side use.
    /// Multiple values are configured by [ResolveOptions::alias_fields].
    ///
    /// <https://github.com/defunctzombie/package-browser-field-spec>
    pub browser_fields: Vec<BrowserField>,
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
        let mut raw_json: JSONValue = serde_json::from_str(json)?;
        let mut package_json = Self::default();

        package_json.browser_fields.reserve_exact(options.alias_fields.len());

        if let Some(json_object) = raw_json.as_object_mut() {
            // Remove large fields that are useless for pragmatic use.
            #[cfg(feature = "package_json_raw_json_api")]
            {
                json_object.remove("description");
                json_object.remove("keywords");
                json_object.remove("scripts");
                json_object.remove("dependencies");
                json_object.remove("devDependencies");
                json_object.remove("peerDependencies");
                json_object.remove("optionalDependencies");
            }

            // Add name.
            package_json.name =
                json_object.get("name").and_then(|field| field.as_str()).map(ToString::to_string);

            // Dynamically create `browser_fields`.
            let dir = path.parent().unwrap();
            for object_path in &options.alias_fields {
                if let Some(browser_field) = Self::get_value_by_path(json_object, object_path) {
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
        }

        package_json.path = path;
        package_json.realpath = realpath;
        package_json.raw_json = std::sync::Arc::new(raw_json);
        Ok(package_json)
    }

    fn get_value_by_path<'a>(
        fields: &'a serde_json::Map<String, JSONValue>,
        path: &[String],
    ) -> Option<&'a JSONValue> {
        if path.is_empty() {
            return None;
        }
        let Some(mut value) = fields.get(&path[0]) else {
            return None;
        };
        for key in path.iter().skip(1) {
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
    #[cfg(feature = "package_json_raw_json_api")]
    pub fn raw_json(&self) -> &std::sync::Arc<JSONValue> {
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

    /// The "main" field defines the entry point of a package when imported by name via a node_modules lookup. Its value is a path.
    ///
    /// When a package has an "exports" field, this will take precedence over the "main" field when importing the package by name.
    ///
    /// Values are dynamically retrieved from [ResolveOptions::main_fields].
    ///
    /// <https://nodejs.org/api/packages.html#main>
    pub(crate) fn main_fields<'a>(
        &'a self,
        main_fields: &'a [String],
    ) -> impl Iterator<Item = &'a str> + '_ {
        main_fields
            .iter()
            .filter_map(|main_field| self.raw_json.get(main_field))
            .filter_map(|value| value.as_str())
    }

    /// The "exports" field allows defining the entry points of a package when imported by name loaded either via a node_modules lookup or a self-reference to its own name.
    ///
    /// <https://nodejs.org/api/packages.html#exports>
    pub(crate) fn exports_fields<'a>(
        &'a self,
        exports_fields: &'a [Vec<String>],
    ) -> impl Iterator<Item = &'a JSONValue> + '_ {
        exports_fields.iter().filter_map(|object_path| {
            self.raw_json
                .as_object()
                .and_then(|json_object| Self::get_value_by_path(json_object, object_path))
        })
    }

    /// In addition to the "exports" field, there is a package "imports" field to create private mappings that only apply to import specifiers from within the package itself.
    ///
    /// <https://nodejs.org/api/packages.html#subpath-imports>
    pub(crate) fn imports_fields<'a>(
        &'a self,
        imports_fields: &'a [Vec<String>],
    ) -> impl Iterator<Item = &'a ImportExportMap> + '_ {
        imports_fields.iter().filter_map(|object_path| {
            self.raw_json
                .as_object()
                .and_then(|json_object| Self::get_value_by_path(json_object, object_path))
                .and_then(|value| value.as_object())
        })
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

    fn alias_value<'a>(key: &Path, value: &'a JSONValue) -> Result<Option<&'a str>, ResolveError> {
        match value {
            JSONValue::String(value) => Ok(Some(value.as_str())),
            JSONValue::Bool(b) if !b => Err(ResolveError::Ignored(key.to_path_buf())),
            _ => Ok(None),
        }
    }
}
