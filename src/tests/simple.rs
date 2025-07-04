//! <https://github.com/webpack/enhanced-resolve/blob/main/test/simple.test.js>

use std::env;

use crate::{Resolution, Resolver};

#[test]
fn simple() {
    // mimic `enhanced-resolve/test/simple.test.js`
    let dirname = env::current_dir().unwrap().join("fixtures");
    let f = dirname.join("enhanced_resolve/test");

    let resolver = Resolver::default();

    let data = [
        ("direct", f.clone(), "../lib/index"),
        ("as directory", f, ".."),
        ("as module", dirname.clone(), "./enhanced_resolve"),
    ];

    for (comment, path, request) in data {
        let resolved_path = resolver.resolve(&path, request).map(|f| f.full_path());
        let expected = dirname.join("enhanced_resolve/lib/index.js");
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }
}

#[test]
fn dashed_name() {
    let f = super::fixture();

    let resolver = Resolver::default();

    let data = [
        (f.clone(), "dash", f.join("node_modules/dash/index.js")),
        (f.clone(), "dash-name", f.join("node_modules/dash-name/index.js")),
        (f.join("node_modules/dash"), "dash", f.join("node_modules/dash/index.js")),
        (f.join("node_modules/dash"), "dash-name", f.join("node_modules/dash-name/index.js")),
        (f.join("node_modules/dash-name"), "dash", f.join("node_modules/dash/index.js")),
        (f.join("node_modules/dash-name"), "dash-name", f.join("node_modules/dash-name/index.js")),
    ];

    for (path, request, expected) in data {
        let resolution = resolver.resolve(&path, request).ok();
        let resolved_path = resolution.as_ref().map(Resolution::full_path);
        let resolved_package_json =
            resolution.as_ref().and_then(|r| r.package_json()).map(|p| p.path.clone());
        assert_eq!(resolved_path, Some(expected), "{path:?} {request}");
        let package_json_path = f.join("node_modules").join(request).join("package.json");
        assert_eq!(resolved_package_json, Some(package_json_path), "{path:?} {request}");
    }
}

#[cfg(not(target_os = "windows"))] // MemoryFS's path separator is always `/` so the test will not pass in windows.
mod windows {
    use super::super::memory_fs::MemoryFS;
    use crate::ResolveOptions;

    #[test]
    fn no_package() {
        use std::path::Path;

        use crate::ResolverGeneric;
        let f = Path::new("/");
        let file_system = MemoryFS::new(&[]);
        let resolver =
            ResolverGeneric::new_with_file_system(file_system, ResolveOptions::default());
        let resolved_path = resolver.resolve(f, "package");
        assert!(resolved_path.is_err());
    }
}
