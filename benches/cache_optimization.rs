use criterion2::{Criterion, black_box, criterion_group, criterion_main};
use oxc_resolver::{ResolveOptions, Resolver};
use std::path::PathBuf;

fn create_test_project_structure() -> PathBuf {
    let temp_dir = std::env::temp_dir().join("oxc_resolver_bench");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Create a realistic Node.js project structure
    let node_modules = temp_dir.join("node_modules");
    std::fs::create_dir_all(&node_modules).unwrap();

    // Create some popular packages
    for package in ["react", "lodash", "@types/node", "webpack", "babel-core"] {
        let pkg_dir = if package.starts_with('@') {
            let parts: Vec<&str> = package[1..].split('/').collect();
            let scope_dir = node_modules.join(format!("@{}", parts[0]));
            std::fs::create_dir_all(&scope_dir).unwrap();
            scope_dir.join(parts[1])
        } else {
            node_modules.join(package)
        };

        std::fs::create_dir_all(&pkg_dir).unwrap();

        // Create package.json
        let package_json = format!(
            r#"{{
            "name": "{}",
            "version": "1.0.0",
            "main": "index.js"
        }}"#,
            package
        );
        std::fs::write(pkg_dir.join("package.json"), package_json).unwrap();

        // Create index.js
        std::fs::write(pkg_dir.join("index.js"), "module.exports = {};").unwrap();
    }

    // Create main package.json
    let main_package_json = r#"{
        "name": "test-project",
        "version": "1.0.0",
        "dependencies": {
            "react": "^18.0.0",
            "lodash": "^4.17.21"
        }
    }"#;
    std::fs::write(temp_dir.join("package.json"), main_package_json).unwrap();

    temp_dir
}

fn bench_resolve_operations(c: &mut Criterion) {
    let project_dir = create_test_project_structure();
    let resolver = Resolver::new(ResolveOptions::default());

    c.bench_function("resolve_popular_packages", |b| {
        b.iter(|| {
            let packages = ["react", "lodash", "@types/node", "webpack", "babel-core"];
            for package in packages {
                let result = resolver.resolve(black_box(&project_dir), black_box(package));
                black_box(result);
            }
        })
    });

    c.bench_function("resolve_relative_paths", |b| {
        b.iter(|| {
            let paths = ["./package.json", "../package.json", "./node_modules/react"];
            for path in paths {
                let result = resolver.resolve(black_box(&project_dir), black_box(path));
                black_box(result);
            }
        })
    });

    c.bench_function("resolve_deep_node_modules", |b| {
        b.iter(|| {
            // Simulate deep node_modules traversal
            let mut current = project_dir.clone();
            for _ in 0..5 {
                current = current.join("nested");
                let result = resolver.resolve(black_box(&current), black_box("react"));
                black_box(result);
            }
        })
    });
}

fn bench_cache_performance(c: &mut Criterion) {
    let project_dir = create_test_project_structure();
    let resolver = Resolver::new(ResolveOptions::default());

    // Warm up the cache
    for _ in 0..10 {
        let _ = resolver.resolve(&project_dir, "react");
        let _ = resolver.resolve(&project_dir, "lodash");
    }

    c.bench_function("resolve_cached_packages", |b| {
        b.iter(|| {
            // These should hit the cache
            let result1 = resolver.resolve(black_box(&project_dir), black_box("react"));
            let result2 = resolver.resolve(black_box(&project_dir), black_box("lodash"));
            black_box((result1, result2));
        })
    });
}

fn bench_path_operations(c: &mut Criterion) {
    let project_dir = create_test_project_structure();
    let resolver = Resolver::new(ResolveOptions::default());

    c.bench_function("path_normalization", |b| {
        b.iter(|| {
            let complex_paths = [
                "./foo/../bar/./baz",
                "../../../node_modules/react",
                "./a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/package.json",
                "../../foo/bar/../baz/./qux",
            ];
            for path in complex_paths {
                let result = resolver.resolve(black_box(&project_dir), black_box(path));
                black_box(result);
            }
        })
    });
}

fn print_performance_stats() {
    use oxc_resolver::performance::PERF_COUNTERS;

    println!("\n=== Performance Statistics ===");
    PERF_COUNTERS.print_stats();
    println!("===============================\n");
}

criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(std::time::Duration::from_secs(10));
    targets = bench_resolve_operations, bench_cache_performance, bench_path_operations
);

criterion_main!(benches);
