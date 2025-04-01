//! <https://github.com/webpack/enhanced-resolve/blob/main/test/restrictions.test.js>

use std::sync::Arc;

use regex::Regex;

use crate::{ResolveError, ResolveOptions, Resolver, Restriction};

#[test]
fn should_respect_regexp_restriction() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(sass|scss|css)$").unwrap();
    let resolver1 = Resolver::new(ResolveOptions {
        extensions: vec![".js".into()],
        restrictions: vec![Restriction::Fn(Arc::new(move |path| {
            path.as_os_str().to_str().is_some_and(|s| re.is_match(s))
        }))],
        ..ResolveOptions::default()
    });

    let resolution = resolver1.resolve(&f, "pck1").map(|r| r.full_path());
    assert_eq!(resolution, Err(ResolveError::NotFound("pck1".to_string())));
}

#[test]
fn should_try_to_find_alternative_1() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(sass|scss|css)$").unwrap();
    let resolver1 = Resolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".css".into()],
        main_files: vec!["index".into()],
        restrictions: vec![Restriction::Fn(Arc::new(move |path| {
            path.as_os_str().to_str().is_some_and(|s| re.is_match(s))
        }))],
        ..ResolveOptions::default()
    });

    let resolution = resolver1.resolve(&f, "pck1").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck1/index.css")));
}

#[test]
fn should_respect_string_restriction() {
    let fixture = super::fixture();
    let f = fixture.join("restrictions");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".js".into()],
        restrictions: vec![Restriction::Path(f.clone())],
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve(&f, "pck2");
    assert_eq!(resolution, Err(ResolveError::NotFound("pck2".to_string())));
}

#[test]
fn should_try_to_find_alternative_2() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(sass|scss|css)$").unwrap();
    let resolver1 = Resolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".css".into()],
        main_fields: vec!["main".into(), "style".into()],
        restrictions: vec![Restriction::Fn(Arc::new(move |path| {
            path.as_os_str().to_str().is_some_and(|s| re.is_match(s))
        }))],
        ..ResolveOptions::default()
    });

    let resolution = resolver1.resolve(&f, "pck2").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck2/index.css")));
}

#[test]
fn should_try_to_find_alternative_3() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(sass|scss|css)$").unwrap();
    let resolver1 = Resolver::new(ResolveOptions {
        extensions: vec![".js".into()],
        main_fields: vec!["main".into(), "module".into(), "style".into()],
        restrictions: vec![Restriction::Fn(Arc::new(move |path| {
            path.as_os_str().to_str().is_some_and(|s| re.is_match(s))
        }))],
        ..ResolveOptions::default()
    });

    let resolution = resolver1.resolve(&f, "pck2").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck2/index.css")));
}
