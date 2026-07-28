//! Package map parser for big-endian systems using `serde_json`.

use std::path::PathBuf;

use super::{PackageMap, PackageMapData};
use crate::JSONError;

impl PackageMap {
    /// Parse a `.package-map.json` file from JSON bytes.
    pub fn parse(path: PathBuf, realpath: PathBuf, json: Vec<u8>) -> Result<Self, JSONError> {
        let data = serde_json::from_slice::<PackageMapData>(&json).map_err(|error| JSONError {
            path: path.clone(),
            message: error.to_string(),
            line: error.line(),
            column: error.column(),
        })?;

        Ok(Self { path, realpath, packages: data.packages })
    }
}
