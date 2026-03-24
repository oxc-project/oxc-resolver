//! <https://github.com/webpack/enhanced-resolve/blob/main/test/scoped-packages.test.js>

use crate::{Resolution, ResolveError, ResolveOptions, Resolver};

#[test]
fn scoped_packages() {
    let f = super::fixture().join("scoped");

    let resolver = Resolver::new(ResolveOptions {
        alias_fields: vec![vec!["browser".into()]],
        ..ResolveOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        ("main field should work", f.clone(), "@scope/pack1", "@scope/pack1", f.join("./node_modules/@scope/pack1/main.js")),
        ("browser field should work", f.clone(), "@scope/pack2", "@scope/pack2", f.join("./node_modules/@scope/pack2/main.js")),
        ("folder request should work", f.clone(), "@scope/pack2/lib", "@scope/pack2", f.join("./node_modules/@scope/pack2/lib/index.js"))
    ];

    for (comment, path, request, package, expected) in pass {
        let resolution = resolver.resolve(&path, request).ok();
        let resolved_path = resolution.as_ref().map(Resolution::full_path);
        let resolved_package_json =
            resolution.as_ref().and_then(|r| r.package_json()).map(|p| p.path.clone());
        assert_eq!(resolved_path, Some(expected), "{comment} {path:?} {request}");
        let package_json_path = f.join("node_modules").join(package).join("package.json");
        assert_eq!(resolved_package_json, Some(package_json_path), "{path:?} {request}");
    }
}

#[test]
fn scoped_packages_with_exports() {
    let f = super::fixture().join("scoped");

    let resolver = Resolver::new(ResolveOptions {
        condition_names: vec!["import".into()],
        ..ResolveOptions::default()
    });

    let result = resolver.resolve(&f, "@scope/pack3").map(|r| r.full_path());
    assert_eq!(result, Ok(f.join("node_modules/@scope/pack3/esm/index.js")));
}

#[test]
fn scoped_packages_subpath_export() {
    let f = super::fixture().join("scoped");

    let resolver = Resolver::new(ResolveOptions {
        condition_names: vec!["import".into()],
        ..ResolveOptions::default()
    });

    let result = resolver.resolve(&f, "@scope/pack3/utils").map(|r| r.full_path());
    assert_eq!(result, Ok(f.join("node_modules/@scope/pack3/utils/index.js")));
}

#[test]
fn scoped_packages_not_found() {
    let f = super::fixture().join("scoped");
    let resolver = Resolver::default();
    let result = resolver.resolve(&f, "@scope/nonexistent");
    assert_eq!(result, Err(ResolveError::NotFound("@scope/nonexistent".into())));
}
