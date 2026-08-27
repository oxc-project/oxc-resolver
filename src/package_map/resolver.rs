use std::sync::Arc;

use crate::{CachedPath, Ctx, ResolveError, ResolverImpl, TsConfig};

use super::map::{FindPackageIdError, PackageMap};

impl ResolverImpl {
    pub(crate) fn package_map_resolve(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        package_name: &str,
        subpath: &str,
        tsconfig: Option<&TsConfig>,
        ctx: &mut Ctx,
    ) -> Option<Result<Option<CachedPath>, ResolveError>> {
        self.cache.package_map.as_ref()?;
        Some(
            self.load_package_map_for_importer(
                cached_path,
                specifier,
                package_name,
                subpath,
                tsconfig,
                ctx,
            )
            .map(Some),
        )
    }

    #[inline(never)]
    pub(crate) fn load_package_self_or_package_map(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        tsconfig: Option<&TsConfig>,
        ctx: &mut Ctx,
    ) -> Result<CachedPath, ResolveError> {
        let (package_name, subpath) = Self::parse_package_specifier(specifier);
        if subpath.is_empty() {
            ctx.with_fully_specified(false);
        }
        // 5. LOAD_PACKAGE_SELF(X, dirname(Y))
        if let Some(path) = self.load_package_self(cached_path, specifier, tsconfig, ctx)? {
            return Ok(path);
        }
        self.load_package_map_for_importer(
            cached_path,
            specifier,
            package_name,
            subpath,
            tsconfig,
            ctx,
        )
    }

    /// Implements package-map dispatch from step 6 of Node's CommonJS resolution pseudocode.
    ///
    /// Node permits the importing package ID to be propagated by the caller. The resolver returns
    /// paths rather than package IDs, so this implementation takes the specified fallback and calls
    /// `FIND_PACKAGE_ID(dirname(Y), PACKAGE_MAP)` for every uncached importer path.
    ///
    /// See <https://nodejs.org/api/modules.html#all-together>.
    #[inline(never)]
    fn load_package_map_for_importer(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        name: &str,
        subpath: &str,
        tsconfig: Option<&TsConfig>,
        ctx: &mut Ctx,
    ) -> Result<CachedPath, ResolveError> {
        let package_map = self.package_map(ctx)?;

        // Step 6.a: derive PARENT_PACKAGE_ID from dirname(Y). `cached_path` is already dirname(Y)
        // because the public resolve API accepts the importing directory, not the importing file.
        let parent_path = cached_path.path();
        let parent_package_id =
            package_map.find_package_id(parent_path).map_err(|error| match error {
                FindPackageIdError::AmbiguousResolution => {
                    ResolveError::PackageMapAmbiguousResolution {
                        specifier: specifier.to_string(),
                        parent_path: parent_path.to_path_buf(),
                        package_map_path: package_map.path().to_path_buf(),
                    }
                }
                FindPackageIdError::ExternalFile => ResolveError::PackageMapExternalFile {
                    specifier: specifier.to_string(),
                    parent_path: parent_path.to_path_buf(),
                    package_map_path: package_map.path().to_path_buf(),
                },
            })?;

        self.load_package_map(
            specifier,
            name,
            subpath,
            parent_package_id,
            package_map,
            tsconfig,
            ctx,
        )
    }

    /// Implements `LOAD_PACKAGE_MAP(X, PARENT_PACKAGE_ID, PACKAGE_MAP)`.
    ///
    /// The numbered comments correspond directly to Node's
    /// [CommonJS resolution pseudocode](https://nodejs.org/api/modules.html#all-together).
    fn load_package_map(
        &self,
        specifier: &str,
        name: &str,
        subpath: &str,
        parent_package_id: &str,
        package_map: &PackageMap,
        tsconfig: Option<&TsConfig>,
        ctx: &mut Ctx,
    ) -> Result<CachedPath, ResolveError> {
        // Step 1 was performed once by `parse_package_specifier`: NAME includes an optional
        // `@scope/` prefix and SUBPATH is either empty or begins with `/`.

        // 2. Find the package map entry for key PARENT_PACKAGE_ID.
        let parent_package = package_map
            .package(parent_package_id)
            .expect("a package ID returned by the package map must have a corresponding entry");

        // 3. Look up NAME in the entry's "dependencies" map.
        // 4. If NAME is not found, THROW "not found".
        let dependency_id = parent_package
            .dependency(name)
            .ok_or_else(|| ResolveError::NotFound(specifier.to_string()))?;

        // 5. Let TARGET be PACKAGE_MAP.packages[dependencies[name]].
        let target = package_map
            .package(dependency_id)
            .ok_or_else(|| ResolveError::NotFound(specifier.to_string()))?;

        // 6. Let PACKAGE_PATH be the resolved path of TARGET.
        let package_path =
            target.path().ok_or_else(|| ResolveError::NotFound(specifier.to_string()))?;
        let package_path = self.cache.value(package_path);

        // 7. LOAD_PACKAGE_EXPORTS(SUBPATH, PACKAGE_PATH).
        if self.is_dir(&package_path, ctx)
            && let Some(path) =
                self.load_package_exports(specifier, subpath, &package_path, tsconfig, ctx)?
        {
            return Ok(path);
        }

        // `CachedPath::normalize_with` expects a relative operand. Prefixing the specified `/`
        // SUBPATH with `.` preserves PACKAGE_PATH/SUBPATH instead of treating it as a root path.
        let dot_subpath = Self::dot_subpath(subpath);
        let package_subpath = package_path.normalize_with(dot_subpath.as_ref(), &self.cache);

        // 8. LOAD_AS_FILE(PACKAGE_PATH/SUBPATH).
        if !subpath.ends_with('/')
            && let Some(path) = self.load_as_file(&package_subpath, tsconfig, ctx)?
        {
            return Ok(path);
        }

        // 9. LOAD_AS_DIRECTORY(PACKAGE_PATH/SUBPATH).
        if self.is_dir(&package_subpath, ctx)
            && let Some(path) = self.load_as_directory(&package_subpath, tsconfig, ctx)?
        {
            return Ok(path);
        }

        // 10. THROW "not found".
        Err(ResolveError::NotFound(specifier.to_string()))
    }

    /// Loads the package map selected by `NODE_OPTIONS` synchronously and caches either result.
    ///
    /// Node loads its map synchronously at startup. Loading is deferred here until the first
    /// applicable request because `Resolver::new` cannot report I/O or JSON errors. The map remains
    /// static for the lifetime of this resolver, matching Node's package-map limitation.
    fn package_map(&self, ctx: &mut Ctx) -> Result<&PackageMap, ResolveError> {
        let package_map = self
            .cache
            .package_map
            .as_ref()
            .expect("a package map cache is created when NODE_OPTIONS selects a package map");
        let package_map_path = &package_map.path;
        let package_map = package_map.value.get_or_init(|| {
            let json = self.cache.fs.read(package_map_path)?;
            PackageMap::parse(package_map_path.clone(), json)
                .map(Arc::new)
                .map_err(ResolveError::Json)
        });

        match package_map {
            Ok(package_map) => {
                ctx.add_file_dependency(package_map.path());
                Ok(package_map)
            }
            Err(error) => {
                match error {
                    ResolveError::Json(error) => ctx.add_file_dependency(&error.path),
                    _ => ctx.add_missing_dependency(package_map_path),
                }
                Err(error.clone())
            }
        }
    }
}
