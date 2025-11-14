//! Tests for solution-style tsconfig resolution
//!
//! Tests the `TsConfig::resolve_for_file` method which implements solution-style
//! tsconfig resolution similar to tsconfck's `resolveSolutionTSConfig`.

use std::{path::Path, sync::Arc};

use crate::{ResolveOptions, Resolver, TsConfig};

/// Helper function to load a tsconfig with references
fn load_tsconfig(path: &Path) -> Arc<TsConfig> {
    let resolver = Resolver::new(ResolveOptions::default());
    resolver.resolve_tsconfig(path).unwrap()
}

#[test]
fn resolve_for_file_basic() {
    let f = super::fixture_root().join("tsconfig/cases/solution_style");

    // Load root tsconfig with references
    let root_tsconfig = load_tsconfig(&f.join("tsconfig.json"));

    // File in root src/ should use root tsconfig
    let file_path = f.join("src/index.ts");
    let resolved = root_tsconfig.resolve_for_file(&file_path);
    assert_eq!(resolved.path(), root_tsconfig.path(), "File in src/ should use root tsconfig");

    // File in pkg-a should use pkg-a's tsconfig
    let file_path = f.join("packages/pkg-a/src/index.ts");
    let resolved = root_tsconfig.resolve_for_file(&file_path);
    assert_eq!(
        resolved.path(),
        f.join("packages/pkg-a/tsconfig.json"),
        "File in pkg-a/src/ should use pkg-a's tsconfig"
    );

    // File in pkg-b should use pkg-b's tsconfig
    let file_path = f.join("packages/pkg-b/src/index.ts");
    let resolved = root_tsconfig.resolve_for_file(&file_path);
    assert_eq!(
        resolved.path(),
        f.join("packages/pkg-b/tsconfig.json"),
        "File in pkg-b/src/ should use pkg-b's tsconfig"
    );

    // File in pkg-c's lib/ should use pkg-c's tsconfig
    let file_path = f.join("packages/pkg-c/lib/index.ts");
    let resolved = root_tsconfig.resolve_for_file(&file_path);
    assert_eq!(
        resolved.path(),
        f.join("packages/pkg-c/tsconfig.json"),
        "File in pkg-c/lib/ should use pkg-c's tsconfig"
    );
}

#[test]
fn resolve_for_file_exclude_patterns() {
    let f = super::fixture_root().join("tsconfig/cases/solution_style");
    let root_tsconfig = load_tsconfig(&f.join("tsconfig.json"));

    // pkg-c excludes *.test.ts files
    let excluded_file = f.join("packages/pkg-c/lib/index.test.ts");
    let resolved = root_tsconfig.resolve_for_file(&excluded_file);

    // Should fall back to root tsconfig since file is excluded from pkg-c
    assert_eq!(
        resolved.path(),
        root_tsconfig.path(),
        "Excluded files should fall back to root tsconfig"
    );

    // Non-excluded file should still use pkg-c's tsconfig
    let included_file = f.join("packages/pkg-c/lib/index.ts");
    let resolved = root_tsconfig.resolve_for_file(&included_file);
    assert_eq!(
        resolved.path(),
        f.join("packages/pkg-c/tsconfig.json"),
        "Non-excluded files should use pkg-c's tsconfig"
    );
}

#[test]
fn resolve_for_file_allowjs() {
    let f = super::fixture_root().join("tsconfig/cases/solution_style");
    let root_tsconfig = load_tsconfig(&f.join("tsconfig.json"));

    // pkg-b has allowJs: true, so .js files should use pkg-b's tsconfig
    let js_file = f.join("packages/pkg-b/src/index.js");
    let resolved = root_tsconfig.resolve_for_file(&js_file);
    assert_eq!(
        resolved.path(),
        f.join("packages/pkg-b/tsconfig.json"),
        "With allowJs, .js files should use pkg-b's tsconfig"
    );

    // pkg-a doesn't have allowJs, so .js files should fall back to root
    let js_file = f.join("packages/pkg-a/src/script.js");
    let resolved = root_tsconfig.resolve_for_file(&js_file);
    assert_eq!(
        resolved.path(),
        root_tsconfig.path(),
        "Without allowJs, .js files should fall back to root tsconfig"
    );
}

#[test]
fn resolve_for_file_non_ts_js_files() {
    let f = super::fixture_root().join("tsconfig/cases/solution_style");
    let root_tsconfig = load_tsconfig(&f.join("tsconfig.json"));

    // Non-TS/JS files should not trigger solution-style resolution
    let json_file = f.join("packages/pkg-a/src/data.json");
    let resolved = root_tsconfig.resolve_for_file(&json_file);
    assert_eq!(resolved.path(), root_tsconfig.path(), "Non-TS/JS files should use root tsconfig");

    // .css, .html, etc. should also fall back to root
    let css_file = f.join("packages/pkg-a/src/styles.css");
    let resolved = root_tsconfig.resolve_for_file(&css_file);
    assert_eq!(resolved.path(), root_tsconfig.path(), "CSS files should use root tsconfig");
}

#[test]
fn resolve_for_file_no_references() {
    let f = super::fixture_root().join("tsconfig/cases/solution_style");

    // Load pkg-a's tsconfig which has no references
    let pkg_a_tsconfig = load_tsconfig(&f.join("packages/pkg-a/tsconfig.json"));

    // When a tsconfig has no references, it should always return itself
    let file_in_pkg_a = f.join("packages/pkg-a/src/index.ts");
    let resolved = pkg_a_tsconfig.resolve_for_file(&file_in_pkg_a);
    assert_eq!(
        resolved.path(),
        pkg_a_tsconfig.path(),
        "Tsconfig without references should always return itself"
    );

    // Even for files outside its directory
    let file_in_pkg_b = f.join("packages/pkg-b/src/index.ts");
    let resolved = pkg_a_tsconfig.resolve_for_file(&file_in_pkg_b);
    assert_eq!(
        resolved.path(),
        pkg_a_tsconfig.path(),
        "Tsconfig without references should return itself for any file"
    );
}

#[test]
fn resolve_for_file_extensions() {
    let f = super::fixture_root().join("tsconfig/cases/solution_style");
    let root_tsconfig = load_tsconfig(&f.join("tsconfig.json"));

    // Test all TS extensions
    for ext in ["ts", "tsx", "mts", "cts"] {
        let file = f.join(format!("packages/pkg-a/src/index.{ext}"));
        let resolved = root_tsconfig.resolve_for_file(&file);
        assert_eq!(
            resolved.path(),
            f.join("packages/pkg-a/tsconfig.json"),
            ".{ext} files should trigger solution-style resolution"
        );
    }

    // Test JS extensions with allowJs
    for ext in ["js", "jsx", "mjs", "cjs"] {
        let file = f.join(format!("packages/pkg-b/src/index.{ext}"));
        let resolved = root_tsconfig.resolve_for_file(&file);
        assert_eq!(
            resolved.path(),
            f.join("packages/pkg-b/tsconfig.json"),
            ".{ext} files should trigger solution-style resolution with allowJs"
        );
    }
}
