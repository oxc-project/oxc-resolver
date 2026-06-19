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
use super::cached_meta::CachedMeta;
use super::thread_local::SCRATCH_PATH;
use crate::{FileMetadata, FileSystem, PackageJson, TsConfig, context::ResolveContext as Ctx};

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
        symlinks: bool,
        cache: &Cache<Fs>,
        ctx: &mut Ctx,
    ) -> Option<Self> {
        let cached_path = self.push(module_name, cache);
        cache.is_dir(&cached_path, symlinks, ctx).then_some(cached_path)
    }

    pub(crate) fn cached_node_modules<Fs: FileSystem>(
        &self,
        symlinks: bool,
        cache: &Cache<Fs>,
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
    #[inline]
    pub(crate) fn normalize_with<Fs: FileSystem, P: AsRef<Path>>(
        &self,
        subpath: P,
        cache: &Cache<Fs>,
    ) -> Self {
        // Forward to a single instantiation (per `Fs`) so the many `AsRef<Path>` call
        // sites don't each monomorphize the full body (binary-size win).
        self.normalize_with_impl(subpath.as_ref(), cache)
    }

    fn normalize_with_impl<Fs: FileSystem>(&self, subpath: &Path, cache: &Cache<Fs>) -> Self {
        // Fast path: the overwhelmingly common subpath is a simple relative path with no `.`/`..`
        // segments (e.g. an `exports` target like `./dist/index.js`), so the join is a plain byte
        // append — no `std::path::Components` parsing or per-component `PathBuf::push`, which a Time
        // Profiler shows is the single biggest chunk of resolve CPU. Gated off Windows (needs `/` ->
        // `\` separator normalization) and wasm (strips interior NULs); both keep the slow path.
        #[cfg(not(any(target_os = "windows", target_family = "wasm")))]
        if let Some(result) = self.normalize_with_fast(subpath, cache) {
            return result;
        }

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

    /// Byte-level join for the common case where `subpath` is a simple relative path: returns
    /// `None` (caller falls back to the `Components` path) for anything that needs real
    /// normalization — absolute subpaths, or any `.`/`..`/empty (`//`) segment.
    #[cfg(not(any(target_os = "windows", target_family = "wasm")))]
    fn normalize_with_fast<Fs: FileSystem>(
        &self,
        subpath: &Path,
        cache: &Cache<Fs>,
    ) -> Option<Self> {
        let mut sub = subpath.as_os_str().as_encoded_bytes();
        // Drop a single leading `./`; anything else starting with `.` is a real segment name.
        if let Some(rest) = sub.strip_prefix(b"./") {
            sub = rest;
        }
        // Bail on absolute or empty subpaths, and on any segment that needs normalization.
        if sub.is_empty() || sub[0] == b'/' {
            return None;
        }
        for segment in sub.split(|&b| b == b'/') {
            if segment.is_empty() || segment == b"." || segment == b".." {
                return None;
            }
        }
        let base = self.path.as_os_str().as_encoded_bytes();
        if base.is_empty() {
            return None;
        }
        Some(SCRATCH_PATH.with_borrow_mut(|path| {
            path.clear();
            let buf = path.as_mut_os_string();
            // SAFETY: `base` is a whole `OsStr` encoding, so it ends on a valid `OsStr` boundary.
            buf.push(unsafe { std::ffi::OsStr::from_encoded_bytes_unchecked(base) });
            if base.last() != Some(&b'/') {
                buf.push(std::path::MAIN_SEPARATOR_STR);
            }
            // SAFETY: `sub` is the `subpath` `OsStr` encoding with a leading ASCII `./` stripped,
            // so it also ends on a valid `OsStr` boundary.
            buf.push(unsafe { std::ffi::OsStr::from_encoded_bytes_unchecked(sub) });
            cache.value(path)
        }))
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

/// `Prefix`/`RootDir` can only be a `Components` iterator's first item, which the caller consumes
/// before reaching here, so those arms are unreachable.
#[inline]
fn push_normalized_component(path: &mut PathBuf, component: Component<'_>) {
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
        Component::Prefix(..) | Component::RootDir => unreachable!(),
    }
}

impl CachedPath {
    /// `lstat` view of this path (the link itself), cached.
    ///
    /// Used both to answer `is_file`/`is_dir` for non-symlinks and by canonicalization to decide
    /// whether to follow a symlink — so the two share a single `lstat` syscall per path.
    pub(crate) fn link_metadata<Fs: FileSystem>(&self, fs: &Fs) -> Option<FileMetadata> {
        self.meta.link_or_init(|| fs.symlink_metadata(&self.path).ok())
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
