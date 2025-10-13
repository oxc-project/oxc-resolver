use crate::{ResolveError, ResolveOptions, Resolver};

#[test]
fn declaration_only_resolves_d_ts() {
    let f = super::fixture_root().join("declaration_only");

    let resolver =
        Resolver::new(ResolveOptions { declaration_only: true, ..ResolveOptions::default() });

    #[rustfmt::skip]
    let pass = [
        ("should resolve .d.ts file when both .ts and .d.ts exist", "./src/foo", "src/foo.d.ts"),
        ("should resolve .d.ts file when only .d.ts exists", "./src/bar", "src/bar.d.ts"),
    ];

    for (comment, request, expected_path) in pass {
        let resolved_path = resolver.resolve(&f, request).map(|r| r.full_path());
        let expected = f.join(expected_path);
        assert_eq!(resolved_path, Ok(expected), "{comment} {request} {expected_path}");
    }
}

#[test]
fn declaration_only_ignores_ts_files() {
    let f = super::fixture_root().join("declaration_only");

    let resolver =
        Resolver::new(ResolveOptions { declaration_only: true, ..ResolveOptions::default() });

    let resolution = resolver.resolve(&f, "./src/foo");
    assert!(resolution.is_ok());
    let path = resolution.unwrap().into_path_buf();
    assert!(path.to_string_lossy().ends_with(".d.ts"));
    assert!(!path.to_string_lossy().ends_with("foo.ts"));
}

#[test]
fn declaration_only_ignores_js_files() {
    let f = super::fixture_root().join("declaration_only");

    let resolver =
        Resolver::new(ResolveOptions { declaration_only: true, ..ResolveOptions::default() });

    let resolution = resolver.resolve(&f, "./index");
    assert!(matches!(resolution, Err(ResolveError::NotFound(_))));
}

#[test]
#[cfg(feature = "typescript")]
fn declaration_only_respects_types_field() {
    let f = super::fixture_root().join("declaration_only");

    let resolver =
        Resolver::new(ResolveOptions { declaration_only: true, ..ResolveOptions::default() });

    let resolved_path = resolver.resolve(&f, ".").map(|r| r.full_path());
    let expected = f.join("types/index.d.ts");
    assert_eq!(resolved_path, Ok(expected), "should resolve package types field");
}

#[test]
fn declaration_only_fails_when_no_d_ts() {
    let f = super::fixture().join("extensions");

    let resolver =
        Resolver::new(ResolveOptions { declaration_only: true, ..ResolveOptions::default() });

    let resolution = resolver.resolve(&f, "./foo");
    assert!(matches!(resolution, Err(ResolveError::NotFound(_))));
}

#[test]
fn without_declaration_only_resolves_ts_files() {
    let f = super::fixture_root().join("declaration_only");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".ts".into(), ".d.ts".into(), ".js".into()],
        declaration_only: false,
        ..ResolveOptions::default()
    });

    let resolved_path = resolver.resolve(&f, "./src/foo").map(|r| r.full_path());
    let expected = f.join("src/foo.ts");
    assert_eq!(
        resolved_path,
        Ok(expected),
        "should resolve .ts file when declaration_only is false"
    );
}
