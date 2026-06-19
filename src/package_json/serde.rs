//! package.json backend for big-endian systems (serde_json, owned `Value`).
//!
//! The accessor logic lives in [`super`]; this module only provides the storage
//! (an owned [`Value`]), the [`JsonValue`]/[`JsonObject`] implementations, and `parse`.

use std::path::PathBuf;

use serde_json::{Map, Value};

use super::{
    ImportsExportsArrayGeneric, ImportsExportsEntryGeneric, ImportsExportsKind,
    ImportsExportsMapGeneric, JsonObject, JsonValue, PackageJsonBackend, PackageJsonGeneric,
};
use crate::{FileSystem, JSONError, replace_bom_with_whitespace};

/// `package.json` parsed with serde_json (an owned `Value`).
pub type PackageJson = PackageJsonGeneric<Value>;
pub type ImportsExportsEntry<'a> = ImportsExportsEntryGeneric<'a, Value>;
pub type ImportsExportsArray<'a> = ImportsExportsArrayGeneric<'a, Value>;
pub type ImportsExportsMap<'a> = ImportsExportsMapGeneric<'a, Map<String, Value>>;

impl JsonValue for Value {
    type Object = Map<String, Value>;

    fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn as_slice(&self) -> Option<&[Self]> {
        match self {
            Value::Array(arr) => Some(arr.as_slice()),
            _ => None,
        }
    }

    // `Self::Object` would be ambiguous with the `Value::Object` variant, so name the concrete
    // object type.
    fn as_object(&self) -> Option<&Map<String, Value>> {
        match self {
            Value::Object(obj) => Some(obj),
            _ => None,
        }
    }

    fn entry_kind(&self) -> ImportsExportsKind {
        match self {
            Value::String(_) => ImportsExportsKind::String,
            Value::Array(_) => ImportsExportsKind::Array,
            Value::Object(_) => ImportsExportsKind::Map,
            _ => ImportsExportsKind::Invalid,
        }
    }
}

impl JsonObject for Map<String, Value> {
    type Value = Value;

    fn get(&self, key: &str) -> Option<&Self::Value> {
        // Inherent `Map::get` (takes priority over the trait method being defined).
        self.get(key)
    }

    fn iter(&self) -> impl Iterator<Item = (&str, &Self::Value)> {
        // Inherent `Map::iter` (takes priority over the trait method being defined).
        self.iter().map(|(key, value)| (key.as_str(), value))
    }
}

impl PackageJsonBackend for Value {
    type Value<'a> = Value;

    fn root(&self) -> &Self::Value<'_> {
        self
    }
}

impl PackageJson {
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
        Ok(Self { path, realpath, store: value })
    }
}
