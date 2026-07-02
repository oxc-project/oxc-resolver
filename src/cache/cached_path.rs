use std::{
    convert::AsRef,
    ffi::OsStr,
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Weak,
        atomic::{AtomicU8, Ordering},
    },
};

use cfg_if::cfg_if;
use once_cell::sync::OnceCell as OnceLock;
use rustc_hash::{FxHashMap, FxHashSet};

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
    /// One `read_dir` snapshot of this path as a directory, replacing per-child `lstat`
    /// probes. `None` = listing unavailable (unsupported file system, not a directory, or
    /// read error); children then fall back to per-path metadata calls.
    pub dir_entries: OnceLock<Option<Box<DirListing>>>,
    /// Cold child probes seen before `dir_entries` is built; see
    /// [`CachedPathImpl::child_from_listing`].
    pub child_probes: AtomicU8,
}

/// A directory's entries with their `lstat`-equivalent kinds, from one [`FileSystem::read_dir`].
///
/// Names are keyed by their OS byte encoding. A lookup only reports "definitely absent" when
/// no ASCII case-variant of the queried name exists either, so resolution behaves identically
/// on case-sensitive and case-insensitive file systems; non-ASCII names always fall back to a
/// real metadata call (Unicode case folding and normalization stay the OS's business).
pub struct DirListing {
    /// Entry name bytes -> kind (`None` = kind unknown, caller must `lstat`).
    entries: FxHashMap<Box<[u8]>, Option<FileMetadata>>,
    /// ASCII-lowercased forms of entry names that contain an uppercase ASCII byte.
    lowered: FxHashSet<Box<[u8]>>,
}

/// Verdict of a directory-listing lookup for one child name.
pub enum ChildVerdict {
    /// Child exists with this `lstat`-equivalent kind; no syscall needed.
    Kind(FileMetadata),
    /// Child is definitely absent; no syscall needed.
    Absent,
    /// No verdict — the caller must issue the real metadata call.
    Unknown,
}

impl DirListing {
    fn load(fs: &dyn FileSystem, path: &Path) -> Option<Box<Self>> {
        let raw = fs.read_dir(path).ok()?;
        let mut entries =
            FxHashMap::with_capacity_and_hasher(raw.len(), rustc_hash::FxBuildHasher);
        let mut lowered = FxHashSet::default();
        for (name, kind) in raw {
            let bytes = name.into_encoded_bytes().into_boxed_slice();
            if bytes.is_ascii() && bytes.iter().any(u8::is_ascii_uppercase) {
                lowered.insert(bytes.to_ascii_lowercase().into_boxed_slice());
            }
            entries.insert(bytes, kind);
        }
        Some(Box::new(Self { entries, lowered }))
    }

    fn lookup(&self, name: &OsStr) -> ChildVerdict {
        let bytes = name.as_encoded_bytes();
        if !bytes.is_ascii() {
            return ChildVerdict::Unknown;
        }
        if let Some(kind) = self.entries.get(bytes) {
            return kind.map_or(ChildVerdict::Unknown, ChildVerdict::Kind);
        }
        // Absent by exact bytes. A case-variant entry could still satisfy this name on a
        // case-insensitive file system (macOS, Windows) — only report absent when none exists.
        let has_upper = bytes.iter().any(u8::is_ascii_uppercase);
        if self.lowered.is_empty() && !has_upper {
            return ChildVerdict::Absent;
        }
        let lower = bytes.to_ascii_lowercase();
        if self.lowered.contains(lower.as_slice())
            || (has_upper && self.entries.contains_key(lower.as_slice()))
        {
            return ChildVerdict::Unknown;
        }
        ChildVerdict::Absent
    }
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
            dir_entries: OnceLock::new(),
            child_probes: AtomicU8::new(0),
        }
    }

    /// Answer a child's `lstat`-equivalent metadata from this directory's listing.
    ///
    /// The listing is built lazily on the sixteenth cold child probe. Cold probe counts per
    /// directory are bimodal: path ancestors and one-off package directories see a handful,
    /// while directories a build actually shares — `node_modules` under many bare specifiers,
    /// package/source dirs under many subpath or extension candidates — see dozens to
    /// hundreds. One `read_dir` costs several syscalls (open + fstat + getdirentries + close)
    /// plus the entry-map build, so the threshold sits above the first mode: sparse
    /// resolutions keep exactly their per-path `lstat` behavior, and densely probed
    /// directories collapse every probe past the sixteenth into zero syscalls.
    pub(crate) fn child_from_listing(&self, name: &OsStr, fs: &dyn FileSystem) -> ChildVerdict {
        const LISTING_THRESHOLD: u8 = 16;
        let listing = if let Some(listing) = self.dir_entries.get() {
            listing
        } else {
            if self.child_probes.fetch_add(1, Ordering::Relaxed) < LISTING_THRESHOLD - 1 {
                return ChildVerdict::Unknown;
            }
            self.dir_entries.get_or_init(|| DirListing::load(fs, &self.path))
        };
        listing.as_deref().map_or(ChildVerdict::Unknown, |listing| listing.lookup(name))
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
    ///
    /// The parent directory's listing is consulted first (see
    /// [`CachedPathImpl::child_from_listing`]); when it has a verdict, no syscall is issued
    /// for this path at all.
    pub(crate) fn link_metadata(&self, fs: &dyn FileSystem) -> Option<FileMetadata> {
        self.meta.link_or_init(|| {
            if let Some(parent) = self.parent.as_ref().and_then(Weak::upgrade)
                && let Some(name) = self.path.file_name()
            {
                match parent.child_from_listing(name, fs) {
                    ChildVerdict::Kind(meta) => return Some(meta),
                    ChildVerdict::Absent => return None,
                    ChildVerdict::Unknown => {}
                }
            }
            fs.symlink_metadata(&self.path).ok()
        })
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
