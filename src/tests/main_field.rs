//! Not part of enhanced_resolve's test suite

use crate::{ResolveOptions, Resolver};

#[test]
fn test() {
    let f = super::fixture().join("restrictions");

    let resolver1 = Resolver::new(ResolveOptions {
        main_fields: vec!["style".into()],
        ..ResolveOptions::default()
    });

    let resolution = resolver1.resolve(&f, "pck2").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck2/index.css")));

    let resolver2 = resolver1.clone_with_options(ResolveOptions {
        main_fields: vec!["module".into(), "main".into()],
        ..ResolveOptions::default()
    });

    let resolution = resolver2.resolve(&f, "pck2").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck2/module.js")));
}
