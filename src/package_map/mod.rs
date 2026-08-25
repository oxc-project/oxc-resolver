//! Node.js package map definitions.
//!
//! Package maps describe package locations and their dependency edges. The package manager has
//! already encoded whether those edges use `standard` or `loose` semantics, so both map types
//! share the same representation here.
//!
//! See the [Node.js package-map specification](https://nodejs.org/api/packages.html#package-maps),
//! the [pnpm setting](https://pnpm.io/settings#nodeexperimentalpackagemap), and the
//! [Yarn setting](https://yarnpkg.com/configuration/yarnrc#nodeExperimentalPackageMap).
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
    hash::BuildHasherDefault,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
};

use dashmap::DashMap;
use rustc_hash::{FxHashMap, FxHasher};

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

#[derive(Debug, Clone)]
enum PackageOwner {
    Package(Arc<str>),
    Ambiguous,
    External,
}

/// Parsed Node.js package map, generic over its storage backend.
pub struct PackageMapGeneric<S> {
    /// Path to `.package-map.json`.
    path: PathBuf,

    /// Canonical path to `.package-map.json`.
    realpath: PathBuf,

    /// Parsed package entries.
    store: S,

    /// Resolved package paths keyed by package ID.
    package_paths: FxHashMap<Arc<str>, Arc<Path>>,

    /// Package ownership keyed by resolved package path.
    path_index: FxHashMap<Arc<Path>, PackageOwner>,

    /// Memoized ownership for importer paths.
    path_cache: DashMap<PathBuf, PackageOwner, BuildHasherDefault<FxHasher>>,
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
            .field("resolved_paths", &self.package_paths.len())
            .field("indexed_paths", &self.path_index.len())
            .field("cached_paths", &self.path_cache.len())
            .finish()
    }
}

impl<S: PackageMapBackend> PackageMapGeneric<S> {
    fn new(path: PathBuf, realpath: PathBuf, store: S) -> Self {
        let mut package_paths = FxHashMap::default();
        let mut path_index = FxHashMap::default();

        for (package_id, entry) in store.iter() {
            let Some(package_path) = Self::resolve_url_from(&realpath, entry.url()) else {
                continue;
            };
            let package_id = Arc::<str>::from(package_id);
            let package_path = Arc::<Path>::from(package_path);

            package_paths.insert(Arc::clone(&package_id), Arc::clone(&package_path));
            path_index
                .entry(package_path)
                .and_modify(|owner| *owner = PackageOwner::Ambiguous)
                .or_insert(PackageOwner::Package(package_id));
        }

        Self {
            path,
            realpath,
            store,
            package_paths,
            path_index,
            path_cache: DashMap::with_hasher(BuildHasherDefault::default()),
        }
    }

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
        self.store.package(package_id).map(|entry| PackageMapEntryGeneric {
            entry,
            path: self.package_paths.get(package_id).map(Arc::as_ref),
            marker: PhantomData,
        })
    }

    /// Finds the package ID for the most specific package containing `path`.
    pub fn find_package_id<'a>(&'a self, path: &Path) -> Result<&'a str, FindPackageIdError> {
        if let Some(owner) = self.path_cache.get(path) {
            return self.package_id_for_owner(owner.value());
        }

        let owner = path
            .ancestors()
            .find_map(|path| self.path_index.get(path).cloned())
            .unwrap_or(PackageOwner::External);
        let result = self.package_id_for_owner(&owner);
        self.path_cache.insert(path.to_path_buf(), owner);
        result
    }

    #[cfg(test)]
    pub(crate) fn path_cache_len(&self) -> usize {
        self.path_cache.len()
    }

    fn package_id_for_owner<'a>(
        &'a self,
        owner: &PackageOwner,
    ) -> Result<&'a str, FindPackageIdError> {
        match owner {
            PackageOwner::Package(package_id) => Ok(self
                .package_paths
                .get_key_value(package_id.as_ref())
                .expect("an indexed package ID must have a corresponding resolved path")
                .0
                .as_ref()),
            PackageOwner::Ambiguous => Err(FindPackageIdError::AmbiguousResolution),
            PackageOwner::External => Err(FindPackageIdError::ExternalFile),
        }
    }

    /// Resolves a package entry URL relative to the package map.
    pub fn resolve_url(&self, url: &str) -> Option<PathBuf> {
        Self::resolve_url_from(&self.realpath, url)
    }

    fn resolve_url_from(package_map_path: &Path, url: &str) -> Option<PathBuf> {
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
        // Node resolves the package map itself before using its URL as the base, so relative
        // package URLs are resolved from the package map's canonical location.
        let base = package_map_path.parent()?;
        Some(base.normalize_with(Path::new(decoded.as_ref())))
    }
}

/// A package entry in a Node.js package map.
pub struct PackageMapEntryGeneric<'a, E> {
    entry: E,
    path: Option<&'a Path>,
    marker: PhantomData<&'a ()>,
}

impl<'a, E: PackageMapEntryBackend<'a>> PackageMapEntryGeneric<'a, E> {
    /// Returns the package URL.
    pub fn url(&self) -> &'a str {
        self.entry.url()
    }

    /// Returns the resolved package path.
    pub fn path(&self) -> Option<&'a Path> {
        self.path
    }

    /// Returns the target package ID for a bare package specifier.
    pub fn dependency(&self, specifier: &str) -> Option<&'a str> {
        self.entry.dependency(specifier)
    }
}
