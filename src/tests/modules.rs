//! Tests for ResolveOptions.modules (custom module directories)

use std::path::PathBuf;

use crate::{ResolveError, ResolveOptions, Resolver};

/// The fixture directory standing in for the filesystem root of one case.
fn root(case: &str) -> PathBuf {
    super::fixture_root().join("unit").join("modules").join(case)
}

#[test]
fn custom_module_directory_name() {
    let f = root("custom-name");
    let resolver =
        Resolver::new(ResolveOptions { modules: vec!["libs".into()], ..ResolveOptions::default() });
    let result = resolver.resolve(f.join("project").join("src"), "my-lib").map(|r| r.full_path());
    assert_eq!(result, Ok(super::on_disk(&f, "/project/libs/my-lib/index.js")));
}

#[test]
fn absolute_module_directory() {
    let f = root("absolute");
    let modules = super::on_disk(&f, "/shared/modules").to_string_lossy().into_owned();
    let resolver =
        Resolver::new(ResolveOptions { modules: vec![modules], ..ResolveOptions::default() });
    let result = resolver.resolve(f.join("project").join("src"), "pkg").map(|r| r.full_path());
    assert_eq!(result, Ok(super::on_disk(&f, "/shared/modules/pkg/index.js")));
}

#[test]
fn multiple_module_directories_priority() {
    let f = root("priority");
    let resolver = Resolver::new(ResolveOptions {
        modules: vec!["custom_modules".into(), "node_modules".into()],
        ..ResolveOptions::default()
    });
    let result = resolver.resolve(f.join("project"), "pkg").map(|r| r.full_path());
    assert_eq!(result, Ok(super::on_disk(&f, "/project/custom_modules/pkg/index.js")));
}

#[test]
fn module_directory_fallback() {
    let f = root("fallback");
    let resolver = Resolver::new(ResolveOptions {
        modules: vec!["custom_modules".into(), "node_modules".into()],
        ..ResolveOptions::default()
    });
    let result = resolver.resolve(f.join("project"), "pkg").map(|r| r.full_path());
    assert_eq!(result, Ok(super::on_disk(&f, "/project/node_modules/pkg/index.js")));
}

#[test]
fn empty_modules_list() {
    let f = root("fallback");
    let resolver = Resolver::new(ResolveOptions { modules: vec![], ..ResolveOptions::default() });
    let result = resolver.resolve(f.join("project"), "pkg");
    assert_eq!(result, Err(ResolveError::NotFound("pkg".into())));
}
