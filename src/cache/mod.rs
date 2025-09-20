mod borrowed_path;
mod cache_impl;
mod cached_path;
mod hasher;
mod thread_local;

pub use cache_impl::Cache;
pub use cached_path::CachedPath;

// Internal types used within the cache module
pub(crate) use borrowed_path::BorrowedCachedPath;
pub(crate) use cached_path::CachedPathImpl;
pub(crate) use hasher::IdentityHasher;
pub(crate) use thread_local::{SCRATCH_PATH, THREAD_ID};
