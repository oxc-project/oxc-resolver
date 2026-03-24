//! Test NODE_PATH behavior.

use std::{env, path::PathBuf};

use oxc_resolver::Resolver;

fn dir() -> PathBuf {
    env::current_dir().unwrap()
}

fn fixture() -> PathBuf {
    dir().join("fixtures/integration")
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
