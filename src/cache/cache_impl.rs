use std::{
    borrow::Cow,
    cfg_select,
    collections::HashSet as StdHashSet,
    hash::{BuildHasherDefault, Hash as _, Hasher as _},
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use dashmap::{DashMap, mapref::entry::Entry};
#[cfg(feature = "yarn_pnp")]
use once_cell::sync::OnceCell;
use rustc_hash::FxHasher;

use super::cached_path::{CachedPath, CachedPathImpl};
use super::hasher::IdentityHasher;
use crate::{
    FileMetadata, FileSystem, PackageJson, ResolveError, ResolveOptions, TsConfig,
    context::ResolveContext as Ctx, path::PathUtil as _,
};

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
        let cached_path = self.canonicalize_impl(path)?;
        let path = cached_path.to_path_buf();
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
                    self.canonicalize_impl(path).ok().and_then(|c| c.link_metadata(self.fs()))
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

    /// Returns the canonical path, resolving all symbolic links.
    ///
    /// <https://github.com/parcel-bundler/parcel/blob/4d27ec8b8bd1792f536811fef86e74a31fa0e704/crates/parcel-resolver/src/cache.rs#L232>
    pub(crate) fn canonicalize_impl(&self, path: &CachedPath) -> Result<CachedPath, ResolveError> {
        // Each canonicalization chain gets its own visited set for circular symlink detection
        let mut visited = StdHashSet::with_hasher(BuildHasherDefault::<IdentityHasher>::default());

        // canonicalize_with_visited now handles caching at every recursion level
        self.canonicalize_with_visited(path, &mut visited).or_else(|err| {
            // Fallback: if canonicalization fails and path's cache was cleared,
            // try direct FS canonicalize without caching the result
            self.fs
                .canonicalize(path.path())
                .map(|canonical| self.value(&canonical))
                .map_err(|_| err)
        })
    }

    /// Internal helper for canonicalization with circular symlink detection.
    fn canonicalize_with_visited(
        &self,
        path: &CachedPath,
        visited: &mut StdHashSet<u64, BuildHasherDefault<IdentityHasher>>,
    ) -> Result<CachedPath, ResolveError> {
        // Check cache first - if this path was already canonicalized, return the cached result
        if let Some((weak, path_box)) = path.canonicalized.get() {
            return weak
                .upgrade()
                .map(CachedPath)
                .or_else(|| {
                    // Weak pointer upgrade failed - recreate from the stored canonical path
                    Some(self.value(path_box))
                })
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::NotFound, "Cached path no longer exists").into()
                });
        }

        // Check for circular symlink by tracking visited paths in the current canonicalization chain
        if !visited.insert(path.hash) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Circular symlink").into());
        }

        let res = path.parent(self).map_or_else(
            || Ok(path.normalize_root(self)),
            |parent| {
                let parent_canonical = self.canonicalize_with_visited(&parent, visited)?;
                // When no ancestor is a symlink — the common case — the parent
                // canonicalizes to itself, so `parent_canonical` is `parent`'s own interned
                // Arc and the rebuild below would just re-derive `path`'s existing key.
                // Skip it (the strip_prefix, scratch-buffer copy, hash, and shard probe)
                // and return this entry directly. That is only sound when the key is
                // spelled exactly `<parent><MAIN_SEPARATOR><file_name>`: spellings the
                // rebuild would fold — `.`/`..` tails, trailing or doubled separators, a
                // `/` joint on Windows — fail the shape check and rebuild as before. The
                // byte before the joint must be a non-separator because a root parent
                // already ends with the separator (the rebuild appends none), so with
                // `//x` or `C:\\x` the `+ 1` would count the doubled separator itself.
                // Wasm always rebuilds: component normalization trims uvwasi's trailing
                // NULs, which reuse would keep.
                let path_bytes = path.path().as_os_str().as_encoded_bytes();
                let parent_len = parent.path().as_os_str().len();
                let normalized = if cfg!(not(target_family = "wasm"))
                    && Arc::ptr_eq(&parent_canonical.0, &parent.0)
                    && path.path().file_name().is_some_and(|name| {
                        parent_len + 1 + name.len() == path_bytes.len()
                            && path_bytes[parent_len] == std::path::MAIN_SEPARATOR as u8
                            && path_bytes[parent_len - 1] != std::path::MAIN_SEPARATOR as u8
                    }) {
                    path.clone()
                } else {
                    parent_canonical
                        .normalize_with(path.path().strip_prefix(parent.path()).unwrap(), self)
                };

                if path.link_metadata(self.fs()).is_some_and(|m| m.is_symlink) {
                    let link = self.fs.read_link(normalized.path())?;
                    if link.is_absolute() {
                        return self
                            .canonicalize_with_visited(&self.value(&link.normalize()), visited);
                    } else if let Some(dir) = normalized.parent(self) {
                        // Symlink is relative `../../foo.js`, use the path directory
                        // to resolve this symlink.
                        return self
                            .canonicalize_with_visited(&dir.normalize_with(&link, self), visited);
                    }
                    debug_assert!(
                        false,
                        "Failed to get path parent for {}.",
                        normalized.path().display()
                    );
                }

                Ok(normalized)
            },
        )?;

        // Cache the result before removing from visited set
        // This ensures parent canonicalization results are cached and reused
        let _ = path.canonicalized.set((Arc::downgrade(&res.0), res.0.path.clone()));

        // Remove from visited set when unwinding the recursion
        visited.remove(&path.hash);
        Ok(res)
    }
}
