mod cache_impl;
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

        let path = cache.value(Path::new("/foo/bar"));
        let debug_str = format!("{path:?}");
        assert!(debug_str.contains("FsCachedPath"));
        assert!(debug_str.contains("idx"));
    }
}
