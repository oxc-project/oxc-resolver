use std::{
    borrow::Cow,
    cfg_select,
    collections::HashSet as StdHashSet,
    hash::{BuildHasherDefault, Hash, Hasher},
    io,
    path::{Path, PathBuf},
    sync::{Arc, Weak},
};

use dashmap::{DashMap, mapref::entry::Entry};
#[cfg(feature = "yarn_pnp")]
use once_cell::sync::OnceCell;
use rustc_hash::FxHasher;

use super::cached_path::{CachedPath, CachedPathImpl};
use super::hasher::IdentityHasher;
use crate::{
    FileMetadata, FileSystem, PackageJson, ResolveError, ResolveOptions, TsConfig,
    context::ResolveContext as Ctx,
    path::{PathUtil, push_normalized_component},
};

/// Hashes of the symlink nodes currently being resolved in one canonicalization call, for
/// circular-symlink detection.
type VisitedLinks = StdHashSet<u64, BuildHasherDefault<IdentityHasher>>;

/// Cache implementation used for caching filesystem access.
pub struct Cache {
    pub(crate) fs: Arc<dyn FileSystem>,
    pub(crate) paths: DashMap<CachedPath, (), BuildHasherDefault<IdentityHasher>>,
    /// Cache for raw/unbuilt tsconfigs (used when extending).
    pub(crate) tsconfigs_raw: DashMap<PathBuf, Arc<TsConfig>, BuildHasherDefault<FxHasher>>,
    /// Cache for built/resolved tsconfigs (used for resolution).
    pub(crate) tsconfigs_built: DashMap<PathBuf, Arc<TsConfig>, BuildHasherDefault<FxHasher>>,
    #[cfg(feature = "yarn_pnp")]
    pub(crate) yarn_pnp_manifest: OnceCell<pnp::Manifest>,
}

impl Cache {
    pub fn clear(&self) {
        self.paths.clear();
        self.tsconfigs_raw.clear();
        self.tsconfigs_built.clear();
    }

    /// The underlying filesystem as a trait object.
    #[inline]
    fn fs(&self) -> &dyn FileSystem {
        &*self.fs
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "shard selection needs only the low bits of the hash"
    )]
    pub(crate) fn value(&self, path: &Path) -> CachedPath {
        // `Path::hash` is slow: https://doc.rust-lang.org/std/path/struct.Path.html#impl-Hash-for-Path
        // `path.as_os_str()` hash is not stable because we may joined a path like `foo/bar` and `foo\\bar` on windows.
        let hash = {
            let mut hasher = FxHasher::default();
            path.as_os_str().hash(&mut hasher);
            hasher.finish()
        };
        // Look up by the memoized `hash`. `IdentityHasher` only accepts a single `write_u64`, so the
        // set can't be probed by a borrowed `&Path` through dashmap's `Borrow`-based `get`; instead
        // read the shard directly (raw-api) with the precomputed hash and an `OsStr` equality. This
        // mirrors the `Equivalent` lookup the original `papaya` set used, and keeps it zero-alloc.
        {
            let shard = self.paths.shards()[self.paths.determine_shard(hash as usize)].read();
            if let Some((cached, _)) =
                shard.get(hash, |(k, _)| k.path().as_os_str() == path.as_os_str())
            {
                return cached.clone();
            }
        }
        let parent = path.parent().map(|p| self.value(p));
        let is_node_modules = path.file_name().is_some_and(|name| name == "node_modules");
        let inside_node_modules =
            is_node_modules || parent.as_ref().is_some_and(|parent| parent.inside_node_modules);
        let parent_weak = parent.as_ref().map(|p| Arc::downgrade(&p.0));
        let cached_path = CachedPath(Arc::new(CachedPathImpl::new(
            hash,
            path.to_path_buf().into_boxed_path(),
            is_node_modules,
            inside_node_modules,
            parent_weak,
        )));
        // The shard guard above is dropped before the parent recursion, so a concurrent call may
        // have inserted this same path in the meantime. Dedup via the entry API so every path keeps
        // a single shared `Arc`, which the `canonicalized` / `node_modules` weak-pointer caches rely
        // on for identity.
        match self.paths.entry(cached_path.clone()) {
            Entry::Occupied(occupied) => occupied.key().clone(),
            Entry::Vacant(vacant) => {
                vacant.insert(());
                cached_path
            }
        }
    }

    pub(crate) fn canonicalize(&self, path: &CachedPath) -> Result<PathBuf, ResolveError> {
        let path = self.canonicalize_buf(path, &mut None).or_else(|err| {
            // Fallback: if the cached walk fails (e.g. after a cache clear), try the
            // filesystem's own canonicalize before reporting the original error.
            self.fs.canonicalize(path.path()).map_err(|_| err)
        })?;
        cfg_select! {
            target_os = "windows" => crate::windows::strip_windows_prefix(path),
            _ => Ok(path),
        }
    }

    pub(crate) fn is_file(&self, path: &CachedPath, symlinks: bool, ctx: &mut Ctx) -> bool {
        if self.followed_metadata(path, symlinks).is_some_and(FileMetadata::is_file) {
            ctx.add_file_dependency(path.path());
            true
        } else {
            ctx.add_missing_dependency(path.path());
            false
        }
    }

    pub(crate) fn is_dir(&self, path: &CachedPath, symlinks: bool, ctx: &mut Ctx) -> bool {
        self.followed_metadata(path, symlinks).map_or_else(
            || {
                ctx.add_missing_dependency(path.path());
                false
            },
            FileMetadata::is_dir,
        )
    }

    /// `stat`-equivalent metadata (symlinks followed) for `path`, cached in the `followed` slot.
    ///
    /// For a non-symlink the cached `lstat` already answers this, so no extra syscall is issued.
    /// For a symlink with `symlinks` enabled, reuse canonicalization — which the resolver performs
    /// anyway for the final resolved path — and read the canonical target's already-cached `lstat`,
    /// avoiding a standalone `stat` of the symlink.
    ///
    /// Falls back to a direct `stat` when symlinks are disabled, when canonicalization fails, or
    /// when the canonical target has no metadata. The last case keeps the optimization purely
    /// additive: a custom [`FileSystem`] whose `canonicalize` and `metadata` disagree still gets
    /// the same answer `stat` gave before.
    fn followed_metadata(&self, path: &CachedPath, symlinks: bool) -> Option<FileMetadata> {
        path.meta.followed_or_init(|| match path.link_metadata(self.fs()) {
            Some(meta) if meta.is_symlink() => {
                let followed = if symlinks {
                    self.resolve_symlink_node(path, &mut None)
                        .or_else(|err| {
                            // Same fallback the canonicalize entry point applies: try the
                            // filesystem's own canonicalize before giving up.
                            self.fs
                                .canonicalize(path.path())
                                .map(|canonical| self.value(&canonical))
                                .map_err(|_| err)
                        })
                        .ok()
                        .and_then(|c| c.link_metadata(self.fs()))
                } else {
                    None
                };
                followed.or_else(|| self.fs.metadata(path.path()).ok())
            }
            // A non-symlink's `lstat` already is its `stat`; `None` stays `None`.
            other => other,
        })
    }

    /// Get package.json of a path of `path`.
    ///
    /// # Errors
    ///
    /// * [ResolveError::Json]
    pub(crate) fn get_package_json(
        &self,
        path: &CachedPath,
        options: &ResolveOptions,
        ctx: &mut Ctx,
    ) -> Result<Option<Arc<PackageJson>>, ResolveError> {
        self.find_package_json(path, options, ctx).map(|option_package_json| {
            option_package_json.filter(|package_json| {
                package_json
                    .path()
                    .parent()
                    .is_some_and(|p| p.as_os_str() == path.path().as_os_str())
            })
        })
    }

    /// Find package.json of a path by traversing parent directories.
    ///
    /// # Errors
    ///
    /// * [ResolveError::Json]
    pub(crate) fn find_package_json(
        &self,
        path: &CachedPath,
        options: &ResolveOptions,
        ctx: &mut Ctx,
    ) -> Result<Option<Arc<PackageJson>>, ResolveError> {
        let mut path = path.clone();
        // Go up directories when the querying path is not a directory
        while !self.is_dir(&path, options.symlinks, ctx) {
            if let Some(cv) = path.parent(self) {
                path = cv;
            } else {
                break;
            }
        }
        self.find_package_json_impl(&path, options, ctx)
    }

    /// Find package.json of a path by traversing parent directories.
    ///
    /// # Errors
    ///
    /// * [ResolveError::Json]
    fn find_package_json_impl(
        &self,
        path: &CachedPath,
        options: &ResolveOptions,
        ctx: &mut Ctx,
    ) -> Result<Option<Arc<PackageJson>>, ResolveError> {
        // Change to `std::sync::OnceLock::get_or_try_init` when it is stable.
        path.package_json
            .get_or_try_init(|| {
                let package_json_path = path.path.join("package.json");
                let Ok(package_json_bytes) = self.fs.read(&package_json_path) else {
                    if let Some(deps) = &mut ctx.missing_dependencies {
                        deps.push(package_json_path);
                    }
                    return path.parent(self).map_or(Ok(None), |parent| {
                        self.find_package_json_impl(&parent, options, ctx)
                    });
                };
                let real_path = if options.symlinks {
                    self.canonicalize(path)?.join("package.json")
                } else {
                    package_json_path.clone()
                };
                // Move `package_json_path` into `parse` instead of cloning it: the parsed
                // `PackageJson` stores the path verbatim (`package_json.path()`), and on error
                // `JSONError.path` carries the same path, so the file-dependency record reads it
                // back without a second allocation.
                // https://github.com/webpack/enhanced-resolve/blob/58464fc7cb56673c9aa849e68e6300239601e615/lib/DescriptionFileUtils.js#L68-L82
                match PackageJson::parse(
                    self.fs(),
                    package_json_path,
                    real_path,
                    package_json_bytes,
                ) {
                    Ok(package_json) => {
                        ctx.add_file_dependency(package_json.path());
                        Ok(Some(Arc::new(package_json)))
                    }
                    Err(error) => {
                        if let Some(deps) = &mut ctx.file_dependencies {
                            deps.push(error.path.clone());
                        }
                        Err(ResolveError::Json(error))
                    }
                }
            })
            .cloned()
    }

    pub(crate) fn get_tsconfig<F: FnOnce(&mut TsConfig) -> Result<(), ResolveError>>(
        &self,
        root: bool,
        path: &Path,
        callback: F, // callback for modifying tsconfig with `extends`
    ) -> Result<Arc<TsConfig>, ResolveError> {
        // For root=true (caller tsconfig), check built cache first
        if root && let Some(tsconfig) = self.tsconfigs_built.get(path) {
            return Ok(Arc::clone(tsconfig.value()));
        }

        // Check raw cache (callback applied, not built) - only for root=false
        // For root=true, we need to run the callback to ensure extends are processed
        if !root && let Some(tsconfig) = self.tsconfigs_raw.get(path) {
            return Ok(Arc::clone(tsconfig.value()));
        }

        // Not in any cache, parse from file.
        // Classify file/dir via the cached `lstat` (which the canonicalization below reuses)
        // instead of a standalone `stat`. For a regular file/dir the two agree; only follow the
        // link with a `stat` when `path` is actually a symlink, preserving the symlink-following
        // classification while saving one metadata syscall per tsconfig in the common case.
        let cached_path = self.value(path);
        let meta = match cached_path.link_metadata(self.fs()) {
            Some(m) if m.is_symlink() => self.fs.metadata(path).ok(),
            other => other,
        };
        let tsconfig_path = if meta.is_some_and(|m| m.is_file) {
            Cow::Borrowed(path)
        } else if meta.is_some_and(|m| m.is_dir) {
            Cow::Owned(path.join("tsconfig.json"))
        } else {
            let mut os_string = path.to_path_buf().into_os_string();
            os_string.push(".json");
            Cow::Owned(PathBuf::from(os_string))
        };
        let tsconfig_string = self.fs.read_to_string(&tsconfig_path).map_err(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                ResolveError::TsconfigNotFound(path.to_path_buf())
            } else {
                ResolveError::TsconfigLoadFailed {
                    path: tsconfig_path.to_path_buf(),
                    source: Box::new(ResolveError::from(err)),
                }
            }
        })?;
        let canonical_path = self
            .canonicalize(&self.value(&tsconfig_path))
            .unwrap_or_else(|_| tsconfig_path.to_path_buf());
        let mut tsconfig = TsConfig::parse(root, &tsconfig_path, &canonical_path, tsconfig_string)
            .map_err(|error| ResolveError::TsconfigLoadFailed {
                path: tsconfig_path.to_path_buf(),
                source: Box::new(ResolveError::from_serde_json_error(
                    tsconfig_path.to_path_buf(),
                    &error,
                )),
            })?;

        // Run callback (extends/references processing)
        callback(&mut tsconfig)?;

        // Cache raw version (callback applied, not built)
        tsconfig.set_should_build(false);
        if root {
            self.tsconfigs_raw.insert(path.to_path_buf(), Arc::new(tsconfig.clone()));
            // Build and cache built version
            tsconfig.set_should_build(true);
            let tsconfig = Arc::new(tsconfig.build());
            self.tsconfigs_built.insert(path.to_path_buf(), Arc::clone(&tsconfig));
            Ok(tsconfig)
        } else {
            // Return unbuilt version
            let tsconfig = Arc::new(tsconfig);
            self.tsconfigs_raw.insert(path.to_path_buf(), Arc::clone(&tsconfig));
            Ok(tsconfig)
        }
    }

    #[cfg(feature = "yarn_pnp")]
    pub(crate) fn get_yarn_pnp_manifest(
        &self,
        cwd: Option<&Path>,
    ) -> Result<&pnp::Manifest, ResolveError> {
        self.yarn_pnp_manifest.get_or_try_init(|| {
            let cwd = match cwd {
                Some(path) => Cow::Borrowed(path),
                None => match std::env::current_dir() {
                    Ok(path) => Cow::Owned(path),
                    Err(err) => return Err(ResolveError::from(err)),
                },
            };
            let manifest = match pnp::find_pnp_manifest(&cwd) {
                Ok(manifest) => match manifest {
                    Some(manifest) => manifest,
                    None => {
                        return Err(ResolveError::FailedToFindYarnPnpManifest(cwd.to_path_buf()));
                    }
                },
                Err(err) => return Err(ResolveError::YarnPnpError(err)),
            };
            Ok(manifest)
        })
    }
}

impl Cache {
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self {
            fs,
            paths: DashMap::with_hasher(BuildHasherDefault::default()),
            tsconfigs_raw: DashMap::with_hasher(BuildHasherDefault::default()),
            tsconfigs_built: DashMap::with_hasher(BuildHasherDefault::default()),
            #[cfg(feature = "yarn_pnp")]
            yarn_pnp_manifest: OnceCell::new(),
        }
    }

    /// Returns the canonical form of `path` as a freshly built `PathBuf`, resolving all
    /// symbolic links.
    ///
    /// Canonicalization state is kept only where it is not redundant: a path that is its own
    /// canonical form (no ancestor is a symlink — the large majority) carries a one-bit
    /// `canonical_is_self` flag and no heap, while a symlink or a path below one caches its
    /// canonical form in `canonicalized`. The previous design instead stored a `(Weak, Box<Path>)`
    /// on *every* level and interned the whole canonical-side chain alongside the logical one,
    /// which dominated the cache in symlink-heavy layouts.
    fn canonicalize_buf(
        &self,
        path: &CachedPath,
        visited: &mut Option<VisitedLinks>,
    ) -> Result<PathBuf, ResolveError> {
        // Hot path (warm cache): this exact spelling was already proven canonical, so it is its
        // own answer — one exact-capacity allocation, no scan.
        if path.canonical_is_self() {
            return Ok(path.to_path_buf());
        }
        // Hot path (warm cache): a symlink whose target is cached, or a path below a symlink whose
        // derived canonical form was cached on a previous call. Either way the answer is stored —
        // no scan, no re-derivation.
        if let Some((_, canonical)) = path.canonicalized.get() {
            return Ok(canonical.to_path_buf());
        }

        // Scan bottom-up for the nearest ancestor-or-self (`stop`) whose canonical form
        // (`prefix`) is known, verifying via the cached `lstat` bit that every node below the
        // stop is not a symlink.
        let mut node = path.clone();
        let (prefix, stop) = loop {
            // Proven canonical already: its own spelling is the prefix.
            if node.canonical_is_self() {
                break (node.clone(), node);
            }
            // A node with a cached canonical form (a symlink's target, or an already-derived
            // path below one): use it as the prefix. Consult the cache before `lstat` so a
            // resolved node costs no syscall.
            if let Some(prefix) = self.cached_canonical(&node) {
                break (prefix, node);
            }
            // The root is its own canonical form; roots are never `lstat`'d.
            let Some(parent) = node.parent(self) else {
                break (node.normalize_root(self), node);
            };
            // An unresolved symlink: resolve it (caching the mapping on the node) and use the
            // target as the prefix.
            if node.link_metadata(self.fs()).is_some_and(|m| m.is_symlink) {
                break (self.resolve_symlink_node(&node, visited)?, node);
            }
            node = parent;
        };

        // The scan stopped at `path` itself (a resolved symlink leaf, or the root): the prefix is
        // already the whole answer.
        if Arc::ptr_eq(&stop.0, &path.0) {
            return Ok(prefix.to_path_buf());
        }

        // Rebuild: fold the suffix below the stop node onto the prefix, in one exact-capacity
        // allocation. The suffix is relative (the stop node is a textual ancestor), so
        // `Prefix`/`RootDir` components cannot occur.
        let suffix = path.path().strip_prefix(stop.path()).unwrap();
        let prefix_bytes = prefix.path().as_os_str();
        let extra = path.path().as_os_str().len() - stop.path().as_os_str().len();
        let mut buf = std::ffi::OsString::with_capacity(prefix_bytes.len() + extra);
        buf.push(prefix_bytes);
        let mut result = PathBuf::from(buf);
        for component in suffix.components() {
            push_normalized_component(&mut result, component);
        }

        if result.as_os_str() == path.path().as_os_str() {
            // The rebuild reproduced the key byte-for-byte: no ancestor was a symlink and every
            // joint was already normalized, so this spelling *is* canonical, and so is every
            // prefix of it. Mark the scanned chain so later scans stop immediately — one bit, no
            // heap.
            let mut current = path.clone();
            loop {
                current.set_canonical_is_self();
                if Arc::ptr_eq(&current.0, &stop.0) {
                    break;
                }
                let Some(parent) = current.parent(self) else { break };
                current = parent;
            }
        } else {
            // A path below a symlink: cache its derived canonical form so re-resolving it is O(1)
            // (matching a per-leaf cache) instead of re-scanning to the symlink and re-folding.
            // The `Weak` is left dangling: the canonical target is not interned (no twin entry),
            // and a non-symlink leaf never routes through `resolve_symlink_node`, which is the
            // only reader of the `Weak`. A symlink leaf already had its mapping set during the
            // scan, so this `set` is a no-op for it.
            let _ = path.canonicalized.set((Weak::new(), result.clone().into_boxed_path()));
        }

        Ok(result)
    }

    /// The stored canonical form of `node` as an interned [`CachedPath`], or `None` if the node
    /// has no mapping (it is self-canonical, or not yet resolved).
    ///
    /// A mapping's `Weak` is live for a symlink target and dangling for a derived below-symlink
    /// path; either way, a failed upgrade (a live target evicted from the cache, or the dangling
    /// sentinel) falls back to re-interning the stored canonical bytes.
    fn cached_canonical(&self, node: &CachedPath) -> Option<CachedPath> {
        node.canonicalized.get().map(|(weak, canonical)| {
            weak.upgrade().map_or_else(|| self.value(canonical), CachedPath)
        })
    }

    /// Resolve a symlink node to its canonical target, caching the mapping on the node.
    ///
    /// `node` must have `lstat`'d as a symlink (or already hold a stored mapping).
    fn resolve_symlink_node(
        &self,
        node: &CachedPath,
        visited: &mut Option<VisitedLinks>,
    ) -> Result<CachedPath, ResolveError> {
        if let Some(canonical) = self.cached_canonical(node) {
            return Ok(canonical);
        }

        // A chain that re-enters a symlink still being resolved is circular. The set is
        // allocated lazily: most canonicalize calls never meet a symlink.
        if !visited.get_or_insert_default().insert(node.hash) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Circular symlink").into());
        }

        // Read the link through its canonical parent spelling. A symlink's last component is
        // always a plain name: the OS resolves `.`/`..`/trailing-separator spellings before
        // classifying, so those never `lstat` as symlinks.
        let mut link_path = match node.parent(self) {
            Some(parent) => {
                let mut link_path = self.canonicalize_buf(&parent, visited)?;
                node.path().file_name().map_or_else(
                    || node.to_path_buf(),
                    |name| {
                        link_path.push(name);
                        link_path
                    },
                )
            }
            None => node.to_path_buf(),
        };
        let link = self.fs.read_link(&link_path)?;

        let target = if link.is_absolute() {
            link.normalize()
        } else {
            // Resolve the relative target against the canonical parent directory. `normalize_with`
            // folds `.`/`..` and escapes a rooted-but-not-absolute head (`\dir`, `C:rel` on
            // Windows) or an empty link verbatim.
            link_path.pop();
            link_path.normalize_with(&link)
        };

        // The target chain may itself contain symlinks. Only the target and its canonical form
        // are interned — the mapping needs a durable entry to point at.
        let target = self.value(&target);
        let canonical_buf = self.canonicalize_buf(&target, visited)?;
        let canonical = if canonical_buf.as_os_str() == target.path().as_os_str() {
            target
        } else {
            self.value(&canonical_buf)
        };

        let _ = node.canonicalized.set((Arc::downgrade(&canonical.0), canonical.0.path.clone()));
        if let Some(visited) = visited.as_mut() {
            visited.remove(&node.hash);
        }
        Ok(canonical)
    }
}
