use std::{env, fs, path::PathBuf};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use oxc_resolver::{FileSystem as FileSystemTrait, FileSystemOs, PackageJson};
use resolver_workload::{data, memory_fs::BenchMemoryFS, oxc_resolver_memory};

#[path = "support/resolver.rs"]
mod resolver_workload;

fn bench_resolver_memory(c: &mut Criterion) {
    let data = data();
    let cwd = env::current_dir().unwrap();
    let symlink_test_dir = cwd.join("fixtures/enhanced-resolve/test/temp_symlinks");

    // check validity
    for (path, request) in &data {
        assert!(
            oxc_resolver_memory().resolve(path, request).is_ok(),
            "{} {request}",
            path.display()
        );
    }

    let symlink_specifiers = (0u32..10000).map(|i| format!("./file{i}")).collect::<Vec<_>>();

    {
        let oxc_resolver = oxc_resolver_memory();
        for specifier in &symlink_specifiers {
            assert!(oxc_resolver.resolve(&symlink_test_dir, specifier).is_ok(), "{specifier}");
        }
    }

    let mut group = c.benchmark_group("resolver_memory");

    group.bench_with_input(BenchmarkId::from_parameter("single-thread"), &data, |b, data| {
        let oxc_resolver = oxc_resolver_memory();
        for (path, request) in data {
            oxc_resolver.resolve(path, request).unwrap();
        }
        b.iter(|| {
            for (path, request) in data {
                _ = oxc_resolver.resolve(path, request);
            }
        });
    });

    group.bench_with_input(
        BenchmarkId::from_parameter("cold-resolve-and-drop"),
        &data,
        |b, data| {
            b.iter(|| {
                let oxc_resolver = oxc_resolver_memory();
                for (path, request) in data {
                    _ = oxc_resolver.resolve(path, request);
                }
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::from_parameter("resolve from symlinks"),
        &symlink_specifiers,
        |b, specifiers| {
            let oxc_resolver = oxc_resolver_memory();
            for specifier in specifiers {
                oxc_resolver.resolve(&symlink_test_dir, specifier).unwrap();
            }
            b.iter(|| {
                for specifier in specifiers {
                    _ = oxc_resolver.resolve(&symlink_test_dir, specifier);
                }
            });
        },
    );

    group.bench_with_input(BenchmarkId::from_parameter("find tsconfig"), &data, |b, data| {
        let oxc_resolver = oxc_resolver_memory();
        let paths = data
            .iter()
            .map(|(path, request)| oxc_resolver.resolve(path, request).unwrap().into_path_buf())
            .collect::<Vec<_>>();
        for path in &paths {
            let _ = oxc_resolver.find_tsconfig(path);
        }
        b.iter(|| {
            for path in &paths {
                let _ = oxc_resolver.find_tsconfig(path);
            }
        });
    });
}

fn bench_package_json_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_json_deserialization");

    // Prepare different sizes of package.json content
    let small_json = r#"{
        "name": "test-package",
        "version": "1.0.0"
    }"#;

    let medium_json = r##"{
        "name": "test-package",
        "version": "1.0.0",
        "main": "./lib/index.js",
        "type": "module",
        "exports": {
            ".": "./lib/index.js",
            "./feature": "./lib/feature.js"
        },
        "imports": {
            "#internal": "./src/internal.js"
        },
        "browser": {
            "./lib/node.js": "./lib/browser.js"
        },
        "sideEffects": false
    }"##;

    let large_json = r##"{
        "name": "test-package",
        "version": "1.0.0",
        "main": "./lib/index.js",
        "type": "module",
        "exports": {
            ".": {
                "import": "./lib/index.mjs",
                "require": "./lib/index.cjs",
                "browser": "./lib/browser.js"
            },
            "./feature": {
                "import": "./lib/feature.mjs",
                "require": "./lib/feature.cjs"
            },
            "./utils": "./lib/utils.js",
            "./internal/*": "./lib/internal/*.js"
        },
        "imports": {
            "#internal": "./src/internal.js",
            "#utils/*": "./src/utils/*.js"
        },
        "browser": {
            "./lib/node.js": "./lib/browser.js",
            "module-a": "./browser/module-a.js",
            "module-b": "module-c",
            "./lib/replaced.js": "./lib/browser"
        },
        "sideEffects": ["*.css", "*.scss"],
        "dependencies": {
            "lodash": "^4.17.21",
            "react": "^18.0.0",
            "express": "^4.18.0"
        },
        "devDependencies": {
            "typescript": "^5.0.0",
            "eslint": "^8.0.0",
            "jest": "^29.0.0"
        },
        "scripts": {
            "test": "jest",
            "build": "tsc",
            "lint": "eslint src"
        }
    }"##;

    // Load real complex package.json from fixtures
    let complex_json_path = env::current_dir()
        .unwrap()
        .join("fixtures/enhanced-resolve/test/fixtures/browser-module/package.json");
    let complex_json =
        fs::read_to_string(&complex_json_path).expect("Failed to read complex package.json");

    let test_path = PathBuf::from("/test/package.json");
    let test_realpath = test_path.clone();
    #[cfg(feature = "yarn_pnp")]
    let fs = FileSystemOs::new(false);
    #[cfg(not(feature = "yarn_pnp"))]
    let fs = FileSystemOs::new();

    let data = [
        ("small", small_json.to_string()),
        ("medium", medium_json.to_string()),
        ("large", large_json.to_string()),
        ("complex_real", complex_json),
    ];

    for (name, json) in data {
        group.bench_function(name, |b| {
            b.iter_with_setup_wrapper(|runner| {
                let json = json.clone().into_bytes();
                runner.run(|| {
                    PackageJson::parse(&fs, test_path.clone(), test_realpath.clone(), json)
                        .expect("Failed to parse JSON");
                });
            });
        });
    }

    group.finish();
}

fn bench_tsconfig_paths_aliases(c: &mut Criterion) {
    use oxc_resolver::{
        ResolveOptions, ResolverGeneric, TsconfigDiscovery, TsconfigOptions, TsconfigReferences,
    };

    let mut group = c.benchmark_group("tsconfig_paths_aliases_memory");
    let alias_count = 200usize;
    let fs = BenchMemoryFS::new();
    let root = env::current_dir()
        .unwrap()
        .join(format!("fixtures/tsconfig/bench-tsconfig-paths/{alias_count}"));
    let importer = root.join("app/main.ts");
    let tsconfig_path = root.join("tsconfig.json");
    let requests = (0..alias_count).map(|i| format!("@pkg{i:04}/file")).collect::<Vec<_>>();
    let resolver = ResolverGeneric::new_with_file_system(
        fs,
        ResolveOptions {
            tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
                config_file: tsconfig_path,
                references: TsconfigReferences::Disabled,
            })),
            extensions: vec![".ts".into(), ".js".into()],
            ..ResolveOptions::default()
        },
    );

    // Check validity by resolving every alias once.
    for request in &requests {
        assert!(
            resolver.resolve_file(&importer, request).is_ok(),
            "alias_count={alias_count} request={request}"
        );
    }

    group.bench_with_input(
        BenchmarkId::from_parameter("query each alias"),
        &requests,
        |b, requests| {
            b.iter(|| {
                for request in requests {
                    _ = resolver.resolve_file(&importer, request);
                }
            });
        },
    );
}

criterion_group!(
    resolver,
    bench_resolver_memory,
    bench_package_json_deserialization,
    bench_tsconfig_paths_aliases
);
criterion_main!(resolver);
