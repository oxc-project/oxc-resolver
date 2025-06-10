//! <https://github.com/webpack/enhanced-resolve/blob/main/test/scoped-packages.test.js>

use crate::{Resolution, ResolveOptions, Resolver};

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
