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
            ".yarn/cache/preact-npm-10.26.5-d46ec4e2ac-542a924009.zip/node_modules/preact/dist/preact.mjs"
        )),
    );

    assert_eq!(
        resolver.resolve(&fixture, "preact/devtools").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/preact-npm-10.26.5-d46ec4e2ac-542a924009.zip/node_modules/preact/devtools/dist/devtools.mjs"
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

    let resolver = Resolver::new(ResolveOptions { enable_pnp: false, ..ResolveOptions::default() });

    assert_eq!(
        resolver.resolve(&fixture, "is-even").map(|r| r.full_path()),
        Err(NotFound("is-even".to_string()))
    );
}

#[test]
fn resolve_package_deep_link() {
    let fixture = super::fixture_root().join("pnp");

    let resolver = Resolver::new(ResolveOptions::default());

    assert_eq!(
        resolver.resolve(fixture.join("shared"), "beachball/lib/commands/bump.js").map(|r| r.full_path()),
        Ok(fixture.join(
          ".yarn/cache/beachball-npm-2.52.0-ee48e46454-96b8c49193.zip/node_modules/beachball/lib/commands/bump.js"
      )),
    );
}

#[test]
fn resolve_pnp_nested_package_json() {
    let fixture = super::fixture_root().join("pnp");

    let resolver = Resolver::new({
        ResolveOptions {
            main_fields: vec!["module".into(), "main".into()],
            ..ResolveOptions::default()
        }
    });

    assert_eq!(
        resolver.resolve(&fixture, "@atlaskit/pragmatic-drag-and-drop/combine").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/@atlaskit-pragmatic-drag-and-drop-npm-1.5.2-3241d4f843-1dace49fa3.zip/node_modules/@atlaskit/pragmatic-drag-and-drop/dist/esm/entry-point/combine.js"
        ))
    );
}

#[test]
fn resolve_npm_protocol_alias() {
    let fixture = super::fixture_root().join("pnp");

    let resolver = Resolver::default();

    assert_eq!(
        resolver.resolve(&fixture, "custom-minimist").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/minimist-npm-1.2.8-d7af7b1dce-19d3fcdca0.zip/node_modules/minimist/index.js"
        ))
    );

    assert_eq!(
        resolver.resolve(&fixture, "@custom/pragmatic-drag-and-drop").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/@atlaskit-pragmatic-drag-and-drop-npm-1.5.2-3241d4f843-1dace49fa3.zip/node_modules/@atlaskit/pragmatic-drag-and-drop/dist/cjs/index.js"
        ))
    );

    assert_eq!(
        resolver.resolve(&fixture, "pragmatic-drag-and-drop").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/@atlaskit-pragmatic-drag-and-drop-npm-1.5.2-3241d4f843-1dace49fa3.zip/node_modules/@atlaskit/pragmatic-drag-and-drop/dist/cjs/index.js"
        ))
    );
}

// Windows is blocked by upstream
// see also https://github.com/yarnpkg/pnp-rs/pull/10
#[cfg(not(windows))]
#[test]
fn resolve_global_cache() {
    let home_dir = dirs::home_dir().unwrap();

    #[cfg(windows)]
    let global_cache = home_dir.join("AppData\\Local\\Yarn\\Berry");
    #[cfg(not(windows))]
    let global_cache = home_dir.join(".yarn/berry/cache");

    let resolver = Resolver::new(ResolveOptions {
        roots: vec![super::fixture_root().join("global-pnp")],
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
