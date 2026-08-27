#![cfg(not(target_os = "wasi"))]

use std::{env, path::PathBuf, process::Command};

use oxc_resolver::{
    ResolveError, ResolveOptions, Resolver, TsconfigDiscovery, TsconfigOptions, TsconfigReferences,
};

const CHILD_CASE: &str = "OXC_RESOLVER_PACKAGE_MAP_TEST_CASE";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture(path: &str) -> PathBuf {
    root().join("fixtures").join(path)
}

fn escape_node_option(path: &str) -> String {
    path.replace('\\', "\\\\").replace('"', "\\\"")
}

fn node_options(case: &str) -> String {
    let map = match case {
        "resolution" => "./fixtures/package-map/resolution/node_modules/.package-map.json".into(),
        "owners" => fixture("package-map/find-package-id/.package-map.json"),
        "invalid-json" => fixture("package-map/invalid/.package-map.json"),
        "invalid-shape" => fixture("package-map/invalid/invalid-shape.package-map.json"),
        "missing" => fixture("package-map/invalid/missing.package-map.json"),
        "symlink" => fixture("integration/nested-symlink/apps/tooling/.package-map.json"),
        "no-map" => return "--trace-warnings".into(),
        "empty" => {
            return "--experimental-package-map=valid.json --experimental-package-map=".into();
        }
        "unterminated" => return "--experimental-package-map=\"unterminated".into(),
        "trailing-escape" => return "--experimental-package-map=\"trailing\\".into(),
        _ => unreachable!(),
    };
    let map = escape_node_option(&map.to_string_lossy());
    match case {
        "resolution" => {
            format!(r#"--dummy="escaped\"quote" --experimental-package-map="{map}""#)
        }
        "owners" => format!(r#"--experimental-package-map "{map}""#),
        _ => format!(r#"--experimental-package-map="{map}""#),
    }
}

fn new_resolver(options: ResolveOptions) -> Resolver {
    Resolver::new(options)
}

fn resolution() {
    let fixture = fixture("package-map/resolution");
    let importer = fixture.join("apps/web/src");
    let options = ResolveOptions {
        condition_names: vec!["node".into(), "require".into()],
        ..ResolveOptions::default()
    };
    let resolver = new_resolver(options.clone());

    for (base, specifier, expected) in [
        (&importer, "axios", "node_modules/store/axios/index.js"),
        (&importer, "axios/client", "node_modules/store/axios/lib/client.js"),
        (&importer, "@bench/ui", "packages/ui/src/index.js"),
        (&importer, "@bench/web", "apps/web/src/index.js"),
        (&importer, "#react", "node_modules/store/react/index.js"),
        (&importer, "plain-file", "node_modules/store/plain-file.js"),
        (&importer, "plain-directory", "node_modules/store/plain-directory/index.js"),
        (
            &fixture.join("node_modules/store/axios/lib"),
            "follow-redirects",
            "node_modules/store/follow-redirects/index.js",
        ),
    ] {
        assert_eq!(
            resolver.resolve(base, specifier).map(|resolution| resolution.full_path()),
            Ok(fixture.join(expected)),
        );
    }

    for specifier in [
        "follow-redirects",
        "invalid-target",
        "missing-target",
        "plain-directory/missing",
        "./missing",
    ] {
        assert!(matches!(
            resolver.resolve(&importer, specifier),
            Err(ResolveError::NotFound(not_found)) if not_found == specifier
        ));
    }

    assert_eq!(
        resolver
            .clone_with_options(options.clone())
            .resolve(&importer, "react")
            .map(|resolution| resolution.full_path()),
        Ok(fixture.join("node_modules/store/react/index.js"))
    );
    assert_eq!(
        new_resolver(ResolveOptions { symlinks: false, ..options })
            .resolve(&importer, "react")
            .map(|resolution| resolution.full_path()),
        Ok(fixture.join("node_modules/store/react/index.js"))
    );

    let resolver = new_resolver(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: fixture.join("apps/web/tsconfig.package-map.json"),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });
    for config in ["tsconfig.package-map.json", "tsconfig.package-map-self.json"] {
        resolver.resolve_tsconfig(fixture.join("apps/web").join(config)).unwrap();
    }
}

fn owners() {
    let fixture = fixture("package-map/find-package-id");
    let resolver = new_resolver(ResolveOptions::default());
    for _ in 0..2 {
        assert!(matches!(
            resolver.resolve(fixture.join("packages/duplicate"), "dependency"),
            Err(ResolveError::PackageMapAmbiguousResolution { .. })
        ));
        assert!(matches!(
            resolver.resolve(fixture.join("external"), "dependency"),
            Err(ResolveError::PackageMapExternalFile { .. })
        ));
    }
}

fn invalid() {
    let resolver = new_resolver(ResolveOptions::default());
    for _ in 0..2 {
        assert!(matches!(
            resolver.resolve(fixture("package-map/invalid"), "dependency"),
            Err(ResolveError::Json(_))
        ));
    }
}

fn child(case: &str) {
    match case {
        "resolution" => resolution(),
        "owners" => owners(),
        "invalid-json" | "invalid-shape" => invalid(),
        "missing" => {
            new_resolver(ResolveOptions::default())
                .resolve(fixture("package-map/invalid"), "dependency")
                .unwrap_err();
        }
        "symlink" => {
            let fixture = fixture("integration/nested-symlink");
            if fixture.join("apps/tooling/.package-map.json").is_file() {
                assert_eq!(
                    new_resolver(ResolveOptions::default())
                        .resolve(fixture.join("tooling/typescript-config"), "dep")
                        .map(|resolution| resolution.full_path()),
                    Ok(fixture.join("nm/index.js"))
                );
            }
        }
        "no-map" | "empty" | "unterminated" | "trailing-escape" => {
            assert!(!new_resolver(ResolveOptions::default()).options().modules.is_empty());
        }
        _ => unreachable!(),
    }
}

#[test]
fn package_map() {
    if let Ok(case) = env::var(CHILD_CASE) {
        child(&case);
        return;
    }

    for case in [
        "resolution",
        "owners",
        "invalid-json",
        "invalid-shape",
        "missing",
        "symlink",
        "no-map",
        "empty",
        "unterminated",
        "trailing-escape",
    ] {
        let output = Command::new(env::current_exe().unwrap())
            .args(["--exact", "package_map", "--nocapture"])
            .current_dir(root())
            .env(CHILD_CASE, case)
            .env("NODE_OPTIONS", node_options(case))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{case} failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}
