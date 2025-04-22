#[cfg(not(target_os = "windows"))] // MemoryFS's path separator is always `/` so the test will not pass in windows.
mod windows {
    use std::{path::Path, sync::Arc};

    use super::super::memory_fs::MemoryFS;
    use crate::{Cache, FsCache, PackageJson, ResolveContext, ResolveOptions, ResolverGeneric};

    fn file_system() -> MemoryFS {
        MemoryFS::new(&[
            ("/a/node_modules/package/index.js", ""),
            ("/a/node_modules/package/file.js", ""),
            ("/a/node_modules/package/package.json", r#"{"main":"a", "name": "package"}"#),
            ("/a/node_modules/package/lib/index.js", r"export const a = 1000"),
            ("/a/node_modules/package/lib/package.json", r#"{"sideEffects": true}"#),
        ])
    }

    #[test]
    #[cfg(not(target_os = "windows"))] // MemoryFS's path separator is always `/` so the test will not pass in windows.
    fn resolve_to_context() -> Result<(), crate::ResolveError> {
        use crate::PackageJsonResolutionKind;

        let file_system = file_system();

        let resolver = ResolverGeneric::new_with_cache(
            Arc::new(FsCache::new(file_system)),
            ResolveOptions::default(),
        );

        let package_json_nearest_cases =
            [("/a", "package", Some("package")), ("/a", "package/lib", None)];

        let package_json_resolution_root_cases =
            [("/a", "package", Some("package")), ("/a", "package/lib", Some("package"))];

        for (directory, specifier, expected_name) in package_json_nearest_cases {
            let package_json_name = resolve_file_package_json_name(
                directory,
                specifier,
                PackageJsonResolutionKind::Nearest,
                &resolver,
            )?;
            assert_eq!(package_json_name.as_deref(), expected_name);
        }

        for (directory, specifier, expected_name) in package_json_resolution_root_cases {
            let package_json_name = resolve_file_package_json_name(
                directory,
                specifier,
                PackageJsonResolutionKind::Root,
                &resolver,
            )?;
            assert_eq!(package_json_name.as_deref(), expected_name);
        }
        Ok(())
    }

    fn resolve_file_package_json_name<P: AsRef<Path>, C: Cache>(
        directory: P,
        specifier: &str,
        package_json_resolution_kind: crate::PackageJsonResolutionKind,
        resolver: &ResolverGeneric<C>,
    ) -> Result<Option<String>, crate::ResolveError> {
        let result = resolver.resolve_with_context(
            directory,
            specifier,
            &mut ResolveContext { package_json_resolution_kind, ..ResolveContext::default() },
        )?;
        Ok(result.package_json().and_then(|item| item.name().map(std::string::ToString::to_string)))
    }
}
