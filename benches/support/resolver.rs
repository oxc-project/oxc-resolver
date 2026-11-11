use std::{env, path::PathBuf};

use memory_fs::BenchMemoryFS;

pub fn data() -> Vec<(PathBuf, &'static str)> {
    let cwd = env::current_dir().unwrap();
    let f1 = cwd.join("fixtures/enhanced-resolve");
    let f2 = f1.join("test/fixtures");
    vec![
        // real packages
        (cwd.clone(), "@napi-rs/cli"),
        (cwd.clone(), "@napi-rs/wasm-runtime"),
        (cwd.clone(), "vite-plus"),
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

pub fn resolve_options() -> oxc_resolver::ResolveOptions {
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

pub fn oxc_resolver_memory() -> oxc_resolver::ResolverGeneric<BenchMemoryFS> {
    use oxc_resolver::ResolverGeneric;
    let fs = BenchMemoryFS::new();
    ResolverGeneric::new_with_file_system(fs, resolve_options())
}

pub mod memory_fs {
    //! Memory-based file system implementation for benchmarks.
    //!
    //! This module provides an in-memory file system that loads all fixture data
    //! and node_modules packages at initialization time, eliminating filesystem I/O
    //! variance during benchmark execution.

    use std::{
        fs, io,
        path::{Path, PathBuf},
        sync::{Arc, LazyLock},
    };

    use oxc_resolver::{FileMetadata, FileSystem, ResolveError};
    use rustc_hash::{FxHashMap, FxHashSet};
    use walkdir::WalkDir;

    /// Memory-based file system for benchmarks to eliminate I/O variance
    #[derive(Clone)]
    pub struct BenchMemoryFS {
        snapshot: Arc<Snapshot>,
    }

    struct Snapshot {
        files: FxHashMap<PathBuf, Vec<u8>>,
        directories: FxHashSet<PathBuf>,
        symlinks: FxHashMap<PathBuf, PathBuf>,
    }

    static BENCH_FS: LazyLock<Arc<Snapshot>> = LazyLock::new(|| {
        let mut fs = Snapshot {
            files: FxHashMap::default(),
            directories: FxHashSet::default(),
            symlinks: FxHashMap::default(),
        };
        fs.load_fixtures();
        Arc::new(fs)
    });

    impl BenchMemoryFS {
        /// Share the immutable fixtures; resolver construction must not copy the entire filesystem.
        pub fn new() -> Self {
            Self { snapshot: Arc::clone(&BENCH_FS) }
        }
    }

    impl Snapshot {
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
            self.load_directory_tree(&cwd.join("fixtures/enhanced-resolve"));
            self.load_directory_tree(&cwd.join("fixtures/tsconfig/bench-tsconfig-paths"));

            // Load specific node_modules packages for benchmarks
            self.load_node_modules_packages(&cwd);

            // Create symlink fixtures for benchmark (10000 symlinks)
            self.create_symlink_fixtures(&cwd);
        }

        fn load_directory_tree(&mut self, root: &Path) {
            if !root.exists() {
                return;
            }
            let generated_symlinks = root.join("test/temp_symlinks");
            for entry in WalkDir::new(root)
                .follow_links(false)
                .sort_by_file_name()
                .into_iter()
                // Local OS benchmarks may have run first. Always use the synthetic symlink fixture.
                .filter_entry(|entry| entry.path() != generated_symlinks)
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

        fn load_node_modules_packages(&mut self, cwd: &Path) {
            let node_modules = cwd.join("node_modules");
            if !node_modules.exists() {
                return;
            }

            // Only load these specific packages needed for benchmarks
            let packages =
                ["@napi-rs/cli", "@napi-rs/wasm-runtime", "vite-plus", "emnapi", "typescript"];

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
                .sort_by_file_name()
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
            if let Some(bytes) = self.snapshot.files.get(path) {
                return Ok(bytes.clone());
            }

            // Try following symlinks
            let mut current = path.to_path_buf();
            let mut visited = FxHashSet::default();

            while let Some(target) = self.snapshot.symlinks.get(&current) {
                if !visited.insert(current.clone()) {
                    return Err(io::Error::other("Circular symlink"));
                }

                current = if target.is_relative() {
                    current.parent().unwrap().join(target)
                } else {
                    target.clone()
                };

                if let Some(bytes) = self.snapshot.files.get(&current) {
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
            if self.snapshot.files.contains_key(path) {
                return Ok(FileMetadata::new(true, false, false));
            }

            // Check if it's a directory (direct)
            if self.snapshot.directories.contains(path) {
                return Ok(FileMetadata::new(false, true, false));
            }

            // Follow symlinks to find the target
            let mut current = path.to_path_buf();
            let mut visited = FxHashSet::default();

            while let Some(target) = self.snapshot.symlinks.get(&current) {
                if !visited.insert(current.clone()) {
                    return Err(io::Error::other("Circular symlink"));
                }

                current = if target.is_relative() {
                    current.parent().unwrap().join(target)
                } else {
                    target.clone()
                };

                if self.snapshot.files.contains_key(&current) {
                    return Ok(FileMetadata::new(true, false, false));
                } else if self.snapshot.directories.contains(&current) {
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
            if self.snapshot.symlinks.contains_key(path) {
                return Ok(FileMetadata::new(false, false, true));
            }

            // Otherwise, fall back to regular metadata
            self.metadata(path)
        }

        fn read_link(&self, path: &Path) -> Result<PathBuf, ResolveError> {
            self.snapshot.symlinks.get(path).cloned().ok_or_else(|| {
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

            while let Some(target) = self.snapshot.symlinks.get(&current) {
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
            if self.snapshot.files.contains_key(&current)
                || self.snapshot.directories.contains(&current)
            {
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
