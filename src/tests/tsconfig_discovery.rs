//! Tests for tsconfig discovery
//!
//! Tests that tsconfig.json can be auto-discovered when no explicit tsconfig option is provided.

#[test]
fn tsconfig_discovery() {
    super::tsconfig_paths::tsconfig_resolve_impl(/* tsconfig_discovery */ true);
}
