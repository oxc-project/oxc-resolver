// See documentation at <https://docs.rs/oxc_resolver>

use std::{env, path::PathBuf};

use oxc_resolver::{AliasValue, ResolveOptions, Resolver};

fn main() {
    let path = PathBuf::from(env::args().nth(1).expect("path"));

    assert!(path.is_dir(), "{path:?} must be a directory that will be resolved against.");
    assert!(path.is_absolute(), "{path:?} must be an absolute path.",);

    let specifier = env::args().nth(2).expect("specifier");

    println!("path: {}", path.to_string_lossy());
    println!("specifier: {specifier}");

    let options = ResolveOptions {
        alias_fields: vec![vec!["browser".into()]],
        alias: vec![("asdf".into(), vec![AliasValue::from("./test.js")])],
        extensions: vec![".js".into(), ".ts".into()],
        extension_alias: vec![(".js".into(), vec![".ts".into(), ".js".into()])],
        // ESM
        condition_names: vec!["node".into(), "import".into()],
        // CJS
        // condition_names: vec!["node".into(), "require".into()],
        ..ResolveOptions::default()
    };

    let resolver = Resolver::new(options);

    println!();

    match resolver.resolve(&path, &specifier) {
        Err(error) => println!("Error: {error}"),
        Ok(resolution) => {
            println!("Resolution: {}", resolution.full_path().to_string_lossy());
            println!(
                "package.json: {:?}",
                resolution.package_json().map(|p| p.path.to_string_lossy())
            );
        }
    }

    println!();

    match resolver.resolve_package_dts(&path, &specifier) {
        Err(error) => println!("Error: {error}"),
        Ok(resolution) => {
            println!("DTS Resolution: {}", resolution.full_path().to_string_lossy());
        }
    }
}
