use std::{env, fs};

use rayon::prelude::*;

use oxc_resolver::{ResolveOptions, Resolver};

fn main() {
    let cwd = env::current_dir().expect("Failed to get current directory");
    let node_modules = cwd.join("node_modules");

    if !node_modules.exists() {
        eprintln!("node_modules directory not found at {}", node_modules.display());
        return;
    }

    // Collect all package names
    let mut packages = Vec::new();

    let entries = fs::read_dir(&node_modules).expect("Failed to read node_modules directory");

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = path.file_name().unwrap().to_string_lossy();

        // Skip dot directories
        if dir_name.starts_with('.') {
            continue;
        }

        if dir_name.starts_with('@') {
            // Skip @types packages
            if dir_name == "@types" {
                continue;
            }
            // Scoped package - read subdirectories
            if let Ok(scope_entries) = fs::read_dir(&path) {
                for scope_entry in scope_entries.filter_map(Result::ok) {
                    let scope_path = scope_entry.path();
                    if scope_path.is_dir() {
                        let package_name = scope_path.file_name().unwrap().to_string_lossy();
                        packages.push(format!("{dir_name}/{package_name}"));
                    }
                }
            }
        } else {
            // Regular package
            packages.push(dir_name.to_string());
        }
    }

    let options = ResolveOptions {
        condition_names: vec!["node".into(), "import".into()],
        ..ResolveOptions::default()
    };
    let resolver = Resolver::new(options);

    packages.par_iter().for_each(|package| {
        if let Err(err) = resolver.resolve(&cwd, package) {
            eprintln!("{package}: {err}");
        }
    });
}
