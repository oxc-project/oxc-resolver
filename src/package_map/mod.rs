//! Node.js package map definitions.
//!
//! Package maps describe package locations and their dependency edges. The package manager has
//! already encoded whether those edges use `standard` or `loose` semantics, so both map types
//! share the same representation here.
//!
//! The accessor logic is shared between two storage backends: little-endian systems borrow
//! strings directly from simd-json's input buffer, while big-endian systems store owned
//! [`compact_str::CompactString`]s parsed by serde-json.

#![expect(dead_code, reason = "package-map resolver integration follows the parser")]

#[cfg(target_endian = "big")]
mod serde;
#[cfg(target_endian = "little")]
mod simd;

use std::{
    fmt,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use crate::PathUtil;

/// Error returned when finding the package ID that owns a path.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FindPackageIdError {
    /// Multiple package IDs map to the same owning path.
    AmbiguousResolution,

    /// The path is not contained by any package in the map.
    ExternalFile,
}

/// Storage for the parsed package entries.
pub trait PackageMapBackend {
    type Entry<'a>: PackageMapEntryBackend<'a>
    where
        Self: 'a;

    fn len(&self) -> usize;
    fn package(&self, package_id: &str) -> Option<Self::Entry<'_>>;
    fn iter(&self) -> impl Iterator<Item = (&str, Self::Entry<'_>)>;
}

/// A package entry stored by a package map backend.
pub trait PackageMapEntryBackend<'a> {
    fn url(&self) -> &'a str;
    fn dependency(&self, specifier: &str) -> Option<&'a str>;
}

/// Parsed Node.js package map, generic over its storage backend.
pub struct PackageMapGeneric<S> {
    /// Path to `.package-map.json`.
    path: PathBuf,

    /// Canonical path to `.package-map.json`.
    realpath: PathBuf,

    /// Parsed package entries.
    store: S,
}

/// Parsed Node.js package map for the current target.
#[cfg(target_endian = "big")]
pub type PackageMap = PackageMapGeneric<serde::PackageMapData>;
/// Parsed Node.js package map for the current target.
#[cfg(target_endian = "little")]
pub type PackageMap = PackageMapGeneric<simd::PackageMapCell>;

impl<S: PackageMapBackend> fmt::Debug for PackageMapGeneric<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackageMap")
            .field("path", &self.path)
            .field("realpath", &self.realpath)
            .field("packages", &self.store.len())
            .finish()
    }
}

impl<S: PackageMapBackend> PackageMapGeneric<S> {
    /// Returns the path where `.package-map.json` was found.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the canonical path where `.package-map.json` is stored.
    pub fn realpath(&self) -> &Path {
        &self.realpath
    }

    /// Returns the number of package entries.
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Returns whether the package map contains no package entries.
    pub fn is_empty(&self) -> bool {
        self.store.len() == 0
    }

    /// Returns a package entry by package ID.
    pub fn package<'a>(
        &'a self,
        package_id: &str,
    ) -> Option<PackageMapEntryGeneric<'a, S::Entry<'a>>> {
        self.store
            .package(package_id)
            .map(|entry| PackageMapEntryGeneric { entry, marker: PhantomData })
    }

    /// Finds the package ID for the most specific package containing `path`.
    pub fn find_package_id<'a>(&'a self, path: &Path) -> Result<&'a str, FindPackageIdError> {
        let mut matched = None;
        let mut matched_depth = 0;
        let mut ambiguous = false;

        for (package_id, entry) in self.store.iter() {
            let Some(package_path) = self.resolve_url(entry.url()) else {
                continue;
            };
            if path.starts_with(&package_path) {
                let depth = package_path.components().count();
                if depth > matched_depth {
                    matched = Some(package_id);
                    matched_depth = depth;
                    ambiguous = false;
                } else if depth == matched_depth {
                    matched = None;
                    ambiguous = true;
                }
            }
        }

        if ambiguous {
            Err(FindPackageIdError::AmbiguousResolution)
        } else {
            matched.ok_or(FindPackageIdError::ExternalFile)
        }
    }

    /// Resolves a package entry URL relative to the package map.
    pub fn resolve_url(&self, url: &str) -> Option<PathBuf> {
        #[cfg(not(target_arch = "wasm32"))]
        if url.starts_with("file://") {
            let path = crate::file_url::resolve_file_protocol(url).ok()?;
            return Some(PathBuf::from(path.as_ref()).normalize());
        }

        // The package map specification only permits file URLs.
        if url.contains("://") {
            return None;
        }

        let decoded = percent_encoding::percent_decode_str(url).decode_utf8().ok()?;
        // FIND_PACKAGE_ID receives the logical parent path directly, so relative package URLs
        // must be resolved in the same path namespace. In particular, a canonical path on
        // Windows may have a `\\?\` prefix that the parent path does not have.
        let base = self.path.parent()?;
        Some(base.normalize_with(Path::new(decoded.as_ref())))
    }
}

/// A package entry in a Node.js package map.
pub struct PackageMapEntryGeneric<'a, E> {
    entry: E,
    marker: PhantomData<&'a ()>,
}

impl<'a, E: PackageMapEntryBackend<'a>> PackageMapEntryGeneric<'a, E> {
    /// Returns the package URL.
    pub fn url(&self) -> &'a str {
        self.entry.url()
    }

    /// Returns the target package ID for a bare package specifier.
    pub fn dependency(&self, specifier: &str) -> Option<&'a str> {
        self.entry.dependency(specifier)
    }
}
