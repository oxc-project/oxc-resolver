mod borrowed_path;
mod cache_impl;
mod cached_path;
mod hasher;
mod thread_local;

pub use cache_impl::Cache;
pub use cached_path::CachedPath;

#[cfg(test)]
mod tests {
    use super::borrowed_path::BorrowedCachedPath;
    use std::path::Path;

    #[test]
    fn test_borrowed_cached_path_eq() {
        let path1 = Path::new("/foo/bar");
        let path2 = Path::new("/foo/bar");
        let path3 = Path::new("/foo/baz");

        let borrowed1 = BorrowedCachedPath { hash: 1, path: path1 };
        let borrowed2 = BorrowedCachedPath { hash: 2, path: path2 };
        let borrowed3 = BorrowedCachedPath { hash: 1, path: path3 };

        // Same path should be equal even with different hash
        assert_eq!(borrowed1, borrowed2);
        // Different path should not be equal even with same hash
        assert_ne!(borrowed1, borrowed3);
    }
}
