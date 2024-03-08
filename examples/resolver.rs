use std::{env, path::PathBuf};

use oxc_resolver::{AliasValue, ResolveOptions, Resolver};

fn main() {
    // Path to directory, must be in absolute path.
    let path = ".";
    let specifier = "vue";
    let path = PathBuf::from(path).canonicalize().unwrap();

    println!("path: {path:?}");
    println!("request: {specifier}");

    let options = ResolveOptions {
        alias_fields: vec![vec!["exports".into()]],
        ..ResolveOptions::default()
    };

    match Resolver::new(options).resolve(path, &specifier) {
        Err(error) => println!("Error: {error}"),
        Ok(resolution) => println!("Resolved: {:?}", resolution.full_path()),
    }
}
