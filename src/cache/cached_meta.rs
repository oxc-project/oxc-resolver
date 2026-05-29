//! Two single-byte cache slots for a path's filesystem metadata.
//!
//! A path is looked at two ways during resolution:
//!
//! * `link` — the `lstat` ([`FileSystem::symlink_metadata`]) view of the path *itself*: file,
//!   directory, or symlink? Canonicalization needs this to decide whether to follow a link.
//! * `followed` — the `stat` ([`FileSystem::metadata`]) view *after* following symlinks: does the
//!   path ultimately resolve to a file or a directory? This is what `is_file`/`is_dir` need.
//!
//! For a non-symlink the two views are identical, so a single `lstat` answers both and the
//! follow-up `stat` is skipped. Sharing the cached `link` view lets `is_file`/`is_dir` and
//! canonicalization issue one `lstat` instead of a `stat` *and* an `lstat` for the same path.
//!
//! Each view is packed into one [`AtomicU8`] rather than a `OnceLock<Option<FileMetadata>>` (which
//! would be 16 bytes), saving 30 bytes per cached path entry. Metadata is idempotent, so the lack
//! of `OnceLock`'s exactly-once guarantee is harmless: racing threads recompute the same answer.
//!
//! [`FileSystem::metadata`]: crate::FileSystem::metadata
//! [`FileSystem::symlink_metadata`]: crate::FileSystem::symlink_metadata

use std::sync::atomic::{AtomicU8, Ordering};

use crate::FileMetadata;

// Bit layout of a slot. An all-zero byte (`INITIALIZED` unset) means "not probed yet"; `EXISTS`
// distinguishes a cached `Some` from a cached `None`.
const INITIALIZED: u8 = 1 << 0;
const EXISTS: u8 = 1 << 1;
const IS_FILE: u8 = 1 << 2;
const IS_DIR: u8 = 1 << 3;
const IS_SYMLINK: u8 = 1 << 4;

/// Lazily-populated `lstat` (`link`) and `stat` (`followed`) metadata, one byte each.
#[derive(Default)]
pub struct CachedMeta {
    link: AtomicU8,
    followed: AtomicU8,
}

impl CachedMeta {
    pub const fn new() -> Self {
        Self { link: AtomicU8::new(0), followed: AtomicU8::new(0) }
    }

    /// Return the cached `lstat` view, or populate it from `f` (an `lstat`) and return that.
    pub fn link_or_init<F: FnOnce() -> Option<FileMetadata>>(&self, f: F) -> Option<FileMetadata> {
        get_or_init(&self.link, f)
    }

    /// Return the cached `stat` (symlink-followed) view, or populate it from `f` and return that.
    pub fn followed_or_init<F: FnOnce() -> Option<FileMetadata>>(
        &self,
        f: F,
    ) -> Option<FileMetadata> {
        get_or_init(&self.followed, f)
    }
}

fn get_or_init<F: FnOnce() -> Option<FileMetadata>>(slot: &AtomicU8, f: F) -> Option<FileMetadata> {
    let bits = slot.load(Ordering::Relaxed);
    if (bits & INITIALIZED) != 0 {
        return decode(bits);
    }
    let meta = f();
    slot.store(encode(meta), Ordering::Relaxed);
    meta
}

fn encode(meta: Option<FileMetadata>) -> u8 {
    let Some(meta) = meta else { return INITIALIZED };
    let mut bits = INITIALIZED | EXISTS;
    if meta.is_file() {
        bits |= IS_FILE;
    }
    if meta.is_dir() {
        bits |= IS_DIR;
    }
    if meta.is_symlink() {
        bits |= IS_SYMLINK;
    }
    bits
}

/// Decode a slot known to be [`INITIALIZED`].
fn decode(bits: u8) -> Option<FileMetadata> {
    ((bits & EXISTS) != 0).then(|| {
        FileMetadata::new((bits & IS_FILE) != 0, (bits & IS_DIR) != 0, (bits & IS_SYMLINK) != 0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(meta: FileMetadata) -> (bool, bool, bool) {
        (meta.is_file(), meta.is_dir(), meta.is_symlink())
    }

    #[test]
    fn is_two_bytes() {
        assert_eq!(std::mem::size_of::<CachedMeta>(), 2);
    }

    #[test]
    fn roundtrips_and_caches() {
        let cases = [
            None,
            Some(FileMetadata::new(false, false, false)),
            Some(FileMetadata::new(true, false, false)),
            Some(FileMetadata::new(false, true, false)),
            Some(FileMetadata::new(false, false, true)),
        ];
        for expected in cases {
            let meta = CachedMeta::new();
            assert_eq!(meta.link_or_init(|| expected).map(parts), expected.map(parts));
            // Once populated, the initializer is never called again.
            assert_eq!(
                meta.link_or_init(|| panic!("must not be called")).map(parts),
                expected.map(parts)
            );
        }
    }

    #[test]
    fn link_and_followed_are_separate_slots() {
        let meta = CachedMeta::new();
        meta.link_or_init(|| Some(FileMetadata::new(false, false, true)));
        meta.followed_or_init(|| Some(FileMetadata::new(false, true, false)));
        assert_eq!(meta.link_or_init(|| panic!()).map(parts), Some((false, false, true)));
        assert_eq!(meta.followed_or_init(|| panic!()).map(parts), Some((false, true, false)));
    }
}
