// See documentation at <https://docs.rs/oxc_resolver>

use std::path::PathBuf;

use oxc_resolver::{AliasValue, ResolveOptions, Resolver, TsconfigDiscovery};
use pico_args::Arguments;

fn main() {
    let mut args = Arguments::from_env();

    let path = args.free_from_str::<PathBuf>().expect("path");
    let specifier = args.free_from_str::<String>().expect("specifier");

    assert!(path.is_file(), "{} must be a file that will be resolved against.", path.display());
    assert!(path.is_absolute(), "{} must be an absolute path.", path.display());

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
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    };

    println!();

    match Resolver::new(options).resolve_file(path, &specifier) {
        Err(error) => println!("Error: {error}"),
        Ok(resolution) => {
            println!("Resolution: {}", resolution.full_path().to_string_lossy());
            println!("Module Type: {:?}", resolution.module_type());
            println!(
                "package.json: {:?}",
                resolution.package_json().map(|p| p.path.to_string_lossy())
            );
        }
    }
}
