//! Tests for tsconfig lookup scenarios ported from typescript-go.

use crate::{
    ResolveError, ResolveOptions, Resolver, SpecifierError, TsconfigDiscovery, TsconfigOptions,
    TsconfigReferences,
};

fn manual_resolver(config_file: std::path::PathBuf, extensions: Vec<String>) -> Resolver {
    Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file,
            references: TsconfigReferences::Auto,
        })),
        extensions,
        ..ResolveOptions::default()
    })
}

// ---------------------------------------------------------------------------
// Group 1: tsconfig lookup error scenarios
// ---------------------------------------------------------------------------

#[test]
fn extends_circular_two_files() {
    let f = super::fixture_root().join("tsconfig/cases/extends-circular");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let result = resolver.resolve_tsconfig(&f);
    assert!(
        matches!(&result, Err(ResolveError::TsconfigCircularExtend(_))),
        "expected TsconfigCircularExtend, got {result:?}",
    );
}

#[test]
fn extends_self() {
    let f = super::fixture_root().join("tsconfig/cases/extends-self");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let result = resolver.resolve_tsconfig(&f);
    assert!(
        matches!(&result, Err(ResolveError::TsconfigCircularExtend(_))),
        "expected TsconfigCircularExtend for self-extending tsconfig, got {result:?}",
    );
}

#[test]
fn references_self() {
    let f = super::fixture_root().join("tsconfig/cases/references-self");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });

    let result = resolver.resolve_tsconfig(&f);
    assert!(
        matches!(&result, Err(ResolveError::TsconfigSelfReference(_))),
        "expected TsconfigSelfReference, got {result:?}",
    );
}

#[test]
fn extends_empty_string() {
    let f = super::fixture_root().join("tsconfig/cases/extends-empty-string");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let result = resolver.resolve_tsconfig(&f);
    assert!(
        matches!(&result, Err(ResolveError::Specifier(SpecifierError::Empty(_)))),
        "expected SpecifierError::Empty for empty extends, got {result:?}",
    );
}

// ---------------------------------------------------------------------------
// Group 2: extends path resolution
// ---------------------------------------------------------------------------

#[test]
fn extends_pkg_subpath() {
    let f = super::fixture_root().join("tsconfig/cases/extends-pkg-subpath");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("should resolve through node_modules");
    let compiler_options = &resolution.compiler_options;
    assert_eq!(compiler_options.target, Some("ES2022".to_string()));
    assert_eq!(compiler_options.module, Some("ESNext".to_string()));
    assert_eq!(compiler_options.experimental_decorators, Some(true));
}

#[test]
fn extends_scoped_pkg() {
    let f = super::fixture_root().join("tsconfig/cases/extends-scoped-pkg");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("should resolve through @scope");
    assert_eq!(resolution.compiler_options.target, Some("ES2021".to_string()));
    assert_eq!(resolution.compiler_options.jsx, Some("react-jsx".to_string()));
}

#[test]
fn extends_pkg_tsconfig_file() {
    let f = super::fixture_root().join("tsconfig/cases/extends-pkg-tsconfig-file");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("should resolve explicit subpath");
    assert_eq!(resolution.compiler_options.target, Some("ES2024".to_string()));
    assert_eq!(resolution.compiler_options.module, Some("Preserve".to_string()));
}

#[test]
fn extends_with_json_extension() {
    let f = super::fixture_root().join("tsconfig/cases/extends-with-json");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("explicit .json extension should work");
    assert_eq!(resolution.compiler_options.target, Some("ES2020".to_string()));
    assert_eq!(resolution.compiler_options.module, Some("ESNext".to_string()));
}

#[test]
fn extends_folder_resolves_to_tsconfig_json() {
    let f = super::fixture_root().join("tsconfig/cases/extends-folder-tsconfig");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let resolution = resolver
        .resolve_tsconfig(&f)
        .expect("relative folder extends should resolve tsconfig.json");
    assert_eq!(resolution.compiler_options.target, Some("ES2019".to_string()));
}

// ---------------------------------------------------------------------------
// Group 3: extends array semantics
// ---------------------------------------------------------------------------

#[test]
fn extends_array_last_wins() {
    let f = super::fixture_root().join("tsconfig/cases/extends-array-last-wins");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");
    let compiler_options = &resolution.compiler_options;

    assert_eq!(compiler_options.target, Some("ES2022".to_string()));
    assert_eq!(compiler_options.module, Some("ESNext".to_string()));
    assert_eq!(compiler_options.jsx, Some("preserve".to_string()));
}

#[test]
fn extends_diamond() {
    let f = super::fixture_root().join("tsconfig/cases/extends-diamond");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");
    let compiler_options = &resolution.compiler_options;

    assert_eq!(compiler_options.target, Some("ES2018".to_string()));
    assert_eq!(compiler_options.experimental_decorators, Some(true));
    assert_eq!(compiler_options.jsx, Some("react".to_string()));
    assert_eq!(compiler_options.module, Some("ESNext".to_string()));
}

// ---------------------------------------------------------------------------
// Group 4: paths semantics
// ---------------------------------------------------------------------------

#[test]
fn paths_empty_array() {
    let f = super::fixture_root().join("tsconfig/cases/paths-empty-array");
    let resolver = manual_resolver(f.join("tsconfig.json"), vec![".ts".into()]);

    let resolved_path = resolver.resolve_file(&f, "missing").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("missing.ts")));
}

#[test]
fn paths_exact_wins_over_wildcard() {
    let f = super::fixture_root().join("tsconfig/cases/paths-exact-vs-wildcard");
    let resolver = manual_resolver(f.join("tsconfig.json"), vec![".ts".into()]);

    let resolved_path = resolver.resolve_file(&f, "foo").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("exact-foo.ts")));

    let resolved_path = resolver.resolve_file(&f, "foo/bar").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("wildcard/bar.ts")));
}

#[test]
fn paths_longest_prefix_wins() {
    let f = super::fixture_root().join("tsconfig/cases/paths-longest-prefix");
    let resolver = manual_resolver(f.join("tsconfig.json"), vec![".ts".into()]);

    let resolved_path = resolver.resolve_file(&f, "lib/foo").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("short/foo.ts")));

    let resolved_path = resolver.resolve_file(&f, "lib/sub/baz").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("long/baz.ts")));
}

#[test]
fn paths_multiple_substitutions_pick_first_existing() {
    let f = super::fixture_root().join("tsconfig/cases/paths-multiple-substitutions");
    let resolver = manual_resolver(f.join("tsconfig.json"), vec![".ts".into()]);

    let resolved_path = resolver.resolve_file(&f, "lib/exists").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("second/exists.ts")));

    let resolved_path = resolver.resolve_file(&f, "lib/onlythird").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("third/onlythird.ts")));
}

#[test]
fn paths_child_overrides_extended() {
    let f = super::fixture_root().join("tsconfig/cases/paths-child-overrides-extended");
    let resolver = manual_resolver(f.join("tsconfig.json"), vec![".ts".into()]);

    let resolved_path = resolver.resolve_file(&f, "@/x").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("child/x.ts")));
}

#[test]
fn paths_inherited_paths_base() {
    let f = super::fixture_root().join("tsconfig/cases/paths-inherited-paths-base");
    let resolver = manual_resolver(f.join("child-dir/tsconfig.json"), vec![".ts".into()]);

    let resolved_path =
        resolver.resolve_file(f.join("child-dir/index.ts"), "@/foo").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("parent-dir/lib/foo.ts")));
}

#[test]
fn paths_no_base_url_anchors_at_config_dir() {
    let f = super::fixture_root().join("tsconfig/cases/paths-no-base-url");
    let resolver = manual_resolver(f.join("tsconfig.json"), vec![".ts".into()]);

    let resolved_path = resolver.resolve_file(&f, "@/foo").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("src/foo.ts")));
}

#[test]
fn resolve_path_alias_or_base_url_maps_without_file_check() {
    // `@/*` -> `./src/*`. The mapping must be applied even when the target does not point at a real file,
    // so glob-like specifiers (e.g. `import.meta.glob` patterns) can be aliased.
    let f = super::fixture_root().join("tsconfig/cases/paths-no-base-url");
    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        extensions: vec![".ts".into()],
        ..ResolveOptions::default()
    });

    let map = |importer: &std::path::Path, specifier: &str| {
        resolver
            .find_tsconfig(importer)
            .unwrap()
            .map(|tsconfig| tsconfig.resolve_path_alias_or_base_url(specifier))
            .unwrap_or_default()
    };

    let importer = f.join("src/foo.ts");

    assert_eq!(map(&importer, "@/assets/**/*"), vec![f.join("src/assets/**/*")]);
    assert_eq!(map(&importer, "@/foo"), vec![f.join("src/foo")]);
    assert_eq!(map(&importer, "unmatched/bar"), Vec::<std::path::PathBuf>::new());
    assert_eq!(map(&importer, "./assets/**/*"), Vec::<std::path::PathBuf>::new());
    assert_eq!(
        map(&f.join("node_modules/pkg/index.ts"), "@/assets/**/*"),
        Vec::<std::path::PathBuf>::new()
    );
}

#[test]
fn resolve_path_alias_or_base_url_maps_without_file_check_falls_back_to_base_url() {
    // `baseUrl: "./src"` with no `paths`, so a non-relative specifier resolves under `./src` even for a glob-like specifier.
    let f = super::fixture_root().join("tsconfig/cases/base-url");
    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        extensions: vec![".ts".into()],
        ..ResolveOptions::default()
    });

    let map = |importer: &std::path::Path, specifier: &str| {
        resolver
            .find_tsconfig(importer)
            .unwrap()
            .map(|tsconfig| tsconfig.resolve_path_alias_or_base_url(specifier))
            .unwrap_or_default()
    };

    let importer = f.join("index.ts");

    assert_eq!(map(&importer, "assets/**/*"), vec![f.join("src/assets/**/*")]);
    assert_eq!(map(&importer, "foo.js"), vec![f.join("src/foo.js")]);
}

#[test]
fn paths_explicit_extension_target() {
    let f = super::fixture_root().join("tsconfig/cases/paths-explicit-extension");
    let resolver =
        manual_resolver(f.join("tsconfig.json"), vec![".ts".into(), ".json".into(), ".svg".into()]);

    let resolved_path = resolver.resolve_file(&f, "data.json").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("data-file.json")));

    let resolved_path = resolver.resolve_file(&f, "image").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("image.svg")));
}

#[test]
fn paths_wildcard_prefix_and_suffix() {
    let f = super::fixture_root().join("tsconfig/cases/paths-suffix-and-prefix");
    let resolver = manual_resolver(f.join("tsconfig.json"), vec![".ts".into()]);

    let resolved_path = resolver.resolve_file(&f, "prefix-middle-suffix").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("lib/middle.ts")));
}

// ---------------------------------------------------------------------------
// Group 5: ${configDir} template substitution
// ---------------------------------------------------------------------------

#[test]
fn config_dir_in_root_dirs() {
    let f = super::fixture_root().join("tsconfig/cases/config-dir-root-dirs");
    let resolver = manual_resolver(f.join("tsconfig.json"), vec![".ts".into()]);

    let resolved_path =
        resolver.resolve_file(f.join("src/index.ts"), "./gen").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("generated/gen.ts")));
}

// ---------------------------------------------------------------------------
// Group 6: extends array edge cases
// ---------------------------------------------------------------------------

#[test]
fn extends_empty_array() {
    let f = super::fixture_root().join("tsconfig/cases/extends-empty-array");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let resolution =
        resolver.resolve_tsconfig(&f).expect("empty extends array should be a no-op, not an error");
    assert_eq!(resolution.compiler_options.target, Some("ES2020".to_string()));
}

#[test]
fn extends_no_compiler_options_in_base() {
    let f = super::fixture_root().join("tsconfig/cases/extends-no-compiler-options");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let resolution = resolver
        .resolve_tsconfig(&f)
        .expect("extending a config without compilerOptions should still work");
    assert_eq!(resolution.compiler_options.target, Some("ES2020".to_string()));
}

// ---------------------------------------------------------------------------
// Group 7: paths edge cases — `*` alone, baseUrl only
// ---------------------------------------------------------------------------

#[test]
fn paths_star_only_pattern() {
    let f = super::fixture_root().join("tsconfig/cases/paths-star-only");
    let resolver = manual_resolver(f.join("tsconfig.json"), vec![".ts".into()]);

    let resolved_path = resolver.resolve_file(&f, "anything").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("fallback-dir/anything.ts")));
}

#[test]
fn base_url_only_resolves_bare_specifier() {
    let f = super::fixture_root().join("tsconfig/cases/base-url-only");
    let resolver = manual_resolver(f.join("tsconfig.json"), vec![".ts".into()]);

    let resolved_path = resolver.resolve_file(&f, "utils").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("src/utils.ts")));
}

// ---------------------------------------------------------------------------
// Group 8: project references — multiple references
// ---------------------------------------------------------------------------

#[test]
fn project_references_both_resolve_via_their_own_paths() {
    let f = super::fixture_root().join("tsconfig/cases/references-multi");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        extensions: vec![".ts".into()],
        ..ResolveOptions::default()
    });

    let resolved_path =
        resolver.resolve_file(f.join("pkg-a/index.ts"), "@shared/lib").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("shared/lib.ts")));

    let resolved_path =
        resolver.resolve_file(f.join("pkg-b/index.ts"), "@shared/lib").map(|r| r.full_path());
    assert_eq!(resolved_path, Ok(f.join("shared/lib.ts")));
}
