//! Miscellaneous tests: unicode paths, BOM handling, nested symlinks, NODE_PATH.

use std::{env, path::PathBuf};

use oxc_resolver::{Resolution, ResolveOptions, Resolver};

fn dir() -> PathBuf {
    env::current_dir().unwrap()
}

fn fixture() -> PathBuf {
    dir().join("fixtures/integration")
}

#[test]
fn chinese() {
    let dir = dir();
    let specifier = "./fixtures/misc/中文/中文.js";
    let resolution = Resolver::new(ResolveOptions::default()).resolve(&dir, specifier);
    assert_eq!(
        resolution.map(Resolution::into_path_buf),
        Ok(dir.join("fixtures/misc/中文/中文.js"))
    );
}

#[test]
fn package_json_with_bom() {
    let dir = dir().join("fixtures/misc");
    assert_eq!(
        Resolver::new(ResolveOptions::default())
            .resolve(&dir, "./package-json-with-bom")
            .map(Resolution::into_path_buf),
        Ok(dir.join("package-json-with-bom/index.js"))
    );
}

#[test]
// regression: https://github.com/NicholasLYang/oxc-repro
fn nested_symlinks() {
    let dir = fixture().join("nested-symlink");
    assert_eq!(
        Resolver::new(ResolveOptions::default())
            // ./apps/web/nm/@repo/typescript-config is a symlink
            .resolve(&dir, "./apps/web/nm/@repo/typescript-config/index.js")
            .map(oxc_resolver::Resolution::into_path_buf),
        Ok(dir.join("nm/index.js"))
    );
    assert_eq!(
        Resolver::new(ResolveOptions::default())
            // ./apps/tooling is a symlink
            .resolve(&dir, "./apps/tooling/typescript-config/index.js")
            .map(oxc_resolver::Resolution::into_path_buf),
        Ok(dir.join("nm/index.js"))
    );
}

#[test]
#[cfg_attr(target_family = "wasm", ignore)]
fn node_path_resolves_from_env() {
    let enhanced_resolve = dir().join("fixtures/enhanced-resolve/test/fixtures");
    let project = fixture();
    let node_path_root = enhanced_resolve.join("multiple-modules/node_modules");
    let node_path = env::join_paths([node_path_root]).unwrap();
    // SAFETY: this test sets NODE_PATH before constructing the resolver.
    unsafe {
        env::set_var("NODE_PATH", node_path);
    }

    let expected = enhanced_resolve.join("multiple-modules/node_modules/m1/a.js");
    let resolved = Resolver::default().resolve(&project, "m1/a.js").map(|r| r.full_path());
    assert_eq!(resolved, Ok(expected));
}
