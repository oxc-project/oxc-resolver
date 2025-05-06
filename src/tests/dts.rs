use crate::Resolver;

#[test]
fn package() {
    let f = super::fixture_root().join("dts");

    let resolver = Resolver::default();

    #[rustfmt::skip]
    let data = [
        ("foo", f.join("node_modules/@types/foo/index.d.ts")),
        ("bar", f.join("node_modules/@types/bar/index.d.mts")),
    ];

    for (request, expected) in data {
        let resolution = resolver.resolve_package_dts(&f, request).map(|r| r.path);
        assert_eq!(resolution, Ok(expected), "{request}");
    }
}
