//! <https://github.com/webpack/enhanced-resolve/blob/main/test/incorrect-description-file.test.js>

use rustc_hash::FxHashSet;

use crate::{JSONError, ResolveContext, ResolveError, Resolver};

// should not resolve main in incorrect description file #1
#[test]
fn incorrect_description_file_1() {
    let f = super::fixture().join("incorrect-package");
    let mut ctx = ResolveContext::default();
    let resolution = Resolver::default().resolve_with_context(f.join("pack1"), ".", &mut ctx);
    let error = ResolveError::JSON(JSONError {
        path: f.join("pack1/package.json"),
        message: String::from("EOF while parsing a value at line 3 column 0"),
        line: 3,
        column: 0,
    });
    assert_eq!(resolution, Err(error));
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
    let error = ResolveError::JSON(JSONError {
        path: f.join("pack2/package.json"),
        message: String::from("EOF while parsing a value at line 1 column 0"),
        line: 1,
        column: 0,
    });
    assert_eq!(resolution, Err(error));
}

// should not resolve main in incorrect description file #3
#[test]
fn incorrect_description_file_3() {
    let f = super::fixture().join("incorrect-package");
    let resolution = Resolver::default().resolve(f.join("pack2"), ".");
    assert!(resolution.is_err());
}
