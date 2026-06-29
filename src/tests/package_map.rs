//! Tests for Node.js package maps.
//!
//! <https://nodejs.org/docs/latest/api/packages.html#package-maps>

use std::path::PathBuf;

use crate::{ResolveError, ResolveOptions, Resolver};

fn dir() -> PathBuf {
    super::fixture_root().join("package-map")
}

fn resolver() -> Resolver {
    Resolver::new(ResolveOptions {
        package_map: Some(dir().join(".package-map.json")),
        ..ResolveOptions::default()
    })
}

#[test]
fn maps_bare_specifier_to_dependency() {
    let resolver = resolver();
    let app = dir().join("packages/app");

    // `@myorg/utils` is declared as a dependency of `app`, keyed to package `utils`.
    let resolution = resolver.resolve(&app, "@myorg/utils").map(|r| r.full_path());
    assert_eq!(resolution, Ok(dir().join("packages/utils/index.js")));

    // `@myorg/ui-lib` -> package `ui-lib`.
    let resolution = resolver.resolve(&app, "@myorg/ui-lib").map(|r| r.full_path());
    assert_eq!(resolution, Ok(dir().join("packages/ui-lib/index.js")));
}

#[test]
fn resolves_subpath_through_exports() {
    let resolver = resolver();
    let app = dir().join("packages/app");

    // `@myorg/utils/helper` -> package `utils`, resolved via its `exports` field.
    let resolution = resolver.resolve(&app, "@myorg/utils/helper").map(|r| r.full_path());
    assert_eq!(resolution, Ok(dir().join("packages/utils/helper.js")));
}

#[test]
fn maps_from_a_nested_importer_directory() {
    let resolver = resolver();
    // Importer lives in a subdirectory of the `app` package.
    let nested = dir().join("packages/app/src");

    let resolution = resolver.resolve(&nested, "@myorg/utils").map(|r| r.full_path());
    assert_eq!(resolution, Ok(dir().join("packages/utils/index.js")));
}

#[test]
fn dependency_isolation() {
    let resolver = resolver();
    // `utils` declares no dependencies, so it cannot see `@myorg/ui-lib`.
    let utils = dir().join("packages/utils");
    let resolution = resolver.resolve(&utils, "@myorg/ui-lib");
    assert_eq!(resolution, Err(ResolveError::NotFound("@myorg/ui-lib".into())));

    // `app` does not declare `@myorg/missing`.
    let app = dir().join("packages/app");
    let resolution = resolver.resolve(&app, "@myorg/missing");
    assert_eq!(resolution, Err(ResolveError::NotFound("@myorg/missing".into())));
}

#[test]
fn external_importer_file_errors() {
    let resolver = resolver();
    // The package-map root is not within any mapped package location.
    let resolution = resolver.resolve(dir(), "@myorg/utils");
    assert_eq!(resolution, Err(ResolveError::PackageMapExternalFile(dir())));
}

#[test]
fn relative_specifiers_bypass_the_package_map() {
    let resolver = resolver();
    let app = dir().join("packages/app");
    // Relative specifiers are resolved normally, even from an importer outside any package.
    let resolution = resolver.resolve(&app, "./src/feature.js").map(|r| r.full_path());
    assert_eq!(resolution, Ok(dir().join("packages/app/src/feature.js")));

    // Even from an external directory, relative resolution is unaffected.
    let resolution = resolver.resolve(dir(), "./packages/app/index.js").map(|r| r.full_path());
    assert_eq!(resolution, Ok(dir().join("packages/app/index.js")));
}

#[test]
fn builtin_modules_are_exempt() {
    let resolver = Resolver::new(ResolveOptions {
        package_map: Some(dir().join(".package-map.json")),
        builtin_modules: true,
        ..ResolveOptions::default()
    });
    let app = dir().join("packages/app");
    let resolution = resolver.resolve(&app, "fs");
    assert!(
        matches!(resolution, Err(ResolveError::Builtin { .. })),
        "expected builtin, got {resolution:?}"
    );
}

#[test]
fn multiple_versions_are_isolated() {
    let resolver = Resolver::new(ResolveOptions {
        package_map: Some(dir().join("multi-version.package-map.json")),
        ..ResolveOptions::default()
    });

    // `app` depends on component v2.
    let app = dir().join("mv/app");
    let resolution = resolver.resolve(&app, "component").map(|r| r.full_path());
    assert_eq!(resolution, Ok(dir().join("mv/vendor/component-2.0.0/index.js")));

    // `legacy` depends on component v1.
    let legacy = dir().join("mv/legacy");
    let resolution = resolver.resolve(&legacy, "component").map(|r| r.full_path());
    assert_eq!(resolution, Ok(dir().join("mv/vendor/component-1.0.0/index.js")));
}
