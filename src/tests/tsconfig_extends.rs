//! Tests for tsconfig extends functionality
//!
//! Tests the `extend_tsconfig` method which is responsible for inheriting
//! settings from one tsconfig into another.

use std::path::Path;

use super::manual_tsconfig;
use crate::{ResolveError, ResolveOptions, Resolver, SpecifierError, TsConfig, TsconfigReferences};

#[test]
fn test_extend_tsconfig() {
    let f = super::fixture_root().join("tsconfig/cases/extends");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Auto)),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");

    // Should inherit tsconfig from parent
    assert_eq!(resolution.files, Some(vec![f.join("files")]));
    assert_eq!(resolution.include, Some(vec![f.join("include")]));
    assert_eq!(resolution.exclude, Some(vec![f.join("exclude")]));

    let compiler_options = &resolution.compiler_options;
    assert_eq!(compiler_options.base_url, Some(f.join("src")));
    assert_eq!(compiler_options.allow_js, Some(true));
    assert_eq!(compiler_options.emit_decorator_metadata, Some(true));
    assert_eq!(compiler_options.use_define_for_class_fields, Some(true));
    assert_eq!(compiler_options.rewrite_relative_import_extensions, Some(true));

    assert_eq!(compiler_options.jsx, Some("react-jsx".to_string()));
    assert_eq!(compiler_options.jsx_factory, Some("React.createElement".to_string()));
    assert_eq!(compiler_options.jsx_fragment_factory, Some("React.Fragment".to_string()));
    assert_eq!(compiler_options.jsx_import_source, Some("react".to_string()));

    assert_eq!(compiler_options.out_dir, Some(f.join("dist")));
    assert_eq!(compiler_options.declaration_dir, Some(f.join("types")));
    assert_eq!(compiler_options.resolve_json_module, Some(true));
    assert_eq!(compiler_options.check_js, Some(false));
}

#[test]
fn test_extend_tsconfig_paths() {
    let f = super::fixture_root().join("tsconfig/cases/extends-paths-inheritance");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Auto)),
        extensions: vec![".ts".into(), ".js".into()],
        ..ResolveOptions::default()
    });

    // Test that paths are resolved correctly after inheritance
    let resolved_path =
        resolver.resolve_file(f.join("src").join("test.ts"), "@/test").map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f.join("src/test.ts")));
}

#[test]
fn test_extend_tsconfig_override_behavior() {
    let f = super::fixture_root().join("tsconfig/cases/extends-override");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Auto)),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");
    let compiler_options = &resolution.compiler_options;

    // Child should override parent values
    assert_eq!(compiler_options.jsx, Some("react".to_string()));
    assert_eq!(compiler_options.target, Some("ES2020".to_string()));
}

#[test]
fn test_extend_tsconfig_template_variables() {
    let f = super::fixture_root().join("tsconfig/cases/extends-template-vars");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Auto)),
        extensions: vec![".ts".into(), ".js".into()],
        ..ResolveOptions::default()
    });

    // Test that template variables work correctly with extends
    let resolved_path =
        resolver.resolve_file(f.join("src/utils.ts"), "@/utils").map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f.join("src/utils.ts")));
}

#[test]
fn test_extend_tsconfig_missing_file() {
    let f = super::fixture_root().join("tsconfig/cases");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(
            f.join("nonexistent-tsconfig.json"),
            TsconfigReferences::Auto,
        )),
        ..ResolveOptions::default()
    });

    let result = resolver.resolve_tsconfig(&f);
    assert!(matches!(result, Err(ResolveError::TsconfigNotFound(_))));
}

#[test]
fn test_extend_tsconfig_multiple_inheritance() {
    let f = super::fixture_root().join("tsconfig/cases/extends-chain");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Auto)),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");
    let compiler_options = &resolution.compiler_options;

    // Should have settings from all configs in the chain
    assert_eq!(compiler_options.experimental_decorators, Some(true));
    assert_eq!(compiler_options.target, Some("ES2022".to_string()));
    assert_eq!(compiler_options.module, Some("ESNext".to_string()));
}

#[test]
fn test_extend_tsconfig_preserves_child_settings() {
    let f = super::fixture_root().join("tsconfig/cases/extends-preserve-child");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Auto)),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");
    let compiler_options = &resolution.compiler_options;

    // Child should preserve its own settings and not inherit conflicting ones
    assert_eq!(compiler_options.jsx, Some("preserve".to_string())); // Child value
    assert_eq!(compiler_options.target, Some("ES2020".to_string())); // Inherited from parent
}

#[test]
fn test_extend_tsconfig_no_override_existing() {
    // Test the internal logic directly to ensure extend_tsconfig doesn't override existing values
    let parent_path = Path::new("/parent/tsconfig.json");
    let child_path = Path::new("/child/tsconfig.json");

    let parent_config = serde_json::json!({
        "compilerOptions": {
            "baseUrl": "./src",
            "jsx": "react-jsx",
            "target": "ES2020"
        }
    })
    .to_string();

    let child_config = serde_json::json!({
        "compilerOptions": {
            "jsx": "preserve"  // This should NOT be overridden
        }
    })
    .to_string();

    let parent_tsconfig =
        TsConfig::parse(true, parent_path, parent_path, parent_config).unwrap().build();
    let mut child_tsconfig = TsConfig::parse(true, child_path, child_path, child_config).unwrap();

    // Perform the extension
    child_tsconfig.extend_tsconfig(&parent_tsconfig);
    let child_built = child_tsconfig.build();

    let compiler_options = &child_built.compiler_options;

    // Child's jsx should be preserved
    assert_eq!(compiler_options.jsx, Some("preserve".to_string()));
    // Parent's target should be inherited
    assert_eq!(compiler_options.target, Some("ES2020".to_string()));
    // Parent's baseUrl should be inherited (with proper path resolution)
    assert!(compiler_options.base_url.is_some());
}

/// When a tsconfig's `extends` target does not exist,
/// `resolve_tsconfig` should return `TsconfigNotFound`.
#[test]
fn test_extend_tsconfig_not_found() {
    let f = super::fixture_root().join("tsconfig/cases/extends-not-found");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Disabled)),
        ..ResolveOptions::default()
    });

    let result = resolver.resolve_tsconfig(&f);
    assert!(
        matches!(&result, Err(ResolveError::TsconfigNotFound(_))),
        "expected TsconfigNotFound for missing extends target, got {result:?}",
    );
}

/// When a tsconfig's `extends` target exists but is not readable (e.g. permission denied),
/// `resolve_tsconfig` should return a `TsconfigLoadFailed` wrapping an `IOError`
/// (not silently skip it).
#[test]
fn test_extend_tsconfig_unreadable_file() {
    let f = super::fixture_root().join("tsconfig/cases/extends-unreadable");

    let fs = super::UnreadableFs { unreadable_path: f.join("base.json") };
    let resolver = crate::ResolverGeneric::new_with_file_system(
        fs,
        ResolveOptions {
            tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Disabled)),
            ..ResolveOptions::default()
        },
    );

    let result = resolver.resolve_tsconfig(&f);
    assert!(
        matches!(
            &result,
            Err(ResolveError::TsconfigLoadFailed { source, .. })
                if matches!(source.as_ref(), ResolveError::IOError(_))
        ),
        "expected TsconfigLoadFailed wrapping IOError for unreadable extends target, got {result:?}",
    );
}

#[test]
fn test_extend_package() {
    let f = super::fixture_root().join("tsconfig/cases");

    let data = ["extends-esm", "extends-main"];

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Auto)),
        ..ResolveOptions::default()
    });

    for dir in data {
        let resolution = resolver.resolve_tsconfig(f.join(dir)).expect("resolved");
        let compiler_options = &resolution.compiler_options;
        assert_eq!(compiler_options.target, Some("ES2020".to_string()));
    }
}

#[test]
fn test_extend_imports() {
    let f = super::fixture_root().join("tsconfig/cases/extends-imports");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Auto)),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(f.join("tsconfig-string.json")).expect("resolved");
    assert_eq!(resolution.compiler_options.target, Some("ES2020".to_string()));

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");
    assert_eq!(resolution.compiler_options.target, Some("ES2015".to_string()));

    let result = resolver.resolve_tsconfig(f.join("tsconfig-missing.json"));
    assert!(
        matches!(&result, Err(crate::ResolveError::TsconfigNotFound(_))),
        "expected TsconfigNotFound for an undefined `#` import, got {result:?}",
    );
}

fn assert_extends_symlink_resolves_to_canonical(config_file: &Path) {
    let f = super::fixture_root().join("tsconfig/cases/extends-symlink");
    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(config_file.to_path_buf(), TsconfigReferences::Disabled)),
        extensions: vec![".ts".into()],
        ..ResolveOptions::default()
    });

    let path = resolver.resolve(f.join("project"), "@app/foo").unwrap().full_path();
    assert_eq!(path, f.join("src/foo.ts"), "should resolve via canonical baseUrl, not symlink");

    // The caller-supplied path is preserved as identity; `paths_base` is canonical.
    let tsconfig = resolver.resolve_tsconfig(config_file).unwrap();
    assert_eq!(tsconfig.path.as_path(), config_file);
    assert_eq!(tsconfig.compiler_options.paths_base, f);
}

#[test]
fn test_extend_tsconfig_via_symlink_package() {
    let f = super::fixture_root().join("tsconfig/cases/extends-symlink");
    assert_extends_symlink_resolves_to_canonical(&f.join("project/tsconfig.json"));
}

#[test]
fn test_extend_tsconfig_via_symlink_relative() {
    let f = super::fixture_root().join("tsconfig/cases/extends-symlink");
    assert_extends_symlink_resolves_to_canonical(&f.join("project/tsconfig.relative.json"));
}

#[test]
fn test_merge_tsconfig() {
    let resolver = Resolver::default();
    let dir = super::fixture_root().join("tsconfig/cases/merge-compiler-options");
    let resolution = resolver.resolve_tsconfig(&dir).expect("resolved");
    let compiler_options = &resolution.compiler_options;
    assert_eq!(compiler_options.experimental_decorators, Some(true));
    assert_eq!(compiler_options.jsx, Some("react-jsx".to_string()));
    assert_eq!(compiler_options.jsx_factory, Some("h".to_string()));
    assert_eq!(compiler_options.jsx_fragment_factory, Some("Fragment".to_string()));
    assert_eq!(compiler_options.jsx_import_source, Some("xxx".to_string()));
    assert_eq!(compiler_options.module, Some("ESNext".to_string()));
    assert_eq!(compiler_options.target, Some("ESNext".to_string()));
}

#[test]
fn test_no_merge_tsconfig() {
    let resolver = Resolver::default();
    let dir = super::fixture_root().join("tsconfig/cases/no-merge-compiler-options");
    let resolution = resolver.resolve_tsconfig(&dir).expect("resolved");
    let compiler_options = &resolution.compiler_options;
    assert_eq!(compiler_options.experimental_decorators, Some(true));
    assert_eq!(compiler_options.jsx, Some("react-jsx".to_string()));
    assert_eq!(compiler_options.jsx_factory, Some("h".to_string()));
    assert_eq!(compiler_options.jsx_fragment_factory, Some("Fragment".to_string()));
    assert_eq!(compiler_options.jsx_import_source, Some("xxx".to_string()));
}

// ---------------------------------------------------------------------------
// Ported from typescript-go: extends error scenarios
// ---------------------------------------------------------------------------

#[test]
fn extends_circular_two_files() {
    let f = super::fixture_root().join("tsconfig/cases/extends-circular");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Disabled)),
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
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Disabled)),
        ..ResolveOptions::default()
    });

    let result = resolver.resolve_tsconfig(&f);
    assert!(
        matches!(&result, Err(ResolveError::TsconfigCircularExtend(_))),
        "expected TsconfigCircularExtend for self-extending tsconfig, got {result:?}",
    );
}

#[test]
fn extends_empty_string() {
    let f = super::fixture_root().join("tsconfig/cases/extends-empty-string");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Disabled)),
        ..ResolveOptions::default()
    });

    let result = resolver.resolve_tsconfig(&f);
    assert!(
        matches!(&result, Err(ResolveError::Specifier(SpecifierError::Empty(_)))),
        "expected SpecifierError::Empty for empty extends, got {result:?}",
    );
}

// ---------------------------------------------------------------------------
// Ported from typescript-go: extends path resolution
// ---------------------------------------------------------------------------

#[test]
fn extends_pkg_subpath() {
    let f = super::fixture_root().join("tsconfig/cases/extends-pkg-subpath");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Disabled)),
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
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Disabled)),
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
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Disabled)),
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
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Disabled)),
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
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Disabled)),
        ..ResolveOptions::default()
    });

    let resolution = resolver
        .resolve_tsconfig(&f)
        .expect("relative folder extends should resolve tsconfig.json");
    assert_eq!(resolution.compiler_options.target, Some("ES2019".to_string()));
}

// ---------------------------------------------------------------------------
// Ported from typescript-go: extends array semantics
// ---------------------------------------------------------------------------

#[test]
fn extends_array_last_wins() {
    let f = super::fixture_root().join("tsconfig/cases/extends-array-last-wins");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Disabled)),
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
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Disabled)),
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
// Ported from typescript-go: extends array edge cases
// ---------------------------------------------------------------------------

#[test]
fn extends_empty_array() {
    let f = super::fixture_root().join("tsconfig/cases/extends-empty-array");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Disabled)),
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
        tsconfig: Some(manual_tsconfig(f.join("tsconfig.json"), TsconfigReferences::Disabled)),
        ..ResolveOptions::default()
    });

    let resolution = resolver
        .resolve_tsconfig(&f)
        .expect("extending a config without compilerOptions should still work");
    assert_eq!(resolution.compiler_options.target, Some("ES2020".to_string()));
}
