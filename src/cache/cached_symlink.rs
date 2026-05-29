//! Single-byte cache slot for "is this path a symlink?".
//!
//! Stored separately from [`super::cached_meta::CachedMeta`] because
//! `metadata()` follows symlinks and therefore cannot populate it. The slot is
//! filled by `symlink_metadata()` (the no-follow variant) and lets
//! canonicalization skip the per-component `symlink_metadata` syscall when the
//! answer is already known.

use std::sync::atomic::{AtomicU8, Ordering};

const UNINIT: u8 = 0;
const NOT_SYMLINK: u8 = 1;
const IS_SYMLINK: u8 = 2;

/// Lazily-populated tri-state symlink flag packed into one byte.
pub struct CachedSymlink(AtomicU8);

impl CachedSymlink {
    pub const fn new() -> Self {
        Self(AtomicU8::new(UNINIT))
    }

    /// Return `Some(true)` if the path is known to be a symlink,
    /// `Some(false)` if known not to be, `None` if not yet probed.
    pub fn get(&self) -> Option<bool> {
        match self.0.load(Ordering::Relaxed) {
            NOT_SYMLINK => Some(false),
            IS_SYMLINK => Some(true),
            _ => None,
        }
    }

    /// Store a known answer. Idempotent — multiple writers race harmlessly
    /// because the underlying filesystem property is stable for the lifetime
    /// of a resolver.
    pub fn set(&self, is_symlink: bool) {
        let state = if is_symlink { IS_SYMLINK } else { NOT_SYMLINK };
        self.0.store(state, Ordering::Relaxed);
    }
}

impl Default for CachedSymlink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_symlink_is_one_byte() {
        assert_eq!(std::mem::size_of::<CachedSymlink>(), 1);
    }

    #[test]
    fn returns_none_when_uninitialized() {
        let cell = CachedSymlink::new();
        assert!(cell.get().is_none());
    }

    #[test]
    fn roundtrips_both_states() {
        let cell = CachedSymlink::new();
        cell.set(true);
        assert_eq!(cell.get(), Some(true));
        let cell = CachedSymlink::new();
        cell.set(false);
        assert_eq!(cell.get(), Some(false));
    }
}
