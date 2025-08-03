//! Memory optimization tests and fixes for oxc-resolver
//! 
//! This module focuses on specific memory issues identified during analysis

use std::path::PathBuf;
use oxc_resolver::Resolver;

#[test]
fn investigate_path_normalization_memory_usage() {
    let resolver = create_test_resolver();
    let current_dir = std::env::current_dir().unwrap();
    
    // Test cases with redundant path components that require normalization
    let problematic_paths = vec![
        "./src/../src/./lib.rs",
        "./src/./cache.rs",
        "./src/../src/../src/lib.rs", 
        "././././package.json",
        "./src/./././../src/lib.rs",
    ];
    
    println!("Testing path normalization memory usage:");
    
    for path in problematic_paths {
        let initial_memory = get_process_memory();
        
        // Resolve the path multiple times to see memory growth
        for _ in 0..100 {
            let _ = resolver.resolve(&current_dir, path);
        }
        
        let final_memory = get_process_memory();
        let memory_growth = final_memory.saturating_sub(initial_memory);
        
        println!("Path '{}': {} bytes", path, memory_growth);
        
        // This specific test should pass with lower thresholds
        if memory_growth > 100_000 {
            println!("WARNING: High memory usage for path normalization: {}", path);
        }
    }
}

#[test]
fn investigate_cache_path_storage() {
    let resolver = create_test_resolver();
    let current_dir = std::env::current_dir().unwrap();
    
    // Test many different paths to see cache growth patterns
    let mut test_paths = Vec::new();
    
    // Create various path variations
    for i in 0..50 {
        test_paths.push(format!("./test-path-{}/file.js", i));
        test_paths.push(format!("./nested/deep/path-{}/file.js", i));
        test_paths.push(format!("@scope/package-{}/index.js", i));
    }
    
    let initial_memory = get_process_memory();
    
    // Resolve all paths once
    for path in &test_paths {
        let _ = resolver.resolve(&current_dir, path);
    }
    
    let after_first_round = get_process_memory();
    let first_round_growth = after_first_round.saturating_sub(initial_memory);
    
    // Resolve all paths again (should use cache)
    for path in &test_paths {
        let _ = resolver.resolve(&current_dir, path);
    }
    
    let after_second_round = get_process_memory();
    let second_round_growth = after_second_round.saturating_sub(after_first_round);
    
    println!("Cache path storage analysis:");
    println!("  Paths tested: {}", test_paths.len());
    println!("  First round (cache population): {} bytes", first_round_growth);
    println!("  Second round (cache hits): {} bytes", second_round_growth);
    println!("  Average per path: {} bytes", first_round_growth / test_paths.len().max(1));
    
    // Clear cache and check memory release
    resolver.clear_cache();
    let after_clear = get_process_memory();
    let memory_released = after_second_round.saturating_sub(after_clear);
    
    println!("  Memory released after clear: {} bytes", memory_released);
    
    // Analyze efficiency
    let bytes_per_path = first_round_growth / test_paths.len().max(1);
    if bytes_per_path > 1000 {
        println!("WARNING: High memory usage per cached path: {} bytes", bytes_per_path);
    }
}

#[test]
fn investigate_string_allocations() {
    let resolver = create_test_resolver();
    let current_dir = std::env::current_dir().unwrap();
    
    // Test cases that likely cause string allocations
    let string_allocation_cases = vec![
        ("simple", "./src/lib.rs"),
        ("with_query", "./src/lib.rs?foo=bar"),
        ("with_fragment", "./src/lib.rs#section"),
        ("with_both", "./src/lib.rs?query=value#fragment"),
        ("unicode", "./路径/文件.js"),
        ("scoped", "@scope/package/deep/path.js"),
        ("normalized", "./src/../src/lib.rs"),
        ("complex", "./src/./../../src/../src/lib.rs?q=1&p=2#frag"),
    ];
    
    println!("String allocation analysis:");
    
    for (name, specifier) in string_allocation_cases {
        let initial_memory = get_process_memory();
        
        // Repeat to amplify any allocations
        for _ in 0..200 {
            let _ = resolver.resolve(&current_dir, specifier);
        }
        
        let final_memory = get_process_memory();
        let memory_growth = final_memory.saturating_sub(initial_memory);
        
        println!("  {}: '{}' -> {} bytes", name, specifier, memory_growth);
        
        if memory_growth > 50_000 {
            println!("    WARNING: High string allocation for: {}", specifier);
        }
    }
}

#[test]
fn test_concurrent_resolver_memory() {
    use std::sync::Arc;
    use std::thread;
    
    let resolver = Arc::new(create_test_resolver());
    let current_dir = std::env::current_dir().unwrap();
    
    let initial_memory = get_process_memory();
    
    // Spawn multiple threads that use the resolver concurrently
    let handles: Vec<_> = (0..4).map(|thread_id| {
        let resolver = Arc::clone(&resolver);
        let current_dir = current_dir.clone();
        
        thread::spawn(move || {
            let thread_specific_paths = vec![
                format!("./thread-{}/file1.js", thread_id),
                format!("./thread-{}/file2.js", thread_id),
                format!("./thread-{}/nested/file.js", thread_id),
            ];
            
            for _ in 0..100 {
                for path in &thread_specific_paths {
                    let _ = resolver.resolve(&current_dir, path);
                }
            }
        })
    }).collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_memory = get_process_memory();
    let memory_growth = final_memory.saturating_sub(initial_memory);
    
    println!("Concurrent resolver memory growth: {} bytes", memory_growth);
    
    // Clear cache and check memory release
    resolver.clear_cache();
    let after_clear = get_process_memory();
    let memory_released = final_memory.saturating_sub(after_clear);
    
    println!("Memory released after clear: {} bytes", memory_released);
    
    // Memory growth should be reasonable for concurrent usage
    assert!(memory_growth < 2_000_000, 
            "Excessive memory growth in concurrent usage: {} bytes", memory_growth);
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

/// Simple memory usage estimation based on procfs (Linux) or fallback
fn get_process_memory() -> usize {
    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = std::fs::read_to_string("/proc/self/status") {
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
    0
}