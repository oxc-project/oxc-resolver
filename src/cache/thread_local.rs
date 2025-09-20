use std::{
    cell::RefCell,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static THREAD_COUNT: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// Per-thread pre-allocated path that is used to perform operations on paths more quickly.
    /// Learned from parcel <https://github.com/parcel-bundler/parcel/blob/a53f8f3ba1025c7ea8653e9719e0a61ef9717079/crates/parcel-resolver/src/cache.rs#L394>
  pub static SCRATCH_PATH: RefCell<PathBuf> = RefCell::new(PathBuf::with_capacity(256));
  pub static THREAD_ID: u64 = THREAD_COUNT.fetch_add(1, Ordering::SeqCst);
}
