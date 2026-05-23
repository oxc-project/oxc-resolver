//! A compact, atomically-initialized container holding `Option<Arc<T>>`.
//!
//! [`OnceArc<T>`] is a single `AtomicUsize` (8 bytes) that encodes three states:
//!
//! 1. Uninitialized — `f` has never been called.
//! 2. Initialized with `None` — `f` was called and returned `None`.
//! 3. Initialized with `Some(Arc<T>)` — `f` was called and returned an `Arc`.
//!
//! By packing both the discriminant and the optional pointer into a single word,
//! a `OnceArc<T>` replaces an `OnceLock<Option<Arc<T>>>` (24 bytes on a 64-bit
//! target) and saves 16 bytes per slot. Unlike `OnceLock`, multiple threads may
//! race to compute `f`; the loser drops the `Arc` it created and observes the
//! winner's value. This is acceptable for the resolver's caches because all
//! current users compute idempotent functions of the filesystem.

use std::{
    fmt,
    marker::PhantomData,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

/// Encodes "no `f` call yet" — distinguished from `Some(Arc)` by being zero, and from
/// `None` by being the only zero value. Null pointers from `Arc::into_raw` aren't possible
/// (the `Arc`'s control block is always heap-allocated), so 0 is safe as a sentinel.
const STATE_UNINIT: usize = 0;
/// Encodes "`f` returned `None`". `Arc::into_raw` for any `T` with alignment ≥ 2 cannot
/// produce this value, so 1 is safe as a sentinel.
const STATE_NONE: usize = 1;

/// An atomic, lazily-populated slot holding `Option<Arc<T>>`.
///
/// `T` must have alignment ≥ 2; otherwise `Arc::into_raw` could collide with [`STATE_NONE`].
pub struct OnceArc<T> {
    /// Packed encoding: see the module docs.
    inner: AtomicUsize,
    _phantom: PhantomData<Option<Arc<T>>>,
}

impl<T> OnceArc<T> {
    pub const fn new() -> Self {
        // Trigger a compile-time check that `T`'s alignment leaves room for [`STATE_NONE`].
        const { assert!(std::mem::align_of::<T>() >= 2, "OnceArc requires align_of::<T>() >= 2") };
        Self { inner: AtomicUsize::new(STATE_UNINIT), _phantom: PhantomData }
    }

    /// If this slot has already been populated, return its value (cloning any `Arc`).
    /// Otherwise call `f`, atomically install its result, and return that. If a concurrent
    /// caller installs first, the loser's `Arc` (if any) is dropped and the winner's value
    /// is returned instead.
    pub fn get_or_try_init<F, E>(&self, f: F) -> Result<Option<Arc<T>>, E>
    where
        F: FnOnce() -> Result<Option<Arc<T>>, E>,
    {
        let current = self.inner.load(Ordering::Acquire);
        if current != STATE_UNINIT {
            // SAFETY: the slot is initialized, so `current` is either `STATE_NONE` or a valid
            // `Arc::into_raw` pointer we own a strong ref to.
            return Ok(unsafe { decode(current) });
        }

        let computed = f()?;
        let encoded = encode(computed.as_ref());

        match self.inner.compare_exchange(
            STATE_UNINIT,
            encoded,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                // We installed `encoded`. The strong-ref that was transferred into `encoded`
                // now lives inside `self.inner`; the matching one in `computed` is still
                // alive and returned to the caller.
                Ok(computed)
            }
            Err(actual) => {
                // SAFETY: `encoded` is the value we minted via `encode` immediately above;
                // it is either `STATE_NONE` (no-op for `drop_encoded`) or a fresh `Arc` we
                // need to reclaim now that our CAS lost.
                unsafe { drop_encoded::<T>(encoded) };
                // SAFETY: `actual` was loaded post-CAS and a winning store happened-before our
                // CAS; `actual` is therefore a fully initialized state.
                Ok(unsafe { decode::<T>(actual) })
            }
        }
    }

    /// Whether the slot has been initialized yet (by either `Some` or `None`).
    #[cfg(test)]
    pub fn is_initialized(&self) -> bool {
        self.inner.load(Ordering::Acquire) != STATE_UNINIT
    }

    /// Return the stored value without computing. Panics if the slot is still uninitialized.
    /// Intended for tests only.
    #[cfg(test)]
    pub fn get(&self) -> Option<Arc<T>> {
        let v = self.inner.load(Ordering::Acquire);
        assert!(v != STATE_UNINIT, "OnceArc::get on uninitialized slot");
        // SAFETY: post-acquire load; see `get_or_try_init`.
        unsafe { decode(v) }
    }
}

impl<T> Default for OnceArc<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for OnceArc<T> {
    fn drop(&mut self) {
        let v = *self.inner.get_mut();
        // SAFETY: we have exclusive access via `&mut self`, so no concurrent reads can
        // observe the slot. If a strong ref is parked in `v`, take it back so the `Arc`'s
        // destructor runs.
        unsafe { drop_encoded::<T>(v) };
    }
}

impl<T: fmt::Debug> fmt::Debug for OnceArc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Avoid touching the strong refcount by inspecting the encoded state only.
        let v = self.inner.load(Ordering::Acquire);
        let mut dbg = f.debug_struct("OnceArc");
        match v {
            STATE_UNINIT => dbg.field("state", &"uninit"),
            STATE_NONE => dbg.field("state", &"none"),
            _ => dbg.field("state", &"some"),
        };
        dbg.finish()
    }
}

/// SAFETY: every public path that yields an `Arc` either clones the cached pointer (preserving
/// the parked strong ref) or transfers ownership atomically; no `Arc` value is ever shared
/// without a matching `Arc::into_raw` / `Arc::from_raw` accounting.
unsafe impl<T: Send + Sync> Send for OnceArc<T> {}
/// SAFETY: see [`OnceArc`]'s `Send` impl. Reads are synchronized through `inner`.
unsafe impl<T: Send + Sync> Sync for OnceArc<T> {}

fn encode<T>(value: Option<&Arc<T>>) -> usize {
    value.map_or(STATE_NONE, |arc| Arc::into_raw(Arc::clone(arc)) as usize)
}

/// # Safety
///
/// `v` must be a fully initialized encoding — either [`STATE_NONE`] or a valid pointer parked
/// in `OnceArc::inner` whose strong-ref accounting is owned by the slot. The strong ref stays
/// with the slot; we clone it to hand back a fresh `Arc` to the caller.
unsafe fn decode<T>(v: usize) -> Option<Arc<T>> {
    debug_assert!(v != STATE_UNINIT);
    if v == STATE_NONE {
        return None;
    }
    // SAFETY: `v` came from `Arc::into_raw`; we reconstruct the `Arc`, clone it (incrementing
    // the strong count), then forget the reconstructed one so the count we observe matches
    // what's parked in the slot.
    let parked = unsafe { Arc::from_raw(v as *const T) };
    let cloned = Arc::clone(&parked);
    std::mem::forget(parked);
    Some(cloned)
}

/// # Safety
///
/// `v` must be the result of [`encode`] — i.e. either [`STATE_UNINIT`], [`STATE_NONE`], or a
/// pointer minted by `Arc::into_raw` whose strong ref we are now reclaiming.
unsafe fn drop_encoded<T>(v: usize) {
    if v == STATE_UNINIT || v == STATE_NONE {
        return;
    }
    // SAFETY: `v` came from `Arc::into_raw`; reconstructing the `Arc` and letting it drop
    // decrements the strong count by the one ref the slot was holding.
    drop(unsafe { Arc::from_raw(v as *const T) });
}

#[cfg(test)]
mod tests {
    use super::*;

    // `i32` has `align >= 2` so it's a valid `T` for [`OnceArc`].

    #[test]
    fn uninitialized_slot_is_not_initialized() {
        let slot: OnceArc<i32> = OnceArc::new();
        assert!(!slot.is_initialized());
    }

    #[test]
    fn initialize_with_none_then_observe() {
        let slot: OnceArc<i32> = OnceArc::new();
        let computed: Result<_, ()> = slot.get_or_try_init(|| Ok(None));
        assert!(computed.unwrap().is_none());
        assert!(slot.is_initialized());
        assert!(slot.get().is_none());
    }

    #[test]
    fn initialize_with_some_then_observe() {
        let slot: OnceArc<i32> = OnceArc::new();
        let computed: Result<_, ()> = slot.get_or_try_init(|| Ok(Some(Arc::new(42))));
        let arc = computed.unwrap().unwrap();
        assert_eq!(*arc, 42);
        let observed = slot.get().unwrap();
        assert_eq!(*observed, 42);
        // Two outstanding strong refs + one parked in the slot.
        assert_eq!(Arc::strong_count(&arc), 3);
        drop(observed);
        assert_eq!(Arc::strong_count(&arc), 2);
    }

    #[test]
    fn get_or_try_init_caches_first_result() {
        let slot: OnceArc<i32> = OnceArc::new();
        let _: Result<_, ()> = slot.get_or_try_init(|| Ok(Some(Arc::new(1))));
        let second: Result<_, ()> =
            slot.get_or_try_init(|| panic!("must not re-run on an initialized slot"));
        assert_eq!(*second.unwrap().unwrap(), 1);
    }

    #[test]
    fn drop_releases_strong_ref() {
        let arc = Arc::new(99_i32);
        let slot: OnceArc<i32> = OnceArc::new();
        let _: Result<_, ()> = slot.get_or_try_init(|| Ok(Some(Arc::clone(&arc))));
        assert_eq!(Arc::strong_count(&arc), 2);
        drop(slot);
        assert_eq!(Arc::strong_count(&arc), 1);
    }

    #[test]
    fn propagates_initializer_error() {
        let slot: OnceArc<i32> = OnceArc::new();
        let r: Result<_, &'static str> = slot.get_or_try_init(|| Err("boom"));
        assert_eq!(r.unwrap_err(), "boom");
        // The slot stays uninitialized so a subsequent attempt can retry.
        assert!(!slot.is_initialized());
        let r: Result<_, &'static str> = slot.get_or_try_init(|| Ok(Some(Arc::new(7))));
        assert_eq!(*r.unwrap().unwrap(), 7);
    }
}
