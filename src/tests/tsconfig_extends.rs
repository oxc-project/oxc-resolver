//! Tests for tsconfig extends functionality
//!
//! Tests the `extend_tsconfig` method which is responsible for inheriting
//! settings from one tsconfig into another.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    ResolveOptions, Resolver, TsConfig, TsconfigDiscovery, TsconfigOptions, TsconfigReferences,
};

#[test]
fn test_extend_tsconfig() {
    let f = super::fixture_root().join("tsconfig/cases/extends");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
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
}

#[test]
fn test_extend_tsconfig_paths() {
    let f = super::fixture_root().join("tsconfig/cases/extends-paths-inheritance");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
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
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
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
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
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
    use crate::ResolveError;

    let f = super::fixture_root().join("tsconfig/cases");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("nonexistent-tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });

    let result = resolver.resolve_tsconfig(&f);
    assert!(matches!(result, Err(ResolveError::TsconfigNotFound(_))));
}

#[test]
fn test_extend_tsconfig_multiple_inheritance() {
    let f = super::fixture_root().join("tsconfig/cases/extends-chain");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
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
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
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
    use crate::ResolveError;

    let f = super::fixture_root().join("tsconfig/cases/extends-not-found");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let result = resolver.resolve_tsconfig(&f);
    assert!(
        matches!(&result, Err(ResolveError::TsconfigNotFound(_))),
        "expected TsconfigNotFound for missing extends target, got {result:?}",
    );
}

/// When a tsconfig's `references` target does not exist,
/// `resolve_tsconfig` should return `TsconfigNotFound`.
#[test]
fn test_references_not_found() {
    use crate::ResolveError;

    let f = super::fixture_root().join("tsconfig/cases/references-not-found");

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });

    let result = resolver.resolve_tsconfig(&f);
    assert!(
        matches!(&result, Err(ResolveError::TsconfigNotFound(_))),
        "expected TsconfigNotFound for missing references target, got {result:?}",
    );
}

/// A filesystem wrapper that returns `PermissionDenied` for `read_to_string`
/// on a specific path, delegating everything else to the real OS filesystem.
struct UnreadableFs {
    unreadable_path: PathBuf,
}

impl crate::FileSystem for UnreadableFs {
    #[cfg(not(feature = "yarn_pnp"))]
    fn new() -> Self {
        unreachable!()
    }

    #[cfg(feature = "yarn_pnp")]
    fn new(_yarn_pnp: bool) -> Self {
        unreachable!()
    }

    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        if path == self.unreadable_path {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "permission denied",
            ));
        }
        crate::FileSystemOs::read_to_string(path)
    }

    fn metadata(&self, path: &Path) -> std::io::Result<crate::FileMetadata> {
        crate::FileSystemOs::metadata(path)
    }

    fn symlink_metadata(&self, path: &Path) -> std::io::Result<crate::FileMetadata> {
        crate::FileSystemOs::symlink_metadata(path)
    }

    fn read_link(&self, path: &Path) -> Result<PathBuf, crate::ResolveError> {
        crate::FileSystemOs::read_link(path)
    }

    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf> {
        crate::FileSystemOs::canonicalize(path)
    }
}

/// When a tsconfig's `extends` target exists but is not readable (e.g. permission denied),
/// `resolve_tsconfig` should return an `IOError` (not silently skip it).
#[test]
fn test_extend_tsconfig_unreadable_file() {
    use crate::ResolveError;

    let f = super::fixture_root().join("tsconfig/cases/extends-unreadable");

    let fs = UnreadableFs { unreadable_path: f.join("base.json") };
    let resolver = crate::ResolverGeneric::new_with_file_system(
        fs,
        ResolveOptions {
            tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
                config_file: f.join("tsconfig.json"),
                references: TsconfigReferences::Disabled,
            })),
            ..ResolveOptions::default()
        },
    );

    let result = resolver.resolve_tsconfig(&f);
    assert!(
        matches!(&result, Err(ResolveError::IOError(_))),
        "expected IOError for unreadable extends target, got {result:?}",
    );
}

/// When a tsconfig's `references` target exists but is not readable (e.g. permission denied),
/// `resolve_tsconfig` should return an `IOError`.
#[test]
fn test_references_unreadable_file() {
    use crate::ResolveError;

    let f = super::fixture_root().join("tsconfig/cases/references-unreadable");

    let fs = UnreadableFs { unreadable_path: f.join("referenced/tsconfig.json") };
    let resolver = crate::ResolverGeneric::new_with_file_system(
        fs,
        ResolveOptions {
            tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
                config_file: f.join("tsconfig.json"),
                references: TsconfigReferences::Auto,
            })),
            ..ResolveOptions::default()
        },
    );

    let result = resolver.resolve_tsconfig(&f);
    assert!(
        matches!(&result, Err(ResolveError::IOError(_))),
        "expected IOError for unreadable references target, got {result:?}",
    );
}

#[test]
fn test_extend_package() {
    let f = super::fixture_root().join("tsconfig/cases");

    let data = ["extends-esm", "extends-main"];

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });

    for dir in data {
        let resolution = resolver.resolve_tsconfig(f.join(dir)).expect("resolved");
        let compiler_options = &resolution.compiler_options;
        assert_eq!(compiler_options.target, Some("ES2020".to_string()));
    }
}

/// Create a directory symlink, cleaning up any stale one from previous runs.
/// Returns `false` if symlinks are not supported on this platform.
#[cfg_attr(target_family = "wasm", allow(dead_code))]
fn create_dir_symlink(target: &Path, link: &Path) -> bool {
    let _ = fs::remove_file(link);
    let _ = fs::remove_dir_all(link);

    #[cfg(target_family = "unix")]
    {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }
    #[cfg(target_os = "windows")]
    {
        std::os::windows::fs::symlink_dir(target, link).unwrap();
        true
    }
    #[cfg(target_family = "wasm")]
    {
        false
    }
}

/// Assert that `@app/foo` resolves to `extends-symlink/src/foo.ts` (via the real
/// base config path), not `extends-symlink/project/src/foo.ts` (via the symlink).
fn assert_symlink_extends_resolves_correctly(config_file: PathBuf, resolve_dir: &Path) {
    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file,
            references: TsconfigReferences::Disabled,
        })),
        extensions: vec![".ts".into(), ".js".into()],
        ..ResolveOptions::default()
    });

    let resolved_path = resolver
        .resolve(resolve_dir, "@app/foo")
        .expect("should resolve @app/foo via tsconfig paths")
        .full_path();
    assert!(
        resolved_path.ends_with("src/foo.ts"),
        "expected path ending with src/foo.ts, got {resolved_path:?}"
    );
    assert!(
        !resolved_path.to_string_lossy().contains("project/src"),
        "should resolve to root src/foo.ts, not project/src/foo.ts, got {resolved_path:?}"
    );
}

/// When a tsconfig extends another via a symlinked package name (e.g. pnpm workspace),
/// `baseUrl` and `paths` should be resolved relative to the real (canonical) path
/// of the extended tsconfig, matching TypeScript's behavior.
#[test]
#[cfg_attr(target_family = "wasm", ignore)]
fn test_extend_tsconfig_via_symlink_package() {
    let f = super::fixture_root().join("tsconfig/cases/extends-symlink");
    let symlink_path = f.join("project/node_modules/shared-config");
    let real_target = f.join("real-configs").canonicalize().unwrap();

    if !create_dir_symlink(&real_target, &symlink_path) {
        return;
    }

    // extends: "shared-config/base"
    assert_symlink_extends_resolves_correctly(f.join("project/tsconfig.json"), &f.join("project"));

    let _ = fs::remove_file(&symlink_path);
    let _ = fs::remove_dir_all(&symlink_path);
}

/// Same as above but with a relative `extends` path going through a symlinked directory.
#[test]
#[cfg_attr(target_family = "wasm", ignore)]
fn test_extend_tsconfig_via_symlink_relative() {
    let f = super::fixture_root().join("tsconfig/cases/extends-symlink");
    let symlink_path = f.join("project/configs");
    let real_target = f.join("real-configs").canonicalize().unwrap();

    if !create_dir_symlink(&real_target, &symlink_path) {
        return;
    }

    // extends: "./configs/base.json"
    assert_symlink_extends_resolves_correctly(
        f.join("project/tsconfig.relative.json"),
        &f.join("project"),
    );

    let _ = fs::remove_file(&symlink_path);
    let _ = fs::remove_dir_all(&symlink_path);
}

/// Same as above but with an absolute `extends` path going through a symlinked directory.
#[test]
#[cfg_attr(target_family = "wasm", ignore)]
fn test_extend_tsconfig_via_symlink_absolute() {
    let f = super::fixture_root().join("tsconfig/cases/extends-symlink");
    // Use a unique symlink name to avoid racing with the relative test
    let symlink_path = f.join("project/configs-abs");
    let real_target = f.join("real-configs").canonicalize().unwrap();

    if !create_dir_symlink(&real_target, &symlink_path) {
        return;
    }

    // Write a tsconfig with an absolute extends path at runtime (not portable for fixtures)
    let absolute_tsconfig = f.join("project/tsconfig.absolute.json");
    let absolute_extends = symlink_path.join("base.json");
    fs::write(
        &absolute_tsconfig,
        format!(r#"{{ "extends": "{}" }}"#, absolute_extends.to_string_lossy().replace('\\', "/")),
    )
    .unwrap();

    assert_symlink_extends_resolves_correctly(absolute_tsconfig.clone(), &f.join("project"));

    let _ = fs::remove_file(&absolute_tsconfig);
    let _ = fs::remove_file(&symlink_path);
    let _ = fs::remove_dir_all(&symlink_path);
}
