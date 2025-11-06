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

        // Load fixtures from enhanced_resolve
        let fixtures_base = cwd.join("fixtures/enhanced_resolve");
        if fixtures_base.exists() {
            for entry in
                WalkDir::new(&fixtures_base).follow_links(false).into_iter().filter_map(Result::ok)
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
        let packages = ["@napi-rs/cli", "@napi-rs/wasm-runtime", "vitest", "emnapi", "typescript"];

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
        let temp_path = cwd.join("fixtures/enhanced_resolve/test/temp_symlinks");
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

        Err(io::Error::new(io::ErrorKind::NotFound, format!("File not found: {}", path.display())))
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

        Err(io::Error::new(io::ErrorKind::NotFound, format!("Path not found: {}", path.display())))
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
}
