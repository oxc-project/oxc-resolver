use std::path::{PathBuf, Path};
use oxc_resolver::Resolver;

/// Create a test to demonstrate and fix string allocation issues
#[test]
fn test_string_allocation_fix() {
    let resolver = create_test_resolver();
    let current_dir = std::env::current_dir().unwrap();
    
    // Test case that previously showed high memory usage
    let test_cases = vec![
        "./src/lib.rs",
        "./src/lib.rs#section", 
        "./路径/文件.js",
        "./src/../src/lib.rs",
    ];
    
    println!("Testing string allocation fixes:");
    
    for case in test_cases {
        let initial_memory = get_process_memory();
        
        // Resolve many times to see cumulative allocation pattern
        for _ in 0..100 {
            let _ = resolver.resolve(&current_dir, case);
        }
        
        let final_memory = get_process_memory();
        let memory_diff = final_memory.saturating_sub(initial_memory);
        
        println!("  '{}': {} bytes", case, memory_diff);
        
        // After optimizations, memory usage should be much lower
        if memory_diff > 50_000 {
            println!("    ⚠️  Still high memory usage");
        } else {
            println!("    ✅ Memory usage optimized");
        }
    }
}

/// Test cache internals with detailed monitoring
#[test]
fn test_cache_internal_optimization() {
    let resolver = create_test_resolver();
    let current_dir = std::env::current_dir().unwrap();
    
    // Start with clean state
    resolver.clear_cache();
    let baseline_memory = get_process_memory();
    
    // Test progressive cache growth
    let test_paths = (0..20).map(|i| format!("./test-{}/file.js", i)).collect::<Vec<_>>();
    
    println!("Progressive cache growth analysis:");
    
    for (i, path) in test_paths.iter().enumerate() {
        let before = get_process_memory();
        let _ = resolver.resolve(&current_dir, path);
        let after = get_process_memory();
        
        let growth = after.saturating_sub(before);
        let total_growth = after.saturating_sub(baseline_memory);
        
        if i % 5 == 0 || i == test_paths.len() - 1 {
            println!("  Path {}: +{} bytes (total: +{} bytes)", 
                     i + 1, growth, total_growth);
        }
    }
    
    // Test cache clearing effectiveness
    let before_clear = get_process_memory();
    resolver.clear_cache();
    let after_clear = get_process_memory();
    
    let memory_released = before_clear.saturating_sub(after_clear);
    println!("  Memory released by clear_cache(): {} bytes", memory_released);
    
    if memory_released > 0 {
        println!("  ✅ Cache clearing is releasing memory");
    } else {
        println!("  ⚠️  Cache clearing not releasing memory effectively");
    }
}

/// Test to analyze memory usage patterns in resolver components
#[test] 
fn test_resolver_component_memory() {
    let resolver = create_test_resolver();
    let current_dir = std::env::current_dir().unwrap();
    
    // Test different types of operations that might have different memory patterns
    struct TestCase {
        name: &'static str,
        paths: Vec<&'static str>,
        description: &'static str,
    }
    
    let test_cases = vec![
        TestCase {
            name: "simple_files",
            paths: vec!["./src/lib.rs", "./src/cache.rs", "./src/error.rs"],
            description: "Simple file resolutions",
        },
        TestCase {
            name: "packages",
            paths: vec!["serde", "thiserror", "once_cell"],
            description: "Package resolutions (likely to fail)",
        },
        TestCase {
            name: "complex_paths",
            paths: vec!["./src/../src/lib.rs", "./src/./cache.rs", "././package.json"],
            description: "Paths requiring normalization",
        },
        TestCase {
            name: "query_fragment", 
            paths: vec!["./src/lib.rs?query", "./src/cache.rs#section", "./package.json?q=1#frag"],
            description: "Paths with query and fragment components",
        },
    ];
    
    println!("Component memory usage analysis:");
    
    for test_case in test_cases {
        resolver.clear_cache();
        let initial = get_process_memory();
        
        // Run each path in the test case multiple times
        for path in &test_case.paths {
            for _ in 0..50 {
                let _ = resolver.resolve(&current_dir, path);
            }
        }
        
        let final_memory = get_process_memory();
        let memory_used = final_memory.saturating_sub(initial);
        let avg_per_path = memory_used / test_case.paths.len().max(1);
        
        println!("  {}: {} bytes total, {} bytes/path", 
                 test_case.name, memory_used, avg_per_path);
        println!("    ({})", test_case.description);
        
        if avg_per_path > 10_000 {
            println!("    ⚠️  High memory usage per path");
        }
    }
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