use std::path::Path;

use crate::{
    FileSystem, FileSystemOs, ResolveContext, ResolveError, ResolveOptions, Resolver,
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
    let realpath = fs.canonicalize(package_map_path).unwrap();
    PackageMap::parse(package_map_path.to_path_buf(), realpath, fs.read(package_map_path).unwrap())
}

fn test_package_map(
    importer: &Path,
    package_map_path: &Path,
    specifiers: &[&str],
    assert_package_map: impl FnOnce(&PackageMap),
) {
    let package_map = parse_package_map(package_map_path).unwrap();
    assert_eq!(package_map.path(), package_map_path);
    assert_eq!(package_map.realpath(), package_map_path.canonicalize().unwrap());
    assert_package_map(&package_map);

    let resolver = package_map_resolver();
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

fn package_map_resolver() -> Resolver {
    Resolver::new(ResolveOptions {
        condition_names: vec!["node".into(), "require".into()],
        modules: vec![],
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
fn find_package_id_uses_logical_package_map_path() {
    let fixture = super::fixture_root().join("package-map/find-package-id");
    let package_map_path = fixture.join(".package-map.json");
    let fs = file_system();
    let package_map = PackageMap::parse(
        package_map_path.clone(),
        fixture.join("canonical-location/.package-map.json"),
        fs.read(&package_map_path).unwrap(),
    )
    .unwrap();

    assert_eq!(package_map.find_package_id(&fixture.join("packages/root/index.js")), Ok("root"));
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

    let resolver = package_map_resolver();
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
