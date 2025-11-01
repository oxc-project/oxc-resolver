//! Unit tests for TsconfigFileMatcher pattern matching
//!
//! Tests various glob patterns, include/exclude logic, and edge cases
//! using fixtures from tsconfig/cases/

use crate::tsconfig::TsconfigFileMatcher;
use std::path::PathBuf;

/// Helper to create a TsconfigFileMatcher from a fixture directory
fn create_matcher_from_fixture(fixture_name: &str) -> (TsconfigFileMatcher, PathBuf) {
    let fixture_dir = super::fixture_root().join("tsconfig/cases").join(fixture_name);
    let tsconfig_path = fixture_dir.join("tsconfig.json");

    // Read and parse tsconfig.json
    let tsconfig_str = std::fs::read_to_string(&tsconfig_path)
        .unwrap_or_else(|_| panic!("Failed to read tsconfig.json from {fixture_name}"));
    let json: serde_json::Value = serde_json::from_str(&tsconfig_str)
        .unwrap_or_else(|_| panic!("Failed to parse tsconfig.json from {fixture_name}"));

    // Helper to substitute ${configDir} template variable
    #[allow(clippy::option_if_let_else)] // map_or causes borrow checker issues
    let substitute_template = |s: String| -> String {
        match s.strip_prefix("${configDir}") {
            Some(stripped) => fixture_dir.to_str().unwrap().to_string() + stripped,
            None => s,
        }
    };

    // Extract fields
    let files = json.get("files").and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(String::from))
                .map(substitute_template)
                .collect()
        })
    });

    let include = json.get("include").and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(String::from))
                .map(substitute_template)
                .collect()
        })
    });

    let exclude = json.get("exclude").and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(String::from))
                .map(substitute_template)
                .collect()
        })
    });

    let out_dir = json
        .get("compilerOptions")
        .and_then(|opts| opts.get("outDir"))
        .and_then(|v| v.as_str())
        .map(std::path::Path::new);

    // Create matcher
    let matcher = TsconfigFileMatcher::new(files, include, exclude, out_dir, fixture_dir.clone());

    (matcher, fixture_dir)
}

#[test]
fn test_globstar_patterns() {
    let (matcher, fixture_dir) = create_matcher_from_fixture("globstar_patterns");

    // Test cases: (path, should_match, description)
    let test_cases = [
        ("index.ts", true, "globstar matches root level"),
        ("src/index.ts", true, "globstar matches one level deep"),
        ("src/utils/helper.ts", true, "globstar matches two levels deep"),
        ("src/utils/deep/nested/file.ts", true, "globstar matches deeply nested"),
        ("index.js", false, "globstar doesn't match .js files"),
        ("src/index.js", false, "globstar doesn't match .js files in subdirs"),
    ];

    for (file_path, should_match, comment) in test_cases {
        let full_path = fixture_dir.join(file_path);
        let result = matcher.matches(&full_path);
        assert_eq!(result, should_match, "{comment}: path={file_path}");
    }
}

#[test]
fn test_wildcard_patterns() {
    let (matcher, fixture_dir) = create_matcher_from_fixture("wildcard_patterns");

    let test_cases = [
        ("src/index.ts", true, "wildcard matches files in src/"),
        ("src/helper.ts", true, "wildcard matches files in src/"),
        ("src/utils/helper.ts", false, "wildcard doesn't match subdirectories"),
        ("index.ts", false, "wildcard doesn't match parent directory"),
    ];

    for (file_path, should_match, comment) in test_cases {
        let full_path = fixture_dir.join(file_path);
        let result = matcher.matches(&full_path);
        assert_eq!(result, should_match, "{comment}: path={file_path}");
    }
}

#[test]
fn test_simple_wildcard_patterns() {
    let (matcher, fixture_dir) = create_matcher_from_fixture("character_set_patterns");

    let test_cases = [
        ("App.ts", true, "wildcard matches .ts files"),
        ("Button.ts", true, "wildcard matches .ts files"),
    ];

    for (file_path, should_match, comment) in test_cases {
        let full_path = fixture_dir.join(file_path);
        let result = matcher.matches(&full_path);
        assert_eq!(result, should_match, "{comment}: path={file_path}");
    }
}

#[test]
fn test_complex_patterns() {
    let (matcher, fixture_dir) = create_matcher_from_fixture("complex_patterns");

    let test_cases = [
        ("src/test/unit.spec.ts", true, "complex pattern matches src/test/*.spec.ts"),
        ("src/utils/test/helper.spec.ts", true, "complex pattern matches src/**/test/*.spec.ts"),
        ("src/deep/nested/test/component.spec.ts", true, "complex pattern matches deeply nested"),
        ("src/index.ts", false, "file not in test directory"),
        ("src/utils/helper.ts", false, "file not in test directory"),
        ("src/test/unit.test.ts", false, "wrong extension (.test.ts not .spec.ts)"),
    ];

    for (file_path, should_match, comment) in test_cases {
        let full_path = fixture_dir.join(file_path);
        let result = matcher.matches(&full_path);
        assert_eq!(result, should_match, "{comment}: path={file_path}");
    }
}

#[test]
fn test_monorepo_patterns() {
    let (matcher, fixture_dir) = create_matcher_from_fixture("monorepo_patterns");

    let test_cases = [
        ("packages/pkg-a/src/index.ts", true, "monorepo pattern matches pkg-a"),
        ("packages/pkg-b/src/utils.ts", true, "monorepo pattern matches pkg-b"),
        ("packages/pkg-c/src/deep/nested/file.ts", true, "monorepo pattern matches nested"),
        ("packages/pkg-a/dist/index.js", false, "dist not in src directory"),
        ("shared/utils.ts", false, "shared not in packages/*/src"),
        ("packages/pkg-a/test.ts", false, "test.ts not in src directory"),
    ];

    for (file_path, should_match, comment) in test_cases {
        let full_path = fixture_dir.join(file_path);
        let result = matcher.matches(&full_path);
        assert_eq!(result, should_match, "{comment}: path={file_path}");
    }
}

#[test]
fn test_files_priority() {
    let (matcher, fixture_dir) = create_matcher_from_fixture("files_priority");

    let test_cases = [
        ("test.ts", true, "files field overrides exclude"),
        ("other.ts", false, "file not in files array"),
    ];

    for (file_path, should_match, comment) in test_cases {
        let full_path = fixture_dir.join(file_path);
        let result = matcher.matches(&full_path);
        assert_eq!(result, should_match, "{comment}: path={file_path}");
    }
}

#[test]
fn test_outdir_exclude() {
    let (matcher, fixture_dir) = create_matcher_from_fixture("outdir_exclude");

    let test_cases = [
        ("src/index.ts", true, "source files included"),
        ("dist/index.js", false, "outDir automatically excluded"),
        ("dist/index.d.ts", false, "outDir automatically excluded"),
    ];

    for (file_path, should_match, comment) in test_cases {
        let full_path = fixture_dir.join(file_path);
        let result = matcher.matches(&full_path);
        assert_eq!(result, should_match, "{comment}: path={file_path}");
    }
}

#[test]
fn test_absolute_patterns() {
    let (matcher, fixture_dir) = create_matcher_from_fixture("absolute_patterns");

    let test_cases = [
        ("src/index.ts", true, "absolute pattern matches"),
        ("excluded/file.ts", false, "excluded directory"),
    ];

    for (file_path, should_match, comment) in test_cases {
        let full_path = fixture_dir.join(file_path);
        let result = matcher.matches(&full_path);
        assert_eq!(result, should_match, "{comment}: path={file_path}");
    }
}

#[test]
fn test_configdir_syntax() {
    let (matcher, fixture_dir) = create_matcher_from_fixture("configdir_syntax");

    let test_cases = [
        ("index.ts", true, "${configDir} matches root level .ts files"),
        ("log.ts", true, "${configDir} matches root level .ts files"),
        ("dist/output.js", false, "dist excluded"),
        ("src/index.ts", false, "${configDir}/*.ts doesn't match subdirectories"),
    ];

    for (file_path, should_match, comment) in test_cases {
        let full_path = fixture_dir.join(file_path);
        let result = matcher.matches(&full_path);
        assert_eq!(result, should_match, "{comment}: path={file_path}");
    }
}

#[test]
fn test_without_baseurl() {
    let (matcher, fixture_dir) = create_matcher_from_fixture("without_baseurl");

    let test_cases = [
        ("index.ts", true, "regular files included"),
        ("log.ts", true, "regular files included"),
        ("node_modules/package/index.ts", false, "node_modules excluded by default"),
        ("bower_components/lib.ts", false, "bower_components excluded by default"),
        ("jspm_packages/mod.ts", false, "jspm_packages excluded by default"),
        ("dist/output.js", false, "custom exclude pattern works"),
    ];

    for (file_path, should_match, comment) in test_cases {
        let full_path = fixture_dir.join(file_path);
        let result = matcher.matches(&full_path);
        assert_eq!(result, should_match, "{comment}: path={file_path}");
    }
}
