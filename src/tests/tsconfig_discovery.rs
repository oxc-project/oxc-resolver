//! Tests for tsconfig discovery
//!
//! Tests that tsconfig.json can be auto-discovered when no explicit tsconfig option is provided.

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
        resolver.resolve("\0virtual-module", "random-import").map(|f| f.full_path());
    assert_eq!(resolved_path, Err(ResolveError::NotFound("random-import".into())));
}
