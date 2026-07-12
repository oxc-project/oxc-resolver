use std::{
    convert::AsRef,
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

use once_cell::sync::OnceCell as OnceLock;

use super::cache_impl::Cache;
use super::cached_meta::CachedMeta;
use super::thread_local::SCRATCH_PATH;
use crate::{
    FileMetadata, FileSystem, PackageJson, TsConfig, context::ResolveContext as Ctx,
    path::push_normalized_component,
};

#[derive(Clone)]
pub struct CachedPath(pub Arc<CachedPathImpl>);

pub struct CachedPathImpl {
    pub hash: u64,
    pub path: Box<Path>,
    pub parent: Option<Weak<Self>>,
    pub is_node_modules: bool,
    pub inside_node_modules: bool,
    /// Cached `(is_file, is_dir)` filesystem metadata packed into one byte. See
    /// [`CachedMeta`] for the encoding and the rationale for skipping `OnceLock`.
    pub meta: CachedMeta,
    /// This path's spelling was proven to be its own canonical form: no ancestor (or self) is a
    /// symlink and the key needs no normalization. Lets a canonicalization scan stop here
    /// instead of re-walking to the root. A lost update under a race only costs a re-derivation.
    pub canonical_is_self: AtomicBool,
    /// The canonical form of this path, set only where it differs from the path itself: a symlink
    /// (target, with a live `Weak` used by `followed_metadata`) or a path below a symlink (its
    /// derived canonical form, with a dangling `Weak`). A path that is its own canonical form uses
    /// the [`Self::canonical_is_self`] bit instead, storing no heap — the common case, which
    /// otherwise pinned a `(Weak, Box<Path>)` byte-copy of its own spelling on every cached path.
    /// Stored as `Box<Path>` (not `PathBuf`) to save 8 bytes per cached path entry —
    /// the canonical path is set once and never mutated.
    pub canonicalized: OnceLock<(Weak<Self>, Box<Path>)>,
    pub node_modules: OnceLock<Option<Weak<Self>>>,
    pub package_json: OnceLock<Option<Arc<PackageJson>>>,
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
            meta: CachedMeta::new(),
            canonical_is_self: AtomicBool::new(false),
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

    pub(crate) fn parent(&self, cache: &Cache) -> Option<Self> {
        let weak = self.0.parent.as_ref()?;
        weak.upgrade().map(CachedPath).or_else(|| {
            // Weak pointer upgrade failed - parent was cleared from cache
            // Recreate it by deriving the parent path
            self.path().parent().map(|parent_path| cache.value(parent_path))
        })
    }

    pub(crate) fn is_node_modules(&self) -> bool {
        self.is_node_modules
    }

    pub(crate) fn inside_node_modules(&self) -> bool {
        self.inside_node_modules
    }

    pub(crate) fn module_directory(
        &self,
        module_name: &str,
        symlinks: bool,
        cache: &Cache,
        ctx: &mut Ctx,
    ) -> Option<Self> {
        let cached_path = self.push(module_name, cache);
        cache.is_dir(&cached_path, symlinks, ctx).then_some(cached_path)
    }

    pub(crate) fn cached_node_modules(
        &self,
        symlinks: bool,
        cache: &Cache,
        ctx: &mut Ctx,
    ) -> Option<Self> {
        self.node_modules
            .get_or_init(|| {
                self.module_directory("node_modules", symlinks, cache, ctx)
                    .map(|cp| Arc::downgrade(&cp.0))
            })
            .as_ref()
            .and_then(|weak| {
                weak.upgrade().map(CachedPath).or_else(|| {
                    // Weak pointer upgrade failed - recreate by deriving the node_modules path
                    Some(self.push("node_modules", cache))
                })
            })
    }

    pub(crate) fn push(&self, target: &str, cache: &Cache) -> Self {
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            path.push(&self.path);
            path.push(target);
            cache.value(path)
        })
    }

    pub(crate) fn add_extension(&self, target: &str, cache: &Cache) -> Self {
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            let s = path.as_mut_os_string();
            s.push(self.path.as_os_str());
            s.push(target);
            cache.value(path)
        })
    }

    pub(crate) fn add_name_and_extension(&self, name: &str, ext: &str, cache: &Cache) -> Self {
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

    pub(crate) fn replace_extension(&self, ext: &str, cache: &Cache) -> Self {
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
    #[inline]
    pub(crate) fn normalize_with<P: AsRef<Path>>(&self, subpath: P, cache: &Cache) -> Self {
        // Forward to a single instantiation so the many `AsRef<Path>` call
        // sites don't each monomorphize the full body (binary-size win).
        self.normalize_with_impl(subpath.as_ref(), cache)
    }

    fn normalize_with_impl(&self, subpath: &Path, cache: &Cache) -> Self {
        let mut components = subpath.components();
        let Some(head) = components.next() else { return cache.value(subpath) };
        if matches!(head, Component::Prefix(..) | Component::RootDir) {
            return cache.value(subpath);
        }
        SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            path.push(&self.path);
            // Fold `head` in by hand rather than `std::iter::once(head).chain(components)`, whose
            // `Chain<Once<_>, Components>` adapter bloats the stack frame.
            push_normalized_component(path, head);
            for component in components {
                push_normalized_component(path, component);
            }

            cache.value(path)
        })
    }

    #[inline]
    #[cfg(windows)]
    pub(crate) fn normalize_root(&self, cache: &Cache) -> Self {
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
    pub(crate) fn normalize_root(&self, _cache: &Cache) -> Self {
        self.clone()
    }
}

impl CachedPath {
    /// `lstat` view of this path (the link itself), cached.
    ///
    /// Used both to answer `is_file`/`is_dir` for non-symlinks and by canonicalization to decide
    /// whether to follow a symlink — so the two share a single `lstat` syscall per path.
    pub(crate) fn link_metadata(&self, fs: &dyn FileSystem) -> Option<FileMetadata> {
        self.meta.link_or_init(|| fs.symlink_metadata(&self.path).ok())
    }

    pub(crate) fn canonical_is_self(&self) -> bool {
        self.canonical_is_self.load(Ordering::Relaxed)
    }

    pub(crate) fn set_canonical_is_self(&self) {
        self.canonical_is_self.store(true, Ordering::Relaxed);
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
