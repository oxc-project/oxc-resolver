//! Package map parser for little-endian systems using `simd-json`.

use std::path::PathBuf;

use super::{PackageMap, PackageMapData};
use crate::{FileSystem, JSONError, replace_bom_with_whitespace};

impl PackageMap {
    /// Parse a `.package-map.json` file from JSON bytes.
    pub fn parse(
        fs: &dyn FileSystem,
        path: PathBuf,
        realpath: PathBuf,
        mut json: Vec<u8>,
    ) -> Result<Self, JSONError> {
        replace_bom_with_whitespace(&mut json);
        super::check_if_empty(&json, &path)?;

        let data =
            simd_json::serde::from_slice::<PackageMapData>(&mut json).map_err(|simd_error| {
                let fallback_result = fs
                    .read(&realpath)
                    .map_err(|io_error| JSONError {
                        path: path.clone(),
                        message: format!("Failed to re-read file for error reporting: {io_error}"),
                        line: 0,
                        column: 0,
                    })
                    .and_then(|mut bytes| {
                        replace_bom_with_whitespace(&mut bytes);
                        serde_json::from_slice::<PackageMapData>(&bytes).map_err(|serde_error| {
                            JSONError {
                                path: path.clone(),
                                message: serde_error.to_string(),
                                line: serde_error.line(),
                                column: serde_error.column(),
                            }
                        })
                    });

                match fallback_result {
                    Ok(_) => JSONError {
                        path: path.clone(),
                        message: format!("simd_json parse error: {simd_error}"),
                        line: 0,
                        column: 0,
                    },
                    Err(error) => error,
                }
            })?;

        Ok(Self { path, realpath, packages: data.packages })
    }
}
