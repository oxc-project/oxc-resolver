//! Tests for tsconfig `include`, `exclude`, and `files` fields
//!
//! Tests ported from vite-tsconfig-paths:
//! <https://github.com/aleclarson/vite-tsconfig-paths>

use crate::{ResolveOptions, Resolver, TsconfigDiscovery, TsconfigOptions, TsconfigReferences};

/// Main test for include/exclude/files pattern matching
/// Tests basic glob patterns, exclude filtering, and files field priority
#[test]
fn tsconfig_include_exclude_patterns() {
    let f = super::fixture_root().join("tsconfig/cases");

    // (fixture_dir, file_path, should_match, description)
    #[rustfmt::skip]
    let test_cases = [
        // Include basic - Pattern: src/**/*.ts
        ("include_basic", "src/index.ts", true, "include pattern matches file in src/"),
        ("include_basic", "src/utils/helper.ts", true, "include pattern matches nested file"),
        ("include_basic", "test.ts", false, "file outside include pattern"),
        ("include_basic", "dist/output.js", false, "non-ts file not included"),

        // Exclude basic - Include: **/*.ts, Exclude: **/*.test.ts
        ("exclude_basic", "src/index.ts", true, "file matches include and not exclude"),
        ("exclude_basic", "src/index.test.ts", false, "file excluded by exclude pattern"),
        ("exclude_basic", "node_modules/foo.ts", false, "node_modules excluded by default"),

        // Files priority - Files: [test.ts], Exclude: [test.ts]
        ("files_priority", "test.ts", true, "files field overrides exclude"),
        ("files_priority", "other.ts", false, "file not in files array"),

        // Default include (no include specified, defaults to **/*) - Exclude: [dist]
        ("with_baseurl", "index.ts", true, "default include matches all files"),
        ("with_baseurl", "log.ts", true, "default include matches all files"),
        ("with_baseurl", "dist/output.js", false, "dist directory excluded"),

        // Default exclude (node_modules, bower_components, jspm_packages) - Exclude: [dist]
        ("without_baseurl", "index.ts", true, "regular files included"),
        ("without_baseurl", "log.ts", true, "regular files included"),
        ("without_baseurl", "node_modules/package/index.ts", false, "node_modules excluded by default"),
        ("without_baseurl", "bower_components/lib.ts", false, "bower_components excluded by default"),
        ("without_baseurl", "jspm_packages/mod.ts", false, "jspm_packages excluded by default"),
        ("without_baseurl", "dist/output.js", false, "custom exclude pattern works"),

        // Template variable ${configDir} - Include: ${configDir}/*.ts
        ("configdir_syntax", "index.ts", true, "${configDir} matches root level .ts files"),
        ("configdir_syntax", "log.ts", true, "${configDir} matches root level .ts files"),
        ("configdir_syntax", "dist/output.js", false, "dist excluded"),
        ("configdir_syntax", "src/index.ts", false, "${configDir}/*.ts doesn't match subdirectories"),

        // Extends inheritance - Base: include **/*.ts, exclude **/*.test.ts; Child: include src/**/*.ts
        ("include_exclude_extends", "src/index.ts", true, "child include pattern matches"),
        ("include_exclude_extends", "lib/utils.ts", false, "child include overrides parent (lib not in src)"),

        // Globstar patterns - Include: **/*.ts
        ("globstar_patterns", "index.ts", true, "globstar matches root level"),
        ("globstar_patterns", "src/index.ts", true, "globstar matches one level deep"),
        ("globstar_patterns", "src/utils/helper.ts", true, "globstar matches two levels deep"),
        ("globstar_patterns", "src/utils/deep/nested/file.ts", true, "globstar matches deeply nested"),
        ("globstar_patterns", "index.js", false, "globstar doesn't match .js files"),
        ("globstar_patterns", "src/index.js", false, "globstar doesn't match .js files in subdirs"),

        // Wildcard patterns - Include: src/*.ts
        ("wildcard_patterns", "src/index.ts", true, "wildcard matches files in src/"),
        ("wildcard_patterns", "src/helper.ts", true, "wildcard matches files in src/"),
        ("wildcard_patterns", "src/utils/helper.ts", false, "wildcard doesn't match subdirectories"),
        ("wildcard_patterns", "index.ts", false, "wildcard doesn't match parent directory"),

        // Character set patterns - Include: [A-Z]*.ts
        ("character_set_patterns", "App.ts", true, "character set matches uppercase start"),
        ("character_set_patterns", "Button.ts", true, "character set matches uppercase start"),

        // Monorepo patterns - Include: packages/*/src/**/*
        ("monorepo_patterns", "packages/pkg-a/src/index.ts", true, "monorepo pattern matches pkg-a"),
        ("monorepo_patterns", "packages/pkg-b/src/utils.ts", true, "monorepo pattern matches pkg-b"),
        ("monorepo_patterns", "packages/pkg-c/src/deep/nested/file.ts", true, "monorepo pattern matches nested"),
        ("monorepo_patterns", "packages/pkg-a/dist/index.js", false, "dist not in src directory"),
        ("monorepo_patterns", "shared/utils.ts", false, "shared not in packages/*/src"),
        ("monorepo_patterns", "packages/pkg-a/test.ts", false, "test.ts not in src directory"),

        // outDir auto-exclude - Include: **/*.ts, CompilerOptions.outDir: dist
        ("outdir_exclude", "src/index.ts", true, "source files included"),
        ("outdir_exclude", "dist/index.js", false, "outDir automatically excluded"),
        ("outdir_exclude", "dist/index.d.ts", false, "outDir automatically excluded"),

        // Complex patterns - Include: src/**/test/**/*.spec.ts
        ("complex_patterns", "src/test/unit.spec.ts", true, "complex pattern matches src/test/*.spec.ts"),
        ("complex_patterns", "src/utils/test/helper.spec.ts", true, "complex pattern matches src/**/test/*.spec.ts"),
        ("complex_patterns", "src/deep/nested/test/component.spec.ts", true, "complex pattern matches deeply nested"),
        ("complex_patterns", "src/index.ts", false, "file not in test directory"),
        ("complex_patterns", "src/utils/helper.ts", false, "file not in test directory"),
        ("complex_patterns", "src/test/unit.test.ts", false, "wrong extension (.test.ts not .spec.ts)"),

        // Absolute patterns - Include: src/**/*.ts, Exclude: excluded
        ("absolute_patterns", "src/index.ts", true, "absolute pattern matches"),
        ("absolute_patterns", "excluded/file.ts", false, "excluded directory"),
    ];

    for (fixture, file_path, should_match, comment) in test_cases {
        let fixture_dir = f.join(fixture);
        let resolver = Resolver::new(ResolveOptions {
            tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
                config_file: fixture_dir.join("tsconfig.json"),
                references: TsconfigReferences::Auto,
            })),
            ..ResolveOptions::default()
        });

        let tsconfig = resolver.resolve_tsconfig(&fixture_dir).unwrap();
        let result = tsconfig.matches_file(&fixture_dir.join(file_path));

        assert_eq!(result, should_match, "{comment}: fixture={fixture} file={file_path}");
    }
}

/// Test empty files array with no include
/// When files is explicitly empty and include is missing/empty, no files should match
#[test]
fn test_empty_files_no_include() {
    let f = super::fixture_root().join("tsconfig/cases/empty_files_no_include");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });

    let tsconfig = resolver.resolve_tsconfig(&f).unwrap();

    // With empty files and no include, no files should match
    assert!(!tsconfig.matches_file(&f.join("index.ts")));
    assert!(!tsconfig.matches_file(&f.join("src/index.ts")));
}

/// Test empty include array
/// When include is explicitly set to empty array, no files should be included
#[test]
fn test_empty_include_array() {
    let f = super::fixture_root().join("tsconfig/cases/empty_include");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });

    let tsconfig = resolver.resolve_tsconfig(&f).unwrap();

    // With empty include, no files should match (unless in files array)
    assert!(!tsconfig.matches_file(&f.join("index.ts")));
    assert!(!tsconfig.matches_file(&f.join("src/index.ts")));
}

/// Test extends inheritance behavior for include/exclude
/// Verifies that child config's include/exclude patterns override parent's
#[test]
fn test_extends_include_exclude_inheritance() {
    let f = super::fixture_root().join("tsconfig/cases/include_exclude_extends");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });

    let tsconfig = resolver.resolve_tsconfig(&f).unwrap();

    // Child config's include overrides parent's include
    assert!(tsconfig.matches_file(&f.join("src/index.ts")));

    // Files outside child's include pattern should not be included
    // (even if they would match parent's include)
    assert!(!tsconfig.matches_file(&f.join("lib/utils.ts")));

    // Test whether exclude from parent applies to child's include
    // (behavior to be determined by implementation)
    let _is_test_file_included = tsconfig.matches_file(&f.join("src/index.test.ts"));
}

/// Test project references with include/exclude
/// References allow composing multiple tsconfigs, each with their own include/exclude
#[test]
fn test_project_references_with_include_exclude() {
    let f = super::fixture_root().join("tsconfig/cases/extends_paths");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });

    // Root tsconfig has empty include: []
    let root_tsconfig = resolver.resolve_tsconfig(&f).unwrap();

    // Root has empty include, so no files at root should match
    assert!(!root_tsconfig.matches_file(&f.join("index.ts")));

    // Referenced project should have its own include
    let my_app_dir = f.join("my-app");
    let my_app_tsconfig = resolver.resolve_tsconfig(&my_app_dir).unwrap();

    // my-app has include: ["./"], so files in my-app should match
    assert!(my_app_tsconfig.matches_file(&my_app_dir.join("index.ts")));
    assert!(my_app_tsconfig.matches_file(&my_app_dir.join("message.ts")));
}

/// Test paths outside project root
/// Paths can reference files outside the tsconfig directory
#[test]
fn test_paths_outside_root() {
    let f = super::fixture_root().join("tsconfig/cases/paths_outside_root");
    let my_app_dir = f.join("my-app");
    let my_utils_dir = f.join("my-utils");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: my_app_dir.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });

    let tsconfig = resolver.resolve_tsconfig(&my_app_dir).unwrap();

    // Files in my-app should be included (default include)
    assert!(tsconfig.matches_file(&my_app_dir.join("index.ts")));

    // Files outside the tsconfig directory - behavior depends on implementation
    // Whether include/exclude patterns can reach outside the tsconfig directory
    let _is_outside_file_included = tsconfig.matches_file(&my_utils_dir.join("log.ts"));
}

/// Test case sensitivity on Unix systems
/// On Unix, file paths are case-sensitive
#[test]
#[cfg(not(target_os = "windows"))]
fn test_case_sensitivity_unix() {
    let f = super::fixture_root().join("tsconfig/cases/case_sensitivity");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });

    let tsconfig = resolver.resolve_tsconfig(&f).unwrap();

    // Pattern: src/**/*.ts
    assert!(tsconfig.matches_file(&f.join("src/index.ts")));

    // On Unix, Src != src (case-sensitive)
    assert!(!tsconfig.matches_file(&f.join("Src/index.ts")));
}

/// Test case insensitivity on Windows
/// On Windows, file paths are case-insensitive
#[test]
#[cfg(target_os = "windows")]
fn test_case_insensitivity_windows() {
    let f = super::fixture_root().join("tsconfig/cases/case_sensitivity");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });

    let tsconfig = resolver.resolve_tsconfig(&f).unwrap();

    // Pattern: src/**/*.ts
    assert!(tsconfig.matches_file(&f.join("src/index.ts")));

    // On Windows, Src == src (case-insensitive)
    assert!(tsconfig.matches_file(&f.join("Src/index.ts")));
}
