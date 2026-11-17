//! Tsconfig tests: auto discovery of the tsconfig that owns an importer.

use std::{env, path::PathBuf};

use oxc_resolver::{ResolveOptions, Resolver, TsconfigDiscovery};

fn fixture() -> PathBuf {
    env::current_dir().unwrap().join("fixtures/integration/tsconfig-symlink")
}

fn resolver() -> Resolver {
    Resolver::new(ResolveOptions {
        extensions: vec![".ts".into()],
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    })
}

/// `link` is a symlink to `real`. An importer below `link` is matched against `files`, `include`
/// and `exclude` under the path it was given, so those are anchored at the directory each
/// tsconfig was reached through, not at its real directory.
#[test]
fn auto_discovery_through_symlinked_directory() {
    let dir = fixture();
    let resolver = resolver();

    // `own` declares `include: ["src"]` itself; `inherited` gets `include: ["../src"]` from
    // `config/tsconfig.base.json`.
    for project in ["own", "inherited"] {
        for root in ["link", "real"] {
            let importer = dir.join(root).join(project).join("src/a.ts");
            let resolved_path = resolver.resolve_file(&importer, "@/b").map(|r| r.full_path());
            assert_eq!(
                resolved_path,
                Ok(dir.join("real").join(project).join("src/b.ts")),
                "{project} through {root}"
            );
        }
    }
}

/// `package/node_modules/shared-config` is a symlink to `shared-config`. A base config that
/// `extends` finds as a package is read from its real path, as TypeScript does, so the
/// `include: ["../package/src"]` of `shared-config/base.json` is relative to `shared-config`.
#[test]
fn auto_discovery_with_base_config_from_symlinked_package() {
    let dir = fixture().join("real/package");
    let resolved_path = resolver().resolve_file(dir.join("src/a.ts"), "@/b").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(dir.join("src/b.ts")));
}
