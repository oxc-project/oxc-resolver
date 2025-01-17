//! Not part of enhanced_resolve's test suite
//!
//! enhanced_resolve's test <https://github.com/webpack/enhanced-resolve/blob/main/test/pnp.test.js>
//! cannot be ported over because it uses mocks on `pnpApi` provided by the runtime.

use crate::{ResolveOptions, Resolver};

#[test]
fn pnp1() {
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
            ".yarn/cache/preact-npm-10.25.4-2dd2c0aa44-33a009d614.zip/node_modules/preact/dist/preact.mjs"
        )),
    );

    assert_eq!(
        resolver.resolve(&fixture, "preact/devtools").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/preact-npm-10.25.4-2dd2c0aa44-33a009d614.zip/node_modules/preact/devtools/dist/devtools.mjs"
        )),
    );
}
