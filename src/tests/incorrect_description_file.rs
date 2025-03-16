//! <https://github.com/webpack/enhanced-resolve/blob/main/test/incorrect-description-file.test.js>

use rustc_hash::FxHashSet;

use crate::{JSONError, Resolution, ResolveContext, ResolveError, ResolveOptions, Resolver};

// should not resolve main in incorrect description file #1
#[test]
fn incorrect_description_file_1() {
    let f = super::fixture().join("incorrect-package");
    let mut ctx = ResolveContext::default();
    let resolution = Resolver::default().resolve_with_context(f.join("pack1"), ".", &mut ctx);
    let _error = ResolveError::JSON(JSONError {
        path: f.join("pack1/package.json"),
        message: String::from("EOF while parsing a value at line 3 column 0"),
        line: 3,
        column: 0,
        content: None,
    });

    assert!(matches!(resolution, Err(ResolveError::JSON(_))));
    assert_eq!(
        ctx.file_dependencies,
        FxHashSet::from_iter([f.join("pack1"), f.join("pack1/package.json")])
    );
    assert!(!ctx.missing_dependencies.is_empty());
}

// should not resolve main in incorrect description file #2
#[test]
fn incorrect_description_file_2() {
    let f = super::fixture().join("incorrect-package");
    let resolution = Resolver::default().resolve(f.join("pack2"), ".");
    let _error = ResolveError::JSON(JSONError {
        path: f.join("pack2/package.json"),
        message: String::from("EOF while parsing a value at line 1 column 0"),
        line: 1,
        column: 0,
        content: Some(String::new()),
    });
    assert!(matches!(resolution, Err(ResolveError::JSON(_))));
}

// should not resolve main in incorrect description file #3
#[test]
fn incorrect_description_file_3() {
    let f = super::fixture().join("incorrect-package");
    let resolution = Resolver::default().resolve(f.join("pack2"), ".");
    assert!(resolution.is_err());
}

// `enhanced_resolve` does not have this test case
#[test]
fn no_description_file() {
    let f = super::fixture_root().join("enhanced_resolve");

    // has description file
    let resolver = Resolver::default();
    assert_eq!(
        resolver.resolve(&f, ".").map(Resolution::into_path_buf),
        Ok(f.join("lib/index.js"))
    );

    // without description file
    let resolver =
        Resolver::new(ResolveOptions { description_files: vec![], ..ResolveOptions::default() });
    assert_eq!(resolver.resolve(&f, "."), Err(ResolveError::NotFound(".".into())));
}
