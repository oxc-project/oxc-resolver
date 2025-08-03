use std::{env, path::PathBuf};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use oxc_resolver::{PathUtil, ResolveOptions, Resolver};

fn bench_path_operations(c: &mut Criterion) {
    let cwd = env::current_dir().unwrap();
    let resolver = Resolver::new(ResolveOptions::default());
    
    // Benchmark common path operations
    let paths = vec![
        "./test.js",
        "./src/index.js",
        "./node_modules/package/index.js",
        "../package.json",
        "./deeply/nested/path/to/module.ts",
    ];
    
    c.bench_function("path_resolution_common", |b| {
        b.iter(|| {
            for path in &paths {
                black_box(resolver.resolve(&cwd, path));
            }
        });
    });

    // Benchmark path normalization
    let test_paths = vec![
        "foo/../bar",
        "./src/./index.js",
        "../../node_modules/package",
        "deep/nested/./path/../to/module",
    ];
    
    c.bench_function("path_normalization", |b| {
        b.iter(|| {
            for path in &test_paths {
                let path_buf = PathBuf::from(path);
                black_box(path_buf.normalize());
            }
        });
    });
    
    // Benchmark cache access patterns
    c.bench_function("cache_repeated_access", |b| {
        b.iter(|| {
            // Simulate repeated access to same paths (common pattern)
            for _ in 0..10 {
                let _ = black_box(resolver.resolve(&cwd, "./src/index.js"));
            }
        });
    });
    
    // Benchmark specifier parsing (common operation)
    let specifiers = vec![
        "lodash",
        "./utils/helper.js",
        "@babel/core",
        "../package.json",
        "react-dom/server",
    ];
    
    c.bench_function("specifier_parsing", |b| {
        b.iter(|| {
            for specifier in &specifiers {
                let _ = black_box(resolver.resolve(&cwd, specifier));
            }
        });
    });
}

criterion_group!(path_operations, bench_path_operations);
criterion_main!(path_operations);