//! https://github.com/webpack/enhanced-resolve/blob/main/test/fullSpecified.test.js

use std::path::{Path, PathBuf};

use crate::{AliasValue, ResolveOptions, Resolver};

/// Stands in for the filesystem root: the tables below spell paths relative to it,
/// with a leading `/`.
fn root() -> PathBuf {
    super::fixture_root().join("unit").join("full_specified")
}

/// The request as passed to the resolver, with absolute requests mapped onto disk.
fn on_disk_request(f: &Path, request: &str) -> String {
    if request.starts_with('/') {
        super::on_disk(f, request).to_string_lossy().into_owned()
    } else {
        request.to_string()
    }
}

fn options(f: &Path) -> ResolveOptions {
    let abs = |path: &str| super::on_disk(f, path).to_string_lossy().into_owned();
    ResolveOptions {
        alias: vec![
            ("alias1".into(), vec![AliasValue::from(abs("/a/abc"))]),
            ("alias2".into(), vec![AliasValue::from(abs("/a"))]),
        ],
        alias_fields: vec![vec!["browser".into()]],
        fully_specified: true,
        ..ResolveOptions::default()
    }
}

#[test]
fn test() {
    let f = root();
    let resolver = Resolver::new(options(&f));

    let failing_resolves = [
        ("no extensions", "./abc"),
        ("no extensions (absolute)", "/a/abc"),
        ("no extensions in packages", "package1/file"),
        ("no directories", "."),
        ("no directories 2", "./"),
        ("no directories in packages", "package3/dir"),
        ("no extensions in packages 2", "package3/a"),
    ];

    for (comment, request) in failing_resolves {
        let resolution = resolver.resolve(f.join("a"), &on_disk_request(&f, request));
        assert!(resolution.is_err(), "{comment} {request}");
    }

    let successful_resolves = [
        ("fully relative", "./abc.js", "/a/abc.js"),
        ("fully absolute", "/a/abc.js", "/a/abc.js"),
        ("fully relative in package", "package1/file.js", "/a/node_modules/package1/file.js"),
        ("extensions in mainFiles", "package1", "/a/node_modules/package1/index.js"),
        ("extensions in mainFields", "package2", "/a/node_modules/package2/a.js"),
        ("extensions in alias", "alias1", "/a/abc.js"),
        ("directories in alias", "alias2", "/a/index.js"),
        ("directories in packages", "package3", "/a/node_modules/package3/dir/index.js"),
        ("extensions in aliasFields", "package4/a.js", "/a/node_modules/package4/b.js"),
    ];

    for (comment, request, expected) in successful_resolves {
        let resolution =
            resolver.resolve(f.join("a"), &on_disk_request(&f, request)).map(|r| r.full_path());
        assert_eq!(resolution, Ok(super::on_disk(&f, expected)), "{comment} {request}");
    }
}

#[test]
fn resolve_to_context() {
    let f = root();
    let resolver = Resolver::new(ResolveOptions { resolve_to_context: true, ..options(&f) });

    let successful_resolves = [
        ("current folder", ".", "/a"),
        ("current folder 2", "./", "/a"),
        ("relative directory", "./dir", "/a/dir"),
        ("relative directory 2", "./dir/", "/a/dir"),
        ("relative directory with query and fragment", "./dir?123#456", "/a/dir?123#456"),
        ("relative directory with query and fragment 2", "./dir/?123#456", "/a/dir?123#456"),
        ("absolute directory", "/a/dir", "/a/dir"),
        ("directory in package", "package3/dir", "/a/node_modules/package3/dir"),
    ];

    for (comment, request, expected) in successful_resolves {
        let resolution =
            resolver.resolve(f.join("a"), &on_disk_request(&f, request)).map(|r| r.full_path());
        assert_eq!(resolution, Ok(super::on_disk(&f, expected)), "{comment} {request}");
    }
}
