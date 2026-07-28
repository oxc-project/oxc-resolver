use std::path::Path;

use crate::{ResolveOptions, Resolver};

fn test_package_map(importer: &Path, package_map: &Path, specifiers: &[&str]) {
    assert!(package_map.is_file(), "package map does not exist: {}", package_map.display());

    let resolver = Resolver::new(ResolveOptions { modules: vec![], ..ResolveOptions::default() });
    for specifier in specifiers {
        if let Err(error) = resolver.resolve(importer, specifier) {
            panic!(
                "failed to resolve {specifier:?} from {} using {}: {error}",
                importer.display(),
                package_map.display(),
            );
        }
    }
}

#[test]
fn pnpm() {
    let fixtures = super::fixture_root();
    test_package_map(
        &fixtures.join("pnpm"),
        &fixtures.parent().unwrap().join("node_modules/.package-map.json"),
        &["axios", "decimal.js", "postcss"],
    );
}

#[test]
fn yarn() {
    let fixture = super::fixture_root().join("yarn");
    test_package_map(&fixture, &fixture.join("node_modules/.package-map.json"), &["typescript"]);
}

#[test]
fn pnpm_isolated() {
    let fixture = super::fixture_root().join("bench-pm/installs/pnpm-isolated");
    test_package_map(
        &fixture.join("apps/web/src"),
        &fixture.join("node_modules/.package-map.json"),
        &["react", "axios", "@bench/ui"],
    );
}

#[test]
fn yarn_isolated() {
    let fixture = super::fixture_root().join("bench-pm/installs/yarn-isolated");
    test_package_map(
        &fixture.join("apps/web/src"),
        &fixture.join("node_modules/.package-map.json"),
        &["react", "axios", "@bench/ui"],
    );
}
