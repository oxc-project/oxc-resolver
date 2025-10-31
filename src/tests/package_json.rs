//! Tests for `Resolution::package_json`.

use crate::{ResolveError, Resolver};

#[test]
fn test() {
    let f = super::fixture_root().join("misc");

    let resolver = Resolver::default();

    let data = [
        (f.clone(), "package-json-nested"),
        // Nested package.json do not participate in module resolution.
        (f.clone(), "package-json-nested/foo"),
        (f.clone(), "package-json-nested/foo/bar"),
    ];

    let resolved_package_json_path = f.join("node_modules/package-json-nested/package.json");

    for (path, request) in data {
        let package_json =
            resolver.resolve(&path, request).ok().and_then(|f| f.package_json().cloned());
        let package_json_path = package_json.as_ref().map(|p| &p.path);
        let package_json_name = package_json.as_ref().and_then(|p| p.name());
        assert_eq!(package_json_path, Some(&resolved_package_json_path), "{path:?} {request}");
        assert_eq!(package_json_name, Some("package-json-nested"), "{path:?} {request}");
    }
}

#[test]
fn adjacent_to_node_modules() {
    let f = super::fixture_root().join("misc");

    let resolver = Resolver::default();

    // populate cache
    let _ = resolver.resolve(&f, "package-json-nested");

    let path = f.join("dir-with-index");
    let request = "./index.js";
    let resolved_package_json_path = f.join("package.json");

    let package_json = resolver.resolve(&path, request).unwrap().package_json().cloned();
    let package_json_path = package_json.as_ref().map(|p| &p.path);
    let package_json_name = package_json.as_ref().and_then(|p| p.name());
    assert_eq!(package_json_path, Some(&resolved_package_json_path));
    assert_eq!(package_json_name, Some("misc"));
}

#[test]
fn package_json_with_symlinks_true() {
    use crate::ResolveOptions;

    let f = super::fixture_root().join("misc");
    let resolver = Resolver::new(ResolveOptions { symlinks: true, ..ResolveOptions::default() });

    let path = f.join("dir-with-index");
    let request = "./index.js";
    let resolved_package_json_path = f.join("package.json");

    let package_json = resolver.resolve(&path, request).unwrap().package_json().cloned();
    let package_json_path = package_json.as_ref().map(|p| &p.path);
    assert_eq!(package_json_path, Some(&resolved_package_json_path));
}

#[test]
#[cfg(not(target_os = "windows"))] // MemoryFS's path separator is always `/` so the test will not pass in windows.
fn test_corrupted_package_json() {
    use std::path::Path;

    use super::memory_fs::MemoryFS;
    use crate::ResolverGeneric;

    // Test scenarios for various corrupted package.json files
    let scenarios = [
        (
            "empty_file",
            "",
            "EOF while parsing",
        ),
        (
            "null_byte_at_start",
            "\0",
            "expected value",
        ),
        (
            "json_with_embedded_null",
            "{\"name\":\0\"test\"}",
            "expected value",
        ),
        (
            "trailing_comma",
            "{\"name\":\"test\",}",
            "trailing comma",
        ),
        (
            "unclosed_brace",
            "{\"name\":\"test\"",
            "EOF while parsing",
        ),
        (
            "invalid_escape",
            "{\"name\":\"test\\x\"}",
            "escape",
        ),
    ];

    for (name, content, expected_message_contains) in scenarios {
        let mut fs = MemoryFS::default();

        // Write corrupted package.json
        fs.add_file(Path::new("/test/package.json"), content);

        // Create a simple index.js so resolution can proceed
        fs.add_file(Path::new("/test/index.js"), "export default 42;");

        // Create resolver with VFS
        let resolver = ResolverGeneric::new_with_file_system(fs, Default::default());

        // Attempt to resolve - should fail with JSONError
        let result = resolver.resolve(Path::new("/test"), "./index.js");

        match result {
            Err(ResolveError::Json(json_error)) => {
                assert!(
                    json_error.message.to_lowercase().contains(&expected_message_contains.to_lowercase()),
                    "Test case '{}': Expected error message to contain '{}', but got: {}",
                    name,
                    expected_message_contains,
                    json_error.message
                );
                assert!(
                    json_error.path.ends_with("package.json"),
                    "Test case '{}': Expected path to end with 'package.json', but got: {:?}",
                    name,
                    json_error.path
                );
            }
            Err(other_error) => {
                panic!(
                    "Test case '{}': Expected JSONError but got: {:?}",
                    name, other_error
                );
            }
            Ok(resolution) => {
                panic!(
                    "Test case '{}': Expected error but resolution succeeded: {:?}",
                    name, resolution
                );
            }
        }
    }
}
