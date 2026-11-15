use crate::{
    ResolveError, ResolveOptions, Resolver, TsconfigDiscovery, TsconfigOptions, TypesVersions,
};

fn dts_fixture() -> std::path::PathBuf {
    super::fixture_root().join("dts-resolver")
}

fn containing_file() -> std::path::PathBuf {
    dts_fixture().join("index.ts")
}

fn resolver() -> Resolver {
    Resolver::new(ResolveOptions {
        condition_names: vec!["import".into(), "types".into()],
        ..ResolveOptions::default()
    })
}

// -------- Relative resolution --------

#[test]
fn relative_basic_ts() {
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "./relative-basic/index").unwrap();
    assert_eq!(result.path(), dts_fixture().join("relative-basic/index.ts"));
}

#[test]
fn relative_dts_over_js() {
    // When both .d.ts and .js exist, .ts should be tried first (it doesn't exist),
    // then .tsx (doesn't exist), then .d.ts should win
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "./relative-dts-over-js/index").unwrap();
    assert_eq!(result.path(), dts_fixture().join("relative-dts-over-js/index.d.ts"));
}

#[test]
fn relative_directory_index() {
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "./relative-basic").unwrap();
    assert_eq!(result.path(), dts_fixture().join("relative-basic/index.ts"));
}

// -------- JSON --------

#[test]
fn json_file() {
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "./relative-json/data.json").unwrap();
    assert_eq!(result.path(), dts_fixture().join("relative-json/data.json"));
    assert_eq!(result.module_type(), Some(crate::ModuleType::Json));
}

// -------- Extension substitution --------

#[test]
fn extension_substitution_js_to_dts() {
    // ./foo.js -> ./foo.d.ts (via extension replacement: strip .js, try .ts first (not found), then .d.ts)
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "./extension-substitution/foo.js").unwrap();
    assert_eq!(result.path(), dts_fixture().join("extension-substitution/foo.d.ts"));
}

#[test]
fn extension_substitution_mjs_to_dmts() {
    // ./bar.mjs -> ./bar.d.mts
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "./extension-substitution/bar.mjs").unwrap();
    assert_eq!(result.path(), dts_fixture().join("extension-substitution/bar.d.mts"));
}

#[test]
fn extension_substitution_cjs_to_dcts() {
    // ./baz.cjs -> ./baz.d.cts
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "./extension-substitution/baz.cjs").unwrap();
    assert_eq!(result.path(), dts_fixture().join("extension-substitution/baz.d.cts"));
}

// -------- Extension priority --------

#[test]
fn extension_priority_ts_wins() {
    // When .ts, .tsx, .d.ts, .js all exist, .ts should win
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "./extension-priority/main").unwrap();
    assert_eq!(result.path(), dts_fixture().join("extension-priority/main.ts"));
}

#[test]
fn extension_priority_dts_over_js() {
    // When only .d.ts and .js exist, .d.ts should win
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "./extension-priority/only-dts").unwrap();
    assert_eq!(result.path(), dts_fixture().join("extension-priority/only-dts.d.ts"));
}

// -------- Directory module (package.json main) --------

#[test]
fn directory_module_with_main() {
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "./dir-module").unwrap();
    assert_eq!(result.path(), dts_fixture().join("dir-module/lib/main.ts"));
}

// -------- node_modules: @types basic --------

#[test]
fn at_types_basic() {
    // `debug` has no types, should resolve to @types/debug
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "debug").unwrap();
    assert_eq!(result.path(), dts_fixture().join("node_modules/@types/debug/index.d.ts"));
}

// -------- node_modules: @types scoped --------

#[test]
fn at_types_scoped() {
    // @babel/generator should resolve to @types/babel__generator
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "@babel/generator").unwrap();
    assert_eq!(
        result.path(),
        dts_fixture().join("node_modules/@types/babel__generator/index.d.ts")
    );
}

// -------- node_modules: exports field --------

#[test]
fn exports_field_types_condition() {
    // Package with exports should use "types" condition
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "with-exports").unwrap();
    assert_eq!(result.path(), dts_fixture().join("node_modules/with-exports/types/index.d.ts"));
}

// -------- node_modules: typesVersions --------

#[test]
fn types_versions_root() {
    // Package with typesVersions should redirect root to dist
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "with-types-versions").unwrap();
    assert_eq!(
        result.path(),
        dts_fixture().join("node_modules/with-types-versions/dist/index.d.ts")
    );
}

#[test]
fn types_versions_subpath() {
    // Package with typesVersions should redirect subpath to dist
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "with-types-versions/sub").unwrap();
    assert_eq!(result.path(), dts_fixture().join("node_modules/with-types-versions/dist/sub.d.ts"));
}

#[test]
fn types_versions_ranges() {
    // Modeled after `@types/node`, which keeps legacy typings for old compilers
    let pkg = dts_fixture().join("node_modules/ranged-types-versions");
    let cases = [
        (TypesVersions::First, "ts5.6/"),
        (TypesVersions::Ignore, ""),
        (TypesVersions::Latest, ""),
        (TypesVersions::Version("5.5.0".into()), "ts5.6/"),
        (TypesVersions::Version("5.6.3".into()), "ts5.6/"),
        (TypesVersions::Version("5.7.2".into()), "ts5.7/"),
        (TypesVersions::Version("5.8.0-beta".into()), ""),
        (TypesVersions::Version("6.0.3".into()), ""),
        // Invalid version, no entry matches
        (TypesVersions::Version("next".into()), ""),
    ];
    for (types_versions, dir) in cases {
        let r = Resolver::new(ResolveOptions {
            condition_names: vec!["import".into(), "types".into()],
            types_versions: types_versions.clone(),
            ..ResolveOptions::default()
        });
        for (specifier, file) in
            [("ranged-types-versions", "index.d.ts"), ("ranged-types-versions/fs", "fs.d.ts")]
        {
            let result = r.resolve_dts(containing_file(), specifier).unwrap();
            assert_eq!(
                result.path(),
                pkg.join(format!("{dir}{file}")),
                "{types_versions:?} {specifier}"
            );
        }
    }
}

#[test]
fn types_versions_wildcard_range() {
    // The `*` range matches any version
    for types_versions in [TypesVersions::Latest, TypesVersions::Version("6.0.3".into())] {
        let r = Resolver::new(ResolveOptions {
            condition_names: vec!["import".into(), "types".into()],
            types_versions,
            ..ResolveOptions::default()
        });
        let result = r.resolve_dts(containing_file(), "with-types-versions/sub").unwrap();
        assert_eq!(
            result.path(),
            dts_fixture().join("node_modules/with-types-versions/dist/sub.d.ts")
        );
    }
}

#[test]
fn versioned_types_condition() {
    // https://www.typescriptlang.org/docs/handbook/modules/reference.html#example-versioned-types-condition
    let pkg = dts_fixture().join("node_modules/versioned-types-condition");
    let cases = [
        // Versioned conditions never match without a known compiler version
        (TypesVersions::First, "tsold"),
        (TypesVersions::Ignore, "tsold"),
        (TypesVersions::Latest, "ts5.2"),
        (TypesVersions::Version("6.0.3".into()), "ts5.2"),
        (TypesVersions::Version("5.2.0".into()), "ts5.2"),
        (TypesVersions::Version("5.2.0-beta".into()), "ts5.2"),
        (TypesVersions::Version("5.1.6".into()), "ts4.6"),
        (TypesVersions::Version("4.9.5".into()), "ts4.6"),
        (TypesVersions::Version("4.5.0".into()), "tsold"),
    ];
    for (types_versions, dir) in cases {
        let r = Resolver::new(ResolveOptions {
            condition_names: vec!["import".into(), "types".into()],
            types_versions: types_versions.clone(),
            ..ResolveOptions::default()
        });
        let result = r.resolve_dts(containing_file(), "versioned-types-condition").unwrap();
        assert_eq!(result.path(), pkg.join(dir).join("index.d.ts"), "{types_versions:?}");

        let internal_dir = if dir == "ts4.6" { "tsold" } else { dir };
        let result = r.resolve_dts(pkg.join("index.js"), "#internal").unwrap();
        assert_eq!(
            result.path(),
            pkg.join(internal_dir).join("internal.d.ts"),
            "{types_versions:?} #internal"
        );
    }

    // Versioned conditions require the `types` condition
    let r = Resolver::new(ResolveOptions {
        condition_names: vec!["import".into()],
        types_versions: TypesVersions::Latest,
        ..ResolveOptions::default()
    });
    let result = r.resolve(dts_fixture(), "versioned-types-condition").unwrap();
    assert_eq!(result.path(), pkg.join("index.js"));
}

#[test]
fn versioned_types_condition_jotai_style() {
    // `jotai` maps the plain `types` condition to a stub that tells users to upgrade TypeScript.
    // `exports` takes precedence over its `typesVersions`.
    let pkg = dts_fixture().join("node_modules/jotai-style");
    let stub = "dist/ts_version_5.5_and_above_is_required.d.ts";
    let cases = [
        (TypesVersions::First, false),
        (TypesVersions::Ignore, false),
        (TypesVersions::Latest, true),
        (TypesVersions::Version("6.0.3".into()), true),
        (TypesVersions::Version("5.5.0".into()), true),
        (TypesVersions::Version("5.4.5".into()), false),
    ];
    for (types_versions, matches) in cases {
        let r = Resolver::new(ResolveOptions {
            condition_names: vec!["import".into(), "types".into()],
            types_versions: types_versions.clone(),
            ..ResolveOptions::default()
        });
        for (specifier, file) in [
            ("jotai-style", "dist/index.d.ts"),
            ("jotai-style/utils", "dist/utils.d.ts"),
            ("jotai-style/vanilla/utils", "dist/vanilla/utils.d.ts"),
        ] {
            let result = r.resolve_dts(containing_file(), specifier).unwrap();
            let expected = if matches { file } else { stub };
            assert_eq!(result.path(), pkg.join(expected), "{types_versions:?} {specifier}");
        }
    }
}

// -------- node_modules: typings field --------

#[test]
fn typings_field() {
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "with-typings").unwrap();
    assert_eq!(result.path(), dts_fixture().join("node_modules/with-typings/types.d.ts"));
}

// -------- Non-resolvable specifiers --------

#[test]
fn node_protocol_not_resolved() {
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "node:fs");
    assert!(result.is_err(), "node:fs should not resolve");
}

#[test]
fn no_types_not_resolved() {
    // Package with only JS and no types should resolve to JS (it's in the secondary pass)
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "no-types").unwrap();
    assert_eq!(result.path(), dts_fixture().join("node_modules/no-types/index.js"));
}

// -------- exports: ESM match finds declaration over JS --------

#[test]
fn exports_esm_match_finds_declaration() {
    // When exports resolves to .mjs but .d.mts exists, prefer .d.mts
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "exports-dts-for-mjs").unwrap();
    assert_eq!(
        result.path(),
        dts_fixture().join("node_modules/exports-dts-for-mjs/dist/index.d.mts")
    );
}

#[test]
fn exports_falls_back_to_types_after_js_target() {
    // https://github.com/oxc-project/oxc-resolver/issues/1436
    // `import` matches `./lib/esm/index.js` but there is no declaration next to it,
    // so the `types` condition listed after it should be used, like TypeScript.
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "exports-types-after-import").unwrap();
    assert_eq!(
        result.path(),
        dts_fixture().join("node_modules/exports-types-after-import/lib/types/index.d.ts")
    );
}

#[test]
fn exports_pattern_falls_back_to_types_after_js_target() {
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "exports-types-after-import/sub/a").unwrap();
    assert_eq!(
        result.path(),
        dts_fixture().join("node_modules/exports-types-after-import/lib/types/sub/a.d.ts")
    );
}

#[test]
fn exports_types_condition_is_always_used() {
    // Like TypeScript, `types` is matched even if it is not in `condition_names`.
    let r = Resolver::new(ResolveOptions::default());
    let result = r.resolve_dts(containing_file(), "exports-types-after-import").unwrap();
    assert_eq!(
        result.path(),
        dts_fixture().join("node_modules/exports-types-after-import/lib/types/index.d.ts")
    );
}

#[test]
fn exports_json_target_is_not_a_declaration() {
    // `.json` is only accepted in the JavaScript pass, so `types` is used.
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "exports-json-before-types").unwrap();
    assert_eq!(
        result.path(),
        dts_fixture().join("node_modules/exports-json-before-types/index.d.ts")
    );
}

#[test]
fn exports_js_target_without_declaration() {
    // Without any declaration, the JavaScript target is resolved in the second pass.
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "exports-js-only").unwrap();
    assert_eq!(result.path(), dts_fixture().join("node_modules/exports-js-only/index.js"));
}

// -------- package.json types field (not typings) --------

#[test]
fn types_field() {
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "with-types").unwrap();
    assert_eq!(result.path(), dts_fixture().join("node_modules/with-types/types/index.d.ts"));
}

#[test]
fn typings_takes_precedence_over_types() {
    // pkg.typings().or_else(|| pkg.types()) — typings wins
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "with-both-types-fields").unwrap();
    assert_eq!(
        result.path(),
        dts_fixture().join("node_modules/with-both-types-fields/typings.d.ts")
    );
}

// -------- Error cases --------

#[test]
fn completely_unresolvable_package() {
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "completely-empty");
    result.unwrap_err();
}

#[test]
fn nonexistent_package() {
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "this-package-does-not-exist");
    assert_eq!(result, Err(ResolveError::NotFound("this-package-does-not-exist".into())));
}

#[test]
fn node_protocol_returns_not_found() {
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "node:fs");
    assert_eq!(result, Err(ResolveError::NotFound("node:fs".into())));
}

// -------- Package imports (#) --------

#[test]
fn hash_import() {
    let r = resolver();
    let containing = dts_fixture().join("hash-import/index.ts");
    let result = r.resolve_dts(containing, "#internal").unwrap();
    assert_eq!(result.path(), dts_fixture().join("hash-import/src/internal.d.ts"));
}

// -------- tsconfig paths --------

#[test]
fn tsconfig_paths_in_dts() {
    let r = Resolver::new(ResolveOptions {
        condition_names: vec!["import".into(), "types".into()],
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: dts_fixture().join("with-tsconfig/tsconfig.json"),
            references: crate::TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });
    let containing = dts_fixture().join("with-tsconfig/index.ts");
    let result = r.resolve_dts(containing, "@lib/utils").unwrap();
    assert_eq!(result.path(), dts_fixture().join("with-tsconfig/lib/utils.ts"));
}

#[test]
fn dot_prefixed_tsconfig_paths_in_dts() {
    let f = super::fixture_root().join("tsconfig/cases/dot-prefixed-paths");
    let r = Resolver::new(ResolveOptions {
        condition_names: vec!["import".into(), "types".into()],
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: crate::TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        (".exact", f.join("exact.ts")),
        (".storybook/preview", f.join("alias/preview.ts")),
        ("..alias/module", f.join("alias/module.ts")),
        (".base-url", f.join(".base-url.ts")),
        (".../module", f.join("alias/module.ts")),
        // Relative specifiers must bypass the deliberately conflicting paths mappings.
        ("./alias/module", f.join("alias/module.ts")),
        ("../dot-prefixed-paths/alias/module", f.join("alias/module.ts")),
    ];
    for (request, expected) in pass {
        let result = r.resolve_dts(f.join("index.ts"), request).map(|r| r.full_path());
        assert_eq!(result, Ok(expected), "{request}");
    }
}

// -------- Extension substitution: .mjs → .mts priority --------

#[test]
fn extension_substitution_mjs_prefers_mts() {
    // When both .mts and .d.mts exist, .mts should win (TypeScript first)
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "./extension-substitution/priority.mjs").unwrap();
    assert_eq!(result.path(), dts_fixture().join("extension-substitution/priority.mts"));
}

// -------- Extension substitution: .json → .d.json.ts --------

#[test]
fn extension_substitution_json_to_d_json_ts() {
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "./extension-substitution/qux.json").unwrap();
    assert_eq!(result.path(), dts_fixture().join("extension-substitution/qux.d.json.ts"));
}

// -------- Extension substitution: .tsx --------

#[test]
fn extension_substitution_tsx() {
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "./extension-substitution/comp.tsx").unwrap();
    assert_eq!(result.path(), dts_fixture().join("extension-substitution/comp.tsx"));
}

// -------- Self-referencing package --------

#[test]
fn self_referencing_package() {
    let r = resolver();
    let containing = dts_fixture().join("node_modules/with-self-ref/src/index.ts");
    let result = r.resolve_dts(containing, "with-self-ref").unwrap();
    assert_eq!(result.path(), dts_fixture().join("node_modules/with-self-ref/types/index.d.ts"));
}

// -------- typesVersions subpath --------

#[test]
fn subpath_with_types_versions() {
    let r = resolver();
    let result = r.resolve_dts(containing_file(), "with-subpath/sub/foo").unwrap();
    assert_eq!(result.path(), dts_fixture().join("node_modules/with-subpath/dist/foo.d.ts"));
}

// -------- Module type detection in DTS --------

#[test]
fn dts_module_type_mts() {
    let r = resolver();
    // exports-dts-for-mjs resolves to .d.mts which should be ModuleType::Module
    let result = r.resolve_dts(containing_file(), "exports-dts-for-mjs").unwrap();
    assert_eq!(result.module_type(), Some(crate::ModuleType::Module));
}

#[test]
fn dts_module_type_cts() {
    let r = resolver();
    // .d.cts should be ModuleType::CommonJs
    let result = r.resolve_dts(containing_file(), "./extension-substitution/baz.cjs").unwrap();
    assert_eq!(result.module_type(), Some(crate::ModuleType::CommonJs));
}

// -------- @types name mangling --------

#[test]
fn mangle_unscoped() {
    assert_eq!(
        crate::ResolverGeneric::<crate::FileSystemOs>::dts_mangle_scoped_name("debug"),
        "debug"
    );
}

#[test]
fn mangle_scoped() {
    assert_eq!(
        crate::ResolverGeneric::<crate::FileSystemOs>::dts_mangle_scoped_name("@babel/generator"),
        "babel__generator"
    );
}

#[test]
fn mangle_scoped_multi_slash() {
    assert_eq!(
        crate::ResolverGeneric::<crate::FileSystemOs>::dts_mangle_scoped_name("@scope/pkg/sub"),
        "scope__pkg/sub"
    );
}
