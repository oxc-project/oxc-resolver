//! Test public APIs

use std::{env, path::PathBuf};

use oxc_resolver::{
    AliasValue, EnforceExtension, Resolution, ResolveContext, ResolveError, ResolveOptions,
    Resolver,
};

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
    assert_eq!(package_json.name().unwrap(), "name");
    assert_eq!(package_json.r#type().unwrap().to_string(), "module".to_string());
    assert_eq!(package_json.side_effects(), None);
}

#[test]
fn tsconfig() {
    let resolver = Resolver::new(ResolveOptions::default());
    let tsconfig = resolver.resolve_tsconfig("./tests").unwrap();
    assert!(tsconfig.root);
    assert_eq!(tsconfig.path, PathBuf::from("./tests/tsconfig.json"));
}

#[test]
fn tsconfig_extends_self_reference() {
    let resolver = Resolver::new(ResolveOptions::default());
    let err = resolver.resolve_tsconfig("./tests/tsconfig_self_reference.json").unwrap_err();
    assert_eq!(
        err,
        ResolveError::TsconfigCircularExtend(
            vec![
                "./tests/tsconfig_self_reference.json".into(),
                "./tests/tsconfig_self_reference.json".into()
            ]
            .into()
        )
    );
}

#[test]
fn tsconfig_extends_circular_reference() {
    let resolver = Resolver::new(ResolveOptions::default());
    let err = resolver.resolve_tsconfig("./tests/tsconfig_circular_reference_a.json").unwrap_err();
    assert_eq!(
        err,
        ResolveError::TsconfigCircularExtend(
            vec![
                "./tests/tsconfig_circular_reference_a.json".into(),
                "./tests/tsconfig_circular_reference_b.json".into(),
                "./tests/tsconfig_circular_reference_a.json".into(),
            ]
            .into()
        )
    );
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
        None,
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

#[test]
fn clone_with_options_recompiles_alias() {
    let fixture = dir().join("fixtures/enhanced-resolve/test/fixtures");

    let base_resolver = Resolver::new(ResolveOptions {
        alias: vec![("alias-target".into(), vec![AliasValue::from("./a")])],
        ..ResolveOptions::default()
    });

    let cloned_resolver = base_resolver.clone_with_options(ResolveOptions {
        alias: vec![("alias-target".into(), vec![AliasValue::from("./b")])],
        ..ResolveOptions::default()
    });

    let base = base_resolver.resolve(&fixture, "alias-target").unwrap().into_path_buf();
    let cloned = cloned_resolver.resolve(&fixture, "alias-target").unwrap().into_path_buf();

    assert_eq!(base, fixture.join("a.js"));
    assert_eq!(cloned, fixture.join("b.js"));
}

#[test]
fn node_path_fallback() {
    let fixture = dir().join("fixtures/enhanced-resolve/test/fixtures");
    let project = fixture.clone();
    let node_path_root = fixture.join("multiple-modules/node_modules");
    let expected = fixture.join("node_modules/m1/a.js");
    let node_path = env::join_paths([node_path_root]).unwrap();

    let previous_node_path = env::var_os("NODE_PATH");
    // SAFETY: This test sets NODE_PATH for a local resolution call and restores it right after.
    unsafe { env::set_var("NODE_PATH", node_path) };
    let resolved = Resolver::default().resolve(&project, "m1/a.js").map(|r| r.full_path());
    match previous_node_path {
        Some(previous) => {
            // SAFETY: Restores NODE_PATH to its original value.
            unsafe { env::set_var("NODE_PATH", previous) };
        }
        None => {
            // SAFETY: Restores process env by removing NODE_PATH when it was originally unset.
            unsafe { env::remove_var("NODE_PATH") };
        }
    }
    assert_eq!(resolved, Ok(expected));
}
