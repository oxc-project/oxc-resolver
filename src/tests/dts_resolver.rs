use crate::{ResolveError, ResolveOptions, Resolver, TsconfigDiscovery, TsconfigOptions};

fn dts_fixture() -> std::path::PathBuf {
    super::fixture_root().join("dts_resolver")
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
    assert!(result.is_err());
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
