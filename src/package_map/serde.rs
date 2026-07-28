//! Package map parser for big-endian systems using `serde_json`.

use std::path::PathBuf;

use super::{PackageMap, PackageMapData};
use crate::{FileSystem, JSONError, replace_bom_with_whitespace};

impl PackageMap {
    /// Parse a `.package-map.json` file from JSON bytes.
    pub fn parse(
        _fs: &dyn FileSystem,
        path: PathBuf,
        realpath: PathBuf,
        mut json: Vec<u8>,
    ) -> Result<Self, JSONError> {
        replace_bom_with_whitespace(&mut json);
        super::check_if_empty(&json, &path)?;

        let data = serde_json::from_slice::<PackageMapData>(&json).map_err(|error| JSONError {
            path: path.clone(),
            message: error.to_string(),
            line: error.line(),
            column: error.column(),
        })?;

        Ok(Self { path, realpath, packages: data.packages })
    }
}
