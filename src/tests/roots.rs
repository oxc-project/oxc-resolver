//! <https://github.com/webpack/enhanced-resolve/blob/main/test/roots.test.js>

use std::path::PathBuf;

use crate::{AliasValue, ResolveError, ResolveOptions, Resolver};

fn dirname() -> PathBuf {
    super::fixture_root().join("enhanced_resolve").join("test")
}

#[test]
fn roots() {
    let f = super::fixture();

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".js".into()],
        alias: vec![("foo".into(), vec![AliasValue::from("/fixtures")])],
        roots: vec![dirname(), f.clone()],
        ..ResolveOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        ("should respect roots option", "/fixtures/b.js", f.join("b.js")),
        ("should try another root option, if it exists", "/b.js", f.join("b.js")),
        ("should respect extension", "/fixtures/b", f.join("b.js")),
        ("should resolve in directory", "/fixtures/extensions/dir", f.join("extensions/dir/index.js")),
        ("should respect aliases", "foo/b", f.join("b.js")),
    ];

    for (comment, request, expected) in pass {
        let resolved_path = resolver.resolve(&f, request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {request}");
    }

    #[rustfmt::skip]
    let fail = [
        ("should not work with relative path", "fixtures/b.js", ResolveError::NotFound("fixtures/b.js".into()))
    ];

    for (comment, request, expected) in fail {
        let resolution = resolver.resolve(&f, request);
        assert_eq!(resolution, Err(expected), "{comment} {request}");
    }
}

#[test]
fn resolve_to_context() {
    let f = super::fixture();
    let resolver = Resolver::new(ResolveOptions {
        roots: vec![dirname(), f.clone()],
        resolve_to_context: true,
        ..ResolveOptions::default()
    });
    let resolved_path = resolver.resolve(&f, "/fixtures/lib").map(|r| r.full_path());
    let expected = f.join("lib");
    assert_eq!(resolved_path, Ok(expected));
}

#[test]
fn prefer_absolute() {
    let f = super::fixture();
    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".js".into()],
        alias: vec![("foo".into(), vec![AliasValue::from("/fixtures")])],
        roots: vec![dirname(), f.clone()],
        prefer_absolute: true,
        ..ResolveOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        ("should resolve an absolute path (prefer absolute)", f.join("b.js").to_string_lossy().to_string(), f.join("b.js")),
    ];

    for (comment, request, expected) in pass {
        let resolved_path = resolver.resolve(&f, &request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {request}");
    }
}

#[test]
fn roots_fall_through() {
    let f = super::fixture();
    let absolute_path = f.join("roots_fall_through/index.js");
    let specifier = absolute_path.to_string_lossy();
    let resolution = Resolver::new(ResolveOptions::default().with_root(&f)).resolve(&f, &specifier);
    assert_eq!(
        resolution.map(super::super::resolution::Resolution::into_path_buf),
        Ok(absolute_path)
    );
}

#[test]
fn should_resolve_slash() {
    let f = super::fixture();
    let dir_with_index = super::fixture_root().join("./misc/dir-with-index");

    #[rustfmt::skip]
    let pass = [
        ("should resolve if importer is root", vec![dir_with_index.clone()], &dir_with_index, dir_with_index.join("index.js")),
    ];

    for (comment, roots, directory, expected) in pass {
        let resolver =
            Resolver::new(ResolveOptions { roots: roots.clone(), ..ResolveOptions::default() });
        let resolved_path = resolver.resolve(directory, "/").map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {roots:?}");
    }

    #[rustfmt::skip]
    let fail = [
        ("should not resolve if not found", vec![f.clone()], &f),
        ("should not resolve if importer is not root", vec![dir_with_index], &f)
    ];

    for (comment, roots, directory) in fail {
        let resolver =
            Resolver::new(ResolveOptions { roots: roots.clone(), ..ResolveOptions::default() });
        let resolution = resolver.resolve(directory, "/");
        assert_eq!(resolution, Err(ResolveError::NotFound("/".into())), "{comment} {roots:?}");
    }
}
