//! Package map backend for big-endian systems using serde-json and owned compact strings.

use std::path::PathBuf;

use compact_str::CompactString;
use rustc_hash::FxHashMap;

use crate::JSONError;

use super::map::{PackageMap, PackageMapBackend, PackageMapEntryBackend};

#[derive(Debug, ::serde::Deserialize)]
pub(super) struct PackageMapData {
    packages: FxHashMap<CompactString, PackageMapEntryData>,
}

#[derive(Debug, ::serde::Deserialize)]
pub(super) struct PackageMapEntryData {
    url: CompactString,
    #[serde(default)]
    dependencies: FxHashMap<CompactString, CompactString>,
}

impl PackageMapBackend for PackageMapData {
    type Entry<'a> = &'a PackageMapEntryData;

    fn package(&self, package_id: &str) -> Option<Self::Entry<'_>> {
        self.packages.get(package_id)
    }

    fn iter(&self) -> impl Iterator<Item = (&str, Self::Entry<'_>)> {
        self.packages.iter().map(|(id, entry)| (id.as_str(), entry))
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
    pub(super) fn parse(path: PathBuf, json: Vec<u8>) -> Result<Self, JSONError> {
        let data = serde_json::from_slice::<PackageMapData>(&json).map_err(|error| JSONError {
            path: path.clone(),
            message: error.to_string(),
            line: error.line(),
            column: error.column(),
        })?;

        Ok(Self::new(path, data))
    }
}
