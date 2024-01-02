//! https://github.com/webpack/enhanced-resolve/blob/main/test/missing.test.js

use crate::{ResolveContext, Resolver};

#[test]
fn test() {
    let f = super::fixture();

    let data = [
        (
            "./missing-file",
            vec![f.join("missing-file"), f.join("missing-file.js"), f.join("missing-file.node")],
        ),
        (
            "missing-module",
            vec![
                f.join("node_modules/missing-module"),
                f.parent().unwrap().join("node_modules"), // enhanced-resolve is "node_modules/missing-module"
            ],
        ),
        (
            "missing-module/missing-file",
            vec![
                f.join("node_modules/missing-module"),
                // f.parent().unwrap().join("node_modules/missing-module"), // we don't report this
            ],
        ),
        (
            "m1/missing-file",
            vec![
                f.join("node_modules/m1/missing-file"),
                f.join("node_modules/m1/missing-file.js"),
                f.join("node_modules/m1/missing-file.node"),
                // f.parent().unwrap().join("node_modules/m1"), // we don't report this
            ],
        ),
        (
            "m1/",
            vec![
                f.join("node_modules/m1/index"),
                f.join("node_modules/m1/index.js"),
                f.join("node_modules/m1/index.json"),
                f.join("node_modules/m1/index.node"),
            ],
        ),
        ("m1/a", vec![f.join("node_modules/m1/a")]),
    ];

    let resolver = Resolver::default();

    for (specifier, missing_dependencies) in data {
        let mut ctx = ResolveContext::default();
        let _ = resolver.resolve_with_context(&f, specifier, &mut ctx);

        for dep in missing_dependencies {
            assert!(
                ctx.missing_dependencies.contains(&dep),
                "{specifier}: {dep:?} not in {:?}",
                &ctx.missing_dependencies
            );
        }
    }
}
