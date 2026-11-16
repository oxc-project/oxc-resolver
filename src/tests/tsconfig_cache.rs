//! Tests for the tsconfig cache: resolvers cloned with different tsconfig options, and a
//! config that is loaded both as a root config and through another config's `extends`.

use crate::{ResolveError, ResolveOptions, Resolver, TsconfigDiscovery};

#[test]
fn clone_with_different_tsconfig_options_uses_separate_cache() {
    let f = super::fixture_root().join("tsconfig");
    let importer = f.join("main.ts");

    let resolver = Resolver::default();
    assert!(resolver.find_tsconfig(&importer).unwrap().is_none());

    let auto_resolver = resolver.clone_with_options(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });
    let tsconfig = auto_resolver.find_tsconfig(importer).unwrap().unwrap();
    assert_eq!(tsconfig.path, f.join("tsconfig.json"));
}

#[test]
fn clone_with_different_tsconfig_options_resolves_file_with_separate_cache() {
    let f = super::fixture_root().join("tsconfig");
    let importer = f.join("main.ts");

    let resolver = Resolver::default();
    assert_eq!(
        resolver.resolve_file(&importer, "ts-path"),
        Err(ResolveError::NotFound("ts-path".into()))
    );

    let auto_resolver = resolver.clone_with_options(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });
    let resolved_path =
        auto_resolver.resolve_file(importer, "ts-path").map(|resolution| resolution.full_path());
    assert_eq!(resolved_path, Ok(f.join("src/foo.js")));
}

#[test]
fn test_tsconfig_mixed_root_non_root_cache() {
    let f = super::fixture_root().join("tsconfig");
    let f2 = f.join("cases").join("simple-paths");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });
    resolver.cache.get_tsconfig(false, &f2.join("tsconfig.json"), |_| Ok(())).unwrap();
    let resolved_path =
        resolver.resolve_file(f2.join("foo.ts"), "bar/index.ts").map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f2.join("bar/index.ts")));
}

#[test]
fn test_tsconfig_mixed_root_non_root_cache2() {
    let f = super::fixture_root().join("tsconfig");
    let f2 = f.join("cases").join("extends-paths");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });
    resolver.cache.get_tsconfig(true, &f2.join("tsconfig.base.json"), |_| Ok(())).unwrap();
    let resolved_path =
        resolver.resolve_file(f2.join("test.ts"), "@/index.js").map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f2.join("src/index.js")));
}

#[test]
fn test_tsconfig_mixed_root_non_root_cache3() {
    let f = super::fixture_root().join("tsconfig");
    let f2 = f.join("cases").join("extends-paths");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });
    resolver.cache.get_tsconfig(false, &f2.join("tsconfig.json"), |_| Ok(())).unwrap();
    let resolved_path =
        resolver.resolve_file(f2.join("test.ts"), "@/index.js").map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f2.join("src/index.js")));
}
