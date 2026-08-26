//! Implementation of experimental [Node.js package maps][node-package-maps].
//!
//! A package map is one static JSON file containing a `packages` object. Each key is an opaque,
//! unique package ID. Its value has a required `url` and an optional `dependencies` object that
//! maps the bare package name used by source code to another package ID. Package IDs therefore
//! identify dependency graph nodes independently of their locations and allow different importers
//! to resolve the same package name to different versions.
//!
//! Entry URLs are resolved from the package map's effective location into filesystem paths. The
//! effective location is canonical when symlink resolution is enabled and is otherwise the
//! `NODE_OPTIONS` path. Explicit URLs must use the `file:` protocol. The resulting paths form the
//! index used by [`PackageMap::find_package_id`] to identify the package that owns an importer.
//! Multiple IDs resolving to the same owning path are retained as ambiguous, as required for
//! [multiple packages sharing one URL][shared-url].
//!
//! Package-map resolution is enabled automatically when `NODE_OPTIONS` contains Node's
//! `--experimental-package-map` option. The resolver API does not propagate a package ID between
//! resolutions, so it always uses the specification's path-based fallback to identify the
//! importer. The parsed map and both successful and failed ownership lookups are cached. Parsing
//! itself is synchronous but deferred until the first applicable resolution because resolver
//! construction cannot return an error. Selecting a dependency follows one map edge before regular
//! resolution resumes; package-map dependency cycles are not detected, matching the
//! specification's limitation.
//!
//! The accessor logic is shared between two storage backends: little-endian systems borrow
//! strings directly from simd-json's input buffer, while big-endian systems store owned
//! [`compact_str::CompactString`]s parsed by serde-json.
//!
//! Package-manager configuration:
//! [pnpm](https://pnpm.io/settings#nodeexperimentalpackagemap),
//! [Yarn](https://yarnpkg.com/configuration/yarnrc#nodeExperimentalPackageMap).
//!
//! [node-package-maps]: https://nodejs.org/api/packages.html#package-maps
//! [shared-url]: https://nodejs.org/api/packages.html#multiple-packages-for-the-same-url

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

/// Extracts the last `--experimental-package-map` path from `NODE_OPTIONS`.
///
/// Tokenization follows Node's `ParseNodeOptionsEnvVar`: spaces separate arguments, double quotes
/// group text, and backslashes escape the following character inside quoted text. Both the
/// `--experimental-package-map=<path>` and `--experimental-package-map <path>` forms are accepted.
pub fn package_map_path_from_node_options(node_options: &str, cwd: &Path) -> Option<PathBuf> {
    let arguments = parse_node_options(node_options)?;
    let mut package_map_path = None;
    let mut index = 0;

    while index < arguments.len() {
        let argument = &arguments[index];
        if let Some(path) = argument.strip_prefix("--experimental-package-map=") {
            package_map_path = (!path.is_empty()).then(|| PathBuf::from(path));
        } else if argument == "--experimental-package-map" {
            index += 1;
            package_map_path = arguments
                .get(index)
                .filter(|path| !path.is_empty() && !path.starts_with('-'))
                .map(PathBuf::from);
        }
        index += 1;
    }

    package_map_path
        .map(|path| if path.is_relative() { cwd.normalize_with(path) } else { path.normalize() })
}

fn parse_node_options(node_options: &str) -> Option<Vec<String>> {
    let mut arguments = Vec::new();
    let mut chars = node_options.chars();
    let mut is_in_string = false;
    let mut will_start_new_argument = true;

    while let Some(mut character) = chars.next() {
        if character == '\\' && is_in_string {
            character = chars.next()?;
        } else if character == ' ' && !is_in_string {
            will_start_new_argument = true;
            continue;
        } else if character == '"' {
            is_in_string = !is_in_string;
            continue;
        }

        if will_start_new_argument {
            arguments.push(String::from(character));
            will_start_new_argument = false;
        } else {
            arguments.last_mut().expect("an argument has started").push(character);
        }
    }

    (!is_in_string).then_some(arguments)
}

/// Error returned by the path-based fallback in Node's package-map resolution algorithm.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FindPackageIdError {
    /// Multiple package IDs resolve to the same owning package path.
    AmbiguousResolution,

    /// The importer path is not contained by any package location in the map.
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

/// Parsed Node.js package map and its resolved package-location index.
///
/// This represents the specification's top-level `packages` object. See
/// [Configuration file format](https://nodejs.org/api/packages.html#configuration-file-format).
pub struct PackageMapGeneric<S> {
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
pub type PackageMap = PackageMapGeneric<serde::PackageMapData>;
/// Parsed Node.js package map for the current target.
#[cfg(target_endian = "little")]
pub type PackageMap = PackageMapGeneric<simd::PackageMapCell>;

impl<S: PackageMapBackend> fmt::Debug for PackageMapGeneric<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackageMap")
            .field("path", &self.path)
            .field("packages", &self.store.len())
            .field("resolved_paths", &self.package_paths.len())
            .field("indexed_paths", &self.path_index.len())
            .field("cached_paths", &self.path_cache.len())
            .finish()
    }
}

impl<S: PackageMapBackend> PackageMapGeneric<S> {
    fn new(path: PathBuf, realpath: &Path, store: S) -> Self {
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
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the entry for an opaque package ID from the top-level `packages` object.
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

    /// Implements the path-based fallback for determining which package owns an importer.
    ///
    /// The nearest ancestor present in the resolved-path index owns `path`. If multiple package
    /// IDs resolve to that ancestor, this returns [`FindPackageIdError::AmbiguousResolution`]. If
    /// no mapped package contains `path`, it returns [`FindPackageIdError::ExternalFile`].
    ///
    /// This corresponds to `FIND_PACKAGE_ID(PATH, PACKAGE_MAP)` in Node's
    /// [CommonJS resolution pseudocode](https://nodejs.org/api/modules.html#all-together).
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
pub struct PackageMapEntryGeneric<'a, E> {
    entry: E,
    path: Option<&'a Path>,
    marker: PhantomData<&'a ()>,
}

impl<'a, E: PackageMapEntryBackend<'a>> PackageMapEntryGeneric<'a, E> {
    /// Returns the resolved file path, or `None` when `url` is not a valid file target.
    pub fn path(&self) -> Option<&'a Path> {
        self.path
    }

    /// Looks up a bare package name in `dependencies` and returns its target package ID.
    ///
    /// A missing `dependencies` object behaves as an empty object.
    pub fn dependency(&self, specifier: &str) -> Option<&'a str> {
        self.entry.dependency(specifier)
    }
}
