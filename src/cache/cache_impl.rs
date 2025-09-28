use std::{
    borrow::Cow,
    hash::{BuildHasherDefault, Hash, Hasher},
    io,
    path::{Path, PathBuf},
    sync::{Arc, atomic::Ordering},
};

use cfg_if::cfg_if;
#[cfg(feature = "yarn_pnp")]
use once_cell::sync::OnceCell;
use papaya::{HashMap, HashSet};
use rustc_hash::FxHasher;

use super::borrowed_path::BorrowedCachedPath;
use super::cached_path::{CachedPath, CachedPathImpl};
use super::hasher::IdentityHasher;
use super::thread_local::THREAD_ID;
use crate::{
    FileSystem, PackageJson, ResolveError, ResolveOptions, TsConfig,
    context::ResolveContext as Ctx, path::PathUtil,
};

/// Cache implementation used for caching filesystem access.
#[derive(Default)]
pub struct Cache<Fs> {
    pub(crate) fs: Fs,
    pub(crate) paths: HashSet<CachedPath, BuildHasherDefault<IdentityHasher>>,
    pub(crate) tsconfigs: HashMap<PathBuf, Arc<TsConfig>, BuildHasherDefault<FxHasher>>,
    #[cfg(feature = "yarn_pnp")]
    pub(crate) yarn_pnp_manifest: OnceCell<pnp::Manifest>,
}

impl<Fs: FileSystem> Cache<Fs> {
    pub fn clear(&self) {
        self.paths.pin().clear();
        self.tsconfigs.pin().clear();
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
        if let Some(meta) = path.meta(&self.fs) {
            ctx.add_file_dependency(path.path());
            meta.is_file
        } else {
            ctx.add_missing_dependency(path.path());
            false
        }
    }

    pub(crate) fn is_dir(&self, path: &CachedPath, ctx: &mut Ctx) -> bool {
        path.meta(&self.fs).map_or_else(
            || {
                ctx.add_missing_dependency(path.path());
                false
            },
            |meta| meta.is_dir,
        )
    }

    pub(crate) fn get_package_json(
        &self,
        path: &CachedPath,
        options: &ResolveOptions,
        ctx: &mut Ctx,
    ) -> Result<Option<Arc<PackageJson>>, ResolveError> {
        // Change to `std::sync::OnceLock::get_or_try_init` when it is stable.
        let result = path
            .package_json
            .get_or_try_init(|| {
                let package_json_path = path.path.join("package.json");
                let Ok(package_json_string) =
                    self.fs.read_to_string_bypass_system_cache(&package_json_path)
                else {
                    return Ok(None);
                };

                let real_path = if options.symlinks {
                    self.canonicalize(path)?.join("package.json")
                } else {
                    package_json_path.clone()
                };
                PackageJson::parse(package_json_path.clone(), real_path, &package_json_string)
                    .map(|package_json| Some(Arc::new(package_json)))
                    .map_err(|error| ResolveError::from_serde_json_error(package_json_path, &error))
            })
            .cloned();
        // https://github.com/webpack/enhanced-resolve/blob/58464fc7cb56673c9aa849e68e6300239601e615/lib/DescriptionFileUtils.js#L68-L82
        match &result {
            Ok(Some(package_json)) => {
                ctx.add_file_dependency(&package_json.path);
            }
            Ok(None) => {
                // Avoid an allocation by making this lazy
                if let Some(deps) = &mut ctx.missing_dependencies {
                    deps.push(path.path.join("package.json"));
                }
            }
            Err(_) => {
                if let Some(deps) = &mut ctx.file_dependencies {
                    deps.push(path.path.join("package.json"));
                }
            }
        }
        result
    }

    pub(crate) fn get_tsconfig<F: FnOnce(&mut TsConfig) -> Result<(), ResolveError>>(
        &self,
        root: bool,
        path: &Path,
        callback: F, // callback for modifying tsconfig with `extends`
    ) -> Result<Arc<TsConfig>, ResolveError> {
        let tsconfigs = self.tsconfigs.pin();
        if let Some(tsconfig) = tsconfigs.get(path) {
            return Ok(Arc::clone(tsconfig));
        }
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
        let mut tsconfig_string = self
            .fs
            .read_to_string_bypass_system_cache(&tsconfig_path)
            .map_err(|_| ResolveError::TsconfigNotFound(path.to_path_buf()))?;
        let mut tsconfig =
            TsConfig::parse(root, &tsconfig_path, &mut tsconfig_string).map_err(|error| {
                ResolveError::from_serde_json_error(tsconfig_path.to_path_buf(), &error)
            })?;
        callback(&mut tsconfig)?;
        let tsconfig = Arc::new(tsconfig.build());
        tsconfigs.insert(path.to_path_buf(), Arc::clone(&tsconfig));
        Ok(tsconfig)
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
            tsconfigs: HashMap::builder()
                .hasher(BuildHasherDefault::default())
                .resize_mode(papaya::ResizeMode::Blocking)
                .build(),
            #[cfg(feature = "yarn_pnp")]
            yarn_pnp_manifest: OnceCell::new(),
        }
    }

    /// Returns the canonical path, resolving all symbolic links.
    ///
    /// <https://github.com/parcel-bundler/parcel/blob/4d27ec8b8bd1792f536811fef86e74a31fa0e704/crates/parcel-resolver/src/cache.rs#L232>
    pub(crate) fn canonicalize_impl(&self, path: &CachedPath) -> Result<CachedPath, ResolveError> {
        // Check if this thread is already canonicalizing. If so, we have found a circular symlink.
        // If a different thread is canonicalizing, OnceLock will queue this thread to wait for the result.
        let tid = THREAD_ID.with(|t| *t);
        if path.canonicalizing.load(Ordering::Acquire) == tid {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Circular symlink").into());
        }

        path.canonicalized
            .get_or_init(|| {
                path.canonicalizing.store(tid, Ordering::Release);

                let res = path.parent().map_or_else(
                    || Ok(path.normalize_root(self)),
                    |parent| {
                        self.canonicalize_impl(&parent).and_then(|parent_canonical| {
                            let normalized = parent_canonical.normalize_with(
                                path.path().strip_prefix(parent.path()).unwrap(),
                                self,
                            );

                            if self.fs.symlink_metadata(path.path()).is_ok_and(|m| m.is_symlink) {
                                let link = self.fs.read_link(normalized.path())?;
                                if link.is_absolute() {
                                    return self.canonicalize_impl(&self.value(&link.normalize()));
                                } else if let Some(dir) = normalized.parent() {
                                    // Symlink is relative `../../foo.js`, use the path directory
                                    // to resolve this symlink.
                                    return self
                                        .canonicalize_impl(&dir.normalize_with(&link, self));
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
                );

                path.canonicalizing.store(0, Ordering::Release);
                // Store the canonicalized path in the cache before downgrading to weak reference
                // This ensures there's always at least one strong reference to prevent dropping
                if let Ok(ref cp) = res {
                    // Only insert if not already present to avoid unnecessary operations
                    let paths = self.paths.pin();
                    if !paths.contains(cp) {
                        paths.insert(cp.clone());
                    }
                }
                // Convert to Weak reference for storage
                res.map(|cp| Arc::downgrade(&cp.0))
            })
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|weak| {
                weak.upgrade().map(CachedPath).ok_or_else(|| {
                    ResolveError::from(io::Error::other("Canonicalized path was dropped"))
                })
            })
    }
}
