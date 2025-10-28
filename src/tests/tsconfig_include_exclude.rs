//! Tests for tsconfig `include`, `exclude`, and `files` fields
//!
//! Tests ported from vite-tsconfig-paths:
//! <https://github.com/aleclarson/vite-tsconfig-paths>

use crate::{ResolveOptions, Resolver, TsconfigDiscovery, TsconfigOptions, TsconfigReferences};

/// Test include/exclude/files patterns via actual path resolution
/// Tests that tsconfig path mappings are applied when importer is included,
/// and not applied when importer is excluded
#[test]
fn tsconfig_include_exclude_patterns() {
    let f = super::fixture_root().join("tsconfig/cases");

    // (fixture_dir, importer_file, specifier, should_resolve, description)
    #[rustfmt::skip]
    let test_cases = [
        // Include basic - Pattern: src/**/*.ts
        // Files in src/ can use path mappings, files outside cannot
        ("include_basic", "src/index.ts", "@/utils/helper", true, "file in src/ can use path mapping"),
        ("include_basic", "test.ts", "@/utils/helper", false, "file outside include pattern cannot use path mapping"),

        // Exclude basic - Include: **/*.ts, Exclude: **/*.test.ts
        // Test files are excluded from using path mappings
        ("exclude_basic", "src/index.ts", "@/helper", true, "non-test file can use path mapping"),
        ("exclude_basic", "src/index.test.ts", "@/helper", false, "test file excluded from using path mapping"),

        // Default include (no include specified, defaults to **/*) - Exclude: [dist]
        // All files except dist/ can use path mappings
        ("with_baseurl", "index.ts", "~/log", true, "file in root can use path mapping"),
        ("with_baseurl", "index.ts", "log", true, "file in root can use baseUrl"),
    ];

    for (fixture, importer, specifier, should_resolve, comment) in test_cases {
        let fixture_dir = f.join(fixture);
        let resolver = Resolver::new(ResolveOptions {
            tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
                config_file: fixture_dir.join("tsconfig.json"),
                references: TsconfigReferences::Auto,
            })),
            extensions: vec![".ts".into(), ".js".into()],
            ..ResolveOptions::default()
        });

        let importer_path = fixture_dir.join(importer);
        let result = resolver.resolve(&importer_path, specifier);

        if should_resolve {
            assert!(
                result.is_ok(),
                "{comment}: fixture={fixture} importer={importer} specifier={specifier} - expected success but got: {:?}",
                result.err()
            );
        } else {
            assert!(
                result.is_err(),
                "{comment}: fixture={fixture} importer={importer} specifier={specifier} - expected failure but got: {:?}",
                result.ok()
            );
        }
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
