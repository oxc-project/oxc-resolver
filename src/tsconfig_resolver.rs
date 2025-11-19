use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    CachedPath, Ctx, FileSystem, ResolveError, ResolveOptions, ResolveResult, ResolverGeneric,
    SpecifierError, TsConfig, TsconfigDiscovery, TsconfigReferences, path::PathUtil,
};

#[derive(Default)]
pub struct TsconfigResolveContext {
    extended_configs: Vec<PathBuf>,
}

impl TsconfigResolveContext {
    pub fn with_extended_file<R, T: FnOnce(&mut Self) -> R>(&mut self, path: PathBuf, cb: T) -> R {
        self.extended_configs.push(path);
        let result = cb(self);
        self.extended_configs.pop();
        result
    }

    pub fn is_already_extended(&self, path: &Path) -> bool {
        self.extended_configs.iter().any(|config| config == path)
    }

    pub fn get_extended_configs_with(&self, path: PathBuf) -> Vec<PathBuf> {
        let mut new_vec = Vec::with_capacity(self.extended_configs.len() + 1);
        new_vec.extend_from_slice(&self.extended_configs);
        new_vec.push(path);
        new_vec
    }
}

impl<Fs: FileSystem> ResolverGeneric<Fs> {
    /// Resolve `tsconfig`.
    ///
    /// The path can be:
    ///
    /// * Path to a file with `.json` extension.
    /// * Path to a file without `.json` extension, `.json` will be appended to filename.
    /// * Path to a directory, where the filename is defaulted to `tsconfig.json`
    ///
    /// # Errors
    ///
    /// * See [ResolveError]
    pub fn resolve_tsconfig<P: AsRef<Path>>(&self, path: P) -> Result<Arc<TsConfig>, ResolveError> {
        let path = path.as_ref();
        self.load_tsconfig(
            true,
            path,
            &TsconfigReferences::Auto,
            &mut TsconfigResolveContext::default(),
        )
    }

    fn load_tsconfig(
        &self,
        root: bool,
        path: &Path,
        references: &TsconfigReferences,
        ctx: &mut TsconfigResolveContext,
    ) -> Result<Arc<TsConfig>, ResolveError> {
        self.cache.get_tsconfig(root, path, |tsconfig| {
            let directory = self.cache.value(tsconfig.directory());
            tracing::trace!(tsconfig = ?tsconfig, "load_tsconfig");

            if ctx.is_already_extended(tsconfig.path()) {
                return Err(ResolveError::TsconfigCircularExtend(
                    ctx.get_extended_configs_with(tsconfig.path().to_path_buf()).into(),
                ));
            }

            // Extend tsconfig
            let extended_tsconfig_paths = tsconfig
                .extends()
                .map(|specifier| self.get_extended_tsconfig_path(&directory, tsconfig, specifier))
                .collect::<Result<Vec<_>, _>>()?;
            if !extended_tsconfig_paths.is_empty() {
                ctx.with_extended_file(tsconfig.path().to_owned(), |ctx| {
                    for extended_tsconfig_path in extended_tsconfig_paths {
                        let extended_tsconfig = self.load_tsconfig(
                            /* root */ false,
                            &extended_tsconfig_path,
                            &TsconfigReferences::Disabled,
                            ctx,
                        )?;
                        tsconfig.extend_tsconfig(&extended_tsconfig);
                    }
                    Result::Ok::<(), ResolveError>(())
                })?;
            }

            if tsconfig.load_references(references) {
                let path = tsconfig.path().to_path_buf();
                let directory = tsconfig.directory().to_path_buf();
                for reference in tsconfig.references_mut() {
                    let reference_tsconfig_path = directory.normalize_with(reference.path());
                    let tsconfig = self.cache.get_tsconfig(
                        /* root */ true,
                        &reference_tsconfig_path,
                        |reference_tsconfig| {
                            if reference_tsconfig.path() == path {
                                return Err(ResolveError::TsconfigSelfReference(
                                    reference_tsconfig.path().to_path_buf(),
                                ));
                            }
                            self.extend_tsconfig(
                                &self.cache.value(reference_tsconfig.directory()),
                                reference_tsconfig,
                                ctx,
                            )?;
                            Ok(())
                        },
                    )?;
                    reference.set_tsconfig(tsconfig);
                }
            }
            Ok(())
        })
    }

    fn extend_tsconfig(
        &self,
        directory: &CachedPath,
        tsconfig: &mut TsConfig,
        ctx: &mut TsconfigResolveContext,
    ) -> Result<(), ResolveError> {
        let extended_tsconfig_paths = tsconfig
            .extends()
            .map(|specifier| self.get_extended_tsconfig_path(directory, tsconfig, specifier))
            .collect::<Result<Vec<_>, _>>()?;
        for extended_tsconfig_path in extended_tsconfig_paths {
            let extended_tsconfig = self.load_tsconfig(
                /* root */ false,
                &extended_tsconfig_path,
                &TsconfigReferences::Disabled,
                ctx,
            )?;
            tsconfig.extend_tsconfig(&extended_tsconfig);
        }
        Ok(())
    }

    pub(crate) fn load_tsconfig_paths(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        ctx: &mut Ctx,
    ) -> ResolveResult {
        if cached_path.inside_node_modules() {
            return Ok(None);
        }
        let tsconfig = match &self.options.tsconfig {
            None => return Ok(None),
            Some(TsconfigDiscovery::Manual(tsconfig_options)) => {
                let tsconfig = self.load_tsconfig(
                    /* root */ true,
                    &tsconfig_options.config_file,
                    &tsconfig_options.references,
                    &mut TsconfigResolveContext::default(),
                )?;
                // Cache the loaded tsconfig in the path's directory
                let tsconfig_dir = self.cache.value(tsconfig.directory());
                _ = tsconfig_dir.tsconfig.get_or_init(|| Some(Arc::clone(&tsconfig)));
                tsconfig
            }
            Some(TsconfigDiscovery::Auto) => {
                let Some(tsconfig) = self.find_tsconfig(cached_path, ctx)? else {
                    return Ok(None);
                };
                tsconfig
            }
        };

        let paths = tsconfig.resolve(cached_path.path(), specifier);
        for path in paths {
            let resolved_path = self.cache.value(&path);
            if let Some(resolution) = self.load_as_file_or_directory(&resolved_path, ".", ctx)? {
                // Cache the tsconfig in the resolved path
                _ = resolved_path.tsconfig.get_or_init(|| Some(Arc::clone(&tsconfig)));
                return Ok(Some(resolution));
            }
        }
        Ok(None)
    }

    /// Find tsconfig.json of a path by traversing parent directories.
    ///
    /// # Errors
    ///
    /// * [ResolveError::Json]
    pub(crate) fn find_tsconfig(
        &self,
        cached_path: &CachedPath,
        ctx: &mut Ctx,
    ) -> Result<Option<Arc<TsConfig>>, ResolveError> {
        // Don't discover tsconfig for paths inside node_modules
        if cached_path.inside_node_modules() {
            return Ok(None);
        }
        // Skip non-absolute paths (e.g. virtual modules)
        if !cached_path.path.is_absolute() {
            return Ok(None);
        }

        let mut cache_value = Some(cached_path.clone());
        while let Some(cv) = cache_value {
            if let Some(tsconfig) = cv.tsconfig.get_or_try_init(|| {
                let tsconfig_path = cv.path.join("tsconfig.json");
                let tsconfig_path = self.cache.value(&tsconfig_path);
                if self.cache.is_file(&tsconfig_path, ctx) {
                    self.resolve_tsconfig(tsconfig_path.path()).map(Some)
                } else {
                    Ok(None)
                }
            })? {
                return Ok(Some(Arc::clone(tsconfig)));
            }
            cache_value = cv.parent();
        }
        Ok(None)
    }

    fn get_extended_tsconfig_path(
        &self,
        directory: &CachedPath,
        tsconfig: &TsConfig,
        specifier: &str,
    ) -> Result<PathBuf, ResolveError> {
        match specifier.as_bytes().first() {
            None => Err(ResolveError::Specifier(SpecifierError::Empty(specifier.to_string()))),
            Some(b'/') => Ok(PathBuf::from(specifier)),
            Some(b'.') => Ok(tsconfig.directory().normalize_with(specifier)),
            _ => self
                .clone_with_options(ResolveOptions {
                    tsconfig: None,
                    extensions: vec![".json".into()],
                    main_files: vec!["tsconfig".into()],
                    #[cfg(feature = "yarn_pnp")]
                    yarn_pnp: self.options.yarn_pnp,
                    #[cfg(feature = "yarn_pnp")]
                    cwd: self.options.cwd.clone(),
                    ..ResolveOptions::default()
                })
                .load_package_self_or_node_modules(directory, specifier, &mut Ctx::default())
                .map(|p| p.to_path_buf())
                .map_err(|err| match err {
                    ResolveError::NotFound(_) => {
                        ResolveError::TsconfigNotFound(PathBuf::from(specifier))
                    }
                    _ => err,
                }),
        }
    }
}
