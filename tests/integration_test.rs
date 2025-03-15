//! Test public APIs

use std::{env, path::PathBuf};

use unrspack_resolver::{EnforceExtension, Resolution, ResolveContext, ResolveOptions, Resolver};

fn dir() -> PathBuf {
    env::current_dir().unwrap()
}

fn resolve(specifier: &str) -> Resolution {
    let path = dir();
    Resolver::new(ResolveOptions::default()).resolve(path, specifier).unwrap()
}

#[test]
fn clone() {
    let resolution = resolve("./tests/package.json");
    assert_eq!(resolution.clone(), resolution);
}

#[test]
fn debug() {
    let resolution = resolve("./tests/package.json");
    let s = format!("{resolution:?}");
    assert!(!s.is_empty());
}

#[test]
fn eq() {
    let resolution = resolve("./tests/package.json");
    assert_eq!(resolution, resolution);
}

#[test]
fn package_json() {
    let resolution = resolve("./tests/package.json");
    let package_json = resolution.package_json().unwrap();
    assert_eq!(package_json.name.as_ref().unwrap(), "name");
    assert_eq!(package_json.r#type.as_ref().unwrap().as_str(), "module".into());
    assert!(package_json.side_effects.as_ref().unwrap().is_object());
}

#[cfg(feature = "package_json_raw_json_api")]
#[test]
fn package_json_raw_json_api() {
    let resolution = resolve("./tests/package.json");
    assert!(resolution
        .package_json()
        .unwrap()
        .raw_json()
        .get("name")
        .is_some_and(|name| name == "name"));
}

#[test]
fn clear_cache() {
    let resolver = Resolver::new(ResolveOptions::default());
    resolver.clear_cache(); // exists
}

#[test]
fn options() {
    let resolver = Resolver::new(ResolveOptions::default());
    let options = resolver.options();
    assert!(!format!("{options:?}").is_empty());
}

#[test]
fn debug_resolver() {
    let resolver = Resolver::new(ResolveOptions::default());
    assert!(!format!("{resolver:?}").is_empty());
}

#[test]
fn dependencies() {
    let path = dir();
    let mut ctx = ResolveContext::default();
    let _ = Resolver::new(ResolveOptions::default()).resolve_with_context(
        path,
        "./tests/package.json",
        &mut ctx,
    );
    assert!(!ctx.file_dependencies.is_empty());
    assert!(ctx.missing_dependencies.is_empty());
}

#[test]
fn options_api() {
    _ = ResolveOptions::default()
        .with_builtin_modules(true)
        .with_condition_names(&[])
        .with_extension(".js")
        .with_force_extension(EnforceExtension::Auto)
        .with_fully_specified(true)
        .with_main_field("asdf")
        .with_main_file("main")
        .with_module("module")
        .with_prefer_absolute(true)
        .with_prefer_relative(true)
        .with_root(PathBuf::new())
        .with_symbolic_link(true);
}
