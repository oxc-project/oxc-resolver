//! Not part of enhanced_resolve's test suite
//!
//! enhanced_resolve's test <https://github.com/webpack/enhanced-resolve/blob/main/test/pnp.test.js>
//! cannot be ported over because it uses mocks on `pnpApi` provided by the runtime.

use crate::ResolveError::NotFound;
use crate::{ResolveOptions, Resolver};

#[test]
fn pnp_basic() {
    let fixture = super::fixture_root().join("pnp");

    let resolver = Resolver::new(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        extensions: vec![".js".into()],
        condition_names: vec!["import".into()],
        ..ResolveOptions::default()
    });

    assert_eq!(
        resolver.resolve(&fixture, "is-even").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/is-even-npm-1.0.0-9f726520dc-2728cc2f39.zip/node_modules/is-even/index.js"
        ))
    );

    assert_eq!(
        resolver.resolve(&fixture, "lodash.zip").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/lodash.zip-npm-4.2.0-5299417ec8-e596da80a6.zip/node_modules/lodash.zip/index.js"
        ))
    );

    assert_eq!(
        resolver
            .resolve(
                fixture.join(
                    ".yarn/cache/is-even-npm-1.0.0-9f726520dc-2728cc2f39.zip/node_modules/is-even"
                ),
                "is-odd"
            )
            .map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/is-odd-npm-0.1.2-9d980a9da8-7dc6c6fd00.zip/node_modules/is-odd/index.js"
        )),
    );

    assert_eq!(
        resolver.resolve(&fixture, "is-odd").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/is-odd-npm-3.0.1-93c3c3f41b-89ee2e353c.zip/node_modules/is-odd/index.js"
        )),
    );

    assert_eq!(
        resolver.resolve(&fixture, "preact").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/preact-npm-10.26.9-90e1df1a58-15f187e327.zip/node_modules/preact/dist/preact.mjs"
        )),
    );

    assert_eq!(
        resolver.resolve(&fixture, "preact/devtools").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/preact-npm-10.26.9-90e1df1a58-15f187e327.zip/node_modules/preact/devtools/dist/devtools.mjs"
        )),
    );

    assert_eq!(
        resolver.resolve(&fixture, "pnpapi").map(|r| r.full_path()),
        Ok(fixture.join(".pnp.cjs")),
    );
}

#[test]
fn resolve_in_pnp_linked_folder() {
    let fixture = super::fixture_root().join("pnp");

    let resolver = Resolver::new(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        extensions: vec![".js".into()],
        condition_names: vec!["import".into()],
        ..ResolveOptions::default()
    });

    assert_eq!(
        resolver.resolve(&fixture, "lib/lib.js").map(|r| r.full_path()),
        Ok(fixture.join("shared/lib.js"))
    );
}

#[test]
fn resolve_pnp_pkg_should_failed_while_disable_pnp_mode() {
    let fixture = super::fixture_root().join("pnp");

    let resolver = Resolver::default();

    assert_eq!(
        resolver.resolve(&fixture, "is-even").map(|r| r.full_path()),
        Err(NotFound("is-even".to_string()))
    );
}

#[test]
fn resolve_package_deep_link() {
    let fixture = super::fixture_root().join("pnp");

    let resolver = Resolver::new(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        ..ResolveOptions::default()
    });

    assert_eq!(
        resolver.resolve(fixture.join("shared"), "beachball/lib/commands/bump.js").map(|r| r.full_path()),
        Ok(fixture.join(
          ".yarn/cache/beachball-npm-2.54.0-050eafd5c8-4dd08576af.zip/node_modules/beachball/lib/commands/bump.js"
      )),
    );
}

#[test]
fn resolve_pnp_nested_package_json() {
    let fixture = super::fixture_root().join("pnp");

    let resolver = Resolver::new(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        main_fields: vec!["module".into(), "main".into()],
        ..ResolveOptions::default()
    });

    assert_eq!(
        resolver.resolve(&fixture, "@atlaskit/pragmatic-drag-and-drop/combine").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/@atlaskit-pragmatic-drag-and-drop-npm-1.7.3-a09c04b71c-d7c6e1a2e4.zip/node_modules/@atlaskit/pragmatic-drag-and-drop/dist/esm/entry-point/combine.js"
        ))
    );
}

#[test]
fn resolve_npm_protocol_alias() {
    let fixture = super::fixture_root().join("pnp");

    let resolver = Resolver::new(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        ..ResolveOptions::default()
    });

    assert_eq!(
        resolver.resolve(&fixture, "custom-minimist").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/minimist-npm-1.2.8-d7af7b1dce-19d3fcdca0.zip/node_modules/minimist/index.js"
        ))
    );

    assert_eq!(
        resolver.resolve(&fixture, "@custom/pragmatic-drag-and-drop").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/@atlaskit-pragmatic-drag-and-drop-npm-1.7.3-a09c04b71c-d7c6e1a2e4.zip/node_modules/@atlaskit/pragmatic-drag-and-drop/dist/cjs/index.js"
        ))
    );

    assert_eq!(
        resolver.resolve(&fixture, "pragmatic-drag-and-drop").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/@atlaskit-pragmatic-drag-and-drop-npm-1.7.3-a09c04b71c-d7c6e1a2e4.zip/node_modules/@atlaskit/pragmatic-drag-and-drop/dist/cjs/index.js"
        ))
    );
}

#[test]
#[cfg(target_endian = "little")]
fn resolve_global_cache() {
    let home_dir = dirs::home_dir().unwrap();

    #[cfg(windows)]
    let global_cache = home_dir.join("AppData\\Local\\Yarn\\Berry\\cache");
    #[cfg(not(windows))]
    let global_cache = home_dir.join(".yarn/berry/cache");

    let fixture = super::fixture_root().join("global-pnp");
    let resolver = Resolver::new(ResolveOptions {
        cwd: Some(fixture),
        yarn_pnp: true,
        ..ResolveOptions::default()
    });

    assert_eq!(
        resolver
            .resolve(
                global_cache
                    .join("source-map-support-npm-0.5.21-09ca99e250-10c0.zip")
                    .join("node_modules")
                    .join("source-map-support")
                    .join(""),
                "source-map"
            )
            .map(|r| r.full_path()),
        Ok(global_cache
            .join("source-map-npm-0.6.1-1a3621db16-10c0.zip")
            .join("node_modules")
            .join("source-map")
            .join("source-map.js")),
    );
}

#[test]
fn test_resolve_tsconfig_extends_with_pnp() {
    let fixture = super::fixture_root().join("pnp");
    let resolver = Resolver::new(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&fixture).expect("resolved");
    let compiler_options = &resolution.compiler_options;
    assert_eq!(compiler_options.target, Some("esnext".to_string()));
}

#[test]
fn test_non_pnp_enabled_base() {
    let fixture = super::fixture_root().join("pnp");

    let base_resolver = Resolver::new(ResolveOptions::default());

    let resolver = base_resolver.clone_with_options(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        extensions: vec![".js".into()],
        condition_names: vec!["import".into()],
        ..ResolveOptions::default()
    });

    assert_eq!(
        resolver.resolve(&fixture, "is-even").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/is-even-npm-1.0.0-9f726520dc-2728cc2f39.zip/node_modules/is-even/index.js"
        ))
    );
}

#[test]
fn test_cache_preserved_when_not_toggling_yarn_pnp() {
    let fixture = super::fixture_root().join("pnp");

    // Start with a PnP-enabled resolver
    let base_resolver = Resolver::new(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        extensions: vec![".js".into()],
        condition_names: vec!["import".into()],
        ..ResolveOptions::default()
    });

    // Clone with different options but keeping yarn_pnp: true
    // With the bug from 9ae9056, this would create a new cache unnecessarily
    let cloned_resolver = base_resolver.clone_with_options(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        extensions: vec![".js".into(), ".json".into()], // Different extensions
        condition_names: vec!["import".into()],
        ..ResolveOptions::default()
    });

    // The cache should be shared (same Arc pointer)
    // This assertion would FAIL with the buggy code from 9ae9056
    assert!(
        base_resolver.shares_cache_with(&cloned_resolver),
        "Cache should be preserved when yarn_pnp is not toggled"
    );

    // Verify both resolvers still work correctly
    assert_eq!(
        cloned_resolver.resolve(&fixture, "is-even").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/is-even-npm-1.0.0-9f726520dc-2728cc2f39.zip/node_modules/is-even/index.js"
        ))
    );
}

#[test]
fn test_cache_recreated_when_toggling_yarn_pnp_on() {
    let fixture = super::fixture_root().join("pnp");

    // Start with a non-PnP resolver
    let base_resolver = Resolver::new(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: false,
        extensions: vec![".js".into()],
        ..ResolveOptions::default()
    });

    // Clone with yarn_pnp: true (toggle on)
    let cloned_resolver = base_resolver.clone_with_options(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        extensions: vec![".js".into()],
        condition_names: vec!["import".into()],
        ..ResolveOptions::default()
    });

    // The cache should NOT be shared (different Arc)
    assert!(
        !base_resolver.shares_cache_with(&cloned_resolver),
        "Cache should be recreated when toggling yarn_pnp on"
    );

    // And the new resolver should work with PnP
    assert_eq!(
        cloned_resolver.resolve(&fixture, "is-even").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/is-even-npm-1.0.0-9f726520dc-2728cc2f39.zip/node_modules/is-even/index.js"
        ))
    );
}

#[test]
fn test_cache_recreated_when_toggling_yarn_pnp_off() {
    let fixture = super::fixture_root().join("pnp");

    // Start with a PnP-enabled resolver
    let base_resolver = Resolver::new(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        extensions: vec![".js".into()],
        condition_names: vec!["import".into()],
        ..ResolveOptions::default()
    });

    // Clone with yarn_pnp: false (toggle off)
    let cloned_resolver = base_resolver.clone_with_options(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: false,
        extensions: vec![".js".into()],
        ..ResolveOptions::default()
    });

    // The cache should NOT be shared (different Arc)
    assert!(
        !base_resolver.shares_cache_with(&cloned_resolver),
        "Cache should be recreated when toggling yarn_pnp off"
    );

    // Verify the cloned resolver works without PnP
    assert_eq!(
        cloned_resolver.resolve(&fixture, "is-even").map(|r| r.full_path()),
        Err(crate::ResolveError::NotFound("is-even".to_string()))
    );
}
