//! Tests for tsconfig discovery
//!
//! Tests that tsconfig.json can be auto-discovered when no explicit tsconfig option is provided.

use std::path::PathBuf;

use crate::{ResolveError, ResolveOptions, Resolver, TsconfigDiscovery};

#[test]
fn tsconfig_discovery() {
    super::tsconfig_paths::tsconfig_resolve_impl(/* tsconfig_discovery */ true);
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

/// When the importer path has query parameters (e.g. `file.tsx?custom=foo`),
/// auto-discovery should strip them before walking parent directories
/// and discover the correct tsconfig.json.
///
/// Uses a fixture with project references where the root tsconfig has `include: []`
/// and a referenced tsconfig.app.json has `include: ["src/**/*"]` with path aliases.
/// Without stripping query params, `resolve_tsconfig_solution` fails to match the
/// file against the reference's include pattern, returning the wrong tsconfig.
#[test]
fn tsconfig_discovery_query_params() {
    let f = super::fixture_root().join("tsconfig/cases/query-params");
    let expected_tsconfig = f.join("tsconfig.app.json");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });

    let clean_path = f.join("src/index.ts");

    // Baseline — clean path discovers tsconfig.app.json (via project references)
    let tsconfig = resolver.find_tsconfig(&clean_path).unwrap().unwrap();
    assert_eq!(tsconfig.path, expected_tsconfig, "baseline: should select referenced tsconfig");

    // With query parameter — should discover the same referenced tsconfig
    let path_with_query = format!("{}?custom=foo", clean_path.display());
    let tsconfig = resolver.find_tsconfig(&path_with_query).unwrap().unwrap();
    assert_eq!(tsconfig.path, expected_tsconfig, "query param: should select referenced tsconfig");

    // With fragment — should discover the same referenced tsconfig
    let path_with_fragment = format!("{}#fragment", clean_path.display());
    let tsconfig = resolver.find_tsconfig(&path_with_fragment).unwrap().unwrap();
    assert_eq!(tsconfig.path, expected_tsconfig, "fragment: should select referenced tsconfig");

    // With both query and fragment — should discover the same referenced tsconfig
    let path_with_both = format!("{}?custom=foo#fragment", clean_path.display());
    let tsconfig = resolver.find_tsconfig(&path_with_both).unwrap().unwrap();
    assert_eq!(
        tsconfig.path, expected_tsconfig,
        "query+fragment: should select referenced tsconfig"
    );
}

/// When a tsconfig.json exists but is not readable (e.g. permission denied),
/// auto-discovery should skip it and return `Ok(None)` instead of erroring.
#[test]
#[cfg(unix)]
fn tsconfig_discovery_skips_unreadable_file() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

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
