use std::path::{Path, PathBuf};

use crate::{
    FileSystem, FileSystemOs, ResolveContext, ResolveError, ResolveOptions, Resolver,
    TsconfigDiscovery, TsconfigOptions, TsconfigReferences,
    package_map::{FindPackageIdError, PackageMap},
};

fn file_system() -> FileSystemOs {
    #[cfg(feature = "yarn_pnp")]
    {
        FileSystemOs::new(false)
    }
    #[cfg(not(feature = "yarn_pnp"))]
    {
        FileSystemOs::new()
    }
}

fn assert_dependencies(package_map: &PackageMap, package_id: &str, specifiers: &[&str]) {
    let package = package_map
        .package(package_id)
        .unwrap_or_else(|| panic!("package map does not contain package ID {package_id:?}"));
    assert!(!package.url().is_empty(), "package {package_id:?} has an empty URL");
    assert!(package.path().is_some(), "package {package_id:?} has no resolved path");

    for specifier in specifiers {
        let dependency_id = package.dependency(specifier).unwrap_or_else(|| {
            panic!("package {package_id:?} does not contain dependency {specifier:?}")
        });
        let dependency = package_map.package(dependency_id).unwrap_or_else(|| {
            panic!("package map does not contain dependency package ID {dependency_id:?}")
        });
        assert!(!dependency.url().is_empty(), "package {dependency_id:?} has an empty URL");
    }
}

fn parse_package_map(package_map_path: &Path) -> Result<PackageMap, crate::JSONError> {
    assert!(
        package_map_path.is_file(),
        "package map does not exist: {}",
        package_map_path.display()
    );

    let fs = file_system();
    let realpath = canonicalize_package_map_path(&fs, package_map_path);
    PackageMap::parse(package_map_path.to_path_buf(), realpath, fs.read(package_map_path).unwrap())
}

fn canonicalize_package_map_path(fs: &FileSystemOs, package_map_path: &Path) -> PathBuf {
    let realpath = fs.canonicalize(package_map_path).unwrap();
    #[cfg(target_os = "windows")]
    let realpath = crate::windows::strip_windows_prefix(realpath).unwrap();
    realpath
}

fn test_package_map(
    importer: &Path,
    package_map_path: &Path,
    specifiers: &[&str],
    assert_package_map: impl FnOnce(&PackageMap),
) {
    let package_map = parse_package_map(package_map_path).unwrap();
    assert_eq!(package_map.path(), package_map_path);
    assert_eq!(
        package_map.realpath(),
        canonicalize_package_map_path(&file_system(), package_map_path)
    );
    assert_package_map(&package_map);

    let resolver = package_map_resolver(package_map_path);
    for specifier in specifiers {
        let mut context = ResolveContext::default();
        if let Err(error) = resolver.resolve_with_context(importer, specifier, None, &mut context) {
            panic!(
                "failed to resolve {specifier:?} from {} using {}: {error}",
                importer.display(),
                package_map_path.display(),
            );
        }
        assert!(
            context.file_dependencies.contains(package_map_path),
            "resolution did not request {}",
            package_map_path.display()
        );
    }
}

fn package_map_resolver(package_map_path: &Path) -> Resolver {
    Resolver::new(ResolveOptions {
        condition_names: vec!["node".into(), "require".into()],
        modules: vec![],
        package_map: Some(package_map_path.to_path_buf()),
        ..ResolveOptions::default()
    })
}

#[test]
fn find_package_id() {
    let fixture = super::fixture_root().join("package-map/find-package-id");
    let package_map = parse_package_map(&fixture.join(".package-map.json")).unwrap();
    let root_importer = fixture.join("packages/root/index.js");
    let nested_importer = fixture.join("packages/root/nested/index.js");
    let ambiguous_importer = fixture.join("packages/duplicate/index.js");
    let external_importer = fixture.join("external/index.js");

    assert_eq!(package_map.path_cache_len(), 0);
    assert_eq!(package_map.find_package_id(&root_importer), Ok("root"));
    assert_eq!(package_map.find_package_id(&nested_importer), Ok("nested"));
    assert_eq!(
        package_map.find_package_id(&ambiguous_importer),
        Err(FindPackageIdError::AmbiguousResolution)
    );
    assert_eq!(
        package_map.find_package_id(&external_importer),
        Err(FindPackageIdError::ExternalFile)
    );
    assert_eq!(package_map.path_cache_len(), 4);

    assert_eq!(package_map.find_package_id(&root_importer), Ok("root"));
    assert_eq!(package_map.find_package_id(&nested_importer), Ok("nested"));
    assert_eq!(
        package_map.find_package_id(&ambiguous_importer),
        Err(FindPackageIdError::AmbiguousResolution)
    );
    assert_eq!(
        package_map.find_package_id(&external_importer),
        Err(FindPackageIdError::ExternalFile)
    );
    assert_eq!(package_map.path_cache_len(), 4);

    assert_eq!(
        package_map.package("root").unwrap().path(),
        Some(fixture.join("packages/root").as_path())
    );
}

#[test]
fn find_package_id_uses_canonical_package_map_path() {
    let fixture = super::fixture_root().join("package-map/find-package-id");
    let package_map_path = fixture.join(".package-map.json");
    let canonical_package_map_path = fixture.join("canonical-location/.package-map.json");
    let fs = file_system();
    let package_map = PackageMap::parse(
        package_map_path.clone(),
        canonical_package_map_path,
        fs.read(&package_map_path).unwrap(),
    )
    .unwrap();

    assert_eq!(
        package_map.find_package_id(&fixture.join("canonical-location/packages/root/index.js")),
        Ok("root")
    );
    assert_eq!(
        package_map.package("root").unwrap().path(),
        Some(fixture.join("canonical-location/packages/root").as_path())
    );
}

#[test]
fn relative_package_map_path_uses_cwd() {
    let fixtures = super::fixture_root();
    let fixture = fixtures.join("package-map/resolution");
    let package_map_path = PathBuf::from("package-map/resolution/node_modules/.package-map.json");
    let resolver = Resolver::new(ResolveOptions {
        cwd: Some(fixtures),
        condition_names: vec!["node".into(), "require".into()],
        modules: vec![],
        package_map: Some(package_map_path),
        ..ResolveOptions::default()
    });

    assert_eq!(
        resolver.options().package_map.as_deref(),
        Some(fixture.join("node_modules/.package-map.json").as_path())
    );
    assert_eq!(
        resolver
            .resolve(fixture.join("apps/web/src"), "axios")
            .map(|resolution| resolution.full_path()),
        Ok(fixture.join("node_modules/store/axios/index.js"))
    );
}

#[test]
#[cfg_attr(target_os = "wasi", ignore)]
fn symlinked_package_map_uses_canonical_location_as_base() {
    let fixture = super::fixture_root().join("integration/nested-symlink");
    let package_map_path = fixture.join("apps/tooling/.package-map.json");

    // Some Windows checkouts materialize repository symlinks as plain files.
    if !package_map_path.is_file() {
        return;
    }

    let resolver = package_map_resolver(&package_map_path);
    assert_eq!(
        resolver
            .resolve(fixture.join("tooling/typescript-config"), "dep")
            .map(|resolution| resolution.full_path()),
        Ok(fixture.join("nm/index.js"))
    );
}

#[test]
fn pnpm() {
    let fixtures = super::fixture_root();
    let specifiers = ["axios", "decimal.js", "postcss"];
    test_package_map(
        &fixtures.join("pnpm"),
        &fixtures.parent().unwrap().join("node_modules/.package-map.json"),
        &specifiers,
        |package_map| assert_dependencies(package_map, "fixtures/pnpm", &specifiers),
    );
}

#[test]
fn yarn() {
    let fixture = super::fixture_root().join("yarn");
    let specifiers = ["typescript"];
    test_package_map(
        &fixture,
        &fixture.join("node_modules/.package-map.json"),
        &specifiers,
        |package_map| assert_dependencies(package_map, ".", &specifiers),
    );
}

#[test]
fn resolution() {
    let fixture = super::fixture_root().join("package-map/resolution");
    let importer = fixture.join("apps/web/src");
    let package_map_path = fixture.join("node_modules/.package-map.json");
    let specifiers = ["react", "axios", "@bench/ui"];
    test_package_map(&importer, &package_map_path, &specifiers, |package_map| {
        assert_dependencies(package_map, "apps/web", &specifiers);

        let axios_id = package_map.package("apps/web").unwrap().dependency("axios").unwrap();
        let follow_redirects_id =
            package_map.package(axios_id).unwrap().dependency("follow-redirects").unwrap();
        assert!(package_map.package(follow_redirects_id).is_some());
    });

    let resolver = package_map_resolver(&package_map_path);
    assert_eq!(
        resolver.resolve(&importer, "axios/client").map(|resolution| resolution.full_path()),
        Ok(fixture.join("node_modules/store/axios/lib/client.js"))
    );
    assert_eq!(
        resolver
            .resolve(fixture.join("node_modules/store/axios/lib"), "follow-redirects")
            .map(|resolution| resolution.full_path()),
        Ok(fixture.join("node_modules/store/follow-redirects/index.js"))
    );
    assert!(matches!(
        resolver.resolve(&importer, "follow-redirects"),
        Err(ResolveError::NotFound(specifier)) if specifier == "follow-redirects"
    ));
}

#[test]
fn package_map_accessors_and_urls() {
    let fixture = super::fixture_root().join("package-map/resolution/node_modules");
    let package_map = parse_package_map(&fixture.join(".package-map.json")).unwrap();

    assert_eq!(package_map.len(), 8);
    assert!(!package_map.is_empty());
    assert!(format!("{package_map:?}").contains("packages: 8"));
    assert_eq!(package_map.resolve_url("./store/react"), Some(fixture.join("store/react")));
    assert_eq!(package_map.resolve_url("https://example.com/package"), None);
    assert_eq!(package_map.resolve_url("%FF"), None);
    #[cfg(not(target_arch = "wasm32"))]
    assert!(package_map.resolve_url("file:///tmp/package").is_some());

    let empty =
        parse_package_map(&super::fixture_root().join("package-map/empty/.package-map.json"))
            .unwrap();
    assert!(empty.is_empty());
}

#[test]
fn load_as_file_and_directory() {
    let fixture = super::fixture_root().join("package-map/resolution");
    let importer = fixture.join("apps/web/src");
    let resolver = package_map_resolver(&fixture.join("node_modules/.package-map.json"));

    assert_eq!(
        resolver.resolve(&importer, "plain-file").map(|resolution| resolution.full_path()),
        Ok(fixture.join("node_modules/store/plain-file.js"))
    );
    assert_eq!(
        resolver.resolve(&importer, "plain-directory").map(|resolution| resolution.full_path()),
        Ok(fixture.join("node_modules/store/plain-directory/index.js"))
    );
    for specifier in ["plain-directory/missing", "invalid-target", "missing-target"] {
        assert!(matches!(
            resolver.resolve(&importer, specifier),
            Err(ResolveError::NotFound(not_found)) if not_found == specifier
        ));
    }
}

#[test]
fn package_self_and_imports_target() {
    let fixture = super::fixture_root().join("package-map/resolution");
    let importer = fixture.join("apps/web/src");
    let resolver = package_map_resolver(&fixture.join("node_modules/.package-map.json"));

    assert_eq!(
        resolver.resolve(&importer, "@bench/web").map(|resolution| resolution.full_path()),
        Ok(fixture.join("apps/web/src/index.js"))
    );
    assert_eq!(
        resolver.resolve(&importer, "#react").map(|resolution| resolution.full_path()),
        Ok(fixture.join("node_modules/store/react/index.js"))
    );
}

#[test]
fn package_owner_errors() {
    let fixture = super::fixture_root().join("package-map/find-package-id");
    let package_map_path = fixture.join(".package-map.json");
    let resolver = package_map_resolver(&package_map_path);

    let mut context = ResolveContext::default();
    assert!(matches!(
        resolver.resolve_with_context(
            fixture.join("packages/duplicate"),
            "dependency",
            None,
            &mut context
        ),
        Err(ResolveError::PackageMapAmbiguousResolution { .. })
    ));
    assert!(context.file_dependencies.contains(&package_map_path));

    let mut context = ResolveContext::default();
    assert!(matches!(
        resolver.resolve_with_context(fixture.join("external"), "dependency", None, &mut context),
        Err(ResolveError::PackageMapExternalFile { .. })
    ));
    assert!(context.file_dependencies.contains(&package_map_path));
}

#[test]
fn package_map_load_errors_are_cached() {
    let fixture = super::fixture_root().join("package-map/invalid");
    let package_map_path = fixture.join(".package-map.json");
    let resolver = package_map_resolver(&package_map_path);

    for _ in 0..2 {
        let mut context = ResolveContext::default();
        assert!(matches!(
            resolver.resolve_with_context(&fixture, "dependency", None, &mut context),
            Err(ResolveError::Json(_))
        ));
        assert!(context.file_dependencies.contains(&package_map_path));
    }

    let missing_package_map_path = fixture.join("missing-package-map.json");
    let resolver = package_map_resolver(&missing_package_map_path);
    let mut context = ResolveContext::default();
    resolver.resolve_with_context(&fixture, "dependency", None, &mut context).unwrap_err();
    assert!(context.missing_dependencies.contains(&missing_package_map_path));
}

#[test]
fn package_map_without_symlink_resolution() {
    let fixture = super::fixture_root().join("package-map/resolution");
    let package_map_path = fixture.join("node_modules/.package-map.json");
    let resolver = Resolver::new(ResolveOptions {
        condition_names: vec!["node".into(), "require".into()],
        modules: vec![],
        package_map: Some(package_map_path),
        symlinks: false,
        ..ResolveOptions::default()
    });

    assert_eq!(
        resolver
            .resolve(fixture.join("apps/web/src"), "react")
            .map(|resolution| resolution.full_path()),
        Ok(fixture.join("node_modules/store/react/index.js"))
    );
    assert!(format!("{}", resolver.options()).contains("package_map:"));
}

#[test]
fn package_map_resolves_tsconfig_extends() {
    let fixture = super::fixture_root().join("package-map/resolution");
    let config_file = fixture.join("apps/web/tsconfig.package-map.json");
    let resolver = Resolver::new(ResolveOptions {
        condition_names: vec!["node".into(), "require".into()],
        modules: vec![],
        package_map: Some(fixture.join("node_modules/.package-map.json")),
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: config_file.clone(),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });

    resolver.resolve_tsconfig(config_file).expect("resolve tsconfig through map");
}

#[test]
fn disabled_by_default() {
    let fixture = super::fixture_root().join("package-map/resolution");
    let importer = fixture.join("apps/web/src");
    let resolver = Resolver::new(ResolveOptions { modules: vec![], ..ResolveOptions::default() });

    assert!(matches!(
        resolver.resolve(importer, "axios"),
        Err(ResolveError::NotFound(specifier)) if specifier == "axios"
    ));
}
