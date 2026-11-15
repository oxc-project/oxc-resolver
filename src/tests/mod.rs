mod alias;
mod browser_field;
mod builtins;
mod dependencies;
mod dts_resolver;
mod exports_field;
mod extension_alias;
mod extensions;
mod fallback;
mod full_specified;
mod imports_field;
mod incorrect_description_file;
mod main_field;
mod memory_fs;
mod memory_leak;
mod missing;
mod module_type;
mod modules;
mod package_json;
#[cfg(feature = "yarn_pnp")]
mod pnp;
mod resolution;
mod resolve;
mod restrictions;
mod roots;
mod scoped_packages;
mod simple;
mod symlink;
mod tsconfck;
mod tsconfig_discovery;
mod tsconfig_extends;
mod tsconfig_lookup;
mod tsconfig_paths;
mod tsconfig_project_references;
mod tsconfig_root_dirs;
#[cfg(target_os = "windows")]
mod windows;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};

use crate::{
    Resolver,
    package_json::{ImportsExportsEntry, ImportsExportsEntryGeneric},
};

pub fn fixture_root() -> PathBuf {
    // In WASI, the filesystem is mounted at `/` so we use `/fixtures`.
    // In native builds, we use CARGO_MANIFEST_DIR.
    #[cfg(target_os = "wasi")]
    {
        PathBuf::from("/fixtures")
    }
    #[cfg(not(target_os = "wasi"))]
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
    }
}

pub fn fixture() -> PathBuf {
    fixture_root().join("enhanced-resolve").join("test").join("fixtures")
}

/// Parses `value` with the JSON backend the resolver uses on this target and leaks it, so that
/// test tables can hold `'static` "imports" / "exports" entries.
#[cfg(target_endian = "little")]
pub fn imports_exports_entry(value: &serde_json::Value) -> ImportsExportsEntry<'static> {
    // Serialize back to JSON string and parse with simd_json for little-endian
    let json_str = serde_json::to_string(value).unwrap();
    let bytes = Box::leak::<'static>(Box::new(json_str.into_bytes()));
    let borrowed = simd_json::to_borrowed_value(bytes).unwrap();
    let value = Box::leak::<'static>(Box::new(borrowed));
    ImportsExportsEntryGeneric(value)
}

/// Parses `value` with the JSON backend the resolver uses on this target and leaks it, so that
/// test tables can hold `'static` "imports" / "exports" entries.
#[cfg(target_endian = "big")]
pub fn imports_exports_entry(value: &serde_json::Value) -> ImportsExportsEntry<'static> {
    // Clone and leak the value to get a 'static reference for big-endian
    let value = Box::leak::<'static>(Box::new(value.clone()));
    ImportsExportsEntryGeneric(value)
}

/// A filesystem wrapper that returns `PermissionDenied` for `read_to_string`
/// on a specific path, delegating everything else to the real OS filesystem.
pub struct UnreadableFs {
    pub unreadable_path: PathBuf,
}

impl crate::FileSystem for UnreadableFs {
    #[cfg(not(feature = "yarn_pnp"))]
    fn new() -> Self {
        unreachable!()
    }

    #[cfg(feature = "yarn_pnp")]
    fn new(_yarn_pnp: bool) -> Self {
        unreachable!()
    }

    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        if path == self.unreadable_path {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "permission denied",
            ));
        }
        crate::FileSystemOs::read_to_string(path)
    }

    fn metadata(&self, path: &Path) -> std::io::Result<crate::FileMetadata> {
        crate::FileSystemOs::metadata(path)
    }

    fn symlink_metadata(&self, path: &Path) -> std::io::Result<crate::FileMetadata> {
        crate::FileSystemOs::symlink_metadata(path)
    }

    fn read_link(&self, path: &Path) -> Result<PathBuf, crate::ResolveError> {
        crate::FileSystemOs::read_link(path)
    }

    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf> {
        crate::FileSystemOs::canonicalize(path)
    }
}

#[test]
#[cfg_attr(target_os = "wasi", ignore)]
fn threaded_environment() {
    let cwd = fixture();
    let resolver = Arc::new(Resolver::default());
    for _ in 0..2 {
        let resolved_path = thread::spawn({
            let cwd = cwd.clone();
            let resolver = Arc::clone(&resolver);
            move || resolver.resolve(cwd, "./main1.js").map(|resolution| resolution.full_path())
        })
        .join()
        .unwrap();
        assert_eq!(resolved_path, Ok(cwd.join("main1.js")));
    }
}
