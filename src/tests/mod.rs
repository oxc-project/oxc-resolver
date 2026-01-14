mod alias;
mod browser_field;
mod builtins;
mod clear_cache;
mod dependencies;
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
mod tsconfig_paths;
mod tsconfig_project_references;
mod tsconfig_root_dirs;
#[cfg(target_os = "windows")]
mod windows;

use std::{path::PathBuf, sync::Arc, thread};

use crate::Resolver;

pub fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

pub fn fixture() -> PathBuf {
    fixture_root().join("enhanced-resolve").join("test").join("fixtures")
}

#[test]
#[cfg_attr(target_os = "wasi", ignore)]
fn threaded_environment() {
    let cwd = fixture_root();
    let resolver = Arc::new(Resolver::default());
    for _ in 0..2 {
        _ = thread::spawn({
            let cwd = cwd.clone();
            let resolver = Arc::clone(&resolver);
            move || {
                _ = resolver.resolve(cwd, ".");
            }
        })
        .join();
    }
}
