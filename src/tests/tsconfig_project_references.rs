//! Tests for tsconfig project references

use crate::{
    ResolveError, ResolveOptions, Resolver, TsconfigDiscovery, TsconfigOptions, TsconfigReferences,
};

#[test]
fn auto() {
    let f = super::fixture_root().join("tsconfig/cases/project-references");

    // The following resolver's `config_file` has defined it's own paths alias which has higher priority
    // some cases will work without references
    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("app"),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        // Test normal paths alias
        (f.join("app/index.ts"), "@/index.ts", f.join("app/aliased/index.ts")),
        (f.join("app/index.ts"), "@/../index.ts", f.join("app/index.ts")),
        // Test project reference
        (f.join("project-a/index.ts"), "@/index.ts", f.join("project-a/aliased/index.ts")),
        (f.join("project-b/src/aliased/index.ts"), "@/index.ts", f.join("project-b/src/aliased/index.ts")),
        // Does not have paths alias
        (f.join("project-a/index.ts"), "./index.ts", f.join("project-a/index.ts")),
        (f.join("project-c/index.ts"), "./index.ts", f.join("project-c/index.ts")),
        // Template variable
        {
            let file = f.parent().unwrap().join("paths-template-variable/src/foo.js");
            (file.clone(), "foo", file)
        }
    ];

    for (path, request, expected) in pass {
        let resolved_path = resolver.resolve_file(&path, request).map(|f| f.full_path());
        assert_eq!(resolved_path, Ok(expected), "{request} {path:?}");
    }

    // The following resolver's `config_file` has no `paths` alias with `references` enabled
    let no_paths_resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("app/tsconfig.nopaths.json"),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        // Test normal paths alias
        (f.join("app"), "@/index.ts", Err(ResolveError::NotFound("@/index.ts".into()))),
        (f.join("app"), "@/../index.ts", Ok(f.join("app/index.ts"))),
        // Test project reference
        (f.join("project-a"), "@/index.ts", Ok(f.join("project-a/aliased/index.ts"))),
        (f.join("project-b/src"), "@/index.ts", Ok(f.join("project-b/src/aliased/index.ts"))),
        // Does not have paths alias
        (f.join("project-a"), "./index.ts", Ok(f.join("project-a/index.ts"))),
        (f.join("project-c"), "./index.ts", Ok(f.join("project-c/index.ts"))),
        // Template variable
        {
            let dir = f.parent().unwrap().join("paths-template-variable");
            (dir.clone(), "foo", Ok(dir.join("src/foo.js")))
        }
    ];

    for (path, request, expected) in pass {
        let resolved_path = no_paths_resolver.resolve(&path, request).map(|f| f.full_path());
        assert_eq!(resolved_path, expected, "{request} {path:?}");
    }
}

#[test]
fn disabled() {
    let f = super::fixture_root().join("tsconfig/cases/project-references");

    // The following resolver's `config_file` has defined it's own paths alias which has higher priority
    // some cases will work without references
    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("app"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    // The following resolver's `config_file` has no `paths` alias with `references` enabled
    let no_paths_resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("app/tsconfig.nopaths.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        // Test normal paths alias
        (f.join("app"), "@/index.ts", Ok(f.join("app/aliased/index.ts"))),
        (f.join("app"), "@/../index.ts", Ok(f.join("app/index.ts"))),
        // Test project reference
        (f.join("project-a"), "@/index.ts", Ok(f.join("app/aliased/index.ts"))),
        (f.join("project-b/src"), "@/index.ts", Ok(f.join("app/aliased/index.ts"))),
        // Does not have paths alias
        (f.join("project-a"), "./index.ts", Ok(f.join("project-a/index.ts"))),
        (f.join("project-c"), "./index.ts", Ok(f.join("project-c/index.ts"))),
    ];

    for (path, request, expected) in pass {
        let resolved_path = resolver.resolve(&path, request).map(|f| f.full_path());
        assert_eq!(resolved_path, expected, "{request} {path:?}");
    }

    #[rustfmt::skip]
    let pass = [
        // Test normal paths alias
        (f.join("app"), "@/index.ts", Err(ResolveError::NotFound("@/index.ts".into()))),
        (f.join("app"), "#/../index.ts", Ok(f.join("app/index.ts"))), // This works because "#/../index.ts" is resolved as "./index.ts"
        // Test project reference
        (f.join("project-a"), "@/index.ts", Err(ResolveError::NotFound("@/index.ts".into()))),
        (f.join("project-b/src"), "@/index.ts", Err(ResolveError::NotFound("@/index.ts".into()))),
        // Does not have paths alias
        (f.join("project-a"), "./index.ts", Ok(f.join("project-a/index.ts"))),
        (f.join("project-c"), "./index.ts", Ok(f.join("project-c/index.ts"))),
    ];

    for (path, request, expected) in pass {
        let resolved_path = no_paths_resolver.resolve(&path, request).map(|f| f.full_path());
        assert_eq!(resolved_path, expected, "{request} {path:?}");
    }
}

#[test]
fn manual() {
    let f = super::fixture_root().join("tsconfig/cases/project-references");

    // The following resolver's `config_file` has defined it's own paths alias which has higher priority
    // some cases will work without references
    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("app"),
            references: TsconfigReferences::Paths(vec!["../project-a/conf.json".into()]),
        })),
        ..ResolveOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        // Test normal paths alias
        (f.join("app"), "@/index.ts", Ok(f.join("app/aliased/index.ts"))),
        (f.join("app"), "@/../index.ts", Ok(f.join("app/index.ts"))),
        // Test project reference
        (f.join("project-a"), "@/index.ts", Ok(f.join("project-a/aliased/index.ts"))),
        (f.join("project-b/src"), "@/index.ts", Ok(f.join("app/aliased/index.ts"))),
        // Does not have paths alias
        (f.join("project-a"), "./index.ts", Ok(f.join("project-a/index.ts"))),
        (f.join("project-c"), "./index.ts", Ok(f.join("project-c/index.ts"))),
    ];

    for (path, request, expected) in pass {
        let resolved_path = resolver.resolve(&path, request).map(|f| f.full_path());
        assert_eq!(resolved_path, expected, "{request} {path:?}");
    }

    // The following resolver's `config_file` has no `paths` alias with `references` enabled
    let no_paths_resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("app/tsconfig.nopaths.json"),
            references: TsconfigReferences::Paths(vec!["../project-a/conf.json".into()]),
        })),
        ..ResolveOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        // Test normal paths alias
        (f.join("app"), "@/index.ts", Err(ResolveError::NotFound("@/index.ts".into()))),
        (f.join("app"), "@/../index.ts", Ok(f.join("app/index.ts"))),
        // Test project reference
        (f.join("project-a"), "@/index.ts", Ok(f.join("project-a/aliased/index.ts"))),
        (f.join("project-b/src"), "@/index.ts", Err(ResolveError::NotFound("@/index.ts".into()))),
        // Does not have paths alias
        (f.join("project-a"), "./index.ts", Ok(f.join("project-a/index.ts"))),
        (f.join("project-c"), "./index.ts", Ok(f.join("project-c/index.ts"))),
    ];

    for (path, request, expected) in pass {
        let resolved_path = no_paths_resolver.resolve(&path, request).map(|f| f.full_path());
        assert_eq!(resolved_path, expected, "{request} {path:?}");
    }
}

#[test]
fn self_reference() {
    let f = super::fixture_root().join("tsconfig/cases/project-references");

    #[rustfmt::skip]
    let pass = [
        (f.join("app"), vec!["./tsconfig.json".into()]),
        (f.join("app/tsconfig.json"), vec!["./tsconfig.json".into()]),
        (f.join("app"), vec![f.join("app")]),
        (f.join("app/tsconfig.json"), vec![f.join("app")]),
        (f.join("app/tsconfig.json"), vec![f.join("project-b"), f.join("app")]),
    ];

    for (config_file, reference_paths) in pass {
        let resolver = Resolver::new(ResolveOptions {
            tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
                config_file: config_file.clone(),
                references: TsconfigReferences::Paths(reference_paths.clone()),
            })),
            ..ResolveOptions::default()
        });
        let path = f.join("app");
        let resolved_path = resolver.resolve(&path, "@/index.ts").map(|f| f.full_path());
        assert_eq!(
            resolved_path,
            Err(ResolveError::TsconfigSelfReference(f.join("app/tsconfig.json"))),
            "{config_file:?} {reference_paths:?}"
        );
    }
}

#[test]
fn references_with_extends() {
    let f = super::fixture_root().join("tsconfig/cases/project-references/extends");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".ts".into(), ".tsx".into()],
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.clone(),
            references: TsconfigReferences::Auto,
        })),
        ..ResolveOptions::default()
    });

    let resolved_path = resolver.resolve_file(f.join("src"), "@/pages").map(|f| f.full_path());

    assert_eq!(resolved_path, Ok(f.join("src/pages/index.tsx")));
}
