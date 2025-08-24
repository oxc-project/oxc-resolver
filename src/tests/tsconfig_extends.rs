//! Tests for tsconfig extends functionality
//!
//! Tests the `extend_tsconfig` method which is responsible for inheriting
//! settings from one tsconfig into another.

use std::path::Path;

use crate::{ResolveOptions, Resolver, TsConfig, TsconfigOptions, TsconfigReferences};

#[test]
fn test_extend_tsconfig_base_url() {
    let f = super::fixture_root().join("tsconfig/cases/extends-base-url");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        }),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");
    let compiler_options = resolution.compiler_options();

    // Should inherit baseUrl from parent
    assert_eq!(compiler_options.base_url, Some(f.join("src")));
}

#[test]
fn test_extend_tsconfig_paths() {
    let f = super::fixture_root().join("tsconfig/cases/extends-paths-inheritance");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        }),
        extensions: vec![".ts".into(), ".js".into()],
        ..ResolveOptions::default()
    });

    // Test that paths are resolved correctly after inheritance
    let resolved_path = resolver.resolve(&f, "@/test").map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f.join("src/test.ts")));
}

#[test]
fn test_extend_tsconfig_override_behavior() {
    let f = super::fixture_root().join("tsconfig/cases/extends-override");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        }),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");
    let compiler_options = resolution.compiler_options();

    // Child should override parent values
    assert_eq!(compiler_options.jsx, Some("react".to_string()));
    assert_eq!(compiler_options.target, Some("ES2020".to_string()));
}

#[test]
fn test_extend_tsconfig_jsx_options() {
    let f = super::fixture_root().join("tsconfig/cases/extends-jsx");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        }),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");
    let compiler_options = resolution.compiler_options();

    // Should inherit all JSX-related options
    assert_eq!(compiler_options.jsx, Some("react-jsx".to_string()));
    assert_eq!(compiler_options.jsx_factory, Some("React.createElement".to_string()));
    assert_eq!(compiler_options.jsx_fragment_factory, Some("React.Fragment".to_string()));
    assert_eq!(compiler_options.jsx_import_source, Some("react".to_string()));
}

#[test]
fn test_extend_tsconfig_template_variables() {
    let f = super::fixture_root().join("tsconfig/cases/extends-template-vars");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        }),
        extensions: vec![".ts".into(), ".js".into()],
        ..ResolveOptions::default()
    });

    // Test that template variables work correctly with extends
    let resolved_path = resolver.resolve(&f, "@/utils").map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f.join("src/utils.ts")));
}

#[test]
fn test_extend_tsconfig_multiple_inheritance() {
    let f = super::fixture_root().join("tsconfig/cases/extends-chain");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        }),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");
    let compiler_options = resolution.compiler_options();

    // Should have settings from all configs in the chain
    assert_eq!(compiler_options.experimental_decorators, Some(true));
    assert_eq!(compiler_options.target, Some("ES2022".to_string()));
    assert_eq!(compiler_options.module, Some("ESNext".to_string()));
}

#[test]
fn test_extend_tsconfig_preserves_child_settings() {
    let f = super::fixture_root().join("tsconfig/cases/extends-preserve-child");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        }),
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");
    let compiler_options = resolution.compiler_options();

    // Child should preserve its own settings and not inherit conflicting ones
    assert_eq!(compiler_options.jsx, Some("preserve".to_string())); // Child value
    assert_eq!(compiler_options.target, Some("ES2020".to_string())); // Inherited from parent
}

#[test]
fn test_extend_tsconfig_no_override_existing() {
    // Test the internal logic directly to ensure extend_tsconfig doesn't override existing values
    let parent_path = Path::new("/parent/tsconfig.json");
    let child_path = Path::new("/child/tsconfig.json");

    let mut parent_config = serde_json::json!({
        "compilerOptions": {
            "baseUrl": "./src",
            "jsx": "react-jsx",
            "target": "ES2020"
        }
    })
    .to_string();

    let mut child_config = serde_json::json!({
        "compilerOptions": {
            "jsx": "preserve"  // This should NOT be overridden
        }
    })
    .to_string();

    let parent_tsconfig = TsConfig::parse(true, parent_path, &mut parent_config).unwrap().build();
    let mut child_tsconfig = TsConfig::parse(true, child_path, &mut child_config).unwrap();

    // Perform the extension
    child_tsconfig.extend_tsconfig(&parent_tsconfig);
    let child_built = child_tsconfig.build();

    let compiler_options = child_built.compiler_options();

    // Child's jsx should be preserved
    assert_eq!(compiler_options.jsx, Some("preserve".to_string()));
    // Parent's target should be inherited
    assert_eq!(compiler_options.target, Some("ES2020".to_string()));
    // Parent's baseUrl should be inherited (with proper path resolution)
    assert!(compiler_options.base_url.is_some());
}
