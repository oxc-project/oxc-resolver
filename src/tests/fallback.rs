//! https://github.com/webpack/enhanced-resolve/blob/main/test/fallback.test.js

#[test]
#[cfg(not(target_os = "windows"))] // MemoryFS's path separator is always `/` so the test will not pass in windows.
fn fallback() {
    use std::path::{Path, PathBuf};

    use super::memory_fs::MemoryFS;
    use crate::{AliasValue, ResolveError, ResolveOptions, ResolverGeneric};

    let f = Path::new("/");

    let file_system = MemoryFS::new(&[
        ("/a/index.js", ""),
        ("/a/dir/index.js", ""),
        ("/recursive/index.js", ""),
        ("/recursive/dir/index.js", ""),
        ("/recursive/dir/file", ""),
        ("/recursive/dir/dir/index.js", ""),
        ("/b/index.js", ""),
        ("/b/dir/index.js", ""),
        ("/c/index.js", ""),
        ("/c/dir/index.js", ""),
        ("/d/index.js.js", ""),
        ("/d/dir/.empty", ""),
        ("/e/index.js", ""),
        ("/e/anotherDir/index.js", ""),
        ("/e/dir/file", ""),
    ]);

    let resolver = ResolverGeneric::new_with_file_system(
        file_system,
        ResolveOptions {
            fallback: vec![
                ("aliasA".into(), vec![AliasValue::Path("a".into())]),
                ("b$".into(), vec![AliasValue::Path("a/index.js".into())]),
                ("c$".into(), vec![AliasValue::Path("/a/index.js".into())]),
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
                ("/d/dir".into(), vec![AliasValue::Path("/c/dir".into())]),
                ("/d/index.js.js".into(), vec![AliasValue::Path("/c/index.js".into())]),
                ("ignored".into(), vec![AliasValue::Ignore]),
                ("node:path".into(), vec![AliasValue::Ignore]),
            ],
            modules: vec!["/".into()],
            ..ResolveOptions::default()
        },
    );

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
        let resolved_path = resolver.resolve(f, request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(PathBuf::from(expected)), "{comment} {request}");
    }

    #[rustfmt::skip]
    let ignore = [
        ("should resolve an ignore module", "ignored", ResolveError::Ignored(f.join("ignored"))),
        ("should resolve node: builtin module", "node:path", ResolveError::Ignored(PathBuf::from("/node:path"))),
    ];

    for (comment, request, expected) in ignore {
        let resolution = resolver.resolve(f, request);
        assert_eq!(resolution, Err(expected), "{comment} {request}");
    }
}
