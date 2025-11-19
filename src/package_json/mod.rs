//! package.json definitions
//!
//! This module provides platform-specific implementations for parsing package.json files.
//! On little-endian systems, it uses simd-json for high performance.
//! On big-endian systems, it falls back to serde_json.

#[cfg(target_endian = "big")]
mod serde;
#[cfg(target_endian = "little")]
mod simd;

#[cfg(target_endian = "big")]
pub use serde::*;
#[cfg(target_endian = "little")]
pub use simd::*;

use std::{fmt, path::Path};

use crate::JSONError;

/// Check if JSON content is empty or contains only whitespace
fn check_if_empty(json_bytes: &[u8], path: &Path) -> Result<(), JSONError> {
    // Check if content is empty or whitespace-only
    if json_bytes.iter().all(|&b| b.is_ascii_whitespace()) {
        return Err(JSONError {
            path: path.to_path_buf(),
            message: "File is empty".to_string(),
            line: 0,
            column: 0,
        });
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageType {
    CommonJs,
    Module,
}

impl PackageType {
    pub(super) fn from_str(s: &str) -> Option<Self> {
        match s {
            "commonjs" => Some(Self::CommonJs),
            "module" => Some(Self::Module),
            _ => None,
        }
    }
}

impl fmt::Display for PackageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommonJs => f.write_str("commonjs"),
            Self::Module => f.write_str("module"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportsExportsKind {
    String,
    Array,
    Map,
    Invalid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SideEffects<'a> {
    Bool(bool),
    String(&'a str),
    Array(Vec<&'a str>),
}
