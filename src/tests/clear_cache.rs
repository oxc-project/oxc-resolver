use std::{env, fs};

use rayon::prelude::*;

use crate::Resolver;

#[test]
fn test_parallel_resolve_with_clear_cache() {
    let target_dir = env::current_dir().unwrap().join("./target");
    let test_dir = target_dir.join("test_clear_cache");
    let node_modules = test_dir.join("node_modules");

    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&node_modules).unwrap();

    let packages: Vec<String> = (1..=100).map(|i| format!("package_{i}")).collect();
    for package in &packages {
        let package_dir = node_modules.join(package);
        fs::create_dir_all(&package_dir).unwrap();
        let package_json = format!(r#"{{"name": "{package}", "main": "index.js"}}"#);
        fs::write(package_dir.join("package.json"), package_json).unwrap();
        fs::write(package_dir.join("index.js"), "").unwrap();
    }

    let resolver = Resolver::default();
    for _ in 1..100 {
        packages.par_iter().enumerate().for_each(|(i, package)| {
            if i % 10 == 0 && i > 0 {
                resolver.clear_cache();
            }
            let result = resolver.resolve(&test_dir, package);
            assert!(result.is_ok(), "Failed to resolve {package}: {result:?}");
        });
    }

    let _ = fs::remove_dir_all(&test_dir);
}
