use std::{
    convert::AsRef,
    fmt,
    hash::{Hash, Hasher},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use cfg_if::cfg_if;

use super::cache_impl::Cache;
use super::path_node::PathHandle;
use super::thread_local::SCRATCH_PATH;
use crate::{
    FileMetadata, FileSystem, PackageJson, ResolveError, ResolveOptions, TsConfig,
    context::ResolveContext as Ctx,
};

// CachedPath now wraps PathHandle (generation-based, index-based parent pointers)
#[derive(Clone)]
pub struct CachedPath(pub(crate) PathHandle);

impl CachedPath {
    pub(crate) fn path(&self) -> PathBuf {
        self.0.path()
    }

    pub(crate) fn to_path_buf(&self) -> PathBuf {
        self.0.path()
    }

    pub(crate) fn parent(&self) -> Option<Self> {
        self.0.parent().map(CachedPath)
    }

    pub(crate) fn is_node_modules(&self) -> bool {
        self.0.is_node_modules()
    }

    pub(crate) fn inside_node_modules(&self) -> bool {
        self.0.inside_node_modules()
    }

    // Access to hash
    pub(crate) fn hash(&self) -> u64 {
        self.0.hash()
    }

    // Access to tsconfig - check if already initialized, otherwise initialize it
    pub(crate) fn get_or_init_tsconfig<F>(&self, f: F) -> Option<Arc<TsConfig>>
    where
        F: FnOnce() -> Option<Arc<TsConfig>>,
    {
        let nodes = self.0.generation.nodes.read().unwrap();
        let node = &nodes[self.0.index as usize];
        node.tsconfig.get_or_init(f).clone()
    }

    pub(crate) fn module_directory<Fs: FileSystem>(
        &self,
        module_name: &str,
        cache: &Cache<Fs>,
        ctx: &mut Ctx,
    ) -> Option<Self> {
        let cached_path = cache.value(&self.path().join(module_name));
        cache.is_dir(&cached_path, ctx).then_some(cached_path)
    }

    pub(crate) fn cached_node_modules<Fs: FileSystem>(
        &self,
        cache: &Cache<Fs>,
        ctx: &mut Ctx,
    ) -> Option<Self> {
        // First check if already initialized
        {
            let nodes = self.0.generation.nodes.read().unwrap();
            let node = &nodes[self.0.index as usize];
            if let Some(Some(idx)) = node.node_modules_idx.get() {
                return Some(CachedPath(PathHandle {
                    index: *idx,
                    generation: self.0.generation.clone(),
                }));
            } else if node.node_modules_idx.get().is_some() {
                // Already initialized but is None (node_modules doesn't exist)
                return None;
            }
        }

        // Not initialized, compute it
        let result_idx = self.module_directory("node_modules", cache, ctx).map(|cp| cp.0.index);

        // Store the result
        {
            let nodes = self.0.generation.nodes.read().unwrap();
            let node = &nodes[self.0.index as usize];
            node.node_modules_idx.get_or_init(|| result_idx);
        }

        result_idx
            .map(|idx| CachedPath(PathHandle { index: idx, generation: self.0.generation.clone() }))
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
        let mut cache_value = self.clone();
        // Go up directories when the querying path is not a directory
        while !cache.is_dir(&cache_value, ctx) {
            if let Some(cv) = cache_value.parent() {
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
            cache_value = cv.parent();
        }
        Ok(None)
    }

    pub(crate) fn add_extension<Fs: FileSystem>(&self, ext: &str, cache: &Cache<Fs>) -> Self {
        let self_path = self.path();
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            let s = path.as_mut_os_string();
            s.push(self_path.as_os_str());
            s.push(ext);
            cache.value(path)
        })
    }

    pub(crate) fn replace_extension<Fs: FileSystem>(&self, ext: &str, cache: &Cache<Fs>) -> Self {
        let self_path = self.path();
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            let s = path.as_mut_os_string();
            let self_len = self_path.as_os_str().len();
            let self_bytes = self_path.as_os_str().as_encoded_bytes();
            let slice_to_copy = self_path.extension().map_or(self_bytes, |previous_extension| {
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
        let self_path = self.path();
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            path.push(&self_path);
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
                        unreachable!("Path {:?} Subpath {:?}", self_path, subpath)
                    }
                }
            }

            cache.value(path)
        })
    }

    #[inline]
    #[cfg(windows)]
    pub(crate) fn normalize_root<Fs: FileSystem>(&self, cache: &Cache<Fs>) -> Self {
        let self_path = self.path();
        if self_path.as_os_str().as_encoded_bytes().last() == Some(&b'/') {
            let mut path_string = self_path.to_string_lossy().into_owned();
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
    pub(crate) fn meta<Fs: FileSystem>(&self, fs: &Fs) -> Option<FileMetadata> {
        let nodes = self.0.generation.nodes.read().unwrap();
        let node = &nodes[self.0.index as usize];
        *node.meta.get_or_init(|| fs.metadata(&node.path).ok())
    }
}

impl Hash for CachedPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash().hash(state);
    }
}

impl PartialEq for CachedPath {
    fn eq(&self, other: &Self) -> bool {
        self.path().as_os_str() == other.path().as_os_str()
    }
}

impl Eq for CachedPath {}

impl fmt::Debug for CachedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FsCachedPath").field("path", &self.path()).finish()
    }
}
