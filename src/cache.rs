use std::{
    borrow::Cow,
    cell::RefCell,
    convert::AsRef,
    hash::{BuildHasherDefault, Hash, Hasher},
    io,
    ops::Deref,
    path::{Component, Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use cfg_if::cfg_if;
use dashmap::{DashMap, DashSet};
use once_cell::sync::OnceCell as OnceLock;
use rustc_hash::FxHasher;

use crate::{
    context::ResolveContext as Ctx, package_json::PackageJson, path::PathUtil, FileMetadata,
    FileSystem, ResolveError, ResolveOptions, TsConfig,
};

static THREAD_COUNT: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// Per-thread pre-allocated path that is used to perform operations on paths more quickly.
    /// Learned from parcel <https://github.com/parcel-bundler/parcel/blob/a53f8f3ba1025c7ea8653e9719e0a61ef9717079/crates/parcel-resolver/src/cache.rs#L394>
  pub static SCRATCH_PATH: RefCell<PathBuf> = RefCell::new(PathBuf::with_capacity(256));
  pub static THREAD_ID: u64 = THREAD_COUNT.fetch_add(1, Ordering::SeqCst);
}

#[derive(Default)]
pub struct Cache<Fs> {
    pub(crate) fs: Fs,
    paths: DashSet<PathEntry<'static>, BuildHasherDefault<IdentityHasher>>,
    tsconfigs: DashMap<PathBuf, Arc<TsConfig>, BuildHasherDefault<FxHasher>>,
}

/// An entry in the path cache. Can also be borrowed for lookups without allocations.
enum PathEntry<'a> {
    Owned(CachedPath),
    Borrowed { hash: u64, path: &'a Path },
}

impl Hash for PathEntry<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            PathEntry::Owned(entry) => {
                entry.hash.hash(state);
            }
            PathEntry::Borrowed { hash, .. } => {
                hash.hash(state);
            }
        }
    }
}

impl PartialEq for PathEntry<'_> {
    fn eq(&self, other: &Self) -> bool {
        let self_path = match self {
            PathEntry::Owned(info) => &info.path,
            PathEntry::Borrowed { path, .. } => *path,
        };
        let other_path = match other {
            PathEntry::Owned(info) => &info.path,
            PathEntry::Borrowed { path, .. } => *path,
        };
        self_path.as_os_str() == other_path.as_os_str()
    }
}
impl Eq for PathEntry<'_> {}

impl<Fs: FileSystem> Cache<Fs> {
    pub fn new(fs: Fs) -> Self {
        Self { fs, paths: DashSet::default(), tsconfigs: DashMap::default() }
    }

    pub fn clear(&self) {
        self.paths.clear();
        self.tsconfigs.clear();
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn value(&self, path: &Path) -> CachedPath {
        // `Path::hash` is slow: https://doc.rust-lang.org/std/path/struct.Path.html#impl-Hash-for-Path
        // `path.as_os_str()` hash is not stable because we may joined a path like `foo/bar` and `foo\\bar` on windows.
        let hash = {
            let mut hasher = FxHasher::default();
            path.as_os_str().hash(&mut hasher);
            hasher.finish()
        };
        let key = PathEntry::Borrowed { hash, path };
        // A DashMap is just an array of RwLock<HashSet>, sharded by hash to reduce lock contention.
        // This uses the low level raw API to avoid cloning the value when using the `entry` method.
        // First, find which shard the value is in, and check to see if we already have a value in the map.
        let shard = self.paths.determine_shard(hash as usize);
        {
            // Scope the read lock.
            let map = self.paths.shards()[shard].read();
            if let Some((PathEntry::Owned(entry), _)) = map.get(hash, |v| v.0 == key) {
                return entry.clone();
            }
        }
        let parent = path.parent().map(|p| self.value(p));
        let cached_path = CachedPath(Arc::new(CachedPathImpl::new(
            hash,
            path.to_path_buf().into_boxed_path(),
            parent,
        )));
        self.paths.insert(PathEntry::Owned(cached_path.clone()));
        cached_path
    }

    pub fn tsconfig<F: FnOnce(&mut TsConfig) -> Result<(), ResolveError>>(
        &self,
        root: bool,
        path: &Path,
        callback: F, // callback for modifying tsconfig with `extends`
    ) -> Result<Arc<TsConfig>, ResolveError> {
        if let Some(tsconfig_ref) = self.tsconfigs.get(path) {
            return Ok(Arc::clone(tsconfig_ref.value()));
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
        self.tsconfigs.insert(path.to_path_buf(), Arc::clone(&tsconfig));
        Ok(tsconfig)
    }
}

#[derive(Clone)]
pub struct CachedPath(Arc<CachedPathImpl>);

pub struct CachedPathImpl {
    hash: u64,
    path: Box<Path>,
    parent: Option<CachedPath>,
    meta: OnceLock<Option<FileMetadata>>,
    canonicalized: OnceLock<Result<CachedPath, ResolveError>>,
    canonicalizing: AtomicU64,
    node_modules: OnceLock<Option<CachedPath>>,
    package_json: OnceLock<Option<(CachedPath, Arc<PackageJson>)>>,
}

impl CachedPathImpl {
    const fn new(hash: u64, path: Box<Path>, parent: Option<CachedPath>) -> Self {
        Self {
            hash,
            path,
            parent,
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
    pub fn path(&self) -> &Path {
        &self.0.path
    }

    pub fn to_path_buf(&self) -> PathBuf {
        self.path.to_path_buf()
    }

    pub fn parent(&self) -> Option<&Self> {
        self.0.parent.as_ref()
    }

    fn meta<Fs: FileSystem>(&self, fs: &Fs) -> Option<FileMetadata> {
        *self.meta.get_or_init(|| fs.metadata(&self.path).ok())
    }

    pub fn is_file<Fs: FileSystem>(&self, fs: &Fs, ctx: &mut Ctx) -> bool {
        if let Some(meta) = self.meta(fs) {
            ctx.add_file_dependency(self.path());
            meta.is_file
        } else {
            ctx.add_missing_dependency(self.path());
            false
        }
    }

    pub fn is_dir<Fs: FileSystem>(&self, fs: &Fs, ctx: &mut Ctx) -> bool {
        self.meta(fs).map_or_else(
            || {
                ctx.add_missing_dependency(self.path());
                false
            },
            |meta| meta.is_dir,
        )
    }

    pub fn canonicalize<Fs: FileSystem>(&self, cache: &Cache<Fs>) -> Result<PathBuf, ResolveError> {
        let cached_path = self.canocalize_impl(cache)?;
        let path = cached_path.to_path_buf();
        cfg_if! {
            if #[cfg(windows)] {
                let path = crate::FileSystemOs::strip_windows_prefix(path);
            }
        }
        Ok(path)
    }

    /// Returns the canonical path, resolving all symbolic links.
    ///
    /// <https://github.com/parcel-bundler/parcel/blob/4d27ec8b8bd1792f536811fef86e74a31fa0e704/crates/parcel-resolver/src/cache.rs#L232>
    fn canocalize_impl<Fs: FileSystem>(&self, cache: &Cache<Fs>) -> Result<Self, ResolveError> {
        // Check if this thread is already canonicalizing. If so, we have found a circular symlink.
        // If a different thread is canonicalizing, OnceLock will queue this thread to wait for the result.
        let tid = THREAD_ID.with(|t| *t);
        if self.0.canonicalizing.load(Ordering::Acquire) == tid {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Circular symlink").into());
        }

        self.0
            .canonicalized
            .get_or_init(|| {
                self.0.canonicalizing.store(tid, Ordering::Release);

                let res = self.parent().map_or_else(
                    || Ok(self.clone()),
                    |parent| {
                        parent.canocalize_impl(cache).and_then(|parent_canonical| {
                            let path = parent_canonical.normalize_with(
                                self.path().strip_prefix(parent.path()).unwrap(),
                                cache,
                            );

                            if cache.fs.symlink_metadata(self.path()).is_ok_and(|m| m.is_symlink) {
                                let link = cache.fs.read_link(path.path())?;
                                if link.is_absolute() {
                                    return cache.value(&link.normalize()).canocalize_impl(cache);
                                } else if let Some(dir) = path.parent() {
                                    // Symlink is relative `../../foo.js`, use the path directory
                                    // to resolve this symlink.
                                    return dir.normalize_with(&link, cache).canocalize_impl(cache);
                                }
                                debug_assert!(
                                    false,
                                    "Failed to get path parent for {:?}.",
                                    path.path()
                                );
                            }

                            Ok(path)
                        })
                    },
                );

                self.0.canonicalizing.store(0, Ordering::Release);
                res
            })
            .clone()
    }

    pub fn module_directory<Fs: FileSystem>(
        &self,
        module_name: &str,
        cache: &Cache<Fs>,
        ctx: &mut Ctx,
    ) -> Option<Self> {
        let cached_path = cache.value(&self.path.join(module_name));
        cached_path.is_dir(&cache.fs, ctx).then_some(cached_path)
    }

    pub fn cached_node_modules<Fs: FileSystem>(
        &self,
        cache: &Cache<Fs>,
        ctx: &mut Ctx,
    ) -> Option<Self> {
        self.node_modules.get_or_init(|| self.module_directory("node_modules", cache, ctx)).clone()
    }

    /// Get package.json of the given path.
    ///
    /// # Errors
    ///
    /// * [ResolveError::JSON]
    pub fn package_json<Fs: FileSystem>(
        &self,
        options: &ResolveOptions,
        cache: &Cache<Fs>,
        ctx: &mut Ctx,
    ) -> Result<Option<(Self, Arc<PackageJson>)>, ResolveError> {
        // Change to `std::sync::OnceLock::get_or_try_init` when it is stable.
        let result = self
            .package_json
            .get_or_try_init(|| {
                let package_json_path = self.path.join("package.json");
                let Ok(package_json_string) = cache.fs.read_to_string(&package_json_path) else {
                    return Ok(None);
                };
                let real_path = if options.symlinks {
                    self.canonicalize(cache)?.join("package.json")
                } else {
                    package_json_path.clone()
                };
                PackageJson::parse(package_json_path.clone(), real_path, &package_json_string)
                    .map(|package_json| Some((self.clone(), (Arc::new(package_json)))))
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
                    deps.push(self.path.join("package.json"));
                }
            }
            Err(_) => {
                if let Some(deps) = &mut ctx.file_dependencies {
                    deps.push(self.path.join("package.json"));
                }
            }
        }
        result
    }

    /// Find package.json of a path by traversing parent directories.
    ///
    /// # Errors
    ///
    /// * [ResolveError::JSON]
    pub fn find_package_json<Fs: FileSystem>(
        &self,
        options: &ResolveOptions,
        cache: &Cache<Fs>,
        ctx: &mut Ctx,
    ) -> Result<Option<(Self, Arc<PackageJson>)>, ResolveError> {
        let mut cache_value = self;
        // Go up directories when the querying path is not a directory
        while !cache_value.is_dir(&cache.fs, ctx) {
            if let Some(cv) = &cache_value.parent {
                cache_value = cv;
            } else {
                break;
            }
        }
        let mut cache_value = Some(cache_value);
        while let Some(cv) = cache_value {
            if let Some(package_json) = cv.package_json(options, cache, ctx)? {
                return Ok(Some(package_json));
            }
            cache_value = cv.parent.as_ref();
        }
        Ok(None)
    }

    pub fn add_extension<Fs: FileSystem>(&self, ext: &str, cache: &Cache<Fs>) -> Self {
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            let s = path.as_mut_os_string();
            s.push(self.path.as_os_str());
            s.push(ext);
            cache.value(path)
        })
    }

    pub fn replace_extension<Fs: FileSystem>(&self, ext: &str, cache: &Cache<Fs>) -> Self {
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
    pub fn normalize_with<P, Fs>(&self, subpath: P, cache: &Cache<Fs>) -> Self
    where
        P: AsRef<Path>,
        Fs: FileSystem,
    {
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
