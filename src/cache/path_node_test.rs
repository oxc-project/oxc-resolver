// Tests for the new generation-based PathHandle implementation

#[cfg(test)]
mod tests {
    use crate::{Cache, FileSystem, FileSystemOs};
    use std::path::Path;

    fn create_cache() -> Cache<FileSystemOs> {
        #[cfg(feature = "yarn_pnp")]
        return Cache::new(FileSystemOs::new(false));
        #[cfg(not(feature = "yarn_pnp"))]
        return Cache::new(FileSystemOs::new());
    }

    #[test]
    fn test_value_creates_handle() {
        let cache = create_cache();
        let handle = cache.value(Path::new("/foo/bar"));
        assert_eq!(handle.path().to_str().unwrap(), "/foo/bar");
    }

    #[test]
    fn test_value_parent_traversal() {
        let cache = create_cache();
        let handle = cache.value(Path::new("/foo/bar/baz"));

        // Traverse up the tree
        let parent = handle.parent().expect("should have parent");
        assert_eq!(parent.path().to_str().unwrap(), "/foo/bar");

        let grandparent = parent.parent().expect("should have grandparent");
        assert_eq!(grandparent.path().to_str().unwrap(), "/foo");
    }

    #[test]
    fn test_value_deduplication() {
        let cache = create_cache();
        let handle1 = cache.value(Path::new("/foo/bar"));
        let handle2 = cache.value(Path::new("/foo/bar"));

        // Should return same index and generation
        assert_eq!(handle1.0.index, handle2.0.index);
        assert!(std::sync::Arc::ptr_eq(&handle1.0.generation, &handle2.0.generation));
    }

    #[test]
    fn test_clear_creates_new_generation() {
        let cache = create_cache();

        // Create a handle in generation 1
        let handle1 = cache.value(Path::new("/foo/bar"));
        let gen1_ptr = std::sync::Arc::as_ptr(&handle1.0.generation);

        // Clear cache (swaps to generation 2)
        cache.clear();

        // Create a handle in generation 2
        let handle2 = cache.value(Path::new("/foo/bar"));
        let gen2_ptr = std::sync::Arc::as_ptr(&handle2.0.generation);

        // Generations should be different
        assert_ne!(gen1_ptr, gen2_ptr);

        // But old handle should still work!
        assert_eq!(handle1.path().to_str().unwrap(), "/foo/bar");
        assert_eq!(handle2.path().to_str().unwrap(), "/foo/bar");
    }

    #[test]
    fn test_clear_ongoing_resolution_safety() {
        let cache = create_cache();

        // Simulate ongoing resolution
        let path1 = cache.value(Path::new("/foo/bar/baz"));
        let parent1 = path1.parent().unwrap();

        // Clear cache while "resolution" is in progress
        cache.clear();

        // Old handles should still work
        assert_eq!(path1.path().to_str().unwrap(), "/foo/bar/baz");
        assert_eq!(parent1.path().to_str().unwrap(), "/foo/bar");

        // Can still traverse parent chain
        let grandparent1 = parent1.parent().unwrap();
        assert_eq!(grandparent1.path().to_str().unwrap(), "/foo");

        // New resolutions get fresh generation
        let path2 = cache.value(Path::new("/foo/bar/baz"));
        assert_eq!(path2.path().to_str().unwrap(), "/foo/bar/baz");

        // Different generations
        assert!(!std::sync::Arc::ptr_eq(&path1.0.generation, &path2.0.generation));
    }

    #[test]
    fn test_path_handle_equality() {
        let cache = create_cache();
        let handle1 = cache.value(Path::new("/foo/bar"));
        let handle2 = cache.value(Path::new("/foo/bar"));
        let handle3 = cache.value(Path::new("/foo/baz"));

        assert_eq!(handle1, handle2); // Same path
        assert_ne!(handle1, handle3); // Different path
    }

    #[test]
    fn test_node_modules_detection() {
        let cache = create_cache();

        let nm_handle = cache.value(Path::new("/foo/node_modules"));
        assert!(nm_handle.is_node_modules());

        let inside_nm = cache.value(Path::new("/foo/node_modules/bar"));
        assert!(!inside_nm.is_node_modules()); // Not itself node_modules
        assert!(inside_nm.inside_node_modules()); // But inside one
    }
}
