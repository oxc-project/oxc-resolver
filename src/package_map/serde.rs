//! Package map backend for big-endian systems using serde-json and owned compact strings.

use std::path::PathBuf;

use compact_str::CompactString;
use rustc_hash::FxHashMap;

use super::{PackageMap, PackageMapBackend, PackageMapEntryBackend};
use crate::JSONError;

#[derive(Debug, ::serde::Deserialize)]
pub struct PackageMapData {
    packages: FxHashMap<CompactString, PackageMapEntryData>,
}

#[derive(Debug, ::serde::Deserialize)]
pub struct PackageMapEntryData {
    url: CompactString,
    #[serde(default)]
    dependencies: FxHashMap<CompactString, CompactString>,
}

impl PackageMapBackend for PackageMapData {
    type Entry<'a> = &'a PackageMapEntryData;

    fn len(&self) -> usize {
        self.packages.len()
    }

    fn package(&self, package_id: &str) -> Option<Self::Entry<'_>> {
        self.packages.get(package_id)
    }
}

impl<'a> PackageMapEntryBackend<'a> for &'a PackageMapEntryData {
    fn url(&self) -> &'a str {
        self.url.as_str()
    }

    fn dependency(&self, specifier: &str) -> Option<&'a str> {
        self.dependencies.get(specifier).map(CompactString::as_str)
    }
}

impl PackageMap {
    /// Parse a `.package-map.json` file from JSON bytes.
    pub fn parse(path: PathBuf, realpath: PathBuf, json: Vec<u8>) -> Result<Self, JSONError> {
        let data = serde_json::from_slice::<PackageMapData>(&json).map_err(|error| JSONError {
            path: path.clone(),
            message: error.to_string(),
            line: error.line(),
            column: error.column(),
        })?;

        Ok(Self { path, realpath, store: data })
    }
}
