//! package.json definitions (SIMD implementation for little-endian systems)
//!
//! Code related to export field are copied from [Parcel's resolver](https://github.com/parcel-bundler/parcel/blob/v2/packages/utils/node-resolver-rs/src/package_json.rs)

use std::{
    fmt,
    path::{Path, PathBuf},
};

use self_cell::MutBorrow;
use simd_json::{BorrowedValue, prelude::*};

use super::{ImportsExportsKind, PackageType, SideEffects};
use crate::{FileSystem, JSONError, ResolveError, path::PathUtil, replace_bom_with_whitespace};

// Use simd_json's Object type which handles the hasher correctly based on features
type BorrowedObject<'a> = simd_json::value::borrowed::Object<'a>;

self_cell::self_cell! {
    struct PackageJsonCell {
        owner: MutBorrow<Vec<u8>>,

        #[covariant]
        dependent: BorrowedValue,
    }
}

/// Serde implementation for the deserialized `package.json`.
///
/// This implementation is used by the [crate::Cache] and enabled through the
/// `fs_cache` feature.
pub struct PackageJson {
    /// Path to `package.json`. Contains the `package.json` filename.
    pub path: PathBuf,

    /// Realpath to `package.json`. Contains the `package.json` filename.
    pub realpath: PathBuf,

    cell: PackageJsonCell,
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
        self.cell
            .borrow_dependent()
            .as_object()
            .and_then(|obj| obj.get("name"))
            .and_then(|v| v.as_str())
    }

    /// Version of the package.
    ///
    /// <https://nodejs.org/api/packages.html#name>
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.cell
            .borrow_dependent()
            .as_object()
            .and_then(|obj| obj.get("version"))
            .and_then(|v| v.as_str())
    }

    /// Returns the package type, if one is configured in the `package.json`.
    ///
    /// <https://nodejs.org/api/packages.html#type>
    #[must_use]
    pub fn r#type(&self) -> Option<PackageType> {
        self.cell
            .borrow_dependent()
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
        self.cell.borrow_dependent().as_object().and_then(|obj| obj.get("sideEffects")).and_then(
            |value| match value {
                BorrowedValue::Static(simd_json::StaticNode::Bool(b)) => {
                    Some(SideEffects::Bool(*b))
                }
                BorrowedValue::String(s) => Some(SideEffects::String(s.as_ref())),
                BorrowedValue::Array(arr) => {
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
        self.cell
            .borrow_dependent()
            .as_object()
            .and_then(|obj| obj.get("exports"))
            .map(ImportsExportsEntry)
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
        let json_value = self.cell.borrow_dependent();
        let json_object = json_value.as_object();

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
        let json_value = self.cell.borrow_dependent();

        exports_fields
            .iter()
            .filter_map(move |object_path| {
                json_value
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
        let json_value = self.cell.borrow_dependent();

        imports_fields
            .iter()
            .filter_map(move |object_path| {
                json_value
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
                    let joined = dir.normalize_with(key.as_ref());
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
    /// # Panics
    /// # Errors
    pub fn parse<Fs: FileSystem>(
        fs: &Fs,
        path: PathBuf,
        realpath: PathBuf,
        json: Vec<u8>,
    ) -> Result<Self, JSONError> {
        let mut json = json;
        replace_bom_with_whitespace(&mut json);

        // Check if empty after BOM stripping
        super::check_if_empty(&json, &path)?;

        // Create the self-cell with the JSON bytes and parsed BorrowedValue
        let cell = PackageJsonCell::try_new(MutBorrow::new(json), |bytes| {
            // Use MutBorrow to safely get mutable access for simd_json parsing
            simd_json::to_borrowed_value(bytes.borrow_mut())
        })
        .map_err(|simd_error| {
            // Fallback: re-read the file and parse with serde_json to get detailed error information
            // We re-read because simd_json may have mutated the buffer during its failed parse attempt
            // simd_json doesn't provide line/column info, so we use serde_json for better error messages
            let fallback_result = fs
                .read(&realpath)
                .map_err(|io_error| JSONError {
                    path: path.clone(),
                    message: format!("Failed to re-read file for error reporting: {io_error}"),
                    line: 0,
                    column: 0,
                })
                .and_then(|bytes| {
                    serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|serde_error| {
                        JSONError {
                            path: path.clone(),
                            message: serde_error.to_string(),
                            line: serde_error.line(),
                            column: serde_error.column(),
                        }
                    })
                });

            match fallback_result {
                Ok(_) => {
                    // serde_json succeeded but simd_json failed - this shouldn't happen
                    // for valid JSON, but could indicate simd_json is more strict
                    JSONError {
                        path: path.clone(),
                        message: format!("simd_json parse error: {simd_error}"),
                        line: 0,
                        column: 0,
                    }
                }
                Err(error) => error,
            }
        })?;

        Ok(Self { path, realpath, cell })
    }

    fn get_value_by_path<'a>(
        fields: &'a BorrowedObject<'a>,
        path: &[String],
    ) -> Option<&'a BorrowedValue<'a>> {
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
    ) -> impl Iterator<Item = &'a BorrowedObject<'a>> + 'a {
        let json_value = self.cell.borrow_dependent();

        alias_fields.iter().filter_map(move |object_path| {
            json_value
                .as_object()
                .and_then(|json_object| Self::get_value_by_path(json_object, object_path))
                // Only object is valid, all other types are invalid
                // https://github.com/webpack/enhanced-resolve/blob/3a28f47788de794d9da4d1702a3a583d8422cd48/lib/AliasFieldPlugin.js#L44-L52
                .and_then(|value| value.as_object())
        })
    }

    pub(crate) fn alias_value<'a>(
        key: &Path,
        value: &'a BorrowedValue<'a>,
    ) -> Result<Option<&'a str>, ResolveError> {
        match value {
            BorrowedValue::String(s) => Ok(Some(s.as_ref())),
            BorrowedValue::Static(simd_json::StaticNode::Bool(false)) => {
                Err(ResolveError::Ignored(key.to_path_buf()))
            }
            _ => Ok(None),
        }
    }
}

#[derive(Clone)]
pub struct ImportsExportsEntry<'a>(pub(crate) &'a BorrowedValue<'a>);

impl<'a> ImportsExportsEntry<'a> {
    #[must_use]
    pub fn kind(&self) -> ImportsExportsKind {
        match self.0 {
            BorrowedValue::String(_) => ImportsExportsKind::String,
            BorrowedValue::Array(_) => ImportsExportsKind::Array,
            BorrowedValue::Object(_) => ImportsExportsKind::Map,
            BorrowedValue::Static(_) => ImportsExportsKind::Invalid,
        }
    }

    #[must_use]
    pub fn as_string(&self) -> Option<&'a str> {
        match self.0 {
            BorrowedValue::String(s) => Some(s.as_ref()),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_array(&self) -> Option<ImportsExportsArray<'a>> {
        match self.0 {
            BorrowedValue::Array(arr) => Some(ImportsExportsArray(arr)),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_map(&self) -> Option<ImportsExportsMap<'a>> {
        match self.0 {
            BorrowedValue::Object(obj) => Some(ImportsExportsMap(obj)),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct ImportsExportsArray<'a>(&'a [BorrowedValue<'a>]);

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
    slice: &'a [BorrowedValue<'a>],
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
pub struct ImportsExportsMap<'a>(pub(crate) &'a BorrowedObject<'a>);

impl<'a> ImportsExportsMap<'a> {
    pub fn get(&self, key: &str) -> Option<ImportsExportsEntry<'a>> {
        self.0.get(key).map(ImportsExportsEntry)
    }

    pub fn keys(&self) -> impl Iterator<Item = &'a str> {
        self.0.keys().map(std::convert::AsRef::as_ref)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'a str, ImportsExportsEntry<'a>)> {
        self.0.iter().map(|(k, v)| (k.as_ref(), ImportsExportsEntry(v)))
    }
}
