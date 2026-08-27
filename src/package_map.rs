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
mod serde {
    //! Package map backend for big-endian systems using serde-json and owned compact strings.

    use std::path::{Path, PathBuf};

    use compact_str::CompactString;
    use rustc_hash::FxHashMap;

    use super::{PackageMap, PackageMapBackend, PackageMapEntryBackend};
    use crate::JSONError;

    #[derive(Debug, ::serde::Deserialize)]
    pub struct PackageMapData {
        packages: FxHashMap<CompactString, PackageMapEntryData>,
    }

    #[derive(Debug, ::serde::Deserialize)]
    pub struct PackageMapEntryData {
        url: CompactString,
        #[serde(default)]
        dependencies: FxHashMap<CompactString, CompactString>,
    }

    impl PackageMapBackend for PackageMapData {
        type Entry<'a> = &'a PackageMapEntryData;

        fn package(&self, package_id: &str) -> Option<Self::Entry<'_>> {
            self.packages.get(package_id)
        }

        fn iter(&self) -> impl Iterator<Item = (&str, Self::Entry<'_>)> {
            self.packages.iter().map(|(id, entry)| (id.as_str(), entry))
        }
    }

    impl<'a> PackageMapEntryBackend<'a> for &'a PackageMapEntryData {
        fn url(&self) -> &'a str {
            self.url.as_str()
        }

        fn dependency(&self, specifier: &str) -> Option<&'a str> {
            self.dependencies.get(specifier).map(CompactString::as_str)
        }
    }

    impl PackageMap {
        /// Parse a `.package-map.json` file from JSON bytes.
        pub fn parse(path: PathBuf, realpath: &Path, json: Vec<u8>) -> Result<Self, JSONError> {
            let data =
                serde_json::from_slice::<PackageMapData>(&json).map_err(|error| JSONError {
                    path: path.clone(),
                    message: error.to_string(),
                    line: error.line(),
                    column: error.column(),
                })?;

            Ok(Self::new(path, realpath, data))
        }
    }
}
#[cfg(target_endian = "little")]
mod simd {
    //! Package map backend for little-endian systems using simd-json and borrowed strings.

    #![expect(
        clippy::impl_trait_in_params,
        reason = "`self_cell!` generates `pub` constructors with `impl FnOnce` parameters"
    )]

    use std::path::{Path, PathBuf};

    use self_cell::MutBorrow;
    use simd_json::{
        BorrowedValue,
        prelude::{ValueAsObject, ValueAsScalar},
    };

    use super::{PackageMap, PackageMapBackend, PackageMapEntryBackend};
    use crate::JSONError;

    type BorrowedObject<'a> = simd_json::value::borrowed::Object<'a>;

    self_cell::self_cell! {
        pub struct PackageMapCell {
            owner: MutBorrow<Vec<u8>>,

            #[covariant]
            dependent: BorrowedValue,
        }
    }

    impl PackageMapBackend for PackageMapCell {
        type Entry<'a> = &'a BorrowedValue<'a>;

        fn package(&self, package_id: &str) -> Option<Self::Entry<'_>> {
            self.packages().get(package_id)
        }

        fn iter(&self) -> impl Iterator<Item = (&str, Self::Entry<'_>)> {
            self.packages().iter().map(|(id, entry)| (id.as_ref(), entry))
        }
    }

    impl PackageMapCell {
        fn packages(&self) -> &BorrowedObject<'_> {
            self.borrow_dependent()
                .as_object()
                .and_then(|root| root.get("packages"))
                .and_then(ValueAsObject::as_object)
                .expect("package map shape is validated during parsing")
        }
    }

    impl<'a> PackageMapEntryBackend<'a> for &'a BorrowedValue<'a> {
        fn url(&self) -> &'a str {
            let value: &'a BorrowedValue<'a> = self;
            value
                .as_object()
                .and_then(|entry| entry.get("url"))
                .and_then(ValueAsScalar::as_str)
                .expect("package map shape is validated during parsing")
        }

        fn dependency(&self, specifier: &str) -> Option<&'a str> {
            let value: &'a BorrowedValue<'a> = self;
            value.as_object()?.get("dependencies")?.as_object()?.get(specifier)?.as_str()
        }
    }

    fn has_valid_shape(value: &BorrowedValue<'_>) -> bool {
        let BorrowedValue::Object(root) = value else { return false };
        let Some(BorrowedValue::Object(packages)) = root.get("packages") else { return false };
        packages.values().all(|entry| {
            let BorrowedValue::Object(entry) = entry else { return false };
            matches!(entry.get("url"), Some(BorrowedValue::String(_)))
                && entry.get("dependencies").is_none_or(|dependencies| {
                    let BorrowedValue::Object(dependencies) = dependencies else { return false };
                    dependencies.values().all(|value| matches!(value, BorrowedValue::String(_)))
                })
        })
    }

    impl PackageMap {
        /// Parse a `.package-map.json` file from JSON bytes.
        pub fn parse(path: PathBuf, realpath: &Path, json: Vec<u8>) -> Result<Self, JSONError> {
            let cell = PackageMapCell::try_new(MutBorrow::new(json), |bytes| {
                simd_json::to_borrowed_value(bytes.borrow_mut())
            })
            .map_err(|error| JSONError {
                path: path.clone(),
                message: error.to_string(),
                line: 0,
                column: 0,
            })?;

            if !has_valid_shape(cell.borrow_dependent()) {
                return Err(JSONError {
                    path,
                    message:
                        "package map must contain package entries with string URLs and dependencies"
                            .to_string(),
                    line: 0,
                    column: 0,
                });
            }

            Ok(Self::new(path, realpath, cell))
        }
    }
}

use std::{
    hash::BuildHasherDefault,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, OnceLock},
};

use dashmap::DashMap;
use rustc_hash::{FxHashMap, FxHasher};

use crate::{CachedPath, Ctx, PathUtil, ResolveError, ResolveOptions, ResolverImpl, TsConfig};

pub struct PackageMapCache {
    path: PathBuf,
    value: OnceLock<Result<Arc<PackageMap>, ResolveError>>,
}

impl PackageMapCache {
    fn new(path: PathBuf) -> Self {
        Self { path, value: OnceLock::new() }
    }
}

static NODE_OPTIONS_PACKAGE_MAP_PATH: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    let node_options = std::env::var("NODE_OPTIONS").ok()?;
    let cwd = std::env::current_dir().ok()?;
    package_map_path_from_node_options(&node_options, &cwd)
});

pub fn configure(options: ResolveOptions) -> (ResolveOptions, Option<Box<PackageMapCache>>) {
    let package_map = NODE_OPTIONS_PACKAGE_MAP_PATH
        .as_ref()
        .map(|path| Box::new(PackageMapCache::new(path.clone())));
    (options.sanitize(), package_map)
}

pub fn reconfigure(
    options: ResolveOptions,
    package_map: Option<&PackageMapCache>,
) -> (ResolveOptions, Option<Box<PackageMapCache>>) {
    let package_map =
        package_map.map(|package_map| Box::new(PackageMapCache::new(package_map.path.clone())));
    (options.sanitize(), package_map)
}

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

impl ResolverImpl {
    /// Resolves a bare package through the active package map or regular package lookup.
    ///
    /// # Errors
    ///
    /// Returns the underlying package-map or filesystem resolution error.
    pub fn load_bare_package(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        tsconfig: Option<&TsConfig>,
        ctx: &mut Ctx,
    ) -> Result<CachedPath, ResolveError> {
        if self.package_map.is_some() {
            return self.load_package_self_or_package_map(cached_path, specifier, tsconfig, ctx);
        }
        self.load_package_self_or_node_modules(cached_path, specifier, tsconfig, ctx)
    }

    pub fn package_map_resolve(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        package_name: &str,
        subpath: &str,
        tsconfig: Option<&TsConfig>,
        ctx: &mut Ctx,
    ) -> Option<Result<Option<CachedPath>, ResolveError>> {
        self.package_map.as_ref()?;
        Some(
            self.load_package_map_for_importer(
                cached_path,
                specifier,
                package_name,
                subpath,
                tsconfig,
                ctx,
            )
            .map(Some),
        )
    }

    #[cold]
    #[inline(never)]
    fn load_package_self_or_package_map(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        tsconfig: Option<&TsConfig>,
        ctx: &mut Ctx,
    ) -> Result<CachedPath, ResolveError> {
        let (package_name, subpath) = Self::parse_package_specifier(specifier);
        if subpath.is_empty() {
            ctx.with_fully_specified(false);
        }
        // 5. LOAD_PACKAGE_SELF(X, dirname(Y))
        if let Some(path) = self.load_package_self(cached_path, specifier, tsconfig, ctx)? {
            return Ok(path);
        }
        self.load_package_map_for_importer(
            cached_path,
            specifier,
            package_name,
            subpath,
            tsconfig,
            ctx,
        )
    }

    /// Implements package-map dispatch from step 6 of Node's CommonJS resolution pseudocode.
    ///
    /// Node permits the importing package ID to be propagated by the caller. The resolver returns
    /// paths rather than package IDs, so this implementation takes the specified fallback and calls
    /// `FIND_PACKAGE_ID(dirname(Y), PACKAGE_MAP)` for every uncached importer path.
    ///
    /// See <https://nodejs.org/api/modules.html#all-together>.
    #[cold]
    #[inline(never)]
    fn load_package_map_for_importer(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        name: &str,
        subpath: &str,
        tsconfig: Option<&TsConfig>,
        ctx: &mut Ctx,
    ) -> Result<CachedPath, ResolveError> {
        let package_map = self.package_map(ctx)?;

        // Step 6.a: derive PARENT_PACKAGE_ID from dirname(Y). `cached_path` is already dirname(Y)
        // because the public resolve API accepts the importing directory, not the importing file.
        let parent_path = cached_path.path();
        let parent_package_id =
            package_map.find_package_id(parent_path).map_err(|error| match error {
                FindPackageIdError::AmbiguousResolution => {
                    ResolveError::PackageMapAmbiguousResolution {
                        specifier: specifier.to_string(),
                        parent_path: parent_path.to_path_buf(),
                        package_map_path: package_map.path().to_path_buf(),
                    }
                }
                FindPackageIdError::ExternalFile => ResolveError::PackageMapExternalFile {
                    specifier: specifier.to_string(),
                    parent_path: parent_path.to_path_buf(),
                    package_map_path: package_map.path().to_path_buf(),
                },
            })?;

        self.load_package_map(
            specifier,
            name,
            subpath,
            parent_package_id,
            package_map,
            tsconfig,
            ctx,
        )
    }

    /// Implements `LOAD_PACKAGE_MAP(X, PARENT_PACKAGE_ID, PACKAGE_MAP)`.
    ///
    /// The numbered comments correspond directly to Node's
    /// [CommonJS resolution pseudocode](https://nodejs.org/api/modules.html#all-together).
    fn load_package_map(
        &self,
        specifier: &str,
        name: &str,
        subpath: &str,
        parent_package_id: &str,
        package_map: &PackageMap,
        tsconfig: Option<&TsConfig>,
        ctx: &mut Ctx,
    ) -> Result<CachedPath, ResolveError> {
        // Step 1 was performed once by `parse_package_specifier`: NAME includes an optional
        // `@scope/` prefix and SUBPATH is either empty or begins with `/`.

        // 2. Find the package map entry for key PARENT_PACKAGE_ID.
        let parent_package = package_map
            .package(parent_package_id)
            .expect("a package ID returned by the package map must have a corresponding entry");

        // 3. Look up NAME in the entry's "dependencies" map.
        // 4. If NAME is not found, THROW "not found".
        let dependency_id = parent_package
            .dependency(name)
            .ok_or_else(|| ResolveError::NotFound(specifier.to_string()))?;

        // 5. Let TARGET be PACKAGE_MAP.packages[dependencies[name]].
        let target = package_map
            .package(dependency_id)
            .ok_or_else(|| ResolveError::NotFound(specifier.to_string()))?;

        // 6. Let PACKAGE_PATH be the resolved path of TARGET.
        let package_path =
            target.path().ok_or_else(|| ResolveError::NotFound(specifier.to_string()))?;
        let package_path = self.cache.value(package_path);

        // 7. LOAD_PACKAGE_EXPORTS(SUBPATH, PACKAGE_PATH).
        if self.is_dir(&package_path, ctx)
            && let Some(path) =
                self.load_package_exports(specifier, subpath, &package_path, tsconfig, ctx)?
        {
            return Ok(path);
        }

        // `CachedPath::normalize_with` expects a relative operand. Prefixing the specified `/`
        // SUBPATH with `.` preserves PACKAGE_PATH/SUBPATH instead of treating it as a root path.
        let dot_subpath = Self::dot_subpath(subpath);
        let package_subpath = package_path.normalize_with(dot_subpath.as_ref(), &self.cache);

        // 8. LOAD_AS_FILE(PACKAGE_PATH/SUBPATH).
        if !subpath.ends_with('/')
            && let Some(path) = self.load_as_file(&package_subpath, tsconfig, ctx)?
        {
            return Ok(path);
        }

        // 9. LOAD_AS_DIRECTORY(PACKAGE_PATH/SUBPATH).
        if self.is_dir(&package_subpath, ctx)
            && let Some(path) = self.load_as_directory(&package_subpath, tsconfig, ctx)?
        {
            return Ok(path);
        }

        // 10. THROW "not found".
        Err(ResolveError::NotFound(specifier.to_string()))
    }

    /// Loads the package map selected by `NODE_OPTIONS` synchronously and caches either result.
    ///
    /// Node loads its map synchronously at startup. Loading is deferred here until the first
    /// applicable request because `Resolver::new` cannot report I/O or JSON errors. The map remains
    /// static for the lifetime of this resolver, matching Node's package-map limitation.
    fn package_map(&self, ctx: &mut Ctx) -> Result<&PackageMap, ResolveError> {
        let package_map = self
            .package_map
            .as_ref()
            .expect("a package map cache is created when NODE_OPTIONS selects a package map");
        let package_map_path = &package_map.path;
        let package_map = package_map.value.get_or_init(|| {
            let json = self.cache.fs.read(package_map_path)?;
            let realpath = if self.options.symlinks {
                self.cache.canonicalize(&self.cache.value(package_map_path))?
            } else {
                package_map_path.clone()
            };
            PackageMap::parse(package_map_path.clone(), &realpath, json)
                .map(Arc::new)
                .map_err(ResolveError::Json)
        });

        match package_map {
            Ok(package_map) => {
                ctx.add_file_dependency(package_map.path());
                Ok(package_map)
            }
            Err(error) => {
                match error {
                    ResolveError::Json(error) => ctx.add_file_dependency(&error.path),
                    _ => ctx.add_missing_dependency(package_map_path),
                }
                Err(error.clone())
            }
        }
    }
}
