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

/// Test to ensure canonicalized paths remain accessible after being stored
#[test]
fn test_canonicalized_path_not_dropped() {
    use crate::ResolveOptions;

    let f = super::fixture_root().join("misc");

    let resolver = Resolver::new(ResolveOptions { symlinks: true, ..Default::default() });

    // Create a path and canonicalize it
    let path = resolver.cache.value(&f);

    // This should work without "Canonicalized path was dropped" error
    let canonicalized = resolver.cache.canonicalize(&path);
    assert!(canonicalized.is_ok());

    // Try canonicalizing again - should still work
    let canonicalized2 = resolver.cache.canonicalize(&path);
    assert!(canonicalized2.is_ok());
    assert_eq!(canonicalized.unwrap(), canonicalized2.unwrap());
}

/// Test to ensure canonicalized paths that are not in cache remain accessible
#[test]
fn test_canonicalized_path_weak_reference() {
    let f = super::fixture_root().join("misc");

    let resolver = Resolver::default();

    // Create a new path that's not previously in the cache
    let new_path = f.join("some_unique_path");

    // Get the cached path - this will be the only strong reference
    let path = resolver.cache.value(&new_path);

    // Canonicalize a path that doesn't exist in the cache's hashmap yet
    // This might fail with "Canonicalized path was dropped" if the implementation is wrong
    match resolver.cache.canonicalize(&path) {
        Ok(_) => {
            // If canonicalization succeeded, try again to ensure consistency
            let result2 = resolver.cache.canonicalize(&path);
            assert_eq!(
                resolver.cache.canonicalize(&path).ok(),
                result2.ok(),
                "Canonicalization results should be consistent"
            );
        }
        Err(e) => {
            // It's okay if canonicalization fails for other reasons (e.g., path doesn't exist)
            // but it should NOT fail with "Canonicalized path was dropped"
            let error_msg = e.to_string();
            assert!(
                !error_msg.contains("Canonicalized path was dropped"),
                "Should not fail with 'Canonicalized path was dropped' error, got: {error_msg}"
            );
        }
    }
}
