//! Package map parser for little-endian systems using `simd-json`.

use std::path::PathBuf;

use super::{PackageMap, PackageMapData};
use crate::JSONError;

impl PackageMap {
    /// Parse a `.package-map.json` file from JSON bytes.
    pub fn parse(path: PathBuf, realpath: PathBuf, mut json: Vec<u8>) -> Result<Self, JSONError> {
        let data = simd_json::serde::from_slice::<PackageMapData>(&mut json).map_err(|error| {
            JSONError { path: path.clone(), message: error.to_string(), line: 0, column: 0 }
        })?;

        Ok(Self { path, realpath, packages: data.packages })
    }
}
