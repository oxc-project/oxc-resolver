//! package.json definitions
//!
//! Code related to export field are copied from [Parcel's resolver](https://github.com/parcel-bundler/parcel/blob/v2/packages/utils/node-resolver-rs/src/package_json.rs)
use std::path::{Path, PathBuf};

use serde_json::Value as JSONValue;

use crate::{
    ImportsExportsKind, ImportsExportsMap, PackageJson, ResolveError,
    package_json::{ImportsExportsArray, ImportsExportsEntry, PackageType},
    path::PathUtil,
};

pub type JSONMap = serde_json::Map<String, JSONValue>;

/// Serde implementation for the deserialized `package.json`.
///
/// This implementation is used by the [crate::FsCache] and enabled through the
/// `fs_cache` feature.
#[cfg(feature = "fs_cache")]
#[derive(Debug, Default)]
pub struct PackageJsonSerde {
    /// Path to `package.json`. Contains the `package.json` filename.
    pub path: PathBuf,

    /// Realpath to `package.json`. Contains the `package.json` filename.
    pub realpath: PathBuf,

    /// Name of the package.
    pub name: Option<String>,

    /// The "type" field.
    ///
    /// <https://nodejs.org/api/packages.html#type>
    pub r#type: Option<PackageType>,

    /// The "sideEffects" field.
    ///
    /// <https://webpack.js.org/guides/tree-shaking>
    pub side_effects: Option<JSONValue>,

    raw_json: std::sync::Arc<JSONValue>,
}

#[allow(refining_impl_trait)]
impl PackageJson for PackageJsonSerde {
    fn path(&self) -> &Path {
        &self.path
    }

    fn realpath(&self) -> &Path {
        &self.realpath
    }

    fn directory(&self) -> &Path {
        debug_assert!(self.realpath.file_name().is_some_and(|x| x == "package.json"));
        self.realpath.parent().unwrap()
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn r#type(&self) -> Option<PackageType> {
        self.r#type
    }

    fn main_fields<'a>(&'a self, main_fields: &'a [String]) -> impl Iterator<Item = &'a str> + 'a {
        main_fields
            .iter()
            .filter_map(|main_field| self.raw_json.get(main_field))
            .filter_map(JSONValue::as_str)
    }

    fn exports_fields<'a>(
        &'a self,
        exports_fields: &'a [Vec<String>],
    ) -> impl Iterator<Item = ImportsExportsSerdeEntry<'a>> + 'a {
        exports_fields
            .iter()
            .filter_map(|object_path| {
                self.raw_json
                    .as_object()
                    .and_then(|json_object| Self::get_value_by_path(json_object, object_path))
            })
            .map(ImportsExportsSerdeEntry)
    }

    fn imports_fields<'a>(
        &'a self,
        imports_fields: &'a [Vec<String>],
    ) -> impl Iterator<Item = ImportsExportsSerdeMap<'a>> + 'a {
        imports_fields
            .iter()
            .filter_map(|object_path| {
                self.raw_json
                    .as_object()
                    .and_then(|json_object| Self::get_value_by_path(json_object, object_path))
                    .and_then(JSONValue::as_object)
            })
            .map(ImportsExportsSerdeMap)
    }

    fn resolve_browser_field<'a>(
        &'a self,
        path: &Path,
        request: Option<&str>,
        alias_fields: &'a [Vec<String>],
    ) -> Result<Option<&'a str>, ResolveError> {
        for object in self.browser_fields(alias_fields) {
            if let Some(request) = request {
                if let Some(value) = object.get(request) {
                    return Self::alias_value(path, value);
                }
            } else {
                let dir = self.path.parent().unwrap();
                for (key, value) in object {
                    let joined = dir.normalize_with(key);
                    if joined == path {
                        return Self::alias_value(path, value);
                    }
                }
            }
        }
        Ok(None)
    }
}

impl PackageJsonSerde {
    /// # Panics
    /// # Errors
    pub(crate) fn parse(
        path: PathBuf,
        realpath: PathBuf,
        json: &str,
    ) -> Result<Self, serde_json::Error> {
        let mut raw_json: JSONValue = serde_json::from_str(json)?;
        let mut package_json = Self::default();

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

            // Add name, type and sideEffects.
            package_json.name =
                json_object.get("name").and_then(|field| field.as_str()).map(ToString::to_string);
            package_json.r#type =
                json_object.get("type").and_then(|ty| serde_json::from_value(ty.clone()).ok());
            package_json.side_effects = json_object.get("sideEffects").cloned();
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
        let mut value = fields.get(&path[0])?;
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
    #[must_use]
    pub const fn raw_json(&self) -> &std::sync::Arc<JSONValue> {
        &self.raw_json
    }

    /// The "browser" field is provided by a module author as a hint to javascript bundlers or component tools when packaging modules for client side use.
    /// Multiple values are configured by [ResolveOptions::alias_fields].
    ///
    /// <https://github.com/defunctzombie/package-browser-field-spec>
    fn browser_fields<'a>(
        &'a self,
        alias_fields: &'a [Vec<String>],
    ) -> impl Iterator<Item = &'a JSONMap> + 'a {
        alias_fields.iter().filter_map(|object_path| {
            self.raw_json
                .as_object()
                .and_then(|json_object| Self::get_value_by_path(json_object, object_path))
                // Only object is valid, all other types are invalid
                // https://github.com/webpack/enhanced-resolve/blob/3a28f47788de794d9da4d1702a3a583d8422cd48/lib/AliasFieldPlugin.js#L44-L52
                .and_then(|value| value.as_object())
        })
    }

    fn alias_value<'a>(key: &Path, value: &'a JSONValue) -> Result<Option<&'a str>, ResolveError> {
        match value {
            JSONValue::String(value) => Ok(Some(value.as_str())),
            JSONValue::Bool(b) if !b => Err(ResolveError::Ignored(key.to_path_buf())),
            _ => Ok(None),
        }
    }
}

#[derive(Clone)]
pub struct ImportsExportsSerdeEntry<'a>(pub(crate) &'a JSONValue);

impl<'a> ImportsExportsEntry<'a> for ImportsExportsSerdeEntry<'a> {
    type Array = ImportsExportsSerdeArray<'a>;
    type Map = ImportsExportsSerdeMap<'a>;

    fn kind(&self) -> ImportsExportsKind {
        match &self.0 {
            JSONValue::String(_) => ImportsExportsKind::String,
            JSONValue::Array(_) => ImportsExportsKind::Array,
            JSONValue::Object(_) => ImportsExportsKind::Map,
            _ => ImportsExportsKind::Invalid,
        }
    }

    fn as_string(&self) -> Option<&'a str> {
        match &self.0 {
            JSONValue::String(string) => Some(string),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<ImportsExportsSerdeArray<'a>> {
        match &self.0 {
            JSONValue::Array(vec) => Some(ImportsExportsSerdeArray(vec)),
            _ => None,
        }
    }

    fn as_map(&self) -> Option<ImportsExportsSerdeMap<'a>> {
        match &self.0 {
            JSONValue::Object(map) => Some(ImportsExportsSerdeMap(map)),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct ImportsExportsSerdeArray<'a>(&'a Vec<JSONValue>);

impl<'a> ImportsExportsArray<'a> for ImportsExportsSerdeArray<'a> {
    type Entry = ImportsExportsSerdeEntry<'a>;

    fn len(&self) -> usize {
        self.0.len()
    }

    fn iter(&self) -> impl Iterator<Item = ImportsExportsSerdeEntry<'a>> {
        ImportsExportsSerdeArrayIter { vec: self.0, index: 0 }
    }
}

struct ImportsExportsSerdeArrayIter<'a> {
    vec: &'a Vec<JSONValue>,
    index: usize,
}

impl<'a> Iterator for ImportsExportsSerdeArrayIter<'a> {
    type Item = ImportsExportsSerdeEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.vec.get(self.index).map(|value| {
            self.index += 1;
            ImportsExportsSerdeEntry(value)
        })
    }
}

#[derive(Clone)]
pub struct ImportsExportsSerdeMap<'a>(pub(crate) &'a JSONMap);

impl<'a> ImportsExportsMap<'a> for ImportsExportsSerdeMap<'a> {
    type Entry = ImportsExportsSerdeEntry<'a>;

    fn get(&self, key: &str) -> Option<Self::Entry> {
        self.0.get(key).map(ImportsExportsSerdeEntry)
    }

    fn keys(&self) -> impl Iterator<Item = &'a str> {
        ImportsExportsSerdeMapKeysIter { inner: self.0.keys() }
    }

    fn iter(&self) -> impl Iterator<Item = (&'a str, ImportsExportsSerdeEntry<'a>)> {
        ImportsExportsSerdeMapIter { inner: self.0.iter() }
    }
}

struct ImportsExportsSerdeMapIter<'a> {
    inner: serde_json::map::Iter<'a>,
}

impl<'a> Iterator for ImportsExportsSerdeMapIter<'a> {
    type Item = (&'a str, ImportsExportsSerdeEntry<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(key, value)| (key.as_str(), ImportsExportsSerdeEntry(value)))
    }
}

struct ImportsExportsSerdeMapKeysIter<'a> {
    inner: serde_json::map::Keys<'a>,
}

impl<'a> Iterator for ImportsExportsSerdeMapKeysIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(String::as_str)
    }
}
