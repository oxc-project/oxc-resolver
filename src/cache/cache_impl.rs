use std::{
    borrow::Cow,
    hash::{BuildHasherDefault, Hash, Hasher},
    io,
    path::{Path, PathBuf},
    sync::{Arc, atomic::Ordering},
};

use boxcar::Vec as BoxcarVec;
use cfg_if::cfg_if;
#[cfg(feature = "yarn_pnp")]
use once_cell::sync::OnceCell;
use papaya::HashMap;
use rustc_hash::FxHasher;

use super::cached_path::{CachedPath, CachedPathImpl};
use super::thread_local::THREAD_ID;
use crate::{
    FileSystem, PackageJson, ResolveError, ResolveOptions, TsConfig,
    context::ResolveContext as Ctx, path::PathUtil,
};

/// Cache implementation used for caching filesystem access.
#[derive(Default)]
pub struct Cache<Fs> {
    pub(crate) fs: Fs,
    pub(crate) nodes: BoxcarVec<CachedPathImpl>,
    pub(crate) path_index: HashMap<PathBuf, usize, BuildHasherDefault<FxHasher>>,
    pub(crate) tsconfigs: HashMap<PathBuf, Arc<TsConfig>, BuildHasherDefault<FxHasher>>,
    #[cfg(feature = "yarn_pnp")]
    pub(crate) yarn_pnp_manifest: OnceCell<pnp::Manifest>,
}

impl<Fs: FileSystem> Cache<Fs> {
    pub fn clear(&self) {
        // Note: Can't clear boxcar vec, but can clear the index
        self.path_index.pin().clear();
        self.tsconfigs.pin().clear();
    }

    pub(crate) fn get_node(&self, idx: usize) -> &CachedPathImpl {
        self.nodes.get(idx).expect("Invalid node index")
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

        let path_buf = path.to_path_buf();
        let path_index = self.path_index.pin();

        if let Some(idx) = path_index.get(&path_buf) {
            return CachedPath(*idx);
        }

        let parent = path.parent().map(|p| self.value(p).0);
        let is_node_modules = path.file_name().as_ref().is_some_and(|&name| name == "node_modules");
        let inside_node_modules = is_node_modules
            || parent
                .as_ref()
                .is_some_and(|&parent_idx| self.get_node(parent_idx).inside_node_modules);

        let cached_path_impl = CachedPathImpl::new(
            hash,
            path_buf.clone(),
            is_node_modules,
            inside_node_modules,
            parent,
        );

        let idx = self.nodes.push(cached_path_impl);
        path_index.insert(path_buf, idx);
        CachedPath(idx)
    }

    pub(crate) fn canonicalize(&self, path: &CachedPath) -> Result<PathBuf, ResolveError> {
        let cached_path = self.canonicalize_impl(path)?;
        let path = cached_path.to_path_buf(self);
        cfg_if! {
            if #[cfg(target_os = "windows")] {
                crate::windows::strip_windows_prefix(path)
            } else {
                Ok(path)
            }
        }
    }

    pub(crate) fn is_file(&self, path: &CachedPath, ctx: &mut Ctx) -> bool {
        if let Some(meta) = path.meta(self, &self.fs) {
            ctx.add_file_dependency(path.path(self));
            meta.is_file
        } else {
            ctx.add_missing_dependency(path.path(self));
            false
        }
    }

    pub(crate) fn is_dir(&self, path: &CachedPath, ctx: &mut Ctx) -> bool {
        path.meta(self, &self.fs).map_or_else(
            || {
                ctx.add_missing_dependency(path.path(self));
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
        let node = self.get_node(path.0);
        // Change to `std::sync::OnceLock::get_or_try_init` when it is stable.
        let result = node
            .package_json
            .get_or_try_init(|| {
                let package_json_path = node.path.join("package.json");
                let Ok(package_json_bytes) = self.fs.read(&package_json_path) else {
                    return Ok(None);
                };

                let real_path = if options.symlinks {
                    self.canonicalize(path)?.join("package.json")
                } else {
                    package_json_path.clone()
                };
                PackageJson::parse(&self.fs, package_json_path, real_path, package_json_bytes)
                    .map(|package_json| Some(Arc::new(package_json)))
                    .map_err(ResolveError::Json)
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
                    deps.push(node.path.join("package.json"));
                }
            }
            Err(_) => {
                if let Some(deps) = &mut ctx.file_dependencies {
                    deps.push(node.path.join("package.json"));
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
            .read_to_string(&tsconfig_path)
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
            nodes: BoxcarVec::new(),
            path_index: HashMap::builder()
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
        let node = self.get_node(path.0);

        // Check if this thread is already canonicalizing. If so, we have found a circular symlink.
        let tid = THREAD_ID.with(|t| *t);
        if node.canonicalizing.load(Ordering::Acquire) == tid {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Circular symlink").into());
        }

        // Lock, check cached value, and unlock - CRITICAL: don't hold lock during recursion
        let cached = {
            let guard = node.canonicalized.lock().unwrap();
            guard.clone()
        }; // MutexGuard dropped here!

        if let Ok(Some(idx)) = cached {
            return Ok(CachedPath(idx));
        }
        // If we got an error last time, return it
        if let Err(e) = cached {
            return Err(e);
        }

        // Compute without holding any locks
        node.canonicalizing.store(tid, Ordering::Release);

        let res = node.parent.map_or_else(
            || Ok(path.normalize_root(self)),
            |parent_idx| {
                let parent = CachedPath(parent_idx);
                self.canonicalize_impl(&parent).and_then(|parent_canonical| {
                    let parent_path = self.get_node(parent_idx).path.as_path();
                    let normalized = parent_canonical
                        .normalize_with(node.path.strip_prefix(parent_path).unwrap(), self);

                    if self.fs.symlink_metadata(&node.path).is_ok_and(|m| m.is_symlink) {
                        let link = self.fs.read_link(normalized.path(self))?;
                        if link.is_absolute() {
                            return self.canonicalize_impl(&self.value(&link.normalize()));
                        } else if let Some(dir_idx) = normalized.parent(self) {
                            // Symlink is relative `../../foo.js`, use the path directory
                            // to resolve this symlink.
                            return self
                                .canonicalize_impl(&dir_idx.normalize_with(&link, self));
                        }
                        debug_assert!(
                            false,
                            "Failed to get path parent for {}.",
                            normalized.path(self).display()
                        );
                    }

                    Ok(normalized)
                })
            },
        );

        node.canonicalizing.store(0, Ordering::Release);

        // Store result - lock briefly just to write
        *node.canonicalized.lock().unwrap() =
            res.as_ref().map(|cp| Some(cp.0)).map_err(Clone::clone);

        res
    }
}
