use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use oxc_resolver::{FileSystem as FileSystemTrait, FileSystemOs, PackageJson};
use rayon::prelude::*;

mod memory_fs;

use memory_fs::BenchMemoryFS;

fn data() -> Vec<(PathBuf, &'static str)> {
    let cwd = env::current_dir().unwrap();
    let f1 = cwd.join("fixtures/enhanced_resolve");
    let f2 = f1.join("test/fixtures");
    vec![
        // real packages
        (cwd.clone(), "@napi-rs/cli"),
        (cwd.clone(), "@napi-rs/wasm-runtime"),
        (cwd.clone(), "vitest"),
        (cwd.clone(), "emnapi"),
        (cwd, "typescript"),
        // relative path
        (f1.clone(), "./"),
        (f1.clone(), "./lib/index"),
        // absolute path
        (f1.clone(), "/absolute/path"),
        // query fragment
        (f2.clone(), "./main1.js#fragment?query"),
        (f2.clone(), "m1/a.js?query#fragment"),
        // browserField
        (f2.join("browser-module"), "./lib/replaced"),
        (f2.join("browser-module/lib"), "./replaced"),
        // exportsField
        (f2.join("exports-field"), "exports-field"),
        (f2.join("exports-field"), "exports-field/dist/main.js"),
        (f2.join("exports-field"), "exports-field/dist/main.js?foo"),
        (f2.join("exports-field"), "exports-field/dist/main.js#foo"),
        (f2.join("exports-field"), "@exports-field/core"),
        (f2.join("imports-exports-wildcard"), "m/features/f.js"),
        // extensionAlias
        (f2.join("extension-alias"), "./index.js"),
        (f2.join("extension-alias"), "./dir2/index.mjs"),
        // extensions
        (f2.join("extensions"), "./foo"),
        (f2.join("extensions"), "."),
        (f2.join("extensions"), "./dir"),
        (f2.join("extensions"), "module/"),
        // importsField
        (f2.join("imports-field"), "#imports-field"),
        (f2.join("imports-exports-wildcard/node_modules/m/"), "#internal/i.js"),
        // scoped
        (f2.join("scoped"), "@scope/pack1"),
        (f2.join("scoped"), "@scope/pack2/lib"),
        // dashed name
        (f2.clone(), "dash"),
        (f2.clone(), "dash-name"),
        (f2.join("node_modules/dash"), "dash"),
        (f2.join("node_modules/dash"), "dash-name"),
        (f2.join("node_modules/dash-name"), "dash"),
        (f2.join("node_modules/dash-name"), "dash-name"),
        // alias
        (f1.clone(), "aaa"),
        (f1.clone(), "ggg"),
        (f1.clone(), "rrr"),
        (f1.clone(), "@"),
        (f1, "@@@"),
    ]
}

fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    #[cfg(target_family = "unix")]
    {
        std::os::unix::fs::symlink(original, link)
    }

    #[cfg(target_family = "windows")]
    {
        std::os::windows::fs::symlink_file(original, link)
    }
}

fn create_symlinks() -> io::Result<PathBuf> {
    let root = env::current_dir()?.join("fixtures/enhanced_resolve");
    let dirname = root.join("test");
    let temp_path = dirname.join("temp_symlinks");
    let create_symlink_fixtures = || -> io::Result<()> {
        fs::create_dir(&temp_path)?;
        let mut index = fs::File::create(temp_path.join("index.js"))?;
        index.write_all(b"console.log('Hello, World!')")?;
        // create 10000 symlink files pointing to the index.js
        for i in 0..10000 {
            symlink(temp_path.join("index.js"), temp_path.join(format!("file{i}.js")))?;
        }
        Ok(())
    };
    if !temp_path.exists() {
        if let Err(err) = create_symlink_fixtures() {
            let _ = fs::remove_dir_all(&temp_path);
            return Err(err);
        }
    }
    Ok(temp_path)
}

fn resolve_options() -> oxc_resolver::ResolveOptions {
    use oxc_resolver::{AliasValue, ResolveOptions};
    let alias_value = AliasValue::from("./");
    ResolveOptions {
        extensions: vec![".ts".into(), ".js".into()],
        condition_names: vec!["webpack".into(), "require".into()],
        alias_fields: vec![vec!["browser".into()]],
        extension_alias: vec![
            (".js".into(), vec![".ts".into(), ".js".into()]),
            (".mjs".into(), vec![".mts".into()]),
        ],
        // Real projects LOVE setting these many aliases.
        // I saw them with my own eyes.
        alias: vec![
            ("/absolute/path".into(), vec![alias_value.clone()]),
            ("aaa".into(), vec![alias_value.clone()]),
            ("bbb".into(), vec![alias_value.clone()]),
            ("ccc".into(), vec![alias_value.clone()]),
            ("ddd".into(), vec![alias_value.clone()]),
            ("eee".into(), vec![alias_value.clone()]),
            ("fff".into(), vec![alias_value.clone()]),
            ("ggg".into(), vec![alias_value.clone()]),
            ("hhh".into(), vec![alias_value.clone()]),
            ("iii".into(), vec![alias_value.clone()]),
            ("jjj".into(), vec![alias_value.clone()]),
            ("kkk".into(), vec![alias_value.clone()]),
            ("lll".into(), vec![alias_value.clone()]),
            ("mmm".into(), vec![alias_value.clone()]),
            ("nnn".into(), vec![alias_value.clone()]),
            ("ooo".into(), vec![alias_value.clone()]),
            ("ppp".into(), vec![alias_value.clone()]),
            ("qqq".into(), vec![alias_value.clone()]),
            ("rrr".into(), vec![alias_value.clone()]),
            ("sss".into(), vec![alias_value.clone()]),
            ("@".into(), vec![alias_value.clone()]),
            ("@@".into(), vec![alias_value.clone()]),
            ("@@@".into(), vec![alias_value]),
        ],
        ..ResolveOptions::default()
    }
}

fn oxc_resolver_memory() -> oxc_resolver::ResolverGeneric<BenchMemoryFS> {
    use oxc_resolver::ResolverGeneric;
    let fs = BenchMemoryFS::new();
    ResolverGeneric::new_with_file_system(fs, resolve_options())
}

fn oxc_resolver_real() -> oxc_resolver::Resolver {
    use oxc_resolver::Resolver;
    Resolver::new(resolve_options())
}

fn bench_resolver_memory(c: &mut Criterion) {
    let data = data();
    let cwd = env::current_dir().unwrap();
    let symlink_test_dir = cwd.join("fixtures/enhanced_resolve/test/temp_symlinks");

    // check validity
    for (path, request) in &data {
        assert!(
            oxc_resolver_memory().resolve(path, request).is_ok(),
            "{} {request}",
            path.display()
        );
    }

    let symlinks_range = 0u32..10000;

    for i in symlinks_range.clone() {
        assert!(
            oxc_resolver_memory().resolve(&symlink_test_dir, &format!("./file{i}")).is_ok(),
            "file{i}.js"
        );
    }

    let mut group = c.benchmark_group("resolver_memory");

    group.bench_with_input(BenchmarkId::from_parameter("single-thread"), &data, |b, data| {
        let oxc_resolver = oxc_resolver_memory();
        b.iter(|| {
            for (path, request) in data {
                _ = oxc_resolver.resolve(path, request);
            }
        });
    });

    group.bench_with_input(BenchmarkId::from_parameter("multi-thread"), &data, |b, data| {
        let oxc_resolver = oxc_resolver_memory();
        b.iter(|| {
            data.par_iter().for_each(|(path, request)| {
                _ = oxc_resolver.resolve(path, request);
            });
        });
    });

    group.bench_with_input(
        BenchmarkId::from_parameter("resolve from symlinks"),
        &symlinks_range,
        |b, data| {
            let oxc_resolver = oxc_resolver_memory();
            b.iter(|| {
                for i in data.clone() {
                    assert!(
                        oxc_resolver.resolve(&symlink_test_dir, &format!("./file{i}")).is_ok(),
                        "file{i}.js"
                    );
                }
            });
        },
    );
}

fn bench_resolver_real(c: &mut Criterion) {
    let data = data();
    let symlink_test_dir = create_symlinks().expect("Create symlink fixtures failed");

    // check validity
    for (path, request) in &data {
        assert!(oxc_resolver_real().resolve(path, request).is_ok(), "{} {request}", path.display());
    }

    let symlinks_range = 0u32..10000;

    for i in symlinks_range.clone() {
        assert!(
            oxc_resolver_real().resolve(&symlink_test_dir, &format!("./file{i}")).is_ok(),
            "file{i}.js"
        );
    }

    let mut group = c.benchmark_group("resolver_real");

    group.bench_with_input(BenchmarkId::from_parameter("single-thread"), &data, |b, data| {
        let oxc_resolver = oxc_resolver_real();
        b.iter(|| {
            for (path, request) in data {
                _ = oxc_resolver.resolve(path, request);
            }
        });
    });

    group.bench_with_input(BenchmarkId::from_parameter("multi-thread"), &data, |b, data| {
        let oxc_resolver = oxc_resolver_real();
        b.iter(|| {
            data.par_iter().for_each(|(path, request)| {
                _ = oxc_resolver.resolve(path, request);
            });
        });
    });

    group.bench_with_input(
        BenchmarkId::from_parameter("resolve from symlinks"),
        &symlinks_range,
        |b, data| {
            let oxc_resolver = oxc_resolver_real();
            b.iter(|| {
                for i in data.clone() {
                    assert!(
                        oxc_resolver.resolve(&symlink_test_dir, &format!("./file{i}")).is_ok(),
                        "file{i}.js"
                    );
                }
            });
        },
    );
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
        .join("fixtures/enhanced_resolve/test/fixtures/browser-module/package.json");
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
                let json = json.clone();
                runner.run(|| {
                    PackageJson::parse(&fs, test_path.clone(), test_realpath.clone(), json)
                        .expect("Failed to parse JSON");
                });
            });
        });
    }

    group.finish();
}

criterion_group!(
    resolver,
    bench_resolver_memory,
    bench_resolver_real,
    bench_package_json_deserialization
);
criterion_main!(resolver);
