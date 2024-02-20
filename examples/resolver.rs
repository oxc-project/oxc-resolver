use std::{env, path::PathBuf};

use oxc_resolver::{AliasValue, ResolveOptions, Resolver};

fn main() {
    // Path to directory, must be in absolute path.
    let path = env::args().nth(1).expect("path");
    let specifier = env::args().nth(2).expect("specifier");
    let path = PathBuf::from(path).canonicalize().unwrap();

    println!("path: {path:?}");
    println!("request: {specifier}");

    let options = ResolveOptions {
        alias_fields: vec![vec!["browser".into()]],
        alias: vec![("asdf".into(), vec![AliasValue::Path("./test.js".into())])],
        ..ResolveOptions::default()
    };

    match Resolver::new(options).resolve(path, &specifier) {
        Err(error) => println!("Error: {error}"),
        Ok(resolution) => println!("Resolved: {:?}", resolution.full_path()),
    }
}
