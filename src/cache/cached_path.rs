use std::{
    convert::AsRef,
    fmt,
    hash::{Hash, Hasher},
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex, atomic::AtomicU64},
};

use cfg_if::cfg_if;
use once_cell::sync::OnceCell as OnceLock;

use super::cache_impl::Cache;
use super::thread_local::SCRATCH_PATH;
use crate::{
    FileMetadata, FileSystem, PackageJson, ResolveError, ResolveOptions, TsConfig,
    context::ResolveContext as Ctx,
};

#[derive(Clone, Copy)]
pub struct CachedPath(pub usize);

pub struct CachedPathImpl {
    pub hash: u64,
    pub path: PathBuf,
    pub parent: Option<usize>,
    pub is_node_modules: bool,
    pub inside_node_modules: bool,
    pub meta: OnceLock<Option<FileMetadata>>,
    pub canonicalized: Mutex<Result<Option<usize>, ResolveError>>,
    pub canonicalizing: AtomicU64,
    pub node_modules: OnceLock<Option<usize>>,
    pub package_json: OnceLock<Option<Arc<PackageJson>>>,
    pub tsconfig: OnceLock<Option<Arc<TsConfig>>>,
}

impl CachedPathImpl {
    pub fn new(
        hash: u64,
        path: PathBuf,
        is_node_modules: bool,
        inside_node_modules: bool,
        parent: Option<usize>,
    ) -> Self {
        Self {
            hash,
            path,
            parent,
            is_node_modules,
            inside_node_modules,
            meta: OnceLock::new(),
            canonicalized: Mutex::new(Ok(None)),
            canonicalizing: AtomicU64::new(0),
            node_modules: OnceLock::new(),
            package_json: OnceLock::new(),
            tsconfig: OnceLock::new(),
        }
    }
}

impl CachedPath {
    pub(crate) fn path<'a, Fs: FileSystem>(&self, cache: &'a Cache<Fs>) -> &'a Path {
        &cache.get_node(self.0).path
    }

    pub(crate) fn to_path_buf<Fs: FileSystem>(&self, cache: &Cache<Fs>) -> PathBuf {
        self.path(cache).to_path_buf()
    }

    pub(crate) fn parent<Fs: FileSystem>(&self, cache: &Cache<Fs>) -> Option<Self> {
        cache.get_node(self.0).parent.map(CachedPath)
    }

    pub(crate) fn is_node_modules<Fs: FileSystem>(&self, cache: &Cache<Fs>) -> bool {
        cache.get_node(self.0).is_node_modules
    }

    pub(crate) fn inside_node_modules<Fs: FileSystem>(&self, cache: &Cache<Fs>) -> bool {
        cache.get_node(self.0).inside_node_modules
    }

    pub(crate) fn module_directory<Fs: FileSystem>(
        &self,
        module_name: &str,
        cache: &Cache<Fs>,
        ctx: &mut Ctx,
    ) -> Option<Self> {
        let path = self.path(cache).join(module_name);
        let cached_path = cache.value(&path);
        cache.is_dir(&cached_path, ctx).then_some(cached_path)
    }

    pub(crate) fn cached_node_modules<Fs: FileSystem>(
        &self,
        cache: &Cache<Fs>,
        ctx: &mut Ctx,
    ) -> Option<Self> {
        let node = cache.get_node(self.0);
        let result = *node.node_modules.get_or_init(|| {
            self.module_directory("node_modules", cache, ctx).map(|cp| cp.0)
        });
        result.map(CachedPath)
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
    ) -> Result<Option<Arc<PackageJson>>, ResolveError> {
        let mut cache_value = *self;
        // Go up directories when the querying path is not a directory
        while !cache.is_dir(&cache_value, ctx) {
            if let Some(cv) = cache_value.parent(cache) {
                cache_value = cv;
            } else {
                break;
            }
        }
        let mut cache_value = Some(cache_value);
        while let Some(cv) = cache_value {
            if let Some(package_json) = cache.get_package_json(&cv, options, ctx)? {
                return Ok(Some(package_json));
            }
            cache_value = cv.parent(cache);
        }
        Ok(None)
    }

    pub(crate) fn add_extension<Fs: FileSystem>(&self, ext: &str, cache: &Cache<Fs>) -> Self {
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            let s = path.as_mut_os_string();
            s.push(self.path(cache).as_os_str());
            s.push(ext);
            cache.value(path)
        })
    }

    pub(crate) fn replace_extension<Fs: FileSystem>(&self, ext: &str, cache: &Cache<Fs>) -> Self {
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            let s = path.as_mut_os_string();
            let node_path = self.path(cache);
            let self_len = node_path.as_os_str().len();
            let self_bytes = node_path.as_os_str().as_encoded_bytes();
            let slice_to_copy = node_path.extension().map_or(self_bytes, |previous_extension| {
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
            path.push(self.path(cache));
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
                        unreachable!("Path {:?} Subpath {:?}", self.path(cache), subpath)
                    }
                }
            }

            cache.value(path)
        })
    }

    #[inline]
    #[cfg(windows)]
    pub(crate) fn normalize_root<Fs: FileSystem>(&self, cache: &Cache<Fs>) -> Self {
        if self.path(cache).as_os_str().as_encoded_bytes().last() == Some(&b'/') {
            let mut path_string = self.path(cache).to_string_lossy().into_owned();
            path_string.pop();
            path_string.push('\\');
            cache.value(&PathBuf::from(path_string))
        } else {
            *self
        }
    }

    #[inline]
    #[cfg(not(windows))]
    pub(crate) fn normalize_root<Fs: FileSystem>(&self, _cache: &Cache<Fs>) -> Self {
        *self
    }
}

impl CachedPath {
    pub(crate) fn meta<Fs: FileSystem>(&self, cache: &Cache<Fs>, fs: &Fs) -> Option<FileMetadata> {
        let node = cache.get_node(self.0);
        *node.meta.get_or_init(|| fs.metadata(&node.path).ok())
    }
}

impl Hash for CachedPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialEq for CachedPath {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for CachedPath {}

impl fmt::Debug for CachedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FsCachedPath").field("idx", &self.0).finish()
    }
}
