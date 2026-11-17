//! Tsconfig tests: auto discovery of the tsconfig that owns an importer.

use std::{env, path::PathBuf};

use oxc_resolver::{ResolveOptions, Resolver, TsconfigDiscovery};

fn fixture() -> PathBuf {
    env::current_dir().unwrap().join("fixtures/integration")
}

/// `link` is a symlink to `real`. An importer below `link` is matched against `files`, `include`
/// and `exclude` under the path it was given, so those are anchored at the directory each
/// tsconfig was reached through, not at its real directory.
#[test]
fn auto_discovery_through_symlinked_directory() {
    let dir = fixture().join("tsconfig-symlink");
    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".ts".into()],
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });

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
