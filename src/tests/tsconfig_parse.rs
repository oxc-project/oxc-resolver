//! Tests for reading tsconfig files: comments and trailing commas, a byte order mark,
//! broken JSON and an empty file.

use super::manual_tsconfig;
use crate::{JSONError, ResolveError, ResolveOptions, Resolver, TsconfigReferences};

#[test]
fn json_with_comments() {
    let f = super::fixture_root().join("tsconfig/cases/trailing-comma");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Auto)),
        ..ResolveOptions::default()
    });

    let resolved_path = resolver.resolve_file(&f, "foo").map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f.join("bar.js")));
}

#[test]
fn with_bom() {
    let f = super::fixture_root().join("tsconfig/cases/with-bom");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Auto)),
        ..ResolveOptions::default()
    });

    let resolved_path = resolver.resolve_file(&f, "foo").map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f.join("bar.js")));
}

#[test]
fn broken() {
    let f = super::fixture_root().join("tsconfig");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig_broken.json"), TsconfigReferences::Auto)),
        ..ResolveOptions::default()
    });

    let resolved_path = resolver.resolve_file(&f, "/");
    let error = ResolveError::TsconfigLoadFailed {
        path: f.join("tsconfig_broken.json"),
        source: Box::new(ResolveError::Json(JSONError {
            path: f.join("tsconfig_broken.json"),
            message: String::from("EOF while parsing an object at line 2 column 0"),
            line: 2,
            column: 0,
        })),
    };
    assert_eq!(resolved_path, Err(error));
}

#[test]
fn empty() {
    let f = super::fixture_root().join("tsconfig/cases/empty");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Auto)),
        ..ResolveOptions::default()
    });

    let resolved_path = resolver.resolve_file(f.join("index.js"), "./index").map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f.join("index.js")));
}
