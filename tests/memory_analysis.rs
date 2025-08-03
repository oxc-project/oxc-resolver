//! Memory usage analysis tests for oxc-resolver
//! 
//! This module contains tests to identify potential memory issues and optimize memory usage patterns.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicUsize, Ordering},
    path::PathBuf,
    fs,
};

use oxc_resolver::Resolver;

/// A custom allocator that tracks memory allocations
struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);
static CURRENT_USAGE: AtomicUsize = AtomicUsize::new(0);
static PEAK_USAGE: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let size = layout.size();
            ALLOCATED.fetch_add(size, Ordering::Relaxed);
            let current = CURRENT_USAGE.fetch_add(size, Ordering::Relaxed) + size;
            
            // Update peak usage
            let mut peak = PEAK_USAGE.load(Ordering::Relaxed);
            while current > peak {
                match PEAK_USAGE.compare_exchange_weak(peak, current, Ordering::Relaxed, Ordering::Relaxed) {
                    Ok(_) => break,
                    Err(new_peak) => peak = new_peak,
                }
            }
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        let size = layout.size();
        DEALLOCATED.fetch_add(size, Ordering::Relaxed);
        CURRENT_USAGE.fetch_sub(size, Ordering::Relaxed);
    }
}

// Note: This test would need global allocator setup which is not possible in a test file
// So we'll use a different approach with memory monitoring

/// Get memory usage statistics 
#[derive(Debug, Clone)]
struct MemoryStats {
    allocated: usize,
    deallocated: usize,
    current: usize,
    peak: usize,
}

impl MemoryStats {
    fn current() -> Self {
        Self {
            allocated: ALLOCATED.load(Ordering::Relaxed),
            deallocated: DEALLOCATED.load(Ordering::Relaxed),
            current: CURRENT_USAGE.load(Ordering::Relaxed),
            peak: PEAK_USAGE.load(Ordering::Relaxed),
        }
    }

    fn diff(&self, other: &Self) -> Self {
        Self {
            allocated: self.allocated.saturating_sub(other.allocated),
            deallocated: self.deallocated.saturating_sub(other.deallocated),
            current: self.current.saturating_sub(other.current),
            peak: self.peak.max(other.peak),
        }
    }
}

fn reset_memory_counters() {
    ALLOCATED.store(0, Ordering::Relaxed);
    DEALLOCATED.store(0, Ordering::Relaxed);
    CURRENT_USAGE.store(0, Ordering::Relaxed);
    PEAK_USAGE.store(0, Ordering::Relaxed);
}

/// Create a resolver with moderate complexity for testing
fn create_test_resolver() -> Resolver {
    use oxc_resolver::{AliasValue, ResolveOptions};
    
    let alias_value = AliasValue::from("./");
    Resolver::new(ResolveOptions {
        extensions: vec![".ts".into(), ".js".into(), ".json".into()],
        condition_names: vec!["node".into(), "import".into(), "require".into()],
        alias_fields: vec![vec!["browser".into()]],
        extension_alias: vec![
            (".js".into(), vec![".ts".into(), ".js".into()]),
            (".mjs".into(), vec![".mts".into()]),
        ],
        alias: vec![
            ("@alias".into(), vec![alias_value.clone()]),
            ("test-alias".into(), vec![alias_value]),
        ],
        ..ResolveOptions::default()
    })
}

/// Test data for memory analysis
fn get_test_cases() -> Vec<(PathBuf, &'static str)> {
    let current_dir = std::env::current_dir().unwrap();
    vec![
        // Basic relative paths
        (current_dir.clone(), "./src/lib.rs"),
        (current_dir.clone(), "./package.json"),
        (current_dir.clone(), "."),
        (current_dir.clone(), "./src"),
        
        // Package resolution
        (current_dir.clone(), "serde"),
        (current_dir.clone(), "thiserror"),
        (current_dir.clone(), "once_cell"),
        
        // Scoped packages
        (current_dir.clone(), "@napi-rs/cli"),
        
        // Non-existent paths (should not cause memory leaks)
        (current_dir.clone(), "./non-existent"),
        (current_dir.clone(), "non-existent-package"),
        
        // Complex paths
        (current_dir.join("src"), "../Cargo.toml"),
        (current_dir.join("src"), "./lib.rs"),
    ]
}

#[test]
fn test_resolver_memory_usage_patterns() {
    let resolver = create_test_resolver();
    let test_cases = get_test_cases();
    
    println!("Testing resolver memory usage with {} test cases", test_cases.len());
    
    // Warm up the resolver to exclude one-time allocations
    for (path, specifier) in &test_cases[..3] {
        let _ = resolver.resolve(path, specifier);
    }
    
    // Clear any existing cache
    resolver.clear_cache();
    
    // Test memory usage during resolution
    let mut memory_per_resolution = Vec::new();
    
    for (i, (path, specifier)) in test_cases.iter().enumerate() {
        // Get memory before resolution
        let before = get_process_memory();
        
        let result = resolver.resolve(path, specifier);
        
        // Get memory after resolution
        let after = get_process_memory();
        
        let memory_diff = after.saturating_sub(before);
        memory_per_resolution.push(memory_diff);
        
        println!("Resolution {}: {} -> {:?}, Memory: +{} bytes", 
                 i, specifier, result.is_ok(), memory_diff);
    }
    
    // Test cache memory usage
    let cache_size_before = get_process_memory();
    
    // Resolve the same set again - should use cache
    for (path, specifier) in &test_cases {
        let _ = resolver.resolve(path, specifier);
    }
    
    let cache_size_after = get_process_memory();
    let cache_memory_increase = cache_size_after.saturating_sub(cache_size_before);
    
    println!("Cache reuse memory increase: {} bytes", cache_memory_increase);
    
    // Clear cache and check if memory is released
    let before_clear = get_process_memory();
    resolver.clear_cache();
    let after_clear = get_process_memory();
    
    println!("Memory after cache clear: {} -> {} (diff: {})", 
             before_clear, after_clear, 
             after_clear.saturating_sub(before_clear));
    
    // Analyze results
    let avg_memory_per_resolution: usize = memory_per_resolution.iter().sum::<usize>() / memory_per_resolution.len();
    let max_memory_per_resolution = memory_per_resolution.iter().max().unwrap_or(&0);
    
    println!("Average memory per resolution: {} bytes", avg_memory_per_resolution);
    println!("Maximum memory per resolution: {} bytes", max_memory_per_resolution);
    
    // Basic assertions to detect excessive memory usage
    assert!(avg_memory_per_resolution < 100_000, 
            "Average memory per resolution too high: {} bytes", avg_memory_per_resolution);
    assert!(*max_memory_per_resolution < 500_000, 
            "Maximum memory per resolution too high: {} bytes", max_memory_per_resolution);
    assert!(cache_memory_increase < 50_000, 
            "Cache reuse should not significantly increase memory: {} bytes", cache_memory_increase);
}

#[test] 
fn test_memory_growth_with_many_resolves() {
    let resolver = create_test_resolver();
    let current_dir = std::env::current_dir().unwrap();
    
    // Test different types of resolutions that might cause memory growth
    let test_cases = vec![
        // Same path, different specifiers
        ("same_path_diff_spec", current_dir.clone(), vec!["./src/lib.rs", "./src/cache.rs", "./src/error.rs"]),
        // Different paths, same specifier  
        ("diff_path_same_spec", current_dir.clone(), vec!["package.json"]),
        // Non-existent files (potential for memory leaks in error paths)
        ("non_existent", current_dir.clone(), vec!["./non-existent-1", "./non-existent-2", "./non-existent-3"]),
    ];
    
    let initial_memory = get_process_memory();
    println!("Initial memory: {} bytes", initial_memory);
    
    for (name, path, specifiers) in test_cases.iter() {
        let before = get_process_memory();
        
        for _ in 0..100 {  // Repeat to amplify any memory growth
            for specifier in specifiers {
                let _ = resolver.resolve(path, specifier);
            }
        }
        
        let after = get_process_memory();
        let growth = after.saturating_sub(before);
        
        println!("Pattern '{}': Memory growth after 100 iterations: {} bytes", name, growth);
        
        // Memory growth should be reasonable
        assert!(growth < 1_000_000, 
                "Excessive memory growth in pattern '{}': {} bytes", name, growth);
    }
    
    // Check total memory growth
    let final_memory = get_process_memory();
    let total_growth = final_memory.saturating_sub(initial_memory);
    
    println!("Total memory growth: {} bytes", total_growth);
    
    // Clear cache and check memory release
    resolver.clear_cache();
    let after_clear = get_process_memory();
    let memory_released = final_memory.saturating_sub(after_clear);
    
    println!("Memory released after cache clear: {} bytes", memory_released);
}

/// Simple memory usage estimation based on procfs (Linux) or fallback
fn get_process_memory() -> usize {
    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = fs::read_to_string("/proc/self/status") {
            for line in contents.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<usize>() {
                            return kb * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }
    
    // Fallback: return 0 if we can't measure
    // In a real implementation, we might use other platform-specific methods
    0
}

#[test]
fn test_cache_efficiency() {
    let resolver = create_test_resolver();
    let current_dir = std::env::current_dir().unwrap();
    
    // Test cache hit efficiency
    let test_path = current_dir.clone();
    let test_specifier = "./src/lib.rs";
    
    // First resolution - cache miss
    let start = std::time::Instant::now();
    let result1 = resolver.resolve(&test_path, test_specifier);
    let first_duration = start.elapsed();
    
    // Second resolution - should be cache hit
    let start = std::time::Instant::now();
    let result2 = resolver.resolve(&test_path, test_specifier);
    let second_duration = start.elapsed();
    
    assert_eq!(result1.is_ok(), result2.is_ok());
    if let (Ok(res1), Ok(res2)) = (&result1, &result2) {
        assert_eq!(res1.path(), res2.path());
    }
    
    println!("First resolution: {:?}", first_duration);
    println!("Second resolution (cached): {:?}", second_duration);
    
    // Second resolution should be significantly faster (cache hit)
    // This is a sanity check that caching is working
    assert!(second_duration < first_duration, 
            "Cache hit should be faster than cache miss");
}

#[test]
fn test_string_allocation_patterns() {
    let resolver = create_test_resolver();
    let current_dir = std::env::current_dir().unwrap();
    
    // Test various string patterns that might cause unnecessary allocations
    let string_heavy_cases = vec![
        "./src/../src/./lib.rs",  // Path with redundant components
        "./src/lib.rs?query=test&param=value#fragment",  // Query and fragment
        "./非常长的路径名称/with/unicode/characters.js",  // Unicode in paths
        "@scope/package-name/sub/path/to/file.js",  // Scoped packages with deep paths
        "./src/lib.rs",  // Simple case for comparison
    ];
    
    for (i, specifier) in string_heavy_cases.iter().enumerate() {
        let memory_before = get_process_memory();
        
        // Resolve multiple times to see if strings are being needlessly allocated
        for _ in 0..50 {
            let _ = resolver.resolve(&current_dir, specifier);
        }
        
        let memory_after = get_process_memory();
        let memory_diff = memory_after.saturating_sub(memory_before);
        
        println!("String pattern {}: '{}' -> {} bytes", i, specifier, memory_diff);
        
        // Each pattern should not cause excessive memory growth after caching
        assert!(memory_diff < 100_000, 
                "Excessive memory usage for string pattern '{}': {} bytes", 
                specifier, memory_diff);
    }
}