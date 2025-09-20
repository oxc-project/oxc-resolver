/// Simple performance test for cache optimizations
/// This will be used to validate our cache-friendly data layout improvements
use crate::{ResolveOptions, Resolver, performance::PERF_COUNTERS};
use std::{path::PathBuf, time::Instant};

pub fn run_performance_test() {
    let temp_dir = create_test_project();
    let resolver = Resolver::new(ResolveOptions::default());

    println!("Running performance test...");
    PERF_COUNTERS.reset();

    let start = Instant::now();

    // Test resolve operations that stress the cache
    for _ in 0..100 {
        let _ = resolver.resolve(&temp_dir, "react");
        let _ = resolver.resolve(&temp_dir, "lodash");
        let _ = resolver.resolve(&temp_dir, "@types/node");
        let _ = resolver.resolve(&temp_dir, "./package.json");
        let _ = resolver.resolve(&temp_dir, "../package.json");
    }

    let duration = start.elapsed();

    println!("Test completed in {:?}", duration);
    PERF_COUNTERS.print_stats();

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

fn create_test_project() -> PathBuf {
    let temp_dir = std::env::temp_dir().join("oxc_resolver_perf_test");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Create node_modules structure
    let node_modules = temp_dir.join("node_modules");
    std::fs::create_dir_all(&node_modules).unwrap();

    for package in ["react", "lodash"] {
        let pkg_dir = node_modules.join(package);
        std::fs::create_dir_all(&pkg_dir).unwrap();

        let package_json =
            format!(r#"{{"name": "{}", "version": "1.0.0", "main": "index.js"}}"#, package);
        std::fs::write(pkg_dir.join("package.json"), package_json).unwrap();
        std::fs::write(pkg_dir.join("index.js"), "module.exports = {};").unwrap();
    }

    // Create @types/node
    let types_dir = node_modules.join("@types");
    std::fs::create_dir_all(&types_dir).unwrap();
    let node_types_dir = types_dir.join("node");
    std::fs::create_dir_all(&node_types_dir).unwrap();
    std::fs::write(
        node_types_dir.join("package.json"),
        r#"{"name": "@types/node", "version": "1.0.0"}"#,
    )
    .unwrap();

    // Create main package.json
    std::fs::write(temp_dir.join("package.json"), r#"{"name": "test", "version": "1.0.0"}"#)
        .unwrap();

    temp_dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance() {
        run_performance_test();
    }
}
