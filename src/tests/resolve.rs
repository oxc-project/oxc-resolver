//! <https://github.com/webpack/enhanced-resolve/blob/main/test/resolve.test.js>

use crate::{
    Resolution, ResolveError, ResolveOptions, Resolver, TsconfigDiscovery, TsconfigOptions,
    TsconfigReferences, package_map::package_map_path_from_node_options,
};

#[test]
fn resolve() {
    let f = super::fixture();

    let resolver = Resolver::default();

    let main1_js_path = f.join("main1.js").to_string_lossy().to_string();
    let m2 = f.join("node_modules").join("m2");
    let m2_specifier = m2.to_string_lossy().to_string();
    let m2_trailing_slash = m2_specifier.clone() + "/";

    #[rustfmt::skip]
    let pass = [
        ("absolute path", f.clone(), main1_js_path.as_str(), f.join("main1.js")),
        ("absolute path to package", f.clone(), m2_specifier.as_str(), m2.join("b.js")),
        ("absolute path to package with trailing slash", f.clone(), m2_trailing_slash.as_str(), m2.join("b.js")),
        ("file with .js", f.clone(), "./main1.js", f.join("main1.js")),
        ("file without extension", f.clone(), "./main1", f.join("main1.js")),
        ("another file with .js", f.clone(), "./a.js", f.join("a.js")),
        ("another file without extension", f.clone(), "./a", f.join("a.js")),
        ("file in module with .js", f.clone(), "m1/a.js", f.join("node_modules/m1/a.js")),
        ("file in module without extension", f.clone(), "m1/a", f.join("node_modules/m1/a.js")),
        ("another file in module without extension", f.clone(), "complexm/step1", f.join("node_modules/complexm/step1.js")),
        ("from submodule to file in sibling module", f.join("node_modules/complexm"), "m2/b.js", f.join("node_modules/m2/b.js")),
        ("from nested directory to overwritten file in module", f.join("multiple-modules"), "m1/a.js", f.join("multiple-modules/node_modules/m1/a.js")),
        ("from nested directory to not overwritten file in module", f.join("multiple-modules"), "m1/b.js", f.join("node_modules/m1/b.js")),
        ("file with query", f.clone(), "./main1.js?query", f.join("main1.js?query")),
        ("file with fragment", f.clone(), "./main1.js#fragment", f.join("main1.js#fragment")),
        ("file with fragment and query", f.clone(), "./main1.js#fragment?query", f.join("main1.js#fragment?query")),
        ("file with query and fragment", f.clone(), "./main1.js?#fragment", f.join("main1.js?#fragment")),

        ("file with query (unicode)", f.clone(), "./测试.js?query", f.join("测试.js?query")),
        ("file with fragment (unicode)", f.clone(), "./测试.js#fragment", f.join("测试.js#fragment")),
        ("file with fragment and query (unicode)", f.clone(), "./测试.js#fragment?query", f.join("测试.js#fragment?query")),
        ("file with query and fragment (unicode)", f.clone(), "./测试.js?#fragment", f.join("测试.js?#fragment")),

        ("file in module with query", f.clone(), "m1/a?query", f.join("node_modules/m1/a.js?query")),
        ("file in module with fragment", f.clone(), "m1/a#fragment", f.join("node_modules/m1/a.js#fragment")),
        ("file in module with fragment and query", f.clone(), "m1/a#fragment?query", f.join("node_modules/m1/a.js#fragment?query")),
        ("file in module with query and fragment", f.clone(), "m1/a?#fragment", f.join("node_modules/m1/a.js?#fragment")),
        ("differ between directory and file, resolve file", f.clone(), "./dir-or-file", f.join("dir-or-file.js")),
        ("differ between directory and file, resolve directory", f.clone(), "./dir-or-file/", f.join("dir-or-file/index.js")),
        ("find node_modules outside of node_modules", f.join("browser-module/node_modules"), "m1/a", f.join("node_modules/m1/a.js")),
        ("don't crash on main field pointing to self", f.clone(), "./main-field-self", f.join("./main-field-self/index.js")),
        ("don't crash on main field pointing to self (2)", f.clone(), "./main-field-self2", f.join("./main-field-self2/index.js")),
        // enhanced-resolve has `#` prepended with a `\0`, they are removed from the
        // following 3 expected test results.
        // See https://github.com/webpack/enhanced-resolve#escaping
        ("handle fragment edge case (no fragment)", f.clone(), "./no#fragment/#/#", f.join("no#fragment/#/#.js")),
        ("handle fragment edge case (fragment)", f.clone(), "./no#fragment/#/", f.join("no.js#fragment/#/")),
        ("handle fragment escaping", f.clone(), "./no\0#fragment/\0#/\0##fragment", f.join("no#fragment/#/#.js#fragment")),
        // Test `node_modules/X/foo/` and `node_modules/X/foo.js` precedence.
        ("file and dir precedence 1", f.clone(), "dir-and-file/foo", f.join("node_modules/dir-and-file/foo.js")),
        ("file and dir precedence 2", f.clone(), "@scope/dir-and-file/foo", f.join("node_modules/@scope/dir-and-file/foo.js")),
        ("file and dir precedence 1", f.clone(), "dir-and-file/foo/", f.join("node_modules/dir-and-file/foo/index.js")),
        ("file and dir precedence 2", f.clone(), "@scope/dir-and-file/foo/", f.join("node_modules/@scope/dir-and-file/foo/index.js")),
    ];

    for (comment, path, request, expected) in pass {
        let resolution = resolver.resolve(&path, request).ok();
        let resolved_path = resolution.as_ref().map(Resolution::full_path);
        let resolved_package_json =
            resolution.as_ref().and_then(|r| r.package_json()).map(|p| p.path.clone());
        if expected.to_str().unwrap().contains("node_modules") {
            assert!(resolved_package_json.is_some(), "{comment} {path:?} {request}");
        }
        assert_eq!(resolved_path, Some(expected), "{comment} {path:?} {request}");
    }
}

#[test]
fn issue238_resolve() {
    let f = super::fixture().join("issue-238");
    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".jsx".into(), ".ts".into(), ".tsx".into()],
        modules: vec!["src/a".into(), "src/b".into(), "src/common".into(), "node_modules".into()],
        ..ResolveOptions::default()
    });
    let resolved_path =
        resolver.resolve(f.join("src/common"), "config/myObjectFile").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("src/common/config/myObjectFile.js")));
}

#[test]
fn package_map_resolution() {
    let fixture = super::fixture_root().join("package-map/resolution");
    let importer = fixture.join("apps/web/src");
    let package_map = fixture.join("node_modules/.package-map.json");
    let options = ResolveOptions {
        condition_names: vec!["node".into(), "require".into()],
        ..ResolveOptions::default()
    };
    let resolver = Resolver::new_with_package_map(options.clone(), package_map.clone());

    for (base, request, expected) in [
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
            resolver.resolve(base, request).map(|resolution| resolution.full_path()),
            Ok(fixture.join(expected)),
            "failed to resolve {request:?}",
        );
    }

    for request in
        ["follow-redirects", "invalid-target", "missing-target", "plain-directory/missing"]
    {
        assert!(matches!(
            resolver.resolve(&importer, request),
            Err(ResolveError::NotFound(specifier)) if specifier == request
        ));
    }

    let cloned = resolver.clone_with_options(options.clone());
    assert_eq!(
        cloned.resolve(&importer, "react").map(|resolution| resolution.full_path()),
        Ok(fixture.join("node_modules/store/react/index.js"))
    );

    let without_symlinks =
        Resolver::new_with_package_map(ResolveOptions { symlinks: false, ..options }, package_map);
    assert_eq!(
        without_symlinks.resolve(&importer, "react").map(|resolution| resolution.full_path()),
        Ok(fixture.join("node_modules/store/react/index.js"))
    );
}

#[test]
fn package_map_resolution_errors() {
    let fixture = super::fixture_root().join("package-map/find-package-id");
    let resolver = Resolver::new_with_package_map(
        ResolveOptions::default(),
        fixture.join(".package-map.json"),
    );
    assert!(matches!(
        resolver.resolve(fixture.join("packages/duplicate"), "dependency"),
        Err(ResolveError::PackageMapAmbiguousResolution { .. })
    ));
    assert!(matches!(
        resolver.resolve(fixture.join("external"), "dependency"),
        Err(ResolveError::PackageMapExternalFile { .. })
    ));

    let fixture = super::fixture_root().join("package-map/invalid");
    for package_map in [".package-map.json", "invalid-shape.package-map.json"] {
        let resolver =
            Resolver::new_with_package_map(ResolveOptions::default(), fixture.join(package_map));
        for _ in 0..2 {
            assert!(matches!(resolver.resolve(&fixture, "dependency"), Err(ResolveError::Json(_))));
        }
    }

    let resolver = Resolver::new_with_package_map(
        ResolveOptions::default(),
        fixture.join("missing.package-map.json"),
    );
    resolver.resolve(&fixture, "dependency").unwrap_err();
}

#[test]
fn package_map_resolves_tsconfig_extends() {
    let fixture = super::fixture_root().join("package-map/resolution");
    let resolver = Resolver::new_with_package_map(
        ResolveOptions {
            condition_names: vec!["node".into(), "require".into()],
            tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
                config_file: fixture.join("apps/web/tsconfig.package-map.json"),
                references: TsconfigReferences::Auto,
            })),
            ..ResolveOptions::default()
        },
        fixture.join("node_modules/.package-map.json"),
    );

    for config_file in ["tsconfig.package-map.json", "tsconfig.package-map-self.json"] {
        resolver
            .resolve_tsconfig(fixture.join("apps/web").join(config_file))
            .expect("resolve tsconfig through package map");
    }
}

#[test]
#[cfg_attr(target_os = "wasi", ignore)]
fn package_map_resolves_from_canonical_location() {
    let fixture = super::fixture_root().join("integration/nested-symlink");
    let package_map = fixture.join("apps/tooling/.package-map.json");

    // Some Windows checkouts materialize repository symlinks as plain files.
    if !package_map.is_file() {
        return;
    }

    let resolver = Resolver::new_with_package_map(ResolveOptions::default(), package_map);
    assert_eq!(
        resolver
            .resolve(fixture.join("tooling/typescript-config"), "dep")
            .map(|resolution| resolution.full_path()),
        Ok(fixture.join("nm/index.js"))
    );
}

#[test]
fn package_map_node_options() {
    let cwd = super::fixture_root();
    assert_eq!(package_map_path_from_node_options("--trace-warnings", &cwd), None);
    assert_eq!(
        package_map_path_from_node_options(
            "--experimental-package-map=./first.json \
             --experimental-package-map=\"./last map.json\"",
            &cwd,
        ),
        Some(cwd.join("last map.json"))
    );
    assert_eq!(
        package_map_path_from_node_options(
            "--experimental-package-map \"./separate map.json\"",
            &cwd,
        ),
        Some(cwd.join("separate map.json"))
    );
    assert_eq!(
        package_map_path_from_node_options(
            r#"--experimental-package-map="./escaped\"quote.json""#,
            &cwd,
        ),
        Some(cwd.join("escaped\"quote.json"))
    );
    assert_eq!(
        package_map_path_from_node_options(
            "--experimental-package-map=./valid.json --experimental-package-map=",
            &cwd,
        ),
        None
    );
    assert_eq!(
        package_map_path_from_node_options(
            "--experimental-package-map=\"./unterminated.json",
            &cwd,
        ),
        None
    );
    assert_eq!(
        package_map_path_from_node_options("--experimental-package-map=\"./trailing\\", &cwd),
        None
    );
}

#[test]
fn prefer_relative() {
    let f = super::fixture();

    let resolver =
        Resolver::new(ResolveOptions { prefer_relative: true, ..ResolveOptions::default() });

    #[rustfmt::skip]
    let pass = [
        ("should correctly resolve with preferRelative 1", "main1.js", f.join("main1.js")),
        ("should correctly resolve with preferRelative 2", "m1/a.js", f.join("node_modules/m1/a.js")),
    ];

    for (comment, request, expected) in pass {
        let resolved_path = resolver.resolve(&f, request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {request}");
    }
}

#[test]
fn prefer_relative_local_over_node_modules() {
    // When both ./main1.js and node_modules/main1 exist, prefer_relative picks local file
    let f = super::fixture().join("prefer-relative");
    let resolver =
        Resolver::new(ResolveOptions { prefer_relative: true, ..ResolveOptions::default() });
    let resolved_path = resolver.resolve(&f, "main1.js").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("main1.js")));
}

#[test]
fn no_prefer_relative_uses_node_modules() {
    // Without prefer_relative, bare specifier goes to node_modules
    let f = super::fixture().join("prefer-relative");
    let resolver = Resolver::default();
    let resolved_path = resolver.resolve(&f, "main1").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("node_modules/main1/index.js")));
}

#[test]
fn resolve_to_context() {
    let f = super::fixture();
    let resolver =
        Resolver::new(ResolveOptions { resolve_to_context: true, ..ResolveOptions::default() });

    #[rustfmt::skip]
    let data = [
        ("context for fixtures", f.clone(), "./", f.clone()),
        ("context for fixtures/lib", f.clone(), "./lib", f.join("lib")),
        ("context for fixtures with ..", f.clone(), "./lib/../../fixtures/./lib/..", f.clone()),
        ("context for fixtures with query", f.clone(), "./?query", f.clone().with_file_name("fixtures?query")),
    ];

    for (comment, path, request, expected) in data {
        let resolved_path = resolver.resolve(&path, request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }
}

#[test]
fn resolve_hash_as_module() {
    let f = super::fixture();
    let resolver = Resolver::default();
    let resolution = resolver.resolve(f, "#a");
    assert_eq!(resolution, Err(ResolveError::NotFound("#a".into())));
}

#[test]
fn resolve_edge_cases() {
    let f = super::fixture();
    let resolver = Resolver::default();

    // Test various edge cases for path resolution
    let data = [("resolve with multiple dots", f.clone(), "./a/../main1.js", f.join("main1.js"))];

    for (comment, path, request, expected) in data {
        let resolved_path = resolver.resolve(&path, request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }
}

#[test]
fn resolve_file_rejects_parentless_path() {
    let resolver = Resolver::default();
    let root = std::env::current_dir()
        .expect("get current dir")
        .ancestors()
        .last()
        .expect("get root ancestor")
        .to_path_buf();

    let error =
        resolver.resolve_file(&root, "./main1.js").expect_err("expected invalid input error");
    let ResolveError::IOError(io_error) = error else {
        panic!("expected IOError, got {error:?}");
    };
    let io_error: std::io::Error = io_error.into();
    assert_eq!(io_error.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn resolve_dot() {
    let f = super::fixture_root().join("integration/dot");
    let foo_dir: std::path::PathBuf = f.join("foo");
    let resolver = Resolver::default();
    let foo_index = foo_dir.join("index.js");
    let data = [
        ("dot dir", foo_dir.clone(), ".", foo_index.clone()),
        ("dot dir slash", foo_dir.clone(), "./", foo_index),
    ];
    for (comment, path, request, expected) in data {
        let resolved_path = resolver.resolve(&path, request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }

    let resolver =
        Resolver::new(ResolveOptions { main_files: vec![], ..ResolveOptions::default() });
    let data = [
        ("dot dir", foo_dir.clone(), ".", ResolveError::NotFound(".".into())),
        ("dot dir slash", foo_dir, "./", ResolveError::NotFound("./".into())),
    ];
    for (comment, path, request, expected) in data {
        let resolve_error = resolver.resolve(&path, request);
        assert_eq!(resolve_error, Err(expected), "{comment} {path:?} {request}");
    }
}

#[test]
fn abnormal_relative() {
    let f = super::fixture_root().join("integration/abnormal-relative-with-node-modules");

    let base = f.join("foo/bar/baz");

    let resolver = Resolver::default();

    let data = [
        ("2-level abnormal relative path 1", "jest-runner-../../.."),
        ("2-level abnormal relative path 2", "jest-runner-../../../"),
        ("2-level abnormal relative path 3", "jest-runner-/../.."),
        ("2-level abnormal relative path 4", "jest-runner-/../../"),
    ];

    for (comment, request) in data {
        let resolved_path = resolver.resolve(&base, request).map(|r| r.full_path()).unwrap();
        assert_eq!(resolved_path, f.join("runner.js"), "{comment} {}", resolved_path.display());
    }

    let data = [
        ("1-level abnormal relative path 1", "jest-runner-../.."),
        ("1-level abnormal relative path 2", "jest-runner-../../"),
        ("1-level abnormal relative path 3", "jest-runner-/.."),
        ("1-level abnormal relative path 4", "jest-runner-/../"),
    ];

    for (comment, request) in data {
        let resolved_path = resolver.resolve(&base, request);
        assert_eq!(
            resolved_path,
            Err(ResolveError::NotFound(request.into())),
            "{comment} {request}"
        );
    }

    let f = super::fixture_root().join("integration/abnormal-relative-without-node-modules");

    let base = f.join("foo/bar/baz");

    let data = [
        ("2-level abnormal relative path 1", "jest-runner-../../.."),
        ("2-level abnormal relative path 2", "jest-runner-../../../"),
        ("2-level abnormal relative path 3", "jest-runner-/../.."),
        ("2-level abnormal relative path 4", "jest-runner-/../../"),
    ];

    for (comment, request) in data {
        let resolved_path = resolver.resolve(&base, request);
        assert_eq!(
            resolved_path,
            Err(ResolveError::NotFound(request.into())),
            "{comment} {request}"
        );
    }
}

#[cfg(windows)]
#[test]
fn resolve_normalized_on_windows() {
    use crate::PathUtil;

    let f = super::fixture();
    let absolute = f.join("./foo/index.js").normalize();
    let absolute_str = absolute.to_string_lossy();
    let normalized_absolute = absolute_str.replace('\\', "/");
    let resolver = Resolver::default();

    let resolution = resolver.resolve(&f, &normalized_absolute).map(|r| r.full_path());
    assert_eq!(
        resolution.map(|r| r.to_string_lossy().into_owned()),
        Ok(absolute_str.clone().into_owned())
    );

    let normalized_f = f.to_str().unwrap().replace('\\', "/");
    let resolution = resolver.resolve(normalized_f, ".\\foo\\index.js").map(|r| r.full_path());
    assert_eq!(
        resolution.map(|r| r.to_string_lossy().into_owned()),
        Ok(absolute_str.clone().into_owned())
    );
}

#[cfg(windows)]
#[test]
fn file_protocol() {
    let f = super::fixture();

    let main1_js_path = f.join("main1.js").to_string_lossy().to_string();
    // Construct file:/// URL manually: forward-slash the path and prepend file:///
    let file_protocol_path = format!("file:///{}", main1_js_path.replace('\\', "/"));

    let resolver = Resolver::default();

    let resolution = resolver.resolve(&f, &file_protocol_path).ok();
    let resolved_path = resolution.as_ref().map(Resolution::full_path);
    assert_eq!(resolved_path, Some(f.join("main1.js")));

    let resolve_error = ResolveError::NotFound("\\\\.\\main.js".into());

    assert_eq!(resolver.resolve(f, "file://./main.js"), Err(resolve_error));
}
