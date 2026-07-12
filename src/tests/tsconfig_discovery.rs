//! Tests for tsconfig discovery
//!
//! Tests that tsconfig.json can be auto-discovered when no explicit tsconfig option is provided.

use std::path::PathBuf;

use crate::{ResolveError, ResolveOptions, Resolver, TsconfigDiscovery};

#[test]
fn tsconfig_discovery() {
    super::tsconfig_paths::tsconfig_resolve_impl(/* tsconfig_discovery */ true);
}

/// An extensionless file is owned through a `files` entry — a literal exact-path
/// match — even when a *nearer* `tsconfig.json` exists that does not list it (an
/// `include` glob cannot match an extensionless path). Ownership belongs to the
/// outer config that lists `sub/ccc` in `files`, so only its `@x/*` alias applies.
///
/// Previously `claims_ownership_of` returned `true` for every extensionless path,
/// so the nearer `sub/tsconfig.json` wrongly claimed `ccc` by proximity.
#[test]
fn extensionless_file_owned_via_files_array() {
    let f = super::fixture_root().join("tsconfig/cases/extensionless-file");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".ts".into()],
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });

    let importer = f.join("sub/ccc");

    // Owned by the outer config (via `files`), not the nearer `sub/tsconfig.json`.
    let tsconfig = resolver.find_tsconfig(&importer).unwrap().unwrap();
    assert_eq!(tsconfig.path, f.join("tsconfig.json"));

    // The outer `@x/*` alias applies.
    let resolved_path = resolver.resolve_file(&importer, "@x/foo").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("x/foo.ts")));

    // The nearer config's `@y/*` alias does not (it does not own the file).
    let not_owned = resolver.resolve_file(&importer, "@y/foo").map(|r| r.full_path());
    assert_eq!(not_owned, Err(ResolveError::NotFound("@y/foo".into())));
}

#[test]
fn tsconfig_discovery_virtual_file_importer() {
    let f = super::fixture_root().join("tsconfig");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        cwd: Some(f.join("cases/index")),
        ..ResolveOptions::default()
    });

    let resolved_path =
        resolver.resolve_file("\0virtual-module/foo.js", "random-import").map(|f| f.full_path());
    assert_eq!(resolved_path, Err(ResolveError::NotFound("random-import".into())));
}

/// When a tsconfig.json exists but is not readable (e.g. permission denied),
/// auto-discovery should skip it and return `Ok(None)` instead of erroring.
#[test]
#[cfg(unix)]
fn tsconfig_discovery_skips_unreadable_file() {
    use std::{fs, os::unix::fs::PermissionsExt};

    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("test_unreadable_tsconfig");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let tsconfig_path = dir.join("tsconfig.json");
    fs::write(&tsconfig_path, r#"{"compilerOptions": {}}"#).unwrap();
    fs::set_permissions(&tsconfig_path, fs::Permissions::from_mode(0o000)).unwrap();

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });

    // Should return Ok(None), not an error
    let file_path = dir.join("main.ts");
    let result = resolver.find_tsconfig(&file_path);

    // Restore permissions before asserting so cleanup always works
    fs::set_permissions(&tsconfig_path, fs::Permissions::from_mode(0o644)).unwrap();
    let _ = fs::remove_dir_all(&dir);

    assert!(
        matches!(&result, Ok(None)),
        "expected Ok(None) for unreadable tsconfig, got {result:?}",
    );
}

#[test]
fn tsconfig_discovery_query_params() {
    let f = super::fixture_root().join("tsconfig/cases/query-params");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });

    let p = f.join("src/index.ts");
    let expected_tsconfig = f.join("tsconfig.app.json");

    let tsconfig = resolver.find_tsconfig(&p).unwrap().unwrap();
    assert_eq!(tsconfig.path, expected_tsconfig);

    let path_with_both = format!("{}?custom=foo#fragment", p.display());
    let tsconfig = resolver.find_tsconfig(&path_with_both).unwrap().unwrap();
    assert_eq!(tsconfig.path, expected_tsconfig,);
}
