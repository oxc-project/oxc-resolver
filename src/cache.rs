use std::{
    borrow::Cow,
    cell::RefCell,
    convert::AsRef,
    fmt,
    hash::{BuildHasherDefault, Hash, Hasher},
    io,
    ops::Deref,
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use cfg_if::cfg_if;
use once_cell::sync::OnceCell as OnceLock;
use papaya::{Equivalent, HashMap, HashSet};
use rustc_hash::FxHasher;

use crate::{
    FileMetadata, FileSystem, PackageJson, ResolveError, ResolveOptions, TsConfig,
    context::ResolveContext as Ctx, path::PathUtil,
};

static THREAD_COUNT: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// Per-thread pre-allocated path that is used to perform operations on paths more quickly.
    /// Learned from parcel <https://github.com/parcel-bundler/parcel/blob/a53f8f3ba1025c7ea8653e9719e0a61ef9717079/crates/parcel-resolver/src/cache.rs#L394>
  pub static SCRATCH_PATH: RefCell<PathBuf> = RefCell::new(PathBuf::with_capacity(256));
  pub static THREAD_ID: u64 = THREAD_COUNT.fetch_add(1, Ordering::SeqCst);
}

/// Cache implementation used for caching filesystem access.
#[derive(Default)]
pub struct Cache<Fs> {
    pub(crate) fs: Fs,
    paths: HashSet<CachedPath, BuildHasherDefault<IdentityHasher>>,
    tsconfigs: HashMap<PathBuf, Arc<TsConfig>, BuildHasherDefault<FxHasher>>,
    #[cfg(feature = "yarn_pnp")]
    yarn_pnp_manifest: OnceLock<pnp::Manifest>,
}

impl<Fs: FileSystem> Cache<Fs> {
    pub fn clear(&self) {
        self.paths.pin().clear();
        self.tsconfigs.pin().clear();
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn value(&self, path: &Path) -> CachedPath {
        // `Path::hash` is slow: https://doc.rust-lang.org/std/path/struct.Path.html#impl-Hash-for-Path
        // `path.as_os_str()` hash is not stable because we may joined a path like `foo/bar` and `foo\\bar` on windows.
        let hash = {
            let mut hasher = FxHasher::default();
            path.as_os_str().hash(&mut hasher);
            hasher.finish()
        };
        let paths = self.paths.pin();
        if let Some(entry) = paths.get(&BorrowedCachedPath { hash, path }) {
            return entry.clone();
        }
        let parent = path.parent().map(|p| self.value(p));
        let is_node_modules = path.file_name().as_ref().is_some_and(|&name| name == "node_modules");
        let inside_node_modules =
            is_node_modules || parent.as_ref().is_some_and(|parent| parent.inside_node_modules);
        let cached_path = CachedPath(Arc::new(CachedPathImpl::new(
            hash,
            path.to_path_buf().into_boxed_path(),
            is_node_modules,
            inside_node_modules,
            parent,
        )));
        paths.insert(cached_path.clone());
        cached_path
    }

    pub(crate) fn canonicalize(&self, path: &CachedPath) -> Result<PathBuf, ResolveError> {
        let cached_path = self.canonicalize_impl(path)?;
        let path = cached_path.to_path_buf();
        cfg_if! {
            if #[cfg(target_os = "windows")] {
                crate::windows::try_strip_windows_prefix(path)
            } else {
                Ok(path)
            }
        }
    }

    pub(crate) fn is_file(&self, path: &CachedPath, ctx: &mut Ctx) -> bool {
        if let Some(meta) = path.meta(&self.fs) {
            ctx.add_file_dependency(path.path());
            meta.is_file
        } else {
            ctx.add_missing_dependency(path.path());
            false
        }
    }

    pub(crate) fn is_dir(&self, path: &CachedPath, ctx: &mut Ctx) -> bool {
        path.meta(&self.fs).map_or_else(
            || {
                ctx.add_missing_dependency(path.path());
                false
            },
            |meta| meta.is_dir,
        )
    }

    pub(crate) fn get_package_json(
        &self,
        path: &CachedPath,
        options: &ResolveOptions,
        ctx: &mut Ctx,
    ) -> Result<Option<(CachedPath, Arc<PackageJson>)>, ResolveError> {
        // Change to `std::sync::OnceLock::get_or_try_init` when it is stable.
        let result = path
            .package_json
            .get_or_try_init(|| {
                let package_json_path = path.path.join("package.json");
                let Ok(package_json_string) = self.fs.read_to_string(&package_json_path) else {
                    return Ok(None);
                };
                let real_path = if options.symlinks {
                    self.canonicalize(path)?.join("package.json")
                } else {
                    package_json_path.clone()
                };
                PackageJson::parse(package_json_path.clone(), real_path, &package_json_string)
                    .map(|package_json| Some((path.clone(), (Arc::new(package_json)))))
                    .map_err(|error| ResolveError::from_serde_json_error(package_json_path, &error))
            })
            .cloned();
        // https://github.com/webpack/enhanced-resolve/blob/58464fc7cb56673c9aa849e68e6300239601e615/lib/DescriptionFileUtils.js#L68-L82
        match &result {
            Ok(Some((_, package_json))) => {
                ctx.add_file_dependency(&package_json.path);
            }
            Ok(None) => {
                // Avoid an allocation by making this lazy
                if let Some(deps) = &mut ctx.missing_dependencies {
                    deps.push(path.path.join("package.json"));
                }
            }
            Err(_) => {
                if let Some(deps) = &mut ctx.file_dependencies {
                    deps.push(path.path.join("package.json"));
                }
            }
        }
        result
    }

    pub(crate) fn get_tsconfig<F: FnOnce(&mut TsConfig) -> Result<(), ResolveError>>(
        &self,
        root: bool,
        path: &Path,
        callback: F, // callback for modifying tsconfig with `extends`
    ) -> Result<Arc<TsConfig>, ResolveError> {
        let tsconfigs = self.tsconfigs.pin();
        if let Some(tsconfig) = tsconfigs.get(path) {
            return Ok(Arc::clone(tsconfig));
        }
        let meta = self.fs.metadata(path).ok();
        let tsconfig_path = if meta.is_some_and(|m| m.is_file) {
            Cow::Borrowed(path)
        } else if meta.is_some_and(|m| m.is_dir) {
            Cow::Owned(path.join("tsconfig.json"))
        } else {
            let mut os_string = path.to_path_buf().into_os_string();
            os_string.push(".json");
            Cow::Owned(PathBuf::from(os_string))
        };
        let mut tsconfig_string = self
            .fs
            .read_to_string(&tsconfig_path)
            .map_err(|_| ResolveError::TsconfigNotFound(path.to_path_buf()))?;
        let mut tsconfig =
            TsConfig::parse(root, &tsconfig_path, &mut tsconfig_string).map_err(|error| {
                ResolveError::from_serde_json_error(tsconfig_path.to_path_buf(), &error)
            })?;
        callback(&mut tsconfig)?;
        let tsconfig = Arc::new(tsconfig.build());
        tsconfigs.insert(path.to_path_buf(), Arc::clone(&tsconfig));
        Ok(tsconfig)
    }

    #[cfg(feature = "yarn_pnp")]
    pub(crate) fn get_yarn_pnp_manifest(
        &self,
        cwd: Option<&Path>,
    ) -> Result<&pnp::Manifest, ResolveError> {
        self.yarn_pnp_manifest.get_or_try_init(|| {
            // TODO: map to proper error
            let cwd =
                cwd.map_or_else(|| Cow::Owned(std::env::current_dir().unwrap()), Cow::Borrowed);
            // TODO: map to proper error
            let manifest = pnp::find_pnp_manifest(&cwd).unwrap().unwrap();
            Ok(manifest)
        })
    }
}

impl<Fs: FileSystem> Cache<Fs> {
    pub fn new(fs: Fs) -> Self {
        Self {
            fs,
            paths: HashSet::builder()
                .hasher(BuildHasherDefault::default())
                .resize_mode(papaya::ResizeMode::Blocking)
                .build(),
            tsconfigs: HashMap::builder()
                .hasher(BuildHasherDefault::default())
                .resize_mode(papaya::ResizeMode::Blocking)
                .build(),
            #[cfg(feature = "yarn_pnp")]
            yarn_pnp_manifest: OnceLock::new(),
        }
    }

    /// Returns the canonical path, resolving all symbolic links.
    ///
    /// <https://github.com/parcel-bundler/parcel/blob/4d27ec8b8bd1792f536811fef86e74a31fa0e704/crates/parcel-resolver/src/cache.rs#L232>
    fn canonicalize_impl(&self, path: &CachedPath) -> Result<CachedPath, ResolveError> {
        // Check if this thread is already canonicalizing. If so, we have found a circular symlink.
        // If a different thread is canonicalizing, OnceLock will queue this thread to wait for the result.
        let tid = THREAD_ID.with(|t| *t);
        if path.canonicalizing.load(Ordering::Acquire) == tid {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Circular symlink").into());
        }

        path.canonicalized
            .get_or_init(|| {
                path.canonicalizing.store(tid, Ordering::Release);

                let res = path.parent().map_or_else(
                    || Ok(path.normalize_root(self)),
                    |parent| {
                        self.canonicalize_impl(parent).and_then(|parent_canonical| {
                            let normalized = parent_canonical.normalize_with(
                                path.path().strip_prefix(parent.path()).unwrap(),
                                self,
                            );

                            if self.fs.symlink_metadata(path.path()).is_ok_and(|m| m.is_symlink) {
                                let link = self.fs.read_link(normalized.path())?;
                                if link.is_absolute() {
                                    return self.canonicalize_impl(&self.value(&link.normalize()));
                                } else if let Some(dir) = normalized.parent() {
                                    // Symlink is relative `../../foo.js`, use the path directory
                                    // to resolve this symlink.
                                    return self
                                        .canonicalize_impl(&dir.normalize_with(&link, self));
                                }
                                debug_assert!(
                                    false,
                                    "Failed to get path parent for {}.",
                                    normalized.path().display()
                                );
                            }

                            Ok(normalized)
                        })
                    },
                );

                path.canonicalizing.store(0, Ordering::Release);
                res
            })
            .clone()
    }
}

#[derive(Clone)]
pub struct CachedPath(Arc<CachedPathImpl>);

pub struct CachedPathImpl {
    hash: u64,
    path: Box<Path>,
    parent: Option<CachedPath>,
    is_node_modules: bool,
    inside_node_modules: bool,
    meta: OnceLock<Option<FileMetadata>>,
    canonicalized: OnceLock<Result<CachedPath, ResolveError>>,
    canonicalizing: AtomicU64,
    node_modules: OnceLock<Option<CachedPath>>,
    package_json: OnceLock<Option<(CachedPath, Arc<PackageJson>)>>,
}

impl CachedPathImpl {
    fn new(
        hash: u64,
        path: Box<Path>,
        is_node_modules: bool,
        inside_node_modules: bool,
        parent: Option<CachedPath>,
    ) -> Self {
        Self {
            hash,
            path,
            parent,
            is_node_modules,
            inside_node_modules,
            meta: OnceLock::new(),
            canonicalized: OnceLock::new(),
            canonicalizing: AtomicU64::new(0),
            node_modules: OnceLock::new(),
            package_json: OnceLock::new(),
        }
    }
}

impl Deref for CachedPath {
    type Target = CachedPathImpl;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl CachedPath {
    pub(crate) fn path(&self) -> &Path {
        &self.0.path
    }

    pub(crate) fn to_path_buf(&self) -> PathBuf {
        self.path.to_path_buf()
    }

    pub(crate) fn parent(&self) -> Option<&Self> {
        self.0.parent.as_ref()
    }

    pub(crate) fn is_node_modules(&self) -> bool {
        self.is_node_modules
    }

    pub(crate) fn inside_node_modules(&self) -> bool {
        self.inside_node_modules
    }

    pub(crate) fn module_directory<Fs: FileSystem>(
        &self,
        module_name: &str,
        cache: &Cache<Fs>,
        ctx: &mut Ctx,
    ) -> Option<Self> {
        let cached_path = cache.value(&self.path.join(module_name));
        cache.is_dir(&cached_path, ctx).then_some(cached_path)
    }

    pub(crate) fn cached_node_modules<Fs: FileSystem>(
        &self,
        cache: &Cache<Fs>,
        ctx: &mut Ctx,
    ) -> Option<Self> {
        self.node_modules.get_or_init(|| self.module_directory("node_modules", cache, ctx)).clone()
    }

    /// Find package.json of a path by traversing parent directories.
    ///
    /// # Errors
    ///
    /// * [ResolveError::Json]
    pub(crate) fn find_package_json<Fs: FileSystem>(
        &self,
        options: &ResolveOptions,
        cache: &Cache<Fs>,
        ctx: &mut Ctx,
    ) -> Result<Option<(Self, Arc<PackageJson>)>, ResolveError> {
        let mut cache_value = self;
        // Go up directories when the querying path is not a directory
        while !cache.is_dir(cache_value, ctx) {
            if let Some(cv) = &cache_value.parent {
                cache_value = cv;
            } else {
                break;
            }
        }
        let mut cache_value = Some(cache_value);
        while let Some(cv) = cache_value {
            if let Some(package_json) = cache.get_package_json(cv, options, ctx)? {
                return Ok(Some(package_json));
            }
            cache_value = cv.parent.as_ref();
        }
        Ok(None)
    }

    pub(crate) fn add_extension<Fs: FileSystem>(&self, ext: &str, cache: &Cache<Fs>) -> Self {
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            let s = path.as_mut_os_string();
            s.push(self.path.as_os_str());
            s.push(ext);
            cache.value(path)
        })
    }

    pub(crate) fn replace_extension<Fs: FileSystem>(&self, ext: &str, cache: &Cache<Fs>) -> Self {
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            let s = path.as_mut_os_string();
            let self_len = self.path.as_os_str().len();
            let self_bytes = self.path.as_os_str().as_encoded_bytes();
            let slice_to_copy = self.path.extension().map_or(self_bytes, |previous_extension| {
                &self_bytes[..self_len - previous_extension.len() - 1]
            });
            // SAFETY: ???
            s.push(unsafe { std::ffi::OsStr::from_encoded_bytes_unchecked(slice_to_copy) });
            s.push(ext);
            cache.value(path)
        })
    }

    /// Returns a new path by resolving the given subpath (including "." and ".." components) with this path.
    pub(crate) fn normalize_with<Fs: FileSystem, P: AsRef<Path>>(
        &self,
        subpath: P,
        cache: &Cache<Fs>,
    ) -> Self {
        let subpath = subpath.as_ref();
        let mut components = subpath.components();
        let Some(head) = components.next() else { return cache.value(subpath) };
        if matches!(head, Component::Prefix(..) | Component::RootDir) {
            return cache.value(subpath);
        }
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            path.push(&self.path);
            for component in std::iter::once(head).chain(components) {
                match component {
                    Component::CurDir => {}
                    Component::ParentDir => {
                        path.pop();
                    }
                    Component::Normal(c) => {
                        cfg_if! {
                            if #[cfg(target_family = "wasm")] {
                                // Need to trim the extra \0 introduces by https://github.com/nodejs/uvwasi/issues/262
                                path.push(c.to_string_lossy().trim_end_matches('\0'));
                            } else {
                                path.push(c);
                            }
                        }
                    }
                    Component::Prefix(..) | Component::RootDir => {
                        unreachable!("Path {:?} Subpath {:?}", self.path, subpath)
                    }
                }
            }

            cache.value(path)
        })
    }

    #[inline]
    #[cfg(windows)]
    pub(crate) fn normalize_root<Fs: FileSystem>(&self, cache: &Cache<Fs>) -> Self {
        if self.path().as_os_str().as_encoded_bytes().last() == Some(&b'/') {
            let mut path_string = self.path.to_string_lossy().into_owned();
            path_string.pop();
            path_string.push('\\');
            cache.value(&PathBuf::from(path_string))
        } else {
            self.clone()
        }
    }

    #[inline]
    #[cfg(not(windows))]
    pub(crate) fn normalize_root<Fs: FileSystem>(&self, _cache: &Cache<Fs>) -> Self {
        self.clone()
    }
}

impl CachedPath {
    fn meta<Fs: FileSystem>(&self, fs: &Fs) -> Option<FileMetadata> {
        *self.meta.get_or_init(|| fs.metadata(&self.path).ok())
    }
}

impl Hash for CachedPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl PartialEq for CachedPath {
    fn eq(&self, other: &Self) -> bool {
        self.path.as_os_str() == other.path.as_os_str()
    }
}

impl Eq for CachedPath {}

impl fmt::Debug for CachedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FsCachedPath").field("path", &self.path).finish()
    }
}

struct BorrowedCachedPath<'a> {
    hash: u64,
    path: &'a Path,
}

impl Equivalent<CachedPath> for BorrowedCachedPath<'_> {
    fn equivalent(&self, other: &CachedPath) -> bool {
        self.path.as_os_str() == other.path.as_os_str()
    }
}

impl Hash for BorrowedCachedPath<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl PartialEq for BorrowedCachedPath<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.path.as_os_str() == other.path.as_os_str()
    }
}

/// Since the cache key is memoized, use an identity hasher
/// to avoid double cache.
#[derive(Default)]
struct IdentityHasher(u64);

impl Hasher for IdentityHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("Invalid use of IdentityHasher")
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = n;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}
