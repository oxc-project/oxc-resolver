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
    ) -> Option<Result<CachedPath, ResolveError>> {
        let package_map = match self.package_map(ctx) {
            Ok(Some(package_map)) => package_map,
            Ok(None) => return None,
            Err(error) => return Some(Err(error)),
        };
        Some(self.load_package_map_for_importer(
            cached_path,
            specifier,
            package_name,
            subpath,
            &package_map,
            tsconfig,
            ctx,
        ))
    }

    /// Implements package-map dispatch from step 6 of Node's CommonJS resolution pseudocode.
    ///
    /// Node permits the importing package ID to be propagated by the caller. The resolver returns
    /// paths rather than package IDs, so this implementation uses the importing path to find it.
    ///
    /// See <https://nodejs.org/api/modules.html#all-together>.
    fn load_package_map_for_importer(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        name: &str,
        subpath: &str,
        package_map: &PackageMap,
        tsconfig: Option<&TsConfig>,
        ctx: &mut Ctx,
    ) -> Result<CachedPath, ResolveError> {
        // Step 6.a: derive PARENT_PACKAGE_ID from the importer. `resolve` supplies dirname(Y), while
        // the first `resolve_file` dispatch retains Y so file-valued package entries match Node.
        let package_map_parent = ctx.package_map_parent.take();
        let parent_path = package_map_parent.as_deref().unwrap_or_else(|| cached_path.path());
        let result = (|| {
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
        })();
        if result.is_err() {
            ctx.package_map_parent = package_map_parent;
        }
        result
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
        let target = package_map.package(dependency_id).ok_or_else(|| {
            ResolveError::PackageMapKeyNotFound {
                package_id: dependency_id.to_string(),
                package_map_path: package_map.path().to_path_buf(),
            }
        })?;

        // 6. Let PACKAGE_PATH be the resolved path of TARGET.
        let package_path = target.path();
        tracing::debug!(parent_package_id, dependency_id, ?package_path, "resolve_package_map");
        let package_path = self.cache.value(package_path);

        // 7. LOAD_PACKAGE_EXPORTS(SUBPATH, PACKAGE_PATH).
        if self.is_dir(&package_path, ctx)
            && let Some(path) =
                self.load_package_exports(specifier, subpath, &package_path, tsconfig, ctx)?
        {
            return Ok(path);
        }

        // normalize_with requires a relative operand; subpaths begin with `/`.
        let dot_subpath = Self::dot_subpath(subpath);
        let package_subpath = package_path.normalize_with(dot_subpath.as_ref(), &self.cache);

        // 8. LOAD_AS_FILE(PACKAGE_PATH/SUBPATH).
        // 9. LOAD_AS_DIRECTORY(PACKAGE_PATH/SUBPATH).
        // Apply enhanced-resolve aliases at the same point as regular node_modules resolution.
        if (!self.options.alias_fields.is_empty() || !self.options.alias.is_empty())
            && !self.options.resolve_to_context
            && self.is_dir(&package_subpath, ctx)
            && let Some(path) = self.load_browser_field_or_alias(&package_subpath, tsconfig, ctx)?
        {
            return Ok(path);
        }
        if let Some(path) =
            self.load_as_file_or_directory(&package_subpath, subpath, tsconfig, ctx)?
        {
            return Ok(path);
        }

        // 10. THROW "not found".
        Err(ResolveError::NotFound(specifier.to_string()))
    }

    fn package_map(&self, ctx: &mut Ctx) -> Result<Option<Arc<PackageMap>>, ResolveError> {
        // Resolver construction cannot surface map errors, so cache the first lazy load result.
        let package_map = self.cache.package_map.get_or_init(|package_map_path| {
            tracing::debug!(path = ?package_map_path, "load_package_map");
            let json = self.cache.fs.read(package_map_path)?;
            PackageMap::parse(package_map_path.to_path_buf(), json)
        });

        match package_map {
            Ok(Some(package_map)) => {
                ctx.add_file_dependency(package_map.path());
                Ok(Some(package_map))
            }
            Ok(None) => Ok(None),
            Err((package_map_path, error)) => {
                match &error {
                    ResolveError::Json(error) => ctx.add_file_dependency(&error.path),
                    ResolveError::PackageMapInvalid { package_map_path, .. } => {
                        ctx.add_file_dependency(package_map_path);
                    }
                    _ => ctx.add_missing_dependency(&package_map_path),
                }
                Err(error)
            }
        }
    }
}
