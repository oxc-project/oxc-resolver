//! Single-byte cache slot for `Option<(is_file, is_dir)>` filesystem metadata.
//!
//! Replaces `OnceLock<Option<(bool, bool)>>` (16 bytes) with an `AtomicU8` (1 byte) — saving
//! 15 bytes per cached path entry. The four possible `(is_file, is_dir)` combinations, the
//! "not found" outcome, and the uninitialized state fit comfortably in a single byte.
//!
//! Filesystem metadata is idempotent, so the lack of `OnceLock`'s exactly-once semantics is
//! harmless: if two threads race to populate the slot they both re-`stat` and store the same
//! answer.

use std::sync::atomic::{AtomicU8, Ordering};

const UNINIT: u8 = 0;
const NONE: u8 = 1;
const FALSE_FALSE: u8 = 2;
const TRUE_FALSE: u8 = 3;
const FALSE_TRUE: u8 = 4;
const TRUE_TRUE: u8 = 5;

/// Lazily-populated `Option<(is_file, is_dir)>` packed into one byte.
pub struct CachedMeta(AtomicU8);

impl CachedMeta {
    pub const fn new() -> Self {
        Self(AtomicU8::new(UNINIT))
    }

    /// Return the cached value if the slot has been populated, otherwise call `f`, store its
    /// result, and return it. Multiple threads may race to populate; the last writer wins.
    pub fn get_or_init<F>(&self, f: F) -> Option<(bool, bool)>
    where
        F: FnOnce() -> Option<(bool, bool)>,
    {
        let state = self.0.load(Ordering::Relaxed);
        if state != UNINIT {
            return decode(state);
        }
        let computed = f();
        self.0.store(encode(computed), Ordering::Relaxed);
        computed
    }
}

impl Default for CachedMeta {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn encode(value: Option<(bool, bool)>) -> u8 {
    match value {
        None => NONE,
        Some((false, false)) => FALSE_FALSE,
        Some((true, false)) => TRUE_FALSE,
        Some((false, true)) => FALSE_TRUE,
        Some((true, true)) => TRUE_TRUE,
    }
}

#[inline]
fn decode(state: u8) -> Option<(bool, bool)> {
    match state {
        NONE => None,
        FALSE_FALSE => Some((false, false)),
        TRUE_FALSE => Some((true, false)),
        FALSE_TRUE => Some((false, true)),
        TRUE_TRUE => Some((true, true)),
        // UNINIT is filtered out by the caller; any other byte indicates corruption.
        _ => unreachable!("invalid CachedMeta state"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn cached_meta_is_one_byte() {
        assert_eq!(size_of::<CachedMeta>(), 1);
    }

    #[test]
    fn returns_none_when_initializer_returns_none() {
        let meta = CachedMeta::new();
        assert!(meta.get_or_init(|| None).is_none());
        // And on subsequent calls the initializer is not invoked.
        assert!(meta.get_or_init(|| panic!("must not be called")).is_none());
    }

    #[test]
    fn roundtrips_all_some_combinations() {
        for (is_file, is_dir) in [(false, false), (true, false), (false, true), (true, true)] {
            let meta = CachedMeta::new();
            let got = meta.get_or_init(|| Some((is_file, is_dir)));
            assert_eq!(got, Some((is_file, is_dir)));
            let cached = meta.get_or_init(|| panic!("must not be called"));
            assert_eq!(cached, Some((is_file, is_dir)));
        }
    }
}
