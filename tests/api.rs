//! Test public APIs

use std::{env, path::PathBuf};

use oxc_resolver::{
    AliasValue, EnforceExtension, Resolution, ResolveContext, ResolveError, ResolveOptions,
    Resolver,
};

fn dir() -> PathBuf {
    env::current_dir().unwrap()
}

fn fixture() -> PathBuf {
    dir().join("fixtures/integration")
}

fn resolve(specifier: &str) -> Resolution {
    let path = fixture();
    Resolver::new(ResolveOptions::default()).resolve(path, specifier).unwrap()
}

#[test]
fn clone() {
    let resolution = resolve("./package.json");
    assert_eq!(resolution.clone(), resolution);
}

#[test]
fn debug() {
    let resolution = resolve("./package.json");
    let s = format!("{resolution:?}");
    assert_ne!(s, "");
}

#[test]
fn eq() {
    let resolution = resolve("./package.json");
    assert_eq!(resolution, resolution);
}

#[test]
fn package_json() {
    let resolution = resolve("./package.json");
    let package_json = resolution.package_json().unwrap();
    assert_eq!(package_json.name().unwrap(), "name");
    assert_eq!(package_json.r#type().unwrap().to_string(), "module".to_string());
    assert_eq!(package_json.side_effects(), None);
}

#[test]
fn tsconfig() {
    let resolver = Resolver::new(ResolveOptions::default());
    let tsconfig = resolver.resolve_tsconfig("./fixtures/integration").unwrap();
    assert!(tsconfig.root);
    assert_eq!(tsconfig.path, PathBuf::from("./fixtures/integration/tsconfig.json"));
}

#[test]
fn tsconfig_extends_self_reference() {
    let resolver = Resolver::new(ResolveOptions::default());
    let err = resolver
        .resolve_tsconfig("./fixtures/integration/tsconfig_self_reference.json")
        .unwrap_err();
    assert_eq!(
        err,
        ResolveError::TsconfigCircularExtend(
            vec![
                "./fixtures/integration/tsconfig_self_reference.json".into(),
                "./fixtures/integration/tsconfig_self_reference.json".into()
            ]
            .into()
        )
    );
}

#[test]
fn tsconfig_extends_circular_reference() {
    let resolver = Resolver::new(ResolveOptions::default());
    let err = resolver
        .resolve_tsconfig("./fixtures/integration/tsconfig_circular_reference_a.json")
        .unwrap_err();
    assert_eq!(
        err,
        ResolveError::TsconfigCircularExtend(
            vec![
                "./fixtures/integration/tsconfig_circular_reference_a.json".into(),
                "./fixtures/integration/tsconfig_circular_reference_b.json".into(),
                "./fixtures/integration/tsconfig_circular_reference_a.json".into(),
            ]
            .into()
        )
    );
}

#[test]
fn clear_cache() {
    use std::sync::Arc;

    let resolver = Resolver::new(ResolveOptions::default());
    let resolve = || resolver.resolve(fixture(), "./package.json").unwrap();

    let first = resolve();
    let cached = resolve();
    assert!(Arc::ptr_eq(first.package_json().unwrap(), cached.package_json().unwrap()));

    resolver.clear_cache();

    // `first` is still alive, so a newly parsed package.json cannot reuse its allocation.
    let fresh = resolve();
    assert_eq!(fresh.full_path(), first.full_path());
    assert!(!Arc::ptr_eq(first.package_json().unwrap(), fresh.package_json().unwrap()));
}

#[test]
fn options() {
    let resolver = Resolver::new(ResolveOptions::default());
    let options = resolver.options();
    assert_ne!(format!("{options:?}"), "");
}

#[test]
fn debug_resolver() {
    let resolver = Resolver::new(ResolveOptions::default());
    assert_ne!(format!("{resolver:?}"), "");
}

#[test]
fn dependencies() {
    let path = fixture();
    let mut ctx = ResolveContext::default();
    let _ = Resolver::new(ResolveOptions::default()).resolve_with_context(
        path,
        "./package.json",
        None,
        &mut ctx,
    );
    assert!(!ctx.file_dependencies.is_empty());
    assert!(ctx.missing_dependencies.is_empty());
}

#[test]
fn options_api() {
    // Every value differs from the default, so each assertion can fail.
    let options = ResolveOptions::default()
        .with_builtin_modules(true)
        .with_condition_names(&["node"])
        .with_extension(".ts")
        .with_force_extension(EnforceExtension::Enabled)
        .with_fully_specified(true)
        .with_main_field("asdf")
        .with_main_file("main")
        .with_module("module")
        .with_prefer_absolute(true)
        .with_prefer_relative(true)
        .with_root(PathBuf::from("root"))
        .with_symbolic_link(false);

    assert!(options.builtin_modules);
    assert_eq!(options.condition_names, ["node"]);
    assert_eq!(options.extensions, [".js", ".json", ".node", ".ts"]);
    assert_eq!(options.enforce_extension, EnforceExtension::Enabled);
    assert!(options.fully_specified);
    assert_eq!(options.main_fields, ["main", "asdf"]);
    assert_eq!(options.main_files, ["index", "main"]);
    assert_eq!(options.modules, ["node_modules", "module"]);
    assert!(options.prefer_absolute);
    assert!(options.prefer_relative);
    assert_eq!(options.roots, [PathBuf::from("root")]);
    assert!(!options.symlinks);
}

#[test]
fn error_display_circular_path_bufs() {
    let resolver = Resolver::new(ResolveOptions::default());
    let err = resolver
        .resolve_tsconfig("./fixtures/integration/tsconfig_circular_reference_a.json")
        .unwrap_err();
    let display = format!("{err}");
    assert!(display.contains("circularly"), "{display}");
    assert!(display.contains(" -> "), "should use arrow separator: {display}");

    // Test the inner paths accessor
    if let ResolveError::TsconfigCircularExtend(circular) = &err {
        assert_eq!(circular.paths().len(), 3);
    } else {
        panic!("Expected TsconfigCircularExtend");
    }
}

#[test]
fn error_display_package_path_not_exported() {
    let f = dir().join("fixtures/enhanced-resolve/test/fixtures/exports-field");
    // Single condition name
    let resolver = Resolver::new(ResolveOptions {
        condition_names: vec!["webpack".into()],
        fully_specified: true,
        ..ResolveOptions::default()
    });
    let err = resolver.resolve(&f, "exports-field/anything/else").unwrap_err();
    let display = format!("{err}");
    assert!(display.contains("is not exported"), "{display}");
    assert!(display.contains(r#""webpack""#), "should show condition name: {display}");

    // Multiple conditions
    let resolver2 = Resolver::new(ResolveOptions {
        condition_names: vec!["webpack".into(), "node".into()],
        fully_specified: true,
        ..ResolveOptions::default()
    });
    let err2 = resolver2.resolve(&f, "exports-field/anything/else").unwrap_err();
    let display2 = format!("{err2}");
    assert!(display2.contains("conditions"), "should say 'conditions' for multiple: {display2}");
}

#[test]
fn clone_with_options_recompiles_alias() {
    let fixture = dir().join("fixtures/enhanced-resolve/test/fixtures");

    let base_resolver = Resolver::new(ResolveOptions {
        alias: vec![("alias-target".into(), vec![AliasValue::from("./a")])],
        ..ResolveOptions::default()
    });

    let cloned_resolver = base_resolver.clone_with_options(ResolveOptions {
        alias: vec![("alias-target".into(), vec![AliasValue::from("./b")])],
        ..ResolveOptions::default()
    });

    let base = base_resolver.resolve(&fixture, "alias-target").unwrap().into_path_buf();
    let cloned = cloned_resolver.resolve(&fixture, "alias-target").unwrap().into_path_buf();

    assert_eq!(base, fixture.join("a.js"));
    assert_eq!(cloned, fixture.join("b.js"));
}

#[test]
fn literal_package_keys() {
    let dir = fixture().join("literal-request");
    let file = dir.join("index.cjs");
    for (condition, target) in [("import", "package.json"), ("require", "index.cjs")] {
        let resolver = Resolver::new(ResolveOptions {
            condition_names: vec![condition.into()],
            ..ResolveOptions::default()
        });
        for key in ["#choice?raw", "literal-request/item?raw"] {
            let result = resolver.resolve_file_literal(&file, key).unwrap();
            assert_eq!(result.path(), dir.join(target), "{key}");
            assert_eq!((result.query(), result.fragment()), (None, None), "{key}");
        }
    }
}

#[test]
fn literal_request_requires_the_whole_key() {
    let dir = fixture().join("literal-request");
    let file = dir.join("index.cjs");
    let resolver = Resolver::default();
    for (key, query, fragment) in [
        ("#plain?raw", Some("?raw"), None),
        ("literal-request/plain?raw", Some("?raw"), None),
        ("./index.cjs?raw", Some("?raw"), None),
        ("./index.cjs#raw", None, Some("#raw")),
    ] {
        assert!(
            matches!(
                resolver.resolve_file_literal(&file, key),
                Err(ResolveError::PackageImportNotDefined(..)
                    | ResolveError::PackagePathNotExported { .. }
                    | ResolveError::NotFound(_))
            ),
            "{key}"
        );
        let parsed = resolver.resolve_file(&file, key).unwrap();
        assert_eq!(parsed.path(), file, "{key}");
        assert_eq!((parsed.query(), parsed.fragment()), (query, fragment), "{key}");
    }
    assert!(matches!(resolver.resolve_file_literal(&file, ""), Err(ResolveError::Specifier(_))));
}

#[test]
fn literal_request_preserves_target_query_and_fragment() {
    let dir = fixture().join("literal-request");
    let file = dir.join("index.cjs");
    let resolver = Resolver::default();
    for (key, query, fragment) in [
        ("#star/index.cjs?raw#part", "?raw", "#part"),
        ("literal-request/star/index.cjs?raw#part", "?raw", "#part"),
        ("literal-request/target?raw", "?mode=raw", "#entry"),
    ] {
        let result = resolver.resolve_file_literal(&file, key).unwrap();
        assert_eq!(result.path(), file, "{key}");
        assert_eq!(result.query(), Some(query), "{key}");
        assert_eq!(result.fragment(), Some(fragment), "{key}");
    }
}

#[test]
fn literal_alias_preserves_package_subpaths() {
    let dir = fixture().join("literal-request");
    let file = dir.join("index.cjs");
    let resolver = Resolver::new(ResolveOptions {
        alias: vec![
            ("alias".into(), vec!["literal-request".into()]),
            ("dotted".into(), vec!["literal-request/.".into()]),
            ("configured".into(), vec!["alias?mode=raw#entry".into()]),
            ("wild/*".into(), vec!["literal-request/*?mode=raw#entry".into()]),
            ("query/*".into(), vec!["literal-request/plain?mode=*#entry".into()]),
            ("exact$".into(), vec!["literal-request/plain?mode=raw#entry".into()]),
        ],
        fallback: vec![("fallback/*".into(), vec!["literal-request/*".into()])],
        condition_names: vec!["require".into()],
        ..ResolveOptions::default()
    });
    for subpath in ["item?raw", "item#part"] {
        let direct =
            resolver.resolve_file_literal(&file, &format!("literal-request/{subpath}")).unwrap();
        for (alias, query, fragment) in [
            ("alias", None, None),
            ("configured", Some("?mode=raw"), Some("#entry")),
            ("wild", Some("?mode=raw"), Some("#entry")),
            ("fallback", None, None),
        ] {
            let request = format!("{alias}/{subpath}");
            let result = resolver.resolve_file_literal(&file, &request).unwrap();
            assert_eq!(result.path(), direct.path(), "{request}");
            assert_eq!((result.query(), result.fragment()), (query, fragment), "{request}");
        }
    }
    for (request, query) in [("exact", "?mode=raw"), ("query/item?raw", "?mode=item?raw")] {
        let result = resolver.resolve_file_literal(&file, request).unwrap();
        assert_eq!(result.path(), file, "{request}");
        assert_eq!((result.query(), result.fragment()), (Some(query), Some("#entry")), "{request}");
    }
    for request in [
        "alias/./plain",
        "alias/missing/../item?raw",
        "alias/./item#part",
        "dotted/plain",
        "dotted/item?raw",
    ] {
        let result = resolver.resolve_file_literal(&file, request).unwrap();
        assert_eq!(result.path(), file, "{request}");
        assert_eq!((result.query(), result.fragment()), (None, None), "{request}");
    }
    let parsed = resolver.resolve_file(&file, "fallback/plain?raw#part").unwrap();
    assert_eq!(parsed.path(), file);
    assert_eq!((parsed.query(), parsed.fragment()), (Some("?raw"), Some("#part")));
}

#[test]
fn literal_wildcard_alias_preserves_configured_fragment_lookup() {
    let file = fixture().join("literal-request/index.cjs");
    let resolver = Resolver::new(ResolveOptions {
        alias: vec![("fragment/*".into(), vec!["literal-request/*#part".into()])],
        condition_names: vec!["require".into()],
        ..ResolveOptions::default()
    });
    for (key, fragment) in [("item", None), ("plain", Some("#part")), ("item?raw", None)] {
        let result = resolver.resolve_file_literal(&file, &format!("fragment/{key}")).unwrap();
        assert_eq!(result.path(), file, "{key}");
        assert_eq!((result.query(), result.fragment()), (None, fragment), "{key}");
    }
    assert!(matches!(
        resolver.resolve_file_literal(&file, "fragment/plain?missing"),
        Err(ResolveError::PackagePathNotExported { .. })
    ));
}

#[test]
fn literal_import_redirect_preserves_package_subpath() {
    let dir = fixture();
    let file = dir.join("literal-request/index.cjs");
    let resolver = Resolver::new(ResolveOptions {
        exports_fields: vec![],
        modules: vec![dir.to_string_lossy().into_owned()],
        ..ResolveOptions::default()
    });
    assert_eq!(resolver.resolve_file_literal(&file, "#redirect/index.cjs").unwrap().path(), file);
    assert!(matches!(
        resolver.resolve_file_literal(&file, "#redirect/index.cjs?raw"),
        Err(ResolveError::NotFound(..))
    ));
}
