use crate::{ResolveError, ResolveOptions, Resolver};

#[test]
fn listing_threshold_crossed_in_one_directory() {
    let f = super::fixture();

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".js".into()],
        ..ResolveOptions::default()
    });

    let pass = [
        ("./a", f.join("a.js")),
        ("./b", f.join("b.js")),
        ("./c", f.join("c.js")),
        ("./main1", f.join("main1.js")),
        ("./main2", f.join("main2.js")),
        ("./main3", f.join("main3.js")),
        ("./lib", f.join("lib.js")),
        ("./complex", f.join("complex.js")),
        ("./no", f.join("no.js")),
        ("./dirOrFile", f.join("dirOrFile.js")),
        ("./a.js", f.join("a.js")),
        ("./b.js", f.join("b.js")),
        ("./c.js", f.join("c.js")),
        ("./main1.js", f.join("main1.js")),
        ("./main2.js", f.join("main2.js")),
        ("./lib/complex1", f.join("lib/complex1.js")),
        ("./browser-module/lib/browser", f.join("browser-module/lib/browser.js")),
        ("./main-field-self/index", f.join("main-field-self/index.js")),
    ];

    for _ in 0..2 {
        for (request, expected) in &pass {
            let resolved_path = resolver.resolve(&f, request).map(|r| r.full_path());
            assert_eq!(resolved_path, Ok(expected.clone()), "{request}");
        }
        for request in ["./this-file-does-not-exist", "./a.mjs", "./A_MISSING_FILE.js"] {
            let resolved_path = resolver.resolve(&f, request).map(|r| r.full_path());
            assert_eq!(
                resolved_path,
                Err(ResolveError::NotFound(request.to_string())),
                "{request}"
            );
        }
    }
}
