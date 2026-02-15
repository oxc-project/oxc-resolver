//! package.json definitions (serde implementation for big-endian systems)
//!
//! Code related to export field are copied from [Parcel's resolver](https://github.com/parcel-bundler/parcel/blob/v2/packages/utils/node-resolver-rs/src/package_json.rs)

use std::{
    fmt,
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::{FileSystem, JSONError, ResolveError, path::PathUtil, replace_bom_with_whitespace};

use super::{ImportsExportsKind, PackageType, SideEffects};

/// Serde implementation for the deserialized `package.json`.
///
/// This implementation is used on big-endian systems where simd-json is not available.
pub struct PackageJson {
    /// Path to `package.json`. Contains the `package.json` filename.
    pub path: PathBuf,

    /// Realpath to `package.json`. Contains the `package.json` filename.
    pub realpath: PathBuf,

    /// Parsed JSON value
    value: Value,
}

impl fmt::Debug for PackageJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackageJson")
            .field("path", &self.path)
            .field("realpath", &self.realpath)
            .field("name", &self.name())
            .field("type", &self.r#type())
            .finish_non_exhaustive()
    }
}

impl PackageJson {
    /// Returns the path where the `package.json` was found.
    ///
    /// Contains the `package.json` filename.
    ///
    /// This does not need to be the path where the file is stored on disk.
    /// See [Self::realpath()].
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the path where the `package.json` file was stored on disk.
    ///
    /// Contains the `package.json` filename.
    ///
    /// This is the canonicalized version of [Self::path()], where all symbolic
    /// links are resolved.
    #[must_use]
    pub fn realpath(&self) -> &Path {
        &self.realpath
    }

    /// Directory to `package.json`.
    ///
    /// # Panics
    ///
    /// * When the `package.json` path is misconfigured.
    #[must_use]
    pub fn directory(&self) -> &Path {
        debug_assert!(self.realpath.file_name().is_some_and(|x| x == "package.json"));
        self.realpath.parent().unwrap()
    }

    /// Name of the package.
    ///
    /// The "name" field can be used together with the "exports" field to
    /// self-reference a package using its name.
    ///
    /// <https://nodejs.org/api/packages.html#name>
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.value.as_object().and_then(|obj| obj.get("name")).and_then(|v| v.as_str())
    }

    /// Version of the package.
    ///
    /// <https://nodejs.org/api/packages.html#name>
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.value.as_object().and_then(|obj| obj.get("version")).and_then(|v| v.as_str())
    }

    /// Returns the package type, if one is configured in the `package.json`.
    ///
    /// <https://nodejs.org/api/packages.html#type>
    #[must_use]
    pub fn r#type(&self) -> Option<PackageType> {
        self.value
            .as_object()
            .and_then(|obj| obj.get("type"))
            .and_then(|v| v.as_str())
            .and_then(PackageType::from_str)
    }

    /// The "sideEffects" field.
    ///
    /// <https://webpack.js.org/guides/tree-shaking>
    #[must_use]
    pub fn side_effects(&self) -> Option<SideEffects<'_>> {
        self.value.as_object().and_then(|obj| obj.get("sideEffects")).and_then(
            |value| match value {
                Value::Bool(b) => Some(SideEffects::Bool(*b)),
                Value::String(s) => Some(SideEffects::String(s.as_str())),
                Value::Array(arr) => {
                    let strings: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
                    Some(SideEffects::Array(strings))
                }
                _ => None,
            },
        )
    }

    /// The "exports" field allows defining the entry points of a package.
    ///
    /// <https://nodejs.org/api/packages.html#exports>
    #[must_use]
    pub fn exports(&self) -> Option<ImportsExportsEntry<'_>> {
        self.value.as_object().and_then(|obj| obj.get("exports")).map(ImportsExportsEntry)
    }

    /// The "types" field in package.json.
    ///
    /// Used by TypeScript to find type declarations for a package.
    #[must_use]
    pub fn types(&self) -> Option<&str> {
        self.value.as_object().and_then(|obj| obj.get("types")).and_then(|v| v.as_str())
    }

    /// The "typings" field in package.json (legacy equivalent of "types").
    ///
    /// Used by TypeScript to find type declarations for a package.
    #[must_use]
    pub fn typings(&self) -> Option<&str> {
        self.value.as_object().and_then(|obj| obj.get("typings")).and_then(|v| v.as_str())
    }

    /// The "typesVersions" field in package.json.
    ///
    /// Returns the raw JSON value for the "typesVersions" field, which maps
    /// TypeScript version ranges to path redirect maps.
    ///
    /// <https://www.typescriptlang.org/docs/handbook/declaration-files/publishing.html#version-selection-with-typesversions>
    pub(crate) fn types_versions(&self) -> Option<ImportsExportsMap<'_>> {
        self.value
            .as_object()
            .and_then(|obj| obj.get("typesVersions"))
            .and_then(|v| v.as_object())
            .map(ImportsExportsMap)
    }

    /// The "main" field defines the entry point of a package when imported by
    /// name via a node_modules lookup. Its value should be a path.
    ///
    /// When a package has an "exports" field, this will take precedence over
    /// the "main" field when importing the package by name.
    ///
    /// Values are dynamically retrieved from [crate::ResolveOptions::main_fields].
    ///
    /// <https://nodejs.org/api/packages.html#main>
    pub(crate) fn main_fields<'a>(
        &'a self,
        main_fields: &'a [String],
    ) -> impl Iterator<Item = &'a str> + 'a {
        let json_object = self.value.as_object();

        main_fields
            .iter()
            .filter_map(move |main_field| json_object.and_then(|obj| obj.get(main_field.as_str())))
            .filter_map(|v| v.as_str())
    }

    /// The "exports" field allows defining the entry points of a package when
    /// imported by name loaded either via a node_modules lookup or a
    /// self-reference to its own name.
    ///
    /// <https://nodejs.org/api/packages.html#exports>
    pub(crate) fn exports_fields<'a>(
        &'a self,
        exports_fields: &'a [Vec<String>],
    ) -> impl Iterator<Item = ImportsExportsEntry<'a>> + 'a {
        exports_fields
            .iter()
            .filter_map(move |object_path| {
                self.value
                    .as_object()
                    .and_then(|json_object| Self::get_value_by_path(json_object, object_path))
            })
            .map(ImportsExportsEntry)
    }

    /// In addition to the "exports" field, there is a package "imports" field
    /// to create private mappings that only apply to import specifiers from
    /// within the package itself.
    ///
    /// <https://nodejs.org/api/packages.html#subpath-imports>
    pub(crate) fn imports_fields<'a>(
        &'a self,
        imports_fields: &'a [Vec<String>],
    ) -> impl Iterator<Item = ImportsExportsMap<'a>> + 'a {
        imports_fields
            .iter()
            .filter_map(move |object_path| {
                self.value
                    .as_object()
                    .and_then(|json_object| Self::get_value_by_path(json_object, object_path))
                    .and_then(|v| v.as_object())
            })
            .map(ImportsExportsMap)
    }

    /// Resolves the request string for this `package.json` by looking at the
    /// "browser" field.
    ///
    /// <https://github.com/defunctzombie/package-browser-field-spec>
    pub(crate) fn resolve_browser_field<'a>(
        &'a self,
        path: &Path,
        request: Option<&str>,
        alias_fields: &'a [Vec<String>],
    ) -> Result<Option<&'a str>, ResolveError> {
        for object in self.browser_fields(alias_fields) {
            if let Some(request) = request {
                // Find matching key in object
                if let Some(value) = object.get(request) {
                    return Self::alias_value(path, value);
                }
            } else {
                let dir = self.path.parent().unwrap();
                for (key, value) in object {
                    let joined = dir.normalize_with(key.as_str());
                    if joined == path {
                        return Self::alias_value(path, value);
                    }
                }
            }
        }
        Ok(None)
    }

    /// Parse a package.json file from JSON bytes
    ///
    /// # Errors
    pub fn parse<Fs: FileSystem>(
        _fs: &Fs,
        path: PathBuf,
        realpath: PathBuf,
        json: Vec<u8>,
    ) -> Result<Self, JSONError> {
        let mut json = json;
        replace_bom_with_whitespace(&mut json);
        super::check_if_empty(&json, &path)?;
        let value = serde_json::from_slice::<Value>(&json).map_err(|error| JSONError {
            path: path.clone(),
            message: error.to_string(),
            line: error.line(),
            column: error.column(),
        })?;
        Ok(Self { path, realpath, value })
    }

    fn get_value_by_path<'a>(
        fields: &'a serde_json::Map<String, Value>,
        path: &[String],
    ) -> Option<&'a Value> {
        if path.is_empty() {
            return None;
        }
        let mut value = fields.get(path[0].as_str())?;

        for key in path.iter().skip(1) {
            if let Some(obj) = value.as_object() {
                value = obj.get(key.as_str())?;
            } else {
                return None;
            }
        }
        Some(value)
    }

    /// The "browser" field is provided by a module author as a hint to javascript bundlers or component tools when packaging modules for client side use.
    /// Multiple values are configured by [ResolveOptions::alias_fields].
    ///
    /// <https://github.com/defunctzombie/package-browser-field-spec>
    pub(crate) fn browser_fields<'a>(
        &'a self,
        alias_fields: &'a [Vec<String>],
    ) -> impl Iterator<Item = &'a serde_json::Map<String, Value>> + 'a {
        alias_fields.iter().filter_map(move |object_path| {
            self.value
                .as_object()
                .and_then(|json_object| Self::get_value_by_path(json_object, object_path))
                // Only object is valid, all other types are invalid
                // https://github.com/webpack/enhanced-resolve/blob/3a28f47788de794d9da4d1702a3a583d8422cd48/lib/AliasFieldPlugin.js#L44-L52
                .and_then(|value| value.as_object())
        })
    }

    pub(crate) fn alias_value<'a>(
        key: &Path,
        value: &'a Value,
    ) -> Result<Option<&'a str>, ResolveError> {
        match value {
            Value::String(s) => Ok(Some(s.as_str())),
            Value::Bool(false) => Err(ResolveError::Ignored(key.to_path_buf())),
            _ => Ok(None),
        }
    }
}

#[derive(Clone)]
pub struct ImportsExportsEntry<'a>(pub(crate) &'a Value);

impl<'a> ImportsExportsEntry<'a> {
    #[must_use]
    pub fn kind(&self) -> ImportsExportsKind {
        match self.0 {
            Value::String(_) => ImportsExportsKind::String,
            Value::Array(_) => ImportsExportsKind::Array,
            Value::Object(_) => ImportsExportsKind::Map,
            _ => ImportsExportsKind::Invalid,
        }
    }

    #[must_use]
    pub fn as_string(&self) -> Option<&'a str> {
        match self.0 {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_array(&self) -> Option<ImportsExportsArray<'a>> {
        match self.0 {
            Value::Array(arr) => Some(ImportsExportsArray(arr)),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_map(&self) -> Option<ImportsExportsMap<'a>> {
        match self.0 {
            Value::Object(obj) => Some(ImportsExportsMap(obj)),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct ImportsExportsArray<'a>(&'a [Value]);

impl<'a> ImportsExportsArray<'a> {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = ImportsExportsEntry<'a>> {
        ImportsExportsArrayIter { slice: self.0, index: 0 }
    }
}

struct ImportsExportsArrayIter<'a> {
    slice: &'a [Value],
    index: usize,
}

impl<'a> Iterator for ImportsExportsArrayIter<'a> {
    type Item = ImportsExportsEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.slice.get(self.index).map(|value| {
            self.index += 1;
            ImportsExportsEntry(value)
        })
    }
}

#[derive(Clone)]
pub struct ImportsExportsMap<'a>(pub(crate) &'a serde_json::Map<String, Value>);

impl<'a> ImportsExportsMap<'a> {
    pub fn get(&self, key: &str) -> Option<ImportsExportsEntry<'a>> {
        self.0.get(key).map(ImportsExportsEntry)
    }

    pub fn keys(&self) -> impl Iterator<Item = &'a str> {
        self.0.keys().map(String::as_str)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'a str, ImportsExportsEntry<'a>)> {
        self.0.iter().map(|(k, v)| (k.as_str(), ImportsExportsEntry(v)))
    }
}
