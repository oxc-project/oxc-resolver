use crate::{
    ResolveContext, ResolveError, ResolveOptions, Resolver, TsconfigDiscovery, TsconfigOptions,
    TsconfigReferences,
};

#[test]
fn resolve_tsconfig_with_context_records_self_and_extends() {
    let f = super::fixture_root().join("tsconfig/cases/extends");
    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let mut ctx = ResolveContext::default();
    resolver.resolve_tsconfig_with_context(f.join("tsconfig.json"), &mut ctx).expect("resolved");

    assert!(
        ctx.file_dependencies.contains(&f.join("tsconfig.json")),
        "expected root tsconfig in deps: {:?}",
        ctx.file_dependencies
    );
    assert!(
        ctx.file_dependencies.contains(&f.join("base-tsconfig.json")),
        "expected extended tsconfig in deps: {:?}",
        ctx.file_dependencies
    );
}

#[test]
fn resolve_tsconfig_with_context_records_extends_on_parse_error() {
    let f = super::fixture_root().join("tsconfig/cases/extends-with-broken-base");
    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let mut ctx = ResolveContext::default();
    let result = resolver.resolve_tsconfig_with_context(f.join("tsconfig.json"), &mut ctx);

    assert!(matches!(result, Err(ResolveError::Json(_))), "expected JSON error, got {result:?}");
    assert!(
        ctx.file_dependencies.contains(&f.join("tsconfig.json")),
        "expected root tsconfig in deps: {:?}",
        ctx.file_dependencies
    );
    assert!(
        ctx.file_dependencies.contains(&f.join("tsconfig.base.json")),
        "expected broken extended tsconfig in deps: {:?}",
        ctx.file_dependencies
    );
}

#[test]
fn resolve_tsconfig_with_context_records_missing_extends() {
    let f = super::fixture_root().join("tsconfig/cases/extends-not-found");
    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    let mut ctx = ResolveContext::default();
    let result = resolver.resolve_tsconfig_with_context(f.join("tsconfig.json"), &mut ctx);

    assert!(matches!(result, Err(ResolveError::TsconfigNotFound(_))));
    assert!(
        ctx.file_dependencies.contains(&f.join("tsconfig.json")),
        "expected root tsconfig in deps: {:?}",
        ctx.file_dependencies
    );
}

#[test]
fn find_tsconfig_with_context_records_deps_for_auto_discovery() {
    let f = super::fixture_root().join("tsconfig/cases/extends");
    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });

    let mut ctx = ResolveContext::default();
    resolver.find_tsconfig_with_context(f.join("src.ts"), &mut ctx).expect("resolved");

    assert!(
        ctx.file_dependencies.contains(&f.join("tsconfig.json")),
        "expected discovered tsconfig in deps: {:?}",
        ctx.file_dependencies
    );
    assert!(
        ctx.file_dependencies.contains(&f.join("base-tsconfig.json")),
        "expected extended tsconfig in deps: {:?}",
        ctx.file_dependencies
    );
}

#[test]
fn resolve_tsconfig_with_context_records_transitive_extends() {
    let f = super::fixture_root().join("tsconfig/cases/extends-chain");
    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    resolver.resolve_tsconfig(f.join("tsconfig.json")).expect("resolved");

    let mut ctx = ResolveContext::default();
    resolver.resolve_tsconfig_with_context(f.join("tsconfig.json"), &mut ctx).expect("resolved");

    for name in ["tsconfig.json", "intermediate-tsconfig.json", "base-tsconfig.json"] {
        assert!(
            ctx.file_dependencies.contains(&f.join(name)),
            "expected {name} in deps after warm-cache call: {:?}",
            ctx.file_dependencies
        );
    }
}

#[test]
fn resolve_tsconfig_with_context_replays_deps_on_cache_hit() {
    let f = super::fixture_root().join("tsconfig/cases/extends");
    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file: f.join("tsconfig.json"),
            references: TsconfigReferences::Disabled,
        })),
        ..ResolveOptions::default()
    });

    resolver.resolve_tsconfig(f.join("tsconfig.json")).expect("resolved");

    let mut ctx = ResolveContext::default();
    resolver.resolve_tsconfig_with_context(f.join("tsconfig.json"), &mut ctx).expect("resolved");

    assert!(
        ctx.file_dependencies.contains(&f.join("tsconfig.json")),
        "expected root tsconfig in deps after cache hit: {:?}",
        ctx.file_dependencies
    );
    assert!(
        ctx.file_dependencies.contains(&f.join("base-tsconfig.json")),
        "expected extended tsconfig in deps after cache hit: {:?}",
        ctx.file_dependencies
    );
}
