//! Tests for `Resolution::package_json`.

use crate::Resolver;

#[test]
fn test() {
    let f = super::fixture_root().join("misc");

    let resolver = Resolver::default();

    let data = [
        (f.clone(), "package-json-nested"),
        // Nested package.json do not participate in module resolution.
        (f.clone(), "package-json-nested/foo"),
        (f.clone(), "package-json-nested/foo/bar"),
    ];

    let resolved_package_json_path = f.join("node_modules/package-json-nested/package.json");

    for (path, request) in data {
        let package_json =
            resolver.resolve(&path, request).ok().and_then(|f| f.package_json().cloned());
        let package_json_path = package_json.as_ref().map(|p| &p.path);
        let package_json_name = package_json.as_ref().and_then(|p| p.name.as_deref());
        assert_eq!(package_json_path, Some(&resolved_package_json_path), "{path:?} {request}");
        assert_eq!(package_json_name, Some("package-json-nested"), "{path:?} {request}");
    }
}

#[test]
fn adjacent_to_node_modules() {
    let f = super::fixture_root().join("misc");

    let resolver = Resolver::default();

    // populate cache
    let _ = resolver.resolve(&f, "package-json-nested");

    let path = f.join("dir-with-index");
    let request = "./index.js";
    let resolved_package_json_path = f.join("package.json");

    let package_json = resolver.resolve(&path, request).unwrap().package_json().cloned();
    let package_json_path = package_json.as_ref().map(|p| &p.path);
    let package_json_name = package_json.as_ref().and_then(|p| p.name.as_deref());
    assert_eq!(package_json_path, Some(&resolved_package_json_path));
    assert_eq!(package_json_name, Some("misc"));
}
