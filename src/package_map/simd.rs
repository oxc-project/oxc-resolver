//! Package map backend for little-endian systems using simd-json and borrowed strings.

#![expect(
    clippy::impl_trait_in_params,
    reason = "`self_cell!` generates `pub` constructors with `impl FnOnce` parameters"
)]

use std::path::PathBuf;

use self_cell::MutBorrow;
use simd_json::{
    BorrowedValue,
    prelude::{ValueAsObject, ValueAsScalar},
};

use super::{PackageMap, PackageMapBackend, PackageMapEntryBackend};
use crate::JSONError;

type BorrowedObject<'a> = simd_json::value::borrowed::Object<'a>;

self_cell::self_cell! {
    pub struct PackageMapCell {
        owner: MutBorrow<Vec<u8>>,

        #[covariant]
        dependent: BorrowedValue,
    }
}

impl PackageMapBackend for PackageMapCell {
    type Entry<'a> = &'a BorrowedValue<'a>;

    fn len(&self) -> usize {
        self.packages().len()
    }

    fn package(&self, package_id: &str) -> Option<Self::Entry<'_>> {
        self.packages().get(package_id)
    }

    fn iter(&self) -> impl Iterator<Item = (&str, Self::Entry<'_>)> {
        self.packages().iter().map(|(id, entry)| (id.as_ref(), entry))
    }
}

impl PackageMapCell {
    fn packages(&self) -> &BorrowedObject<'_> {
        self.borrow_dependent()
            .as_object()
            .and_then(|root| root.get("packages"))
            .and_then(ValueAsObject::as_object)
            .expect("package map shape is validated during parsing")
    }
}

impl<'a> PackageMapEntryBackend<'a> for &'a BorrowedValue<'a> {
    fn url(&self) -> &'a str {
        let value: &'a BorrowedValue<'a> = self;
        value
            .as_object()
            .and_then(|entry| entry.get("url"))
            .and_then(ValueAsScalar::as_str)
            .expect("package map shape is validated during parsing")
    }

    fn dependency(&self, specifier: &str) -> Option<&'a str> {
        let value: &'a BorrowedValue<'a> = self;
        value.as_object()?.get("dependencies")?.as_object()?.get(specifier)?.as_str()
    }
}

fn has_valid_shape(value: &BorrowedValue<'_>) -> bool {
    let BorrowedValue::Object(root) = value else { return false };
    let Some(BorrowedValue::Object(packages)) = root.get("packages") else { return false };
    packages.values().all(|entry| {
        let BorrowedValue::Object(entry) = entry else { return false };
        matches!(entry.get("url"), Some(BorrowedValue::String(_)))
            && entry.get("dependencies").is_none_or(|dependencies| {
                let BorrowedValue::Object(dependencies) = dependencies else { return false };
                dependencies.values().all(|value| matches!(value, BorrowedValue::String(_)))
            })
    })
}

impl PackageMap {
    /// Parse a `.package-map.json` file from JSON bytes.
    pub fn parse(path: PathBuf, realpath: PathBuf, json: Vec<u8>) -> Result<Self, JSONError> {
        let cell = PackageMapCell::try_new(MutBorrow::new(json), |bytes| {
            simd_json::to_borrowed_value(bytes.borrow_mut())
        })
        .map_err(|error| JSONError {
            path: path.clone(),
            message: error.to_string(),
            line: 0,
            column: 0,
        })?;

        if !has_valid_shape(cell.borrow_dependent()) {
            return Err(JSONError {
                path,
                message:
                    "package map must contain package entries with string URLs and dependencies"
                        .to_string(),
                line: 0,
                column: 0,
            });
        }

        Ok(Self::new(path, realpath, cell))
    }
}
