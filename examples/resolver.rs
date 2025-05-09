// See documentation at <https://docs.rs/oxc_resolver>

use std::path::PathBuf;

use oxc_resolver::{AliasValue, ResolveOptions, Resolver, TsconfigOptions, TsconfigReferences};
use pico_args::Arguments;

fn main() {
    let mut args = Arguments::from_env();

    let tsconfig_path = args.value_from_str::<&'static str, PathBuf>("--tsconfig").ok();
    let path = args.free_from_str::<PathBuf>().expect("path");
    let specifier = args.free_from_str::<String>().expect("specifier");

    assert!(path.is_dir(), "{path:?} must be a directory that will be resolved against.");
    assert!(path.is_absolute(), "{path:?} must be an absolute path.",);

    println!("path: {}", path.to_string_lossy());
    println!("specifier: {specifier}");
    if let Some(path) = &tsconfig_path {
        println!("tsconfig: {}", path.to_string_lossy());
    }

    let options = ResolveOptions {
        alias_fields: vec![vec!["browser".into()]],
        alias: vec![("asdf".into(), vec![AliasValue::from("./test.js")])],
        extensions: vec![".js".into(), ".ts".into()],
        extension_alias: vec![(".js".into(), vec![".ts".into(), ".js".into()])],
        // ESM
        condition_names: vec!["node".into(), "import".into()],
        // CJS
        // condition_names: vec!["node".into(), "require".into()],
        tsconfig: tsconfig_path.map(|config_file| TsconfigOptions {
            config_file,
            references: TsconfigReferences::Auto,
        }),
        ..ResolveOptions::default()
    };

    println!();

    match Resolver::new(options).resolve(path, &specifier) {
        Err(error) => println!("Error: {error}"),
        Ok(resolution) => {
            println!("Resolution: {}", resolution.full_path().to_string_lossy());
            println!(
                "package json: {:?}",
                resolution.package_json().map(|p| p.path.to_string_lossy())
            );
        }
    }
}
