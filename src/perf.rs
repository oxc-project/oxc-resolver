/// Performance monitoring infrastructure for oxc-resolver
/// Inspired by Bun's approach to measuring system-level performance
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

/// Global performance counters for tracking resolver operations
pub struct PerfCounters {
    /// Number of cache hits for path metadata
    pub cache_hits: AtomicU64,
    /// Number of cache misses for path metadata
    pub cache_misses: AtomicU64,
    /// Number of filesystem operations
    pub fs_operations: AtomicU64,
    /// Time spent in filesystem operations
    pub fs_time_nanos: AtomicU64,
    /// Number of path normalizations
    pub path_normalizations: AtomicU64,
    /// Number of package.json reads
    pub package_json_reads: AtomicU64,
    /// Number of tsconfig reads
    pub tsconfig_reads: AtomicU64,
    /// Total resolution count
    pub resolutions: AtomicU64,
    /// Time spent in hot paths (resolution)
    pub resolution_time_nanos: AtomicU64,
    /// Memory allocations for paths (inline vs heap)
    pub inline_path_allocations: AtomicU64,
    pub heap_path_allocations: AtomicU64,
}

impl Default for PerfCounters {
    fn default() -> Self {
        Self {
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            fs_operations: AtomicU64::new(0),
            fs_time_nanos: AtomicU64::new(0),
            path_normalizations: AtomicU64::new(0),
            package_json_reads: AtomicU64::new(0),
            tsconfig_reads: AtomicU64::new(0),
            resolutions: AtomicU64::new(0),
            resolution_time_nanos: AtomicU64::new(0),
            inline_path_allocations: AtomicU64::new(0),
            heap_path_allocations: AtomicU64::new(0),
        }
    }
}

impl PerfCounters {
    pub fn cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn fs_operation(&self, duration: Duration) {
        self.fs_operations.fetch_add(1, Ordering::Relaxed);
        self.fs_time_nanos.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn path_normalization(&self) {
        self.path_normalizations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn package_json_read(&self) {
        self.package_json_reads.fetch_add(1, Ordering::Relaxed);
    }

    pub fn tsconfig_read(&self) {
        self.tsconfig_reads.fetch_add(1, Ordering::Relaxed);
    }

    pub fn resolution(&self, duration: Duration) {
        self.resolutions.fetch_add(1, Ordering::Relaxed);
        self.resolution_time_nanos.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn inline_path_allocation(&self) {
        self.inline_path_allocations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn heap_path_allocation(&self) {
        self.heap_path_allocations.fetch_add(1, Ordering::Relaxed);
    }

    /// Calculate cache hit rate as a percentage
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        if total > 0 {
            (hits as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate average resolution time in microseconds
    pub fn avg_resolution_time_micros(&self) -> f64 {
        let total_nanos = self.resolution_time_nanos.load(Ordering::Relaxed);
        let count = self.resolutions.load(Ordering::Relaxed);
        if count > 0 {
            (total_nanos as f64 / count as f64) / 1000.0
        } else {
            0.0
        }
    }

    /// Calculate inline vs heap allocation ratio
    pub fn inline_allocation_rate(&self) -> f64 {
        let inline = self.inline_path_allocations.load(Ordering::Relaxed);
        let heap = self.heap_path_allocations.load(Ordering::Relaxed);
        let total = inline + heap;
        if total > 0 {
            (inline as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    pub fn reset(&self) {
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.fs_operations.store(0, Ordering::Relaxed);
        self.fs_time_nanos.store(0, Ordering::Relaxed);
        self.path_normalizations.store(0, Ordering::Relaxed);
        self.package_json_reads.store(0, Ordering::Relaxed);
        self.tsconfig_reads.store(0, Ordering::Relaxed);
        self.resolutions.store(0, Ordering::Relaxed);
        self.resolution_time_nanos.store(0, Ordering::Relaxed);
        self.inline_path_allocations.store(0, Ordering::Relaxed);
        self.heap_path_allocations.store(0, Ordering::Relaxed);
    }

    pub fn print_stats(&self) {
        println!("=== oxc-resolver Performance Statistics ===");
        println!("Cache hit rate: {:.2}%", self.cache_hit_rate());
        println!("Total cache hits: {}", self.cache_hits.load(Ordering::Relaxed));
        println!("Total cache misses: {}", self.cache_misses.load(Ordering::Relaxed));
        println!("Filesystem operations: {}", self.fs_operations.load(Ordering::Relaxed));
        println!("Average resolution time: {:.2}μs", self.avg_resolution_time_micros());
        println!("Total resolutions: {}", self.resolutions.load(Ordering::Relaxed));
        println!("Path normalizations: {}", self.path_normalizations.load(Ordering::Relaxed));
        println!("Package.json reads: {}", self.package_json_reads.load(Ordering::Relaxed));
        println!("TSConfig reads: {}", self.tsconfig_reads.load(Ordering::Relaxed));
        println!("Inline allocation rate: {:.2}%", self.inline_allocation_rate());
        println!("===========================================");
    }
}

/// Global performance counters instance
pub static PERF_COUNTERS: PerfCounters = PerfCounters {
    cache_hits: AtomicU64::new(0),
    cache_misses: AtomicU64::new(0),
    fs_operations: AtomicU64::new(0),
    fs_time_nanos: AtomicU64::new(0),
    path_normalizations: AtomicU64::new(0),
    package_json_reads: AtomicU64::new(0),
    tsconfig_reads: AtomicU64::new(0),
    resolutions: AtomicU64::new(0),
    resolution_time_nanos: AtomicU64::new(0),
    inline_path_allocations: AtomicU64::new(0),
    heap_path_allocations: AtomicU64::new(0),
};

/// RAII timer for measuring operation duration
pub struct Timer<F>
where
    F: FnOnce(Duration),
{
    start: Instant,
    counter: Option<F>,
}

impl<F> Timer<F>
where
    F: FnOnce(Duration),
{
    pub fn new(counter: F) -> Self {
        Self {
            start: Instant::now(),
            counter: Some(counter),
        }
    }
}

impl<F> Drop for Timer<F>
where
    F: FnOnce(Duration),
{
    fn drop(&mut self) {
        if let Some(counter) = self.counter.take() {
            counter(self.start.elapsed());
        }
    }
}

/// Macro for instrumenting filesystem operations
#[macro_export]
macro_rules! instrument_fs {
    ($operation:expr) => {{
        let _timer = $crate::perf::Timer::new(|d| $crate::perf::PERF_COUNTERS.fs_operation(d));
        $operation
    }};
}

/// Macro for instrumenting resolution operations
#[macro_export]
macro_rules! instrument_resolution {
    ($operation:expr) => {{
        let _timer = $crate::perf::Timer::new(|d| $crate::perf::PERF_COUNTERS.resolution(d));
        $operation
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_perf_counters() {
        let counters = PerfCounters::default();

        // Test cache operations
        counters.cache_hit();
        counters.cache_hit();
        counters.cache_miss();

        assert!((counters.cache_hit_rate() - 66.67).abs() < 0.01); // 2/3 ≈ 66.67%

        // Test allocation tracking
        counters.inline_path_allocation();
        counters.inline_path_allocation();
        counters.heap_path_allocation();

        assert!((counters.inline_allocation_rate() - 66.67).abs() < 0.01); // 2/3 ≈ 66.67%
    }

    #[test]
    fn test_timer() {
        let counters = PerfCounters::default();

        {
            let _timer = Timer::new(|d| counters.fs_operation(d));
            thread::sleep(Duration::from_millis(1));
        }

        assert!(counters.fs_operations.load(Ordering::Relaxed) == 1);
        assert!(counters.fs_time_nanos.load(Ordering::Relaxed) > 0);
    }
}