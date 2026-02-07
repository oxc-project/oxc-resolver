use crate::{ResolveOptions, Resolver};

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
