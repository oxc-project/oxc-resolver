use std::sync::Arc;

use crate::Resolver;

/// Test to prove memory leak in `CachedPath` Arc cycles
#[test]
fn test_memory_leak_arc_cycles() {
    let f = super::fixture_root().join("misc");

    let resolver = Resolver::default();

    let path = resolver.cache.value(&f);

    resolver.resolve(&f, "package-json-nested").unwrap();

    // Populated cache - path is now owned in multiple places.
    assert_eq!(Arc::strong_count(&path.0), 2);

    // Drop the resolver.
    drop(resolver);

    // All Arcs must be dropped, leaving the original count of 1.
    assert_eq!(Arc::strong_count(&path.0), 1);
}
