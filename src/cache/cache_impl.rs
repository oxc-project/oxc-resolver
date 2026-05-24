use std::{
    borrow::Cow,
    collections::HashSet as StdHashSet,
    hash::{BuildHasherDefault, Hash, Hasher},
    io,
    path::{Path, PathBuf},
    sync::Arc,
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
    FileSystem, PackageJson, ResolveError, ResolveOptions, TsConfig,
    context::ResolveContext as Ctx, path::PathUtil,
};

/// Returns true if `path` has any ancestor directory whose final segment is
/// `node_modules` (excluding the path itself).
///
/// Used by `Cache::value` to compute `inside_node_modules` without recursively
/// interning the entire parent chain.
fn path_is_inside_node_modules(path: &Path) -> bool {
    let mut current = path.parent();
    while let Some(p) = current {
        if p.file_name().is_some_and(|name| name == "node_modules") {
            return true;
        }
        current = p.parent();
    }
    false
}

/// Cache implementation used for caching filesystem access.
#[derive(Default)]
pub struct Cache<Fs> {
    pub(crate) fs: Fs,
    pub(crate) paths: HashSet<CachedPath, BuildHasherDefault<IdentityHasher>>,
    /// Memo of canonical (symlink-resolved) paths. Keyed by *source* path so the
    /// recursion in `canonicalize_path_arc` can short-circuit on intermediate
    /// ancestors without materialising a full `CachedPath` for each one.
    ///
    /// The value is an `Arc<Path>` so multiple aliases pointing at the same
    /// canonical target share a single allocation of the canonical name.
    pub(crate) canonical_paths: HashMap<Box<Path>, Arc<Path>, BuildHasherDefault<FxHasher>>,
    /// Cache for raw/unbuilt tsconfigs (used when extending).
    pub(crate) tsconfigs_raw: HashMap<PathBuf, Arc<TsConfig>, BuildHasherDefault<FxHasher>>,
    /// Cache for built/resolved tsconfigs (used for resolution).
    pub(crate) tsconfigs_built: HashMap<PathBuf, Arc<TsConfig>, BuildHasherDefault<FxHasher>>,
    #[cfg(feature = "yarn_pnp")]
    pub(crate) yarn_pnp_manifest: OnceCell<pnp::Manifest>,
}

impl<Fs: FileSystem> Cache<Fs> {
    pub fn clear(&self) {
        self.paths.pin().clear();
        self.canonical_paths.pin().clear();
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
        // Compute these flags from the path string alone; do NOT recurse into the
        // parent. Materialising every ancestor in the cache is wasted memory —
        // most ancestors are never queried, and `parent()` lazily inserts on demand.
        let is_node_modules = path.file_name().is_some_and(|name| name == "node_modules");
        let inside_node_modules = is_node_modules || path_is_inside_node_modules(path);
        let cached_path = CachedPath(Arc::new(CachedPathImpl::new(
            hash,
            path.to_path_buf().into_boxed_path(),
            is_node_modules,
            inside_node_modules,
        )));
        paths.insert(cached_path.clone());
        cached_path
    }

    pub(crate) fn canonicalize(&self, path: &CachedPath) -> Result<PathBuf, ResolveError> {
        let canonical = self.canonicalize_arc(path)?;
        let path = canonical.to_path_buf();
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
                let real_path = if options.symlinks {
                    self.canonicalize(path)?.join("package.json")
                } else {
                    package_json_path.clone()
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
            canonical_paths: HashMap::builder()
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
        }
    }

    /// Returns the canonical (symlink-resolved) path of `path` as an
    /// `Arc<Path>`. No `CachedPath` entries are created for the intermediate
    /// ancestors walked during resolution — the whole computation runs on raw
    /// `Path` / `PathBuf` and memoises results in [`Cache::canonical_paths`].
    ///
    /// Used as the back-end for both the public-facing
    /// [`Cache::canonicalize`] and the per-entry slot
    /// `CachedPathImpl::canonicalized`.
    pub(crate) fn canonicalize_arc(&self, path: &CachedPath) -> Result<Arc<Path>, ResolveError> {
        // Per-entry fast path.
        if let Some(canonical) = path.canonicalized.get() {
            return Ok(Arc::clone(canonical));
        }

        let mut visited = StdHashSet::with_hasher(BuildHasherDefault::<IdentityHasher>::default());
        let canonical = self.canonicalize_path_arc(path.path(), &mut visited).or_else(|err| {
            // Fallback: if the recursive canonicalisation fails for any
            // reason (e.g. visited-set corruption from concurrent races),
            // ask the filesystem directly without caching.
            self.fs
                .canonicalize(path.path())
                .map(|p| Arc::<Path>::from(p.into_boxed_path()))
                .map_err(|_| err)
        })?;

        let _ = path.canonicalized.set(Arc::clone(&canonical));
        Ok(canonical)
    }

    /// Recursive canonicalisation that operates on `&Path` rather than
    /// `CachedPath` so that intermediate ancestors don't pollute the main path
    /// cache. Each intermediate result is memoised in
    /// [`Cache::canonical_paths`].
    fn canonicalize_path_arc(
        &self,
        path: &Path,
        visited: &mut StdHashSet<u64, BuildHasherDefault<IdentityHasher>>,
    ) -> Result<Arc<Path>, ResolveError> {
        // Memo lookup. `Box<Path>` borrows as `Path` so we can probe with a
        // borrowed key.
        {
            let canonical_paths = self.canonical_paths.pin();
            if let Some(canonical) = canonical_paths.get(path) {
                return Ok(Arc::clone(canonical));
            }
        }

        // Circular-symlink detection. The hash matches what `Cache::value`
        // uses, so the visited set is consistent with `CachedPath::hash` even
        // though we're not handling `CachedPath` here.
        let hash = {
            let mut hasher = FxHasher::default();
            path.as_os_str().hash(&mut hasher);
            hasher.finish()
        };
        if !visited.insert(hash) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Circular symlink").into());
        }

        let canonical: Arc<Path> = match path.parent() {
            None => normalize_root_path(path),
            Some(parent) => {
                let parent_canonical = self.canonicalize_path_arc(parent, visited)?;
                let suffix = path.strip_prefix(parent).unwrap_or_else(|_| Path::new(""));

                // Build parent_canonical / suffix, normalising any
                // CurDir/ParentDir components defensively.
                let mut normalized = PathBuf::with_capacity(
                    parent_canonical.as_os_str().len() + 1 + suffix.as_os_str().len(),
                );
                normalized.push(&*parent_canonical);
                push_path_normalised(&mut normalized, suffix);

                if self.fs.symlink_metadata(path).is_ok_and(|m| m.is_symlink) {
                    let link = self.fs.read_link(&normalized)?;
                    if link.is_absolute() {
                        self.canonicalize_path_arc(&link.normalize(), visited)?
                    } else if let Some(dir) = normalized.parent() {
                        let mut combined = PathBuf::with_capacity(
                            dir.as_os_str().len() + 1 + link.as_os_str().len(),
                        );
                        combined.push(dir);
                        push_path_normalised(&mut combined, &link);
                        self.canonicalize_path_arc(&combined, visited)?
                    } else {
                        Arc::<Path>::from(normalized.into_boxed_path())
                    }
                } else {
                    Arc::<Path>::from(normalized.into_boxed_path())
                }
            }
        };

        visited.remove(&hash);
        self.canonical_paths.pin().insert(Box::<Path>::from(path), Arc::clone(&canonical));
        Ok(canonical)
    }
}

/// On Windows, paths produced by `Path::parent` may end in `/`; the OS prefers
/// `\`. Mirrors the per-CachedPath fix-up that the old `normalize_root` did,
/// but on a raw `Path`.
#[cfg(target_os = "windows")]
fn normalize_root_path(path: &Path) -> Arc<Path> {
    if path.as_os_str().as_encoded_bytes().last() == Some(&b'/') {
        let mut s = path.to_string_lossy().into_owned();
        s.pop();
        s.push('\\');
        Arc::<Path>::from(PathBuf::from(s).into_boxed_path())
    } else {
        Arc::<Path>::from(path.to_path_buf().into_boxed_path())
    }
}

#[cfg(not(target_os = "windows"))]
fn normalize_root_path(path: &Path) -> Arc<Path> {
    Arc::<Path>::from(path.to_path_buf().into_boxed_path())
}

/// Append `tail`'s components onto `base`, treating `CurDir` and `ParentDir`
/// the way `PathBuf::push` does NOT — i.e. normalising them. Equivalent to the
/// existing `CachedPath::normalize_with` body but operating on a raw `PathBuf`.
fn push_path_normalised(base: &mut PathBuf, tail: &Path) {
    use std::path::Component;

    for component in tail.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                base.pop();
            }
            Component::Normal(c) => {
                cfg_if! {
                    if #[cfg(target_family = "wasm")] {
                        // Strip the trailing \0 introduced by https://github.com/nodejs/uvwasi/issues/262
                        base.push(c.to_string_lossy().trim_end_matches('\0'));
                    } else {
                        base.push(c);
                    }
                }
            }
            Component::Prefix(_) | Component::RootDir => {
                // An absolute component overrides whatever came before.
                base.clear();
                base.push(component);
            }
        }
    }
}
