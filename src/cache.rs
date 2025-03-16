use once_cell::sync::OnceCell as OnceLock;
use std::{
    borrow::{Borrow, Cow},
    convert::AsRef,
    hash::{BuildHasherDefault, Hash, Hasher},
    io,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

use dashmap::{DashMap, DashSet};
use rustc_hash::FxHasher;

use crate::{
    FileMetadata, FileSystem, ResolveError, ResolveOptions, TsConfig,
    context::ResolveContext as Ctx, package_json::PackageJson, path::PathUtil,
};

#[derive(Default)]
pub struct Cache<Fs> {
    pub(crate) fs: Fs,
    paths: DashSet<CachedPath, BuildHasherDefault<IdentityHasher>>,
    tsconfigs: DashMap<PathBuf, Arc<TsConfig>, BuildHasherDefault<FxHasher>>,
}

impl<Fs: FileSystem> Cache<Fs> {
    pub fn new(fs: Fs) -> Self {
        Self { fs, paths: DashSet::default(), tsconfigs: DashMap::default() }
    }

    pub fn clear(&self) {
        self.paths.clear();
        self.tsconfigs.clear();
    }

    pub fn value(&self, path: &Path) -> CachedPath {
        let hash = {
            let mut hasher = FxHasher::default();
            path.hash(&mut hasher);
            hasher.finish()
        };
        if let Some(cache_entry) = self.paths.get((hash, path).borrow() as &dyn CacheKey) {
            return cache_entry.clone();
        }
        let parent = path.parent().map(|p| self.value(p));
        let data = CachedPath(Arc::new(CachedPathImpl::new(
            hash,
            path.to_path_buf().into_boxed_path(),
            parent,
        )));
        self.paths.insert(data.clone());
        data
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
                ResolveError::from_serde_json_error(
                    tsconfig_path.to_path_buf(),
                    &error,
                    Some(tsconfig_string),
                )
            })?;
        callback(&mut tsconfig)?;
        let tsconfig = Arc::new(tsconfig.build());
        self.tsconfigs.insert(path.to_path_buf(), Arc::clone(&tsconfig));
        Ok(tsconfig)
    }
}

#[derive(Clone)]
pub struct CachedPath(Arc<CachedPathImpl>);

impl Hash for CachedPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash.hash(state);
    }
}

impl PartialEq for CachedPath {
    fn eq(&self, other: &Self) -> bool {
        self.0.path == other.0.path
    }
}
impl Eq for CachedPath {}

impl Deref for CachedPath {
    type Target = CachedPathImpl;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl<'a> Borrow<dyn CacheKey + 'a> for CachedPath {
    fn borrow(&self) -> &(dyn CacheKey + 'a) {
        self
    }
}

impl AsRef<CachedPathImpl> for CachedPath {
    fn as_ref(&self) -> &CachedPathImpl {
        self.0.as_ref()
    }
}

impl CacheKey for CachedPath {
    fn tuple(&self) -> (u64, &Path) {
        (self.hash, &self.path)
    }
}

pub struct CachedPathImpl {
    hash: u64,
    path: Box<Path>,
    parent: Option<CachedPath>,
    meta: OnceLock<Option<FileMetadata>>,
    canonicalized: OnceLock<Option<PathBuf>>,
    node_modules: OnceLock<Option<CachedPath>>,
    package_json: OnceLock<Option<Arc<PackageJson>>>,
}

impl CachedPathImpl {
    fn new(hash: u64, path: Box<Path>, parent: Option<CachedPath>) -> Self {
        Self {
            hash,
            path,
            parent,
            meta: OnceLock::new(),
            canonicalized: OnceLock::new(),
            node_modules: OnceLock::new(),
            package_json: OnceLock::new(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn to_path_buf(&self) -> PathBuf {
        self.path.to_path_buf()
    }

    pub fn parent(&self) -> Option<&CachedPath> {
        self.parent.as_ref()
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

    pub fn realpath<Fs: FileSystem>(&self, fs: &Fs) -> io::Result<PathBuf> {
        self.canonicalized
            .get_or_try_init(|| {
                if fs.symlink_metadata(&self.path).is_ok_and(|m| m.is_symlink) {
                    return fs.canonicalize(&self.path).map(Some);
                }
                if let Some(parent) = self.parent() {
                    let parent_path = parent.realpath(fs)?;
                    return Ok(Some(
                        parent_path.normalize_with(self.path.strip_prefix(&parent.path).unwrap()),
                    ));
                }
                Ok(None)
            })
            .cloned()
            .map(|r| r.unwrap_or_else(|| self.path.clone().to_path_buf()))
    }

    pub fn module_directory<Fs: FileSystem>(
        &self,
        module_name: &str,
        cache: &Cache<Fs>,
        ctx: &mut Ctx,
    ) -> Option<CachedPath> {
        let cached_path = cache.value(&self.path.join(module_name));
        cached_path.is_dir(&cache.fs, ctx).then_some(cached_path)
    }

    pub fn cached_node_modules<Fs: FileSystem>(
        &self,
        cache: &Cache<Fs>,
        ctx: &mut Ctx,
    ) -> Option<CachedPath> {
        self.node_modules.get_or_init(|| self.module_directory("node_modules", cache, ctx)).clone()
    }

    /// Find package.json of a path by traversing parent directories.
    ///
    /// # Errors
    ///
    /// * [ResolveError::JSON]
    pub fn find_package_json<Fs: FileSystem>(
        &self,
        fs: &Fs,
        options: &ResolveOptions,
        ctx: &mut Ctx,
    ) -> Result<Option<Arc<PackageJson>>, ResolveError> {
        let mut cache_value = self;
        // Go up directories when the querying path is not a directory
        while !cache_value.is_dir(fs, ctx) {
            if let Some(cv) = &cache_value.parent {
                cache_value = cv.as_ref();
            } else {
                break;
            }
        }
        let mut cache_value = Some(cache_value);
        while let Some(cv) = cache_value {
            if let Some(package_json) = cv.package_json(fs, options, ctx)? {
                return Ok(Some(Arc::clone(&package_json)));
            }
            cache_value = cv.parent.as_deref();
        }
        Ok(None)
    }

    /// Get package.json of the given path.
    ///
    /// # Errors
    ///
    /// * [ResolveError::JSON]
    pub fn package_json<Fs: FileSystem>(
        &self,
        fs: &Fs,
        options: &ResolveOptions,
        ctx: &mut Ctx,
    ) -> Result<Option<Arc<PackageJson>>, ResolveError> {
        // Change to `std::sync::OnceLock::get_or_try_init` when it is stable.
        let result = self
            .package_json
            .get_or_try_init(|| {
                let package_json_path = self.path.join("package.json");
                let Ok(package_json_string) = fs.read_to_string(&package_json_path) else {
                    return Ok(None);
                };
                let real_path = if options.symlinks {
                    self.realpath(fs)?.join("package.json")
                } else {
                    package_json_path.clone()
                };
                PackageJson::parse(package_json_path.clone(), real_path, &package_json_string)
                    .map(Arc::new)
                    .map(Some)
                    .map_err(|error| {
                        ResolveError::from_serde_json_error(
                            package_json_path,
                            &error,
                            Some(package_json_string),
                        )
                    })
            })
            .cloned();
        // https://github.com/webpack/enhanced-resolve/blob/58464fc7cb56673c9aa849e68e6300239601e615/lib/DescriptionFileUtils.js#L68-L82
        match &result {
            Ok(Some(package_json)) => {
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
}

/// Memoized cache key, code adapted from <https://stackoverflow.com/a/50478038>.
trait CacheKey {
    fn tuple(&self) -> (u64, &Path);
}

impl Hash for dyn CacheKey + '_ {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tuple().0.hash(state);
    }
}

impl PartialEq for dyn CacheKey + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.tuple().1 == other.tuple().1
    }
}

impl Eq for dyn CacheKey + '_ {}

impl CacheKey for (u64, &Path) {
    fn tuple(&self) -> (u64, &Path) {
        (self.0, self.1)
    }
}

impl<'a> Borrow<dyn CacheKey + 'a> for (u64, &'a Path) {
    fn borrow(&self) -> &(dyn CacheKey + 'a) {
        self
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
