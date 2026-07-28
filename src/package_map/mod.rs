//! Node.js package map definitions.
//!
//! Package maps describe package locations and their dependency edges. The package manager has
//! already encoded whether those edges use `standard` or `loose` semantics, so both map types
//! share the same representation here.

#![expect(dead_code, reason = "package-map resolver integration follows the parser")]

#[cfg(target_endian = "big")]
mod serde;
#[cfg(target_endian = "little")]
mod simd;

use std::path::{Path, PathBuf};

use compact_str::CompactString;
use rustc_hash::FxHashMap;

use crate::JSONError;

/// Check if JSON content is empty or contains only whitespace.
fn check_if_empty(json_bytes: &[u8], path: &Path) -> Result<(), JSONError> {
    if json_bytes.iter().all(|&byte| byte.is_ascii_whitespace()) {
        return Err(JSONError {
            path: path.to_path_buf(),
            message: "File is empty".to_string(),
            line: 0,
            column: 0,
        });
    }
    Ok(())
}

#[derive(Debug, ::serde::Deserialize)]
struct PackageMapData {
    packages: FxHashMap<CompactString, PackageMapEntry>,
}

/// Parsed Node.js package map.
#[derive(Debug)]
pub struct PackageMap {
    /// Path to `.package-map.json`.
    path: PathBuf,

    /// Canonical path to `.package-map.json`.
    realpath: PathBuf,

    /// Package IDs mapped to their package entries.
    packages: FxHashMap<CompactString, PackageMapEntry>,
}

impl PackageMap {
    /// Returns the path where `.package-map.json` was found.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the canonical path where `.package-map.json` is stored.
    pub fn realpath(&self) -> &Path {
        &self.realpath
    }

    /// Returns all package entries keyed by package ID.
    pub fn packages(&self) -> &FxHashMap<CompactString, PackageMapEntry> {
        &self.packages
    }

    /// Returns a package entry by package ID.
    pub fn package(&self, package_id: &str) -> Option<&PackageMapEntry> {
        self.packages.get(package_id)
    }
}

/// A package entry in a Node.js package map.
#[derive(Debug, ::serde::Deserialize)]
pub struct PackageMapEntry {
    /// Absolute or relative URL for the package.
    url: CompactString,

    /// Bare package specifiers mapped to package IDs.
    #[serde(default)]
    dependencies: FxHashMap<CompactString, CompactString>,
}

impl PackageMapEntry {
    /// Returns the package URL.
    pub fn url(&self) -> &str {
        self.url.as_str()
    }

    /// Returns the target package ID for a bare package specifier.
    pub fn dependency(&self, specifier: &str) -> Option<&str> {
        self.dependencies.get(specifier).map(CompactString::as_str)
    }
}
