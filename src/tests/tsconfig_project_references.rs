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

    let file = f.join("src").join("src").join("index.tsx");
    let resolved_path = resolver.resolve_file(file, "@/pages").map(|f| f.full_path());

    assert_eq!(resolved_path, Ok(f.join("src/pages/index.tsx")));
}

#[test]
fn referenced_paths_win_over_root_with_no_paths() {
    let f = super::fixture_root().join("tsconfig/cases/project-references-priority");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".ts".into()],
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });

    let resolved_path =
        resolver.resolve_file(f.join("app/index.ts"), "1/foo").map(|f| f.full_path());

    assert_eq!(resolved_path, Ok(f.join("lib/foo.ts")));
}

#[test]
fn walk_up_when_ref_files_does_not_cover_file() {
    let f = super::fixture_root().join("tsconfig/cases/project-references-walk-up/files-misses");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".ts".into()],
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });

    let tsconfig = resolver.find_tsconfig(f.join("pkg-a/src/bar.ts")).unwrap().unwrap();
    assert_eq!(tsconfig.path(), f.join("tsconfig.json"));

    let resolved_path =
        resolver.resolve_file(f.join("pkg-a/src/bar.ts"), "@/foo").map(|f| f.full_path());
    assert_eq!(resolved_path, Err(ResolveError::NotFound("@/foo".into())));
}

#[test]
fn walk_up_when_ref_excludes_file() {
    let f = super::fixture_root().join("tsconfig/cases/project-references-walk-up/exclude-pattern");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".ts".into()],
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });

    let tsconfig = resolver.find_tsconfig(f.join("pkg-a/src/excluded/bar.ts")).unwrap().unwrap();
    assert_eq!(tsconfig.path(), f.join("tsconfig.json"));

    let resolved_path =
        resolver.resolve_file(f.join("pkg-a/src/excluded/bar.ts"), "@/foo").map(|f| f.full_path());
    assert_eq!(resolved_path, Err(ResolveError::NotFound("@/foo".into())));
}

#[test]
fn referenced_config_allow_js_uses_own_setting() {
    // A Vite-style solution `tsconfig.json` (empty `include`, only `references`)
    // does not set `allowJs`, but the referenced `tsconfig.app.json` does. When
    // resolving from a `.js` file, the solution must defer to the referenced
    // project — whose own `allowJs` lets it claim the file — so its `paths`
    // alias applies. Previously the solution checked the *parent's* `allowJs`,
    // which dropped the `.js` file before any reference was consulted.
    let f = super::fixture_root().join("tsconfig/cases/project-references-ref-allow-js");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".ts".into()],
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });

    let resolved_path =
        resolver.resolve_file(f.join("src/index.js"), "@alias/foo.js").map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f.join("src/foo.js")));
}

#[test]
fn root_paths_apply_to_default_include_files() {
    let f = super::fixture_root().join("tsconfig/cases/project-references-default-include");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".ts".into()],
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });

    let resolved_path =
        resolver.resolve_file(f.join("index.ts"), "@app/util").map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f.join("src/util.ts")));
}

/// Regression test for <https://github.com/vitejs/vite/issues/22047>.
///
/// A solution-style root `tsconfig.json` (`"files": []` + `references`) whose
/// referenced project declares `paths` and explicitly includes non-TS
/// extensions (`src/**/*.vue`, `src/**/*.svelte`), as scaffolded by Vite. An
/// import originating from such a non-TS file must resolve through the
/// referenced project's `paths`, exactly like an import from a `.ts` file.
///
/// Previously the importer's extension was rejected before the `include` globs
/// were consulted, so a `.vue` / `.svelte` importer never matched the
/// referenced project and the empty solution root (which owns no `paths`) was
/// selected, leaving the `@/*` alias unresolved.
#[test]
fn solution_style_non_ts_extensions() {
    let f = super::fixture_root().join("tsconfig/cases/solution-style-non-ts");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".ts".into(), ".tsx".into(), ".vue".into(), ".svelte".into()],
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });

    let hello = f.join("src/components/HelloWorld.vue");

    #[rustfmt::skip]
    let pass = [
        // `.vue` importer (the original bug) resolves the alias.
        (f.join("src/App.vue"),       "@/components/HelloWorld.vue", hello.clone()),
        // Any explicitly-included extension works, not just `.vue`.
        (f.join("src/Widget.svelte"), "@/components/HelloWorld.vue", hello.clone()),
        // `.ts` importers keep working (no regression).
        (f.join("src/main.ts"),       "@/components/HelloWorld.vue", hello),
        (f.join("src/main.ts"),       "@/util.ts",                   f.join("src/util.ts")),
    ];

    for (path, request, expected) in pass {
        let resolved_path = resolver.resolve_file(&path, request).map(|f| f.full_path());
        assert_eq!(resolved_path, Ok(expected), "{request} from {path:?}");
    }

    // The referenced project owns the `.vue` / `.svelte` / `.ts` files through
    // its `include` globs, so auto-discovery selects it for those importers...
    for importer in ["src/App.vue", "src/Widget.svelte", "src/main.ts"] {
        let tsconfig = resolver.find_tsconfig(f.join(importer)).unwrap().unwrap();
        assert_eq!(tsconfig.path, f.join("tsconfig.app.json"), "find_tsconfig for {importer}");
    }

    // ...while a file whose extension is not matched by any `include` glob
    // (here `.css`) is left to the solution root, which owns no `paths`.
    let tsconfig = resolver.find_tsconfig(f.join("src/styles.css")).unwrap().unwrap();
    assert_eq!(tsconfig.path, f.join("tsconfig.json"));
}
