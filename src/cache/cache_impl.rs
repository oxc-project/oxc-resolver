use std::{
    borrow::Cow,
    hash::{BuildHasherDefault, Hash, Hasher},
    io,
    path::{Path, PathBuf},
    sync::{Arc, atomic::Ordering},
};

use arc_swap::ArcSwap;
use cfg_if::cfg_if;
#[cfg(feature = "yarn_pnp")]
use once_cell::sync::OnceCell;
use papaya::HashMap;
use rustc_hash::FxHasher;

use super::cached_path::CachedPath;
use super::path_node::{CacheGeneration, PathHandle, PathNode};
use super::thread_local::THREAD_ID;
use crate::{
    FileSystem, PackageJson, ResolveError, ResolveOptions, TsConfig,
    context::ResolveContext as Ctx, path::PathUtil,
};

/// Cache implementation used for caching filesystem access.
pub struct Cache<Fs> {
    pub(crate) fs: Fs,
    // Generation-based path storage (replaces old HashSet<CachedPath>)
    pub(crate) generation: ArcSwap<CacheGeneration>,
    pub(crate) tsconfigs: HashMap<PathBuf, Arc<TsConfig>, BuildHasherDefault<FxHasher>>,
    #[cfg(feature = "yarn_pnp")]
    pub(crate) yarn_pnp_manifest: OnceCell<pnp::Manifest>,
}

impl<Fs: Default + FileSystem> Default for Cache<Fs> {
    fn default() -> Self {
        Self::new(Fs::default())
    }
}

impl<Fs: FileSystem> Cache<Fs> {
    pub fn clear(&self) {
        // Swap to new generation
        let new_gen = Arc::new(CacheGeneration::new());
        self.generation.store(new_gen);
        self.tsconfigs.pin().clear();
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn value(&self, path: &Path) -> CachedPath {
        let hash = {
            let mut hasher = FxHasher::default();
            path.as_os_str().hash(&mut hasher);
            hasher.finish()
        };

        let generation = self.generation.load_full();

        // Fast path: lock-free lookup via papaya
        {
            let path_to_idx = generation.path_to_idx.pin();
            if let Some(&idx) = path_to_idx.get(&hash) {
                drop(path_to_idx);
                return CachedPath(PathHandle { index: idx, generation });
            }
        }

        // Slow path: need to insert
        let parent = path.parent().map(|p| self.value(p));
        let is_node_modules = path.file_name().as_ref().is_some_and(|&name| name == "node_modules");
        let inside_node_modules =
            is_node_modules || parent.as_ref().is_some_and(|parent| parent.inside_node_modules());

        let parent_idx = parent.as_ref().map(|p| p.0.index);

        let node = PathNode::new(
            hash,
            path.to_path_buf().into_boxed_path(),
            parent_idx,
            is_node_modules,
            inside_node_modules,
        );

        // Lock Vec for append
        let idx = {
            let mut nodes = generation.nodes.write().unwrap();
            // Double-check after acquiring write lock
            {
                let path_to_idx = generation.path_to_idx.pin();
                if let Some(&idx) = path_to_idx.get(&hash) {
                    drop(path_to_idx);
                    drop(nodes);
                    return CachedPath(PathHandle { index: idx, generation });
                }
            }
            let idx = nodes.len() as u32;
            nodes.push(node);
            idx
        };

        generation.path_to_idx.pin().insert(hash, idx);
        CachedPath(PathHandle { index: idx, generation })
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
            ctx.add_file_dependency(&path.path());
            meta.is_file
        } else {
            ctx.add_missing_dependency(&path.path());
            false
        }
    }

    pub(crate) fn is_dir(&self, path: &CachedPath, ctx: &mut Ctx) -> bool {
        path.meta(&self.fs).map_or_else(
            || {
                ctx.add_missing_dependency(&path.path());
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

        // First check if already initialized
        let existing_result = {
            let nodes = path.0.generation.nodes.read().unwrap();
            let node = &nodes[path.0.index as usize];
            node.package_json.get().cloned()
        };

        if let Some(result) = existing_result {
            match &result {
                Some(package_json) => {
                    ctx.add_file_dependency(&package_json.path);
                }
                None => {
                    if let Some(deps) = &mut ctx.missing_dependencies {
                        deps.push(path.path().join("package.json"));
                    }
                }
            }
            return Ok(result);
        }

        // Not initialized, compute it without holding lock
        let package_json_path = path.path().join("package.json");
        let package_json_bytes = match self.fs.read(&package_json_path) {
            Ok(bytes) => bytes,
            Err(_) => {
                // Store None result
                let nodes = path.0.generation.nodes.read().unwrap();
                let node = &nodes[path.0.index as usize];
                node.package_json.get_or_init(|| None);
                drop(nodes);

                if let Some(deps) = &mut ctx.missing_dependencies {
                    deps.push(package_json_path);
                }
                return Ok(None);
            }
        };

        let real_path = if options.symlinks {
            self.canonicalize(path)?.join("package.json")
        } else {
            package_json_path.clone()
        };

        let parse_result =
            PackageJson::parse(&self.fs, package_json_path.clone(), real_path, package_json_bytes)
                .map(|package_json| Some(Arc::new(package_json)))
                .map_err(ResolveError::Json);

        // Store the result
        {
            let nodes = path.0.generation.nodes.read().unwrap();
            let node = &nodes[path.0.index as usize];
            node.package_json.get_or_init(|| match &parse_result {
                Ok(opt) => opt.clone(),
                Err(_) => None,
            });
        }

        // https://github.com/webpack/enhanced-resolve/blob/58464fc7cb56673c9aa849e68e6300239601e615/lib/DescriptionFileUtils.js#L68-L82
        match &parse_result {
            Ok(Some(package_json)) => {
                ctx.add_file_dependency(&package_json.path);
            }
            Ok(None) => {
                if let Some(deps) = &mut ctx.file_dependencies {
                    deps.push(package_json_path);
                }
            }
            Err(_) => {
                // Even when there's an error, track the file dependency
                if let Some(deps) = &mut ctx.file_dependencies {
                    deps.push(package_json_path);
                }
            }
        }

        parse_result
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
            generation: ArcSwap::from_pointee(CacheGeneration::new()),
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
        let tid = THREAD_ID.with(|t| *t);

        // Access canonicalizing atomic through PathNode
        let canonicalizing_val = {
            let nodes = path.0.generation.nodes.read().unwrap();
            nodes[path.0.index as usize].canonicalizing.load(Ordering::Acquire)
        };

        if canonicalizing_val == tid {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Circular symlink").into());
        }

        // Check if already canonicalized
        let cached_result = {
            let nodes = path.0.generation.nodes.read().unwrap();
            let guard = nodes[path.0.index as usize].canonicalized_idx.lock().unwrap();
            guard.clone()
        };

        if let Ok(Some(idx)) = cached_result {
            return Ok(CachedPath(PathHandle {
                index: idx,
                generation: path.0.generation.clone(),
            }));
        }

        // Set canonicalizing flag
        {
            let nodes = path.0.generation.nodes.read().unwrap();
            nodes[path.0.index as usize].canonicalizing.store(tid, Ordering::Release);
        }

        let res = path.parent().map_or_else(
            || Ok(path.normalize_root(self)),
            |parent| {
                let path_buf = path.path();
                let parent_buf = parent.path();

                self.canonicalize_impl(&parent).and_then(|parent_canonical| {
                    let normalized = parent_canonical
                        .normalize_with(path_buf.strip_prefix(&parent_buf).unwrap(), self);

                    if self.fs.symlink_metadata(&path_buf).is_ok_and(|m| m.is_symlink) {
                        let link = self.fs.read_link(&normalized.path())?;
                        if link.is_absolute() {
                            return self.canonicalize_impl(&self.value(&link.normalize()));
                        } else if let Some(dir) = normalized.parent() {
                            // Symlink is relative `../../foo.js`, use the path directory
                            // to resolve this symlink.
                            return self.canonicalize_impl(&dir.normalize_with(&link, self));
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

        // Clear canonicalizing flag
        {
            let nodes = path.0.generation.nodes.read().unwrap();
            nodes[path.0.index as usize].canonicalizing.store(0, Ordering::Release);
        }

        // Store result as index
        {
            let nodes = path.0.generation.nodes.read().unwrap();
            let mut guard = nodes[path.0.index as usize].canonicalized_idx.lock().unwrap();
            *guard = res.as_ref().map_err(Clone::clone).map(|cp| Some(cp.0.index));
        }

        res
    }
}
