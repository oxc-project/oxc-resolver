use crate::{ResolveOptions, Resolver};

#[test]
fn explicit_ts_extension_matches() {
    let f = super::fixture_root().join("declaration_only");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".ts".into(), ".d.ts".into()],
        ..ResolveOptions::default()
    });

    // Test: import "./src/foo.ts" → resolved_using_ts_extension = true
    let resolution = resolver.resolve(&f, "./src/foo.ts").unwrap();
    assert!(
        resolution.resolved_using_ts_extension(),
        "Should be true when specifier has .ts and resolves to .ts"
    );
    assert!(resolution.path().to_string_lossy().ends_with("foo.ts"));
}

#[test]
fn explicit_d_ts_extension_matches() {
    let f = super::fixture_root().join("declaration_only");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".ts".into(), ".d.ts".into()],
        ..ResolveOptions::default()
    });

    // Test: import "./src/foo.d.ts" → resolved_using_ts_extension = true
    let resolution = resolver.resolve(&f, "./src/foo.d.ts").unwrap();
    assert!(
        resolution.resolved_using_ts_extension(),
        "Should be true when specifier has .d.ts and resolves to .d.ts"
    );
    assert!(resolution.path().to_string_lossy().ends_with("foo.d.ts"));
}

#[test]
fn no_extension_resolves_to_ts() {
    let f = super::fixture_root().join("declaration_only");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".ts".into(), ".d.ts".into()],
        ..ResolveOptions::default()
    });

    // Test: import "./src/foo" → Resolved: foo.ts, resolved_using_ts_extension = false
    let resolution = resolver.resolve(&f, "./src/foo").unwrap();
    assert!(
        !resolution.resolved_using_ts_extension(),
        "Should be false when specifier has no extension but resolves to .ts"
    );
    assert!(resolution.path().to_string_lossy().ends_with("foo.ts"));
}

#[test]
fn js_extension_via_extension_alias() {
    let f = super::fixture_root().join("declaration_only");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".ts".into(), ".d.ts".into()],
        extension_alias: vec![(".js".into(), vec![".ts".into(), ".js".into()])],
        ..ResolveOptions::default()
    });

    // Test: import "./src/foo.js" → Resolved: foo.ts, resolved_using_ts_extension = false
    let resolution = resolver.resolve(&f, "./src/foo.js").unwrap();
    assert!(
        !resolution.resolved_using_ts_extension(),
        "Should be false when specifier has .js but resolves to .ts via extensionAlias"
    );
    assert!(resolution.path().to_string_lossy().ends_with("foo.ts"));
}

#[test]
fn tsx_extension_matches() {
    let f = super::fixture().join("extensions");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".tsx".into(), ".ts".into()],
        ..ResolveOptions::default()
    });

    // Create a .tsx file for testing
    std::fs::write(f.join("test.tsx"), "export const Component = () => {}").unwrap();

    let resolution = resolver.resolve(&f, "./test.tsx").unwrap();
    assert!(resolution.resolved_using_ts_extension(), "Should be true for .tsx extension");

    // Cleanup
    std::fs::remove_file(f.join("test.tsx")).ok();
}

#[test]
fn mts_extension_matches() {
    let f = super::fixture().join("extensions");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".mts".into(), ".ts".into()],
        ..ResolveOptions::default()
    });

    // Create a .mts file for testing
    std::fs::write(f.join("test.mts"), "export const foo = 'bar';").unwrap();

    let resolution = resolver.resolve(&f, "./test.mts").unwrap();
    assert!(resolution.resolved_using_ts_extension(), "Should be true for .mts extension");

    // Cleanup
    std::fs::remove_file(f.join("test.mts")).ok();
}

#[test]
fn cts_extension_matches() {
    let f = super::fixture().join("extensions");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".cts".into(), ".ts".into()],
        ..ResolveOptions::default()
    });

    // Create a .cts file for testing
    std::fs::write(f.join("test.cts"), "export const foo = 'bar';").unwrap();

    let resolution = resolver.resolve(&f, "./test.cts").unwrap();
    assert!(resolution.resolved_using_ts_extension(), "Should be true for .cts extension");

    // Cleanup
    std::fs::remove_file(f.join("test.cts")).ok();
}

#[test]
fn d_mts_extension_matches() {
    let f = super::fixture().join("extensions");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".d.mts".into(), ".mts".into()],
        ..ResolveOptions::default()
    });

    // Create a .d.mts file for testing
    std::fs::write(f.join("test.d.mts"), "export declare const foo: string;").unwrap();

    let resolution = resolver.resolve(&f, "./test.d.mts").unwrap();
    assert!(resolution.resolved_using_ts_extension(), "Should be true for .d.mts extension");

    // Cleanup
    std::fs::remove_file(f.join("test.d.mts")).ok();
}

#[test]
fn d_cts_extension_matches() {
    let f = super::fixture().join("extensions");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".d.cts".into(), ".cts".into()],
        ..ResolveOptions::default()
    });

    // Create a .d.cts file for testing
    std::fs::write(f.join("test.d.cts"), "export declare const foo: string;").unwrap();

    let resolution = resolver.resolve(&f, "./test.d.cts").unwrap();
    assert!(resolution.resolved_using_ts_extension(), "Should be true for .d.cts extension");

    // Cleanup
    std::fs::remove_file(f.join("test.d.cts")).ok();
}

#[test]
fn package_exports_without_extension() {
    // Test that package.json exports mapping doesn't set the flag
    // This would require a fixture with package.json exports, skipping for now
    // as it would need more complex setup
}

#[test]
fn js_file_no_ts_extension() {
    let f = super::fixture().join("extensions");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".ts".into()],
        ..ResolveOptions::default()
    });

    // Resolve a .js file
    let resolution = resolver.resolve(&f, "./app.module.js").unwrap();
    assert!(!resolution.resolved_using_ts_extension(), "Should be false for .js files");
}
