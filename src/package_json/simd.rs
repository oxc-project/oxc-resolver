//! package.json backend for little-endian systems (simd-json, zero-copy `BorrowedValue`).
//!
//! The accessor logic lives in [`super`]; this module only provides the storage
//! ([`PackageJsonCell`]), the [`JsonValue`]/[`JsonObject`] implementations, and `parse`.

// `self_cell!` generates `pub` constructors with `impl FnOnce` parameters.
#![allow(clippy::impl_trait_in_params)]

use std::path::PathBuf;

use self_cell::MutBorrow;
use simd_json::BorrowedValue;

use super::{
    ImportsExportsArrayGeneric, ImportsExportsEntryGeneric, ImportsExportsKind,
    ImportsExportsMapGeneric, JsonObject, JsonValue, PackageJsonBackend, PackageJsonGeneric,
};
use crate::{FileSystem, JSONError, replace_bom_with_whitespace};

// Use simd_json's Object type which handles the hasher correctly based on features
type BorrowedObject<'a> = simd_json::value::borrowed::Object<'a>;

// `pub` because it appears as the type parameter of the public `PackageJson` alias. The
// `self_cell!`-generated `pub` constructors take `impl FnOnce`; the module-level
// `allow(clippy::impl_trait_in_params)` above silences clippy for those generated items.
self_cell::self_cell! {
    pub struct PackageJsonCell {
        owner: MutBorrow<Vec<u8>>,

        #[covariant]
        dependent: BorrowedValue,
    }
}

/// `package.json` parsed with simd-json (the parsed `BorrowedValue` borrows the file bytes).
pub type PackageJson = PackageJsonGeneric<PackageJsonCell>;
pub type ImportsExportsEntry<'a> = ImportsExportsEntryGeneric<'a, BorrowedValue<'a>>;
pub type ImportsExportsArray<'a> = ImportsExportsArrayGeneric<'a, BorrowedValue<'a>>;
pub type ImportsExportsMap<'a> = ImportsExportsMapGeneric<'a, BorrowedObject<'a>>;

impl<'a> JsonValue for BorrowedValue<'a> {
    type Object = BorrowedObject<'a>;

    fn as_str(&self) -> Option<&str> {
        match self {
            BorrowedValue::String(s) => Some(s.as_ref()),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            BorrowedValue::Static(simd_json::StaticNode::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    fn as_slice(&self) -> Option<&[Self]> {
        match self {
            BorrowedValue::Array(arr) => Some(arr.as_slice()),
            _ => None,
        }
    }

    // `Self::Object` would be ambiguous with the `BorrowedValue::Object` variant, so name the
    // concrete object type.
    fn as_object(&self) -> Option<&BorrowedObject<'a>> {
        match self {
            BorrowedValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    fn entry_kind(&self) -> ImportsExportsKind {
        match self {
            BorrowedValue::String(_) => ImportsExportsKind::String,
            BorrowedValue::Array(_) => ImportsExportsKind::Array,
            BorrowedValue::Object(_) => ImportsExportsKind::Map,
            BorrowedValue::Static(_) => ImportsExportsKind::Invalid,
        }
    }
}

impl<'a> JsonObject for BorrowedObject<'a> {
    type Value = BorrowedValue<'a>;

    fn get(&self, key: &str) -> Option<&Self::Value> {
        // Inherent `HashMap::get` (takes priority over the trait method being defined).
        self.get(key)
    }

    fn iter(&self) -> impl Iterator<Item = (&str, &Self::Value)> {
        // Inherent `HashMap::iter` (takes priority over the trait method being defined).
        self.iter().map(|(k, v)| (k.as_ref(), v))
    }
}

impl PackageJsonBackend for PackageJsonCell {
    type Value<'a> = BorrowedValue<'a>;

    fn root(&self) -> &Self::Value<'_> {
        self.borrow_dependent()
    }
}

impl PackageJson {
    /// Parse a package.json file from JSON bytes
    ///
    /// # Panics
    /// # Errors
    pub fn parse(
        fs: &dyn FileSystem,
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

        Ok(Self { path, realpath, store: cell })
    }
}
