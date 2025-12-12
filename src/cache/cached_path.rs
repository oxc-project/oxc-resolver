use std::{
    convert::AsRef,
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
    path::{Component, Path, PathBuf},
    sync::{Arc, Weak},
};

use cfg_if::cfg_if;
use once_cell::sync::OnceCell as OnceLock;

use super::cache_impl::Cache;
use super::cache_impl::PackageJsonIndex;
use super::thread_local::SCRATCH_PATH;
use crate::{FileSystem, TsConfig, context::ResolveContext as Ctx};

#[derive(Clone)]
pub struct CachedPath(pub Arc<CachedPathImpl>);

pub struct CachedPathImpl {
    pub hash: u64,
    pub path: Box<Path>,
    pub parent: Option<Weak<Self>>,
    pub is_node_modules: bool,
    pub inside_node_modules: bool,
    pub meta: OnceLock<Option<(/* is_file */ bool, /* is_dir */ bool)>>, // None means not found.
    pub canonicalized: OnceLock<(Weak<Self>, PathBuf)>,
    pub node_modules: OnceLock<Option<Weak<Self>>>,
    pub package_json: OnceLock<Option<PackageJsonIndex>>,
    /// `tsconfig.json` found at path.
    pub tsconfig: OnceLock<Option<Arc<TsConfig>>>,
    /// `tsconfig.json` after resolving `references`, `files`, `include` and `extend`.
    pub resolved_tsconfig: OnceLock<Option<Arc<TsConfig>>>,
}

impl CachedPathImpl {
    pub fn new(
        hash: u64,
        path: Box<Path>,
        is_node_modules: bool,
        inside_node_modules: bool,
        parent: Option<Weak<Self>>,
    ) -> Self {
        Self {
            hash,
            path,
            parent,
            is_node_modules,
            inside_node_modules,
            meta: OnceLock::new(),
            canonicalized: OnceLock::new(),
            node_modules: OnceLock::new(),
            package_json: OnceLock::new(),
            tsconfig: OnceLock::new(),
            resolved_tsconfig: OnceLock::new(),
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

    pub(crate) fn parent<Fs: FileSystem>(&self, cache: &Cache<Fs>) -> Option<Self> {
        self.0.parent.as_ref().and_then(|weak| {
            weak.upgrade().map(CachedPath).or_else(|| {
                // Weak pointer upgrade failed - parent was cleared from cache
                // Recreate it by deriving the parent path
                self.path().parent().map(|parent_path| cache.value(parent_path))
            })
        })
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
        let cached_path = self.push(module_name, cache);
        cache.is_dir(&cached_path, ctx).then_some(cached_path)
    }

    pub(crate) fn cached_node_modules<Fs: FileSystem>(
        &self,
        cache: &Cache<Fs>,
        ctx: &mut Ctx,
    ) -> Option<Self> {
        self.node_modules
            .get_or_init(|| {
                self.module_directory("node_modules", cache, ctx).map(|cp| Arc::downgrade(&cp.0))
            })
            .as_ref()
            .and_then(|weak| {
                weak.upgrade().map(CachedPath).or_else(|| {
                    // Weak pointer upgrade failed - recreate by deriving the node_modules path
                    Some(self.push("node_modules", cache))
                })
            })
    }

    pub(crate) fn push<Fs: FileSystem>(&self, target: &str, cache: &Cache<Fs>) -> Self {
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            path.push(&self.path);
            path.push(target);
            cache.value(path)
        })
    }

    pub(crate) fn add_extension<Fs: FileSystem>(&self, target: &str, cache: &Cache<Fs>) -> Self {
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            let s = path.as_mut_os_string();
            s.push(self.path.as_os_str());
            s.push(target);
            cache.value(path)
        })
    }

    pub(crate) fn add_name_and_extension<Fs: FileSystem>(
        &self,
        name: &str,
        ext: &str,
        cache: &Cache<Fs>,
    ) -> Self {
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            let s = path.as_mut_os_string();
            s.push(self.path.as_os_str());
            s.push(std::path::MAIN_SEPARATOR_STR);
            s.push(name);
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
    fn metadata<Fs: FileSystem>(&self, fs: &Fs) -> Option<(bool, bool)> {
        *self.meta.get_or_init(|| fs.metadata(&self.path).ok().map(|r| (r.is_file, r.is_dir)))
    }

    pub(crate) fn is_file<Fs: FileSystem>(&self, fs: &Fs) -> Option<bool> {
        self.metadata(fs).map(|r| r.0)
    }

    pub(crate) fn is_dir<Fs: FileSystem>(&self, fs: &Fs) -> Option<bool> {
        self.metadata(fs).map(|r| r.1)
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

impl fmt::Display for CachedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

impl fmt::Debug for CachedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.path)
    }
}
