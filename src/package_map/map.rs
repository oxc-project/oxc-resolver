use std::{
    hash::BuildHasherDefault,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
};

use dashmap::DashMap;
use rustc_hash::{FxHashMap, FxHasher};

use crate::PathUtil;

/// Error returned by the path-based fallback in Node's package-map resolution algorithm.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) enum FindPackageIdError {
    /// Multiple package IDs resolve to the same owning package path.
    AmbiguousResolution,

    /// The importer path is not contained by any package location in the map.
    ExternalFile,
}

/// Storage for the parsed package entries.
pub(super) trait PackageMapBackend {
    type Entry<'a>: PackageMapEntryBackend<'a>
    where
        Self: 'a;

    fn package(&self, package_id: &str) -> Option<Self::Entry<'_>>;
    fn iter(&self) -> impl Iterator<Item = (&str, Self::Entry<'_>)>;
}

/// A package entry stored by a package map backend.
pub(super) trait PackageMapEntryBackend<'a> {
    fn url(&self) -> &'a str;
    fn dependency(&self, specifier: &str) -> Option<&'a str>;
}

#[derive(Debug, Clone)]
enum PackageOwner {
    Package(Arc<str>),
    Ambiguous,
    External,
}

/// Parsed Node.js package map and its resolved package-location index.
///
/// This represents the specification's top-level `packages` object. See
/// [Configuration file format](https://nodejs.org/api/packages.html#configuration-file-format).
pub(super) struct PackageMapGeneric<S> {
    /// Configured package-map path, used for diagnostics and dependency tracking.
    path: PathBuf,

    /// Package entries keyed by their opaque package IDs.
    store: S,

    /// Valid `file:` package locations keyed by package ID.
    package_paths: FxHashMap<Arc<str>, Arc<Path>>,

    /// Package ownership keyed by resolved location; duplicate locations are ambiguous.
    path_index: FxHashMap<Arc<Path>, PackageOwner>,

    /// Memoized path-based ownership results for importer directories.
    path_cache: DashMap<PathBuf, PackageOwner, BuildHasherDefault<FxHasher>>,
}

/// Parsed Node.js package map for the current target.
#[cfg(target_endian = "big")]
pub(super) type PackageMap = PackageMapGeneric<super::serde::PackageMapData>;
/// Parsed Node.js package map for the current target.
#[cfg(target_endian = "little")]
pub(super) type PackageMap = PackageMapGeneric<super::simd::PackageMapCell>;

impl<S: PackageMapBackend> PackageMapGeneric<S> {
    pub(super) fn new(path: PathBuf, realpath: &Path, store: S) -> Self {
        let mut package_paths = FxHashMap::default();
        let mut path_index = FxHashMap::default();

        for (package_id, entry) in store.iter() {
            let Some(package_path) = Self::resolve_url_from(realpath, entry.url()) else {
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
            store,
            package_paths,
            path_index,
            path_cache: DashMap::with_hasher(BuildHasherDefault::default()),
        }
    }

    /// Returns the path where `.package-map.json` was found.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the entry for an opaque package ID from the top-level `packages` object.
    pub(super) fn package<'a>(
        &'a self,
        package_id: &str,
    ) -> Option<PackageMapEntryGeneric<'a, S::Entry<'a>>> {
        self.store.package(package_id).map(|entry| PackageMapEntryGeneric {
            entry,
            path: self.package_paths.get(package_id).map(Arc::as_ref),
            marker: PhantomData,
        })
    }

    /// Implements the path-based fallback for determining which package owns an importer.
    ///
    /// The nearest ancestor present in the resolved-path index owns `path`. If multiple package
    /// IDs resolve to that ancestor, this returns [`FindPackageIdError::AmbiguousResolution`]. If
    /// no mapped package contains `path`, it returns [`FindPackageIdError::ExternalFile`].
    ///
    /// This corresponds to `FIND_PACKAGE_ID(PATH, PACKAGE_MAP)` in Node's
    /// [CommonJS resolution pseudocode](https://nodejs.org/api/modules.html#all-together).
    pub(super) fn find_package_id<'a>(
        &'a self,
        path: &Path,
    ) -> Result<&'a str, FindPackageIdError> {
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

    /// Resolves an entry's `url` from the effective package-map location into a filesystem path.
    ///
    /// `file://` URLs and filesystem paths are accepted. Non-file protocols, percent-decoded paths
    /// that are not UTF-8, and paths without a package-map parent return `None` and cannot be
    /// resolution targets.
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
        // Node uses the package map URL as the base. `package_map_path` is the equivalent effective
        // filesystem location: canonical when symlink resolution is enabled, configured otherwise.
        let base = package_map_path.parent()?;
        Some(base.normalize_with(Path::new(decoded.as_ref())))
    }
}

/// One package entry from the package map's top-level `packages` object.
pub(super) struct PackageMapEntryGeneric<'a, E> {
    entry: E,
    path: Option<&'a Path>,
    marker: PhantomData<&'a ()>,
}

impl<'a, E: PackageMapEntryBackend<'a>> PackageMapEntryGeneric<'a, E> {
    /// Returns the resolved file path, or `None` when `url` is not a valid file target.
    pub(super) fn path(&self) -> Option<&'a Path> {
        self.path
    }

    /// Looks up a bare package name in `dependencies` and returns its target package ID.
    ///
    /// A missing `dependencies` object behaves as an empty object.
    pub(super) fn dependency(&self, specifier: &str) -> Option<&'a str> {
        self.entry.dependency(specifier)
    }
}
