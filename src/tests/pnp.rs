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
        pnp_manifest: Some(pnp::load_pnp_manifest(fixture.join(".pnp.cjs")).unwrap()),
        ..ResolveOptions::default()
    });

    assert_eq!(
        resolver.resolve(&fixture, "is-even").map(|r| r.full_path()),
        Ok(fixture.join(
            ".yarn/cache/is-even-npm-1.0.0-9f726520dc-2728cc2f39.zip/node_modules/is-even/index.js"
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
}
