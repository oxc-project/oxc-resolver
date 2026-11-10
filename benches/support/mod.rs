use std::alloc::{GlobalAlloc, Layout, System};

#[global_allocator]
static ALLOCATOR: NeverGrowInPlace = NeverGrowInPlace;

/// System realloc can grow in place depending on heap state. Use GlobalAlloc's
/// allocate/copy/deallocate implementation so simulation always takes the same path.
/// Allocation costs remain part of the measurement.
struct NeverGrowInPlace;

// SAFETY: Allocation and deallocation use the same System allocator and preserve its contract.
unsafe impl GlobalAlloc for NeverGrowInPlace {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller supplies a valid allocation layout.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: The caller supplies an allocation from this allocator with its original layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}
