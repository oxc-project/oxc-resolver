use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use oxc_resolver::{FileSystem as FileSystemTrait, FileSystemOs, PackageJson};
use rayon::prelude::*;

use memory_fs::BenchMemoryFS;

fn data() -> Vec<(PathBuf, &'static str)> {
    let cwd = env::current_dir().unwrap();
    let f1 = cwd.join("fixtures/enhanced-resolve");
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
    let root = env::current_dir()?.join("fixtures/enhanced-resolve");
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
    if !temp_path.exists()
        && let Err(err) = create_symlink_fixtures()
    {
        let _ = fs::remove_dir_all(&temp_path);
        return Err(err);
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
    let symlink_test_dir = cwd.join("fixtures/enhanced-resolve/test/temp_symlinks");

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

    group.bench_with_input(BenchmarkId::from_parameter("drop"), &data, |b, data| {
        b.iter(|| {
            let oxc_resolver = oxc_resolver_memory(); // Measure `Drop` performance.
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

    group.bench_with_input(BenchmarkId::from_parameter("find tsconfig"), &data, |b, data| {
        let oxc_resolver = oxc_resolver_memory();
        let paths = data
            .iter()
            .map(|(path, request)| oxc_resolver.resolve(path, request).unwrap().into_path_buf())
            .collect::<Vec<_>>();
        b.iter(|| {
            for path in &paths {
                let _ = oxc_resolver.find_tsconfig(path);
            }
        });
    });
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

criterion_group!(
    resolver,
    bench_resolver_memory,
    bench_resolver_real,
    bench_package_json_deserialization
);
criterion_main!(resolver);

mod memory_fs {
    //! Memory-based file system implementation for benchmarks.
    //!
    //! This module provides an in-memory file system that loads all fixture data
    //! and node_modules packages at initialization time, eliminating filesystem I/O
    //! variance during benchmark execution. This ensures stable, reproducible benchmark results.

    use std::{
        fs, io,
        path::{Path, PathBuf},
    };

    use oxc_resolver::{FileMetadata, FileSystem, ResolveError};
    use rustc_hash::{FxHashMap, FxHashSet};
    use std::sync::LazyLock;
    use walkdir::WalkDir;

    /// Memory-based file system for benchmarks to eliminate I/O variance
    #[derive(Clone)]
    pub struct BenchMemoryFS {
        files: FxHashMap<PathBuf, Vec<u8>>,
        directories: FxHashSet<PathBuf>,
        symlinks: FxHashMap<PathBuf, PathBuf>,
    }

    static BENCH_FS: LazyLock<BenchMemoryFS> = LazyLock::new(|| {
        let mut fs = BenchMemoryFS {
            files: FxHashMap::default(),
            directories: FxHashSet::default(),
            symlinks: FxHashMap::default(),
        };
        fs.load_fixtures();
        fs
    });

    impl BenchMemoryFS {
        /// Create a new memory file system and load all fixtures
        pub fn new() -> Self {
            // Return a clone of the pre-loaded static FS
            BENCH_FS.clone()
        }

        fn add_parent_directories(&mut self, path: &Path) {
            // Add all parent directories of a path
            for ancestor in path.ancestors().skip(1) {
                self.directories.insert(ancestor.to_path_buf());
            }
        }

        fn load_fixtures(&mut self) {
            let cwd = std::env::current_dir().unwrap();

            // Add all parent directories for the cwd
            self.add_parent_directories(&cwd);

            // Load fixtures from enhanced-resolve
            let fixtures_base = cwd.join("fixtures/enhanced-resolve");
            if fixtures_base.exists() {
                for entry in WalkDir::new(&fixtures_base)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(Result::ok)
                {
                    let path = entry.path();
                    let Ok(metadata) = fs::symlink_metadata(path) else { continue };

                    // Store with absolute paths
                    let abs_path = path.to_path_buf();

                    if metadata.is_symlink() {
                        if let Ok(target) = fs::read_link(path) {
                            self.symlinks.insert(abs_path.clone(), target);
                            self.add_parent_directories(&abs_path);
                        }
                    } else if metadata.is_dir() {
                        self.directories.insert(abs_path.clone());
                        self.add_parent_directories(&abs_path);
                    } else if metadata.is_file()
                        && let Ok(content) = fs::read(path)
                    {
                        self.files.insert(abs_path.clone(), content);
                        self.add_parent_directories(&abs_path);
                    }
                }
            }

            // Load specific node_modules packages for benchmarks
            self.load_node_modules_packages(&cwd);

            // Create symlink fixtures for benchmark (10000 symlinks)
            self.create_symlink_fixtures(&cwd);
        }

        fn load_node_modules_packages(&mut self, cwd: &Path) {
            let node_modules = cwd.join("node_modules");
            if !node_modules.exists() {
                return;
            }

            // Only load these specific packages needed for benchmarks
            let packages =
                ["@napi-rs/cli", "@napi-rs/wasm-runtime", "vitest", "emnapi", "typescript"];

            for package_name in packages {
                let package_path = node_modules.join(package_name);
                if !package_path.exists() {
                    continue;
                }

                // For scoped packages, also register the parent scope directory
                if package_name.starts_with('@')
                    && let Some(parent) = package_path.parent()
                    && parent != node_modules
                {
                    self.directories.insert(parent.to_path_buf());
                    self.add_parent_directories(parent);
                }

                // Check if it's a symlink and resolve it
                if let Ok(metadata) = fs::symlink_metadata(&package_path) {
                    if metadata.is_symlink() {
                        // Add the symlink itself
                        if let Ok(target) = fs::read_link(&package_path) {
                            self.symlinks.insert(package_path.clone(), target.clone());
                            self.add_parent_directories(&package_path);

                            // Resolve the symlink target (relative to node_modules)
                            let resolved_target = if target.is_relative() {
                                package_path.parent().unwrap().join(&target)
                            } else {
                                target
                            };

                            // Load the actual package directory
                            if resolved_target.exists() {
                                self.load_package_files(&resolved_target);
                            }

                            // ALSO load via the symlink path itself, because the resolver
                            // might query using the symlink path
                            self.load_package_files(&package_path);
                        }
                    } else {
                        // Regular directory, load it directly
                        self.load_package_files(&package_path);
                    }
                }
            }
        }

        fn load_package_files(&mut self, package_root: &Path) {
            // Load package files with limited depth to avoid loading entire dependency trees
            for entry in WalkDir::new(package_root)
                .follow_links(true) // Follow symlinks within the package
                .max_depth(5) // Load a bit deeper to get dist/ and lib/ directories
                .into_iter()
                .filter_map(Result::ok)
            {
                let path = entry.path();
                let Ok(metadata) = fs::metadata(path) else { continue };
                let abs_path = path.to_path_buf();

                if metadata.is_dir() {
                    self.directories.insert(abs_path.clone());
                    self.add_parent_directories(&abs_path);
                } else if metadata.is_file() {
                    // Only load essential file types
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_str();
                        if matches!(
                            ext_str,
                            Some("json" | "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" | "d.ts")
                        ) && let Ok(content) = fs::read(path)
                        {
                            self.files.insert(abs_path.clone(), content);
                            self.add_parent_directories(&abs_path);
                        }
                    } else if path.file_name() == Some(std::ffi::OsStr::new("package.json")) {
                        // Also load package.json even if extension check fails
                        if let Ok(content) = fs::read(path) {
                            self.files.insert(abs_path.clone(), content);
                            self.add_parent_directories(&abs_path);
                        }
                    }
                }
            }
        }

        fn create_symlink_fixtures(&mut self, cwd: &Path) {
            // Create temp_symlinks directory
            let temp_path = cwd.join("fixtures/enhanced-resolve/test/temp_symlinks");
            self.directories.insert(temp_path.clone());
            self.add_parent_directories(&temp_path);

            // Create index.js
            let index_path = temp_path.join("index.js");
            self.files.insert(index_path, b"console.log('Hello, World!')".to_vec());

            // Create 10000 symlinks pointing to index.js
            // These are created in memory during initialization, not during benchmark execution
            for i in 0..10000 {
                let symlink_path = temp_path.join(format!("file{i}.js"));
                self.symlinks.insert(symlink_path, PathBuf::from("index.js"));
            }
        }
    }

    impl Default for BenchMemoryFS {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FileSystem for BenchMemoryFS {
        #[cfg(not(feature = "yarn_pnp"))]
        fn new() -> Self {
            Self::default()
        }

        #[cfg(feature = "yarn_pnp")]
        fn new(_yarn_pnp: bool) -> Self {
            Self::default()
        }

        fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
            // Try direct lookup first
            if let Some(bytes) = self.files.get(path) {
                return Ok(bytes.clone());
            }

            // Try following symlinks
            let mut current = path.to_path_buf();
            let mut visited = FxHashSet::default();

            while let Some(target) = self.symlinks.get(&current) {
                if !visited.insert(current.clone()) {
                    return Err(io::Error::other("Circular symlink"));
                }

                current = if target.is_relative() {
                    current.parent().unwrap().join(target)
                } else {
                    target.clone()
                };

                if let Some(bytes) = self.files.get(&current) {
                    return Ok(bytes.clone());
                }
            }

            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("File not found: {}", path.display()),
            ))
        }

        fn read_to_string(&self, path: &Path) -> io::Result<String> {
            let bytes = self.read(path)?;
            String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }

        fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
            // Check if it's a file (direct)
            if self.files.contains_key(path) {
                return Ok(FileMetadata::new(true, false, false));
            }

            // Check if it's a directory (direct)
            if self.directories.contains(path) {
                return Ok(FileMetadata::new(false, true, false));
            }

            // Follow symlinks to find the target
            let mut current = path.to_path_buf();
            let mut visited = FxHashSet::default();

            while let Some(target) = self.symlinks.get(&current) {
                if !visited.insert(current.clone()) {
                    return Err(io::Error::other("Circular symlink"));
                }

                current = if target.is_relative() {
                    current.parent().unwrap().join(target)
                } else {
                    target.clone()
                };

                if self.files.contains_key(&current) {
                    return Ok(FileMetadata::new(true, false, false));
                } else if self.directories.contains(&current) {
                    return Ok(FileMetadata::new(false, true, false));
                }
            }

            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Path not found: {}", path.display()),
            ))
        }

        fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
            // Check if it's a symlink first (before resolving)
            if self.symlinks.contains_key(path) {
                return Ok(FileMetadata::new(false, false, true));
            }

            // Otherwise, fall back to regular metadata
            self.metadata(path)
        }

        fn read_link(&self, path: &Path) -> Result<PathBuf, ResolveError> {
            self.symlinks.get(path).cloned().ok_or_else(|| {
                ResolveError::from(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Not a symlink: {}", path.display()),
                ))
            })
        }

        fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
            // Follow symlinks to resolve the canonical path
            let mut current = path.to_path_buf();
            let mut visited = FxHashSet::default();

            while let Some(target) = self.symlinks.get(&current) {
                if !visited.insert(current.clone()) {
                    return Err(io::Error::other("Circular symlink"));
                }

                current = if target.is_relative() {
                    current.parent().unwrap().join(target)
                } else {
                    target.clone()
                };
            }

            // Verify the final path exists
            if self.files.contains_key(&current) || self.directories.contains(&current) {
                Ok(current)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Path not found: {}", path.display()),
                ))
            }
        }
    }
}
