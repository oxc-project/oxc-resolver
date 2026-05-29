use std::{
    borrow::Cow,
    collections::HashSet as StdHashSet,
    hash::{BuildHasherDefault, Hash, Hasher},
    io,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use cfg_if::cfg_if;
#[cfg(feature = "yarn_pnp")]
use once_cell::sync::OnceCell;
use papaya::{HashMap, HashSet};
use rustc_hash::FxHasher;

use super::borrowed_path::BorrowedCachedPath;
use super::cached_path::{CachedPath, CachedPathImpl};
use super::hasher::IdentityHasher;
use crate::{
    FileSystem, NodeModulesLayout, PackageJson, ResolveError, ResolveOptions, TsConfig,
    context::ResolveContext as Ctx, path::PathUtil,
};

/// Cache implementation used for caching filesystem access.
#[derive(Default)]
pub struct Cache<Fs> {
    pub(crate) fs: Fs,
    pub(crate) paths: HashSet<CachedPath, BuildHasherDefault<IdentityHasher>>,
    /// Cache for raw/unbuilt tsconfigs (used when extending).
    pub(crate) tsconfigs_raw: HashMap<PathBuf, Arc<TsConfig>, BuildHasherDefault<FxHasher>>,
    /// Cache for built/resolved tsconfigs (used for resolution).
    pub(crate) tsconfigs_built: HashMap<PathBuf, Arc<TsConfig>, BuildHasherDefault<FxHasher>>,
    #[cfg(feature = "yarn_pnp")]
    pub(crate) yarn_pnp_manifest: OnceCell<pnp::Manifest>,
    /// The detected `node_modules/` layout for this resolver. Populated lazily
    /// on the first call to [`Cache::node_modules_layout`] and shared across
    /// resolvers that clone this cache via `clone_with_options`.
    pub(crate) node_modules_layout: OnceLock<NodeModulesLayout>,
}

impl<Fs: FileSystem> Cache<Fs> {
    pub fn clear(&self) {
        self.paths.pin().clear();
        self.tsconfigs_raw.pin().clear();
        self.tsconfigs_built.pin().clear();
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn value(&self, path: &Path) -> CachedPath {
        // `Path::hash` is slow: https://doc.rust-lang.org/std/path/struct.Path.html#impl-Hash-for-Path
        // `path.as_os_str()` hash is not stable because we may joined a path like `foo/bar` and `foo\\bar` on windows.
        let hash = {
            let mut hasher = FxHasher::default();
            path.as_os_str().hash(&mut hasher);
            hasher.finish()
        };
        let paths = self.paths.pin();
        if let Some(entry) = paths.get(&BorrowedCachedPath { hash, path }) {
            return entry.clone();
        }
        let parent = path.parent().map(|p| self.value(p));
        let is_node_modules = path.file_name().as_ref().is_some_and(|&name| name == "node_modules");
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
        paths.insert(cached_path.clone());
        cached_path
    }

    pub(crate) fn canonicalize(&self, path: &CachedPath) -> Result<PathBuf, ResolveError> {
        let cached_path = self.canonicalize_impl(path)?;
        let path = cached_path.to_path_buf();
        cfg_if! {
            if #[cfg(target_os = "windows")] {
                crate::windows::strip_windows_prefix(path)
            } else {
                Ok(path)
            }
        }
    }

    pub(crate) fn is_file(&self, path: &CachedPath, ctx: &mut Ctx) -> bool {
        if path.is_file(&self.fs).is_some_and(|b| b) {
            ctx.add_file_dependency(path.path());
            true
        } else {
            ctx.add_missing_dependency(path.path());
            false
        }
    }

    pub(crate) fn is_dir(&self, path: &CachedPath, ctx: &mut Ctx) -> bool {
        path.is_dir(&self.fs).unwrap_or_else(|| {
            ctx.add_missing_dependency(path.path());
            false
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
        while !self.is_dir(&path, ctx) {
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
                // Skip the per-component canonicalize walk only when we're
                // inside `node_modules/` AND the directory itself isn't a
                // symlink. PMs install package contents as real files so
                // `<pkg>/package.json` already has its canonical prefix.
                // Paths outside node_modules stay on the original code path —
                // some bench workloads (resolver_real with many symlinked
                // files, tsconfig path aliases) live entirely outside
                // node_modules and shouldn't pay any extra probe.
                let real_path = if !options.symlinks
                    || (path.inside_node_modules() && !self.is_symlink_cached(path))
                {
                    package_json_path.clone()
                } else {
                    self.canonicalize(path)?.join("package.json")
                };
                PackageJson::parse(
                    &self.fs,
                    package_json_path.clone(),
                    real_path,
                    package_json_bytes,
                )
                .map(|package_json| Some(Arc::new(package_json)))
                .map_err(ResolveError::Json)
                // https://github.com/webpack/enhanced-resolve/blob/58464fc7cb56673c9aa849e68e6300239601e615/lib/DescriptionFileUtils.js#L68-L82
                .inspect(|_| {
                    ctx.add_file_dependency(&package_json_path);
                })
                .inspect_err(|_| {
                    if let Some(deps) = &mut ctx.file_dependencies {
                        deps.push(package_json_path.clone());
                    }
                })
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
        if root {
            let tsconfigs_built = self.tsconfigs_built.pin();
            if let Some(tsconfig) = tsconfigs_built.get(path) {
                return Ok(Arc::clone(tsconfig));
            }
        }

        // Check raw cache (callback applied, not built) - only for root=false
        // For root=true, we need to run the callback to ensure extends are processed
        if !root {
            let tsconfigs_raw = self.tsconfigs_raw.pin();
            if let Some(tsconfig) = tsconfigs_raw.get(path) {
                return Ok(Arc::clone(tsconfig));
            }
        }

        // Not in any cache, parse from file
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
        let tsconfig_string = self.fs.read_to_string(&tsconfig_path).map_err(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                ResolveError::TsconfigNotFound(path.to_path_buf())
            } else {
                ResolveError::from(err)
            }
        })?;
        let canonical_path = self
            .canonicalize(&self.value(&tsconfig_path))
            .unwrap_or_else(|_| tsconfig_path.to_path_buf());
        let mut tsconfig = TsConfig::parse(root, &tsconfig_path, &canonical_path, tsconfig_string)
            .map_err(|error| {
                ResolveError::from_serde_json_error(tsconfig_path.to_path_buf(), &error)
            })?;

        // Run callback (extends/references processing)
        callback(&mut tsconfig)?;

        // Cache raw version (callback applied, not built)
        tsconfig.set_should_build(false);
        let raw_tsconfig = Arc::new(tsconfig.clone());
        self.tsconfigs_raw.pin().insert(path.to_path_buf(), Arc::clone(&raw_tsconfig));

        if root {
            // Build and cache built version
            tsconfig.set_should_build(true);
            let tsconfig = Arc::new(tsconfig.build());
            self.tsconfigs_built.pin().insert(path.to_path_buf(), Arc::clone(&tsconfig));
            Ok(tsconfig)
        } else {
            // Return unbuilt version
            Ok(raw_tsconfig)
        }
    }

    /// Return the layout of `node_modules/` for the project containing `start`.
    ///
    /// On first call, this walks up from `start` to detect the layout and
    /// caches the result. Subsequent calls return the cached value regardless
    /// of `start` — the layout is treated as a per-project property and is
    /// not re-detected for nested workspaces.
    pub(crate) fn node_modules_layout(&self, start: &Path) -> NodeModulesLayout {
        *self.node_modules_layout.get_or_init(|| self.detect_node_modules_layout(start))
    }

    /// Lazily probes and caches whether `path` is a symlink. Returns `false`
    /// (and caches `false`) if the path doesn't exist or can't be stat'd.
    pub(crate) fn is_symlink_cached(&self, path: &CachedPath) -> bool {
        path.symlink.get().unwrap_or_else(|| {
            let result = self.fs.symlink_metadata(path.path()).is_ok_and(|m| m.is_symlink);
            path.symlink.set(result);
            result
        })
    }

    /// Return the `<...>/node_modules/<pkg>` anchor that `path` lives under,
    /// or `None` if the path is not inside `node_modules/`. Walks the
    /// existing `parent` chain (each step is a `Weak::upgrade` —
    /// allocation-free) rather than running `path::node_modules_anchor`,
    /// which allocates a `Vec<Component>` and two `PathBuf`s on every call.
    /// O(N) in path depth, but each step is cheap.
    pub(crate) fn pkg_anchor(&self, path: &CachedPath) -> Option<CachedPath> {
        if !path.inside_node_modules() {
            return None;
        }
        let mut current = path.clone();
        let mut child = None::<CachedPath>;
        let mut grandchild = None::<CachedPath>;
        loop {
            if current.is_node_modules {
                // `child` is the `<pkg>` for unscoped names. For `@scope/name`
                // it's the `@scope` dir, so the real anchor is `grandchild`.
                let candidate = child?;
                if candidate
                    .path
                    .file_name()
                    .is_some_and(|n| n.as_encoded_bytes().starts_with(b"@"))
                {
                    return grandchild;
                }
                return Some(candidate);
            }
            let parent = current.parent(self)?;
            grandchild = child;
            child = Some(current);
            current = parent;
        }
    }

    fn detect_node_modules_layout(&self, start: &Path) -> NodeModulesLayout {
        for ancestor in start.ancestors() {
            if self.fs.metadata(&ancestor.join(".pnp.cjs")).is_ok_and(|m| m.is_file) {
                return NodeModulesLayout::Pnp;
            }
            let nm = ancestor.join("node_modules");
            if self.fs.metadata(&nm).is_ok_and(|m| m.is_dir) {
                // pnpm default linker, yarn berry `pnpm` linker, bun's isolated linker
                // each install a virtual store under `node_modules/` with a known prefix.
                let has_pnpm_store = self.fs.metadata(&nm.join(".pnpm")).is_ok_and(|m| m.is_dir);
                let has_yarn_store = self.fs.metadata(&nm.join(".store")).is_ok_and(|m| m.is_dir);
                let has_bun_store = self.fs.metadata(&nm.join(".bun")).is_ok_and(|m| m.is_dir);
                if has_pnpm_store || has_yarn_store || has_bun_store {
                    return NodeModulesLayout::Isolated;
                }
                return NodeModulesLayout::Flat;
            }
        }
        NodeModulesLayout::Generic
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

impl<Fs: FileSystem> Cache<Fs> {
    pub fn new(fs: Fs) -> Self {
        Self {
            fs,
            paths: HashSet::builder()
                .hasher(BuildHasherDefault::default())
                .resize_mode(papaya::ResizeMode::Blocking)
                .build(),
            tsconfigs_raw: HashMap::builder()
                .hasher(BuildHasherDefault::default())
                .resize_mode(papaya::ResizeMode::Blocking)
                .build(),
            tsconfigs_built: HashMap::builder()
                .hasher(BuildHasherDefault::default())
                .resize_mode(papaya::ResizeMode::Blocking)
                .build(),
            #[cfg(feature = "yarn_pnp")]
            yarn_pnp_manifest: OnceCell::new(),
            node_modules_layout: OnceLock::new(),
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

        // Anchor fast path: when the path lives below a non-symlink
        // `<...>/node_modules/<pkg>` anchor, nothing below the anchor is a
        // symlink either — the path is its own canonical. This is critical
        // for isolated layouts: the symlink target lives at
        // `<root>/node_modules/.{pnpm,store,bun}/<pkg>@<ver>/node_modules/<pkg>`,
        // also under `node_modules/`, also non-symlink anchor — so the
        // recursive canonicalize triggered by `read_link` short-circuits here
        // instead of walking each component of the virtual-store path.
        if path.inside_node_modules
            && let Some(anchor) = self.pkg_anchor(path)
            && !self.is_symlink_cached(&anchor)
        {
            let _ =
                path.canonicalized.set((Arc::downgrade(&path.0), path.0.path.clone()));
            return Ok(path.clone());
        }

        // Check for circular symlink by tracking visited paths in the current canonicalization chain
        if !visited.insert(path.hash) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Circular symlink").into());
        }

        let res = path.parent(self).map_or_else(
            || Ok(path.normalize_root(self)),
            |parent| {
                self.canonicalize_with_visited(&parent, visited).and_then(|parent_canonical| {
                    let normalized = parent_canonical
                        .normalize_with(path.path().strip_prefix(parent.path()).unwrap(), self);

                    let is_symlink = path.symlink.get().unwrap_or_else(|| {
                        let result =
                            self.fs.symlink_metadata(path.path()).is_ok_and(|m| m.is_symlink);
                        path.symlink.set(result);
                        result
                    });
                    if is_symlink {
                        let link = self.fs.read_link(normalized.path())?;
                        if link.is_absolute() {
                            return self.canonicalize_with_visited(
                                &self.value(&link.normalize()),
                                visited,
                            );
                        } else if let Some(dir) = normalized.parent(self) {
                            // Symlink is relative `../../foo.js`, use the path directory
                            // to resolve this symlink.
                            return self.canonicalize_with_visited(
                                &dir.normalize_with(&link, self),
                                visited,
                            );
                        }
                        debug_assert!(
                            false,
                            "Failed to get path parent for {}.",
                            normalized.path().display()
                        );
                    }

                    Ok(normalized)
                })
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
