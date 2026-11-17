//! https://github.com/webpack/enhanced-resolve/blob/main/test/fallback.test.js

#[test]
fn fallback() {
    use crate::{AliasValue, ResolveError, ResolveOptions, Resolver};

    // Paths below are written relative to this directory, with a leading `/`.
    let f = super::fixture_root().join("unit").join("fallback");
    let abs = |path: &str| super::on_disk(&f, path).to_string_lossy().into_owned();

    let resolver = Resolver::new(ResolveOptions {
        fallback: vec![
            ("aliasA".into(), vec![AliasValue::Path("a".into())]),
            ("b$".into(), vec![AliasValue::Path("a/index.js".into())]),
            ("c$".into(), vec![AliasValue::Path(abs("/a/index.js"))]),
            (
                "multiAlias".into(),
                vec![
                    AliasValue::Path("b".into()),
                    AliasValue::Path("c".into()),
                    AliasValue::Path("d".into()),
                    AliasValue::Path("e".into()),
                    AliasValue::Path("a".into()),
                ],
            ),
            ("recursive".into(), vec![AliasValue::Path("recursive/dir".into())]),
            (abs("/d/dir"), vec![AliasValue::Path(abs("/c/dir"))]),
            (abs("/d/index.js.js"), vec![AliasValue::Path(abs("/c/index.js"))]),
            ("ignored".into(), vec![AliasValue::Ignore]),
            ("node:path".into(), vec![AliasValue::Ignore]),
        ],
        modules: vec![abs("/")],
        ..ResolveOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        ("should resolve a not aliased module 1", "a", "/a/index.js"),
        ("should resolve a not aliased module 2", "a/index.js", "/a/index.js"),
        ("should resolve a not aliased module 3", "a/dir", "/a/dir/index.js"),
        ("should resolve a not aliased module 4", "a/dir/index.js", "/a/dir/index.js"),
        ("should resolve an fallback module 1", "aliasA", "/a/index.js"),
        ("should resolve an fallback module 2", "aliasA/index.js", "/a/index.js"),
        ("should resolve an fallback module 3", "aliasA/dir", "/a/dir/index.js"),
        ("should resolve an fallback module 4", "aliasA/dir/index.js", "/a/dir/index.js"),
        ("should resolve a recursive aliased module 1", "recursive", "/recursive/index.js"),
        ("should resolve a recursive aliased module 2", "recursive/index.js", "/recursive/index.js"),
        ("should resolve a recursive aliased module 3", "recursive/dir", "/recursive/dir/index.js"),
        ("should resolve a recursive aliased module 4", "recursive/dir/index.js", "/recursive/dir/index.js"),
        ("should resolve a recursive aliased module 5", "recursive/file", "/recursive/dir/file"),
        ("should resolve a file aliased module with a query 1", "b?query", "/b/index.js?query"),
        ("should resolve a file aliased module with a query 2", "c?query", "/c/index.js?query"),
        ("should resolve a path in a file aliased module 1", "b/index.js", "/b/index.js"),
        ("should resolve a path in a file aliased module 2", "b/dir", "/b/dir/index.js"),
        ("should resolve a path in a file aliased module 3", "b/dir/index.js", "/b/dir/index.js"),
        ("should resolve a path in a file aliased module 4", "c/index.js", "/c/index.js"),
        ("should resolve a path in a file aliased module 5", "c/dir", "/c/dir/index.js"),
        ("should resolve a path in a file aliased module 6", "c/dir/index.js", "/c/dir/index.js"),
        ("should resolve a file in multiple aliased dirs 1", "multiAlias/dir/file", "/e/dir/file"),
        ("should resolve a file in multiple aliased dirs 2", "multiAlias/anotherDir", "/e/anotherDir/index.js"),
    ];

    for (comment, request, expected) in pass {
        let resolved_path = resolver.resolve(&f, request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(super::on_disk(&f, expected)), "{comment} {request}");
    }

    #[rustfmt::skip]
    let ignore = [
        ("should resolve an ignore module", "ignored", ResolveError::Ignored(f.join("ignored"))),
        ("should resolve node: builtin module", "node:path", ResolveError::Ignored(f.join("node:path"))),
    ];

    for (comment, request, expected) in ignore {
        let resolution = resolver.resolve(&f, request);
        assert_eq!(resolution, Err(expected), "{comment} {request}");
    }
}
