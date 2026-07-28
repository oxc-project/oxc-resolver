use std::path::Path;

use crate::{
    FileSystem, FileSystemOs, ResolveError, ResolveOptions, Resolver, package_map::PackageMap,
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

    let resolver = package_map_resolver(package_map_path);
    for specifier in specifiers {
        if let Err(error) = resolver.resolve(importer, specifier) {
            panic!(
                "failed to resolve {specifier:?} from {} using {}: {error}",
                importer.display(),
                package_map_path.display(),
            );
        }
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
fn invalid() {
    let fixture = super::fixture_root().join("invalid/package-map");
    for filename in ["invalid-dependencies.json", "missing-packages.json", "missing-url.json"] {
        let path = fixture.join(filename);
        let error = parse_package_map(&path).unwrap_err();
        assert_eq!(error.path, path);
    }
}

#[test]
fn package_self_precedes_package_map() {
    let fixture = super::fixture_root().join("package-map/order");
    let resolver = Resolver::new(ResolveOptions {
        package_map: Some(fixture.join(".package-map.json")),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve(fixture.join("self"), "self-package").unwrap();
    assert_eq!(resolution.path(), fixture.join("self/self.js"));
}

#[test]
fn package_map_does_not_fallback_to_node_modules() {
    let fixture = super::fixture_root().join("package-map/order");
    let importer = fixture.join("self");

    let resolution = Resolver::default().resolve(&importer, "fallback-package").unwrap();
    assert_eq!(resolution.path(), fixture.join("self/node_modules/fallback-package/index.js"));

    let resolver = Resolver::new(ResolveOptions {
        package_map: Some(fixture.join(".package-map.json")),
        ..ResolveOptions::default()
    });
    assert!(matches!(
        resolver.resolve(importer, "fallback-package"),
        Err(ResolveError::NotFound(specifier)) if specifier == "fallback-package"
    ));
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
fn pnpm_isolated() {
    let fixture = super::fixture_root().join("bench-pm/installs/pnpm-isolated");
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

    assert!(matches!(
        package_map_resolver(&package_map_path).resolve(&importer, "follow-redirects"),
        Err(ResolveError::NotFound(specifier)) if specifier == "follow-redirects"
    ));
}

#[test]
fn pnpm_hoisted_loose() {
    let fixture = super::fixture_root().join("bench-pm/installs/pnpm-hoisted");
    let importer = fixture.join("node_modules/axios/lib").canonicalize().unwrap();
    let specifiers = ["chalk"];
    test_package_map(
        &importer,
        &fixture.join("node_modules/.package-map.json"),
        &specifiers,
        |package_map| {
            let axios_id = package_map.package(".").unwrap().dependency("axios").unwrap();
            assert_dependencies(package_map, axios_id, &specifiers);
        },
    );
}

#[test]
fn yarn_isolated() {
    let fixture = super::fixture_root().join("bench-pm/installs/yarn-isolated");
    let specifiers = ["react", "axios", "@bench/ui"];
    test_package_map(
        &fixture.join("apps/web/src"),
        &fixture.join("node_modules/.package-map.json"),
        &specifiers,
        |package_map| assert_dependencies(package_map, "../apps/web", &specifiers),
    );
}
