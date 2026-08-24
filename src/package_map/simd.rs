//! Package map backend for little-endian systems using simd-json and borrowed strings.

#![expect(
    clippy::impl_trait_in_params,
    reason = "`self_cell!` generates `pub` constructors with `impl FnOnce` parameters"
)]

use std::path::PathBuf;

use rustc_hash::FxHashMap;
use self_cell::MutBorrow;

use super::{PackageMap, PackageMapBackend, PackageMapEntryBackend};
use crate::JSONError;

#[derive(Debug, ::serde::Deserialize)]
pub struct PackageMapData<'a> {
    #[serde(borrow)]
    packages: FxHashMap<&'a str, PackageMapEntryData<'a>>,
}

#[derive(Debug, ::serde::Deserialize)]
pub struct PackageMapEntryData<'a> {
    url: &'a str,
    #[serde(borrow, default)]
    dependencies: FxHashMap<&'a str, &'a str>,
}

self_cell::self_cell! {
    pub struct PackageMapCell {
        owner: MutBorrow<Vec<u8>>,

        #[covariant]
        dependent: PackageMapData,
    }
}

impl PackageMapBackend for PackageMapCell {
    type Entry<'a> = &'a PackageMapEntryData<'a>;

    fn len(&self) -> usize {
        self.borrow_dependent().packages.len()
    }

    fn package(&self, package_id: &str) -> Option<Self::Entry<'_>> {
        self.borrow_dependent().packages.get(package_id)
    }

    fn iter(&self) -> impl Iterator<Item = (&str, Self::Entry<'_>)> {
        self.borrow_dependent().packages.iter().map(|(id, entry)| (*id, entry))
    }
}

impl<'a> PackageMapEntryBackend<'a> for &'a PackageMapEntryData<'a> {
    fn url(&self) -> &'a str {
        self.url
    }

    fn dependency(&self, specifier: &str) -> Option<&'a str> {
        self.dependencies.get(specifier).copied()
    }
}

impl PackageMap {
    /// Parse a `.package-map.json` file from JSON bytes.
    pub fn parse(path: PathBuf, realpath: PathBuf, json: Vec<u8>) -> Result<Self, JSONError> {
        let cell = PackageMapCell::try_new(MutBorrow::new(json), |bytes| {
            simd_json::serde::from_slice::<PackageMapData<'_>>(bytes.borrow_mut())
        })
        .map_err(|error| JSONError {
            path: path.clone(),
            message: error.to_string(),
            line: 0,
            column: 0,
        })?;

        Ok(Self::new(path, realpath, cell))
    }
}
