mod cache_impl;
mod cached_meta;
mod cached_path;
mod hasher;
mod thread_local;

pub use cache_impl::Cache;
pub use cached_path::CachedPath;

#[cfg(test)]
mod tests {
    use super::cache_impl::Cache;
    use crate::FileSystem;
    use std::path::Path;

    #[test]
    fn test_cached_path_debug() {
        #[cfg(feature = "yarn_pnp")]
        let cache = Cache::new(crate::FileSystemOs::new(false));
        #[cfg(not(feature = "yarn_pnp"))]
        let cache = Cache::new(crate::FileSystemOs::new());

        let path = Path::new("/foo/bar");
        let cached_path = cache.value(path);
        assert!(format!("{cached_path:?}") == format!("{path:?}"));
        assert!(format!("{cached_path}") == format!("{}", path.display()));
    }
}
