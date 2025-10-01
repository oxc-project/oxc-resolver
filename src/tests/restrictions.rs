//! <https://github.com/webpack/enhanced-resolve/blob/main/test/restrictions.test.js>

use std::sync::Arc;

use fancy_regex::Regex;

use crate::{ResolveError, ResolveOptions, Resolver, Restriction};

#[test]
fn should_respect_regexp_restriction() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(sass|scss|css)$").unwrap();
    let resolver1 = Resolver::new(ResolveOptions {
        extensions: vec![".js".into()],
        restrictions: vec![Restriction::Fn(Arc::new(move |path| {
            path.as_os_str().to_str().is_some_and(|s| re.is_match(s).unwrap_or(false))
        }))],
        ..ResolveOptions::default()
    });

    let resolution = resolver1.resolve(&f, "pck1").map(|r| r.full_path());
    assert_eq!(resolution, Err(ResolveError::NotFound("pck1".to_string())));
}

#[test]
fn should_try_to_find_alternative_1() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(sass|scss|css)$").unwrap();
    let resolver1 = Resolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".css".into()],
        main_files: vec!["index".into()],
        restrictions: vec![Restriction::Fn(Arc::new(move |path| {
            path.as_os_str().to_str().is_some_and(|s| re.is_match(s).unwrap_or(false))
        }))],
        ..ResolveOptions::default()
    });

    let resolution = resolver1.resolve(&f, "pck1").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck1/index.css")));
}

#[test]
fn should_respect_string_restriction() {
    let fixture = super::fixture();
    let f = fixture.join("restrictions");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".js".into()],
        restrictions: vec![Restriction::Path(f.clone())],
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve(&f, "pck2");
    assert_eq!(resolution, Err(ResolveError::NotFound("pck2".to_string())));
}

#[test]
fn should_try_to_find_alternative_2() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(sass|scss|css)$").unwrap();
    let resolver1 = Resolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".css".into()],
        main_fields: vec!["main".into(), "style".into()],
        restrictions: vec![Restriction::Fn(Arc::new(move |path| {
            path.as_os_str().to_str().is_some_and(|s| re.is_match(s).unwrap_or(false))
        }))],
        ..ResolveOptions::default()
    });

    let resolution = resolver1.resolve(&f, "pck2").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck2/index.css")));
}

#[test]
fn should_try_to_find_alternative_3() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(sass|scss|css)$").unwrap();
    let resolver1 = Resolver::new(ResolveOptions {
        extensions: vec![".js".into()],
        main_fields: vec!["main".into(), "module".into(), "style".into()],
        restrictions: vec![Restriction::Fn(Arc::new(move |path| {
            path.as_os_str().to_str().is_some_and(|s| re.is_match(s).unwrap_or(false))
        }))],
        ..ResolveOptions::default()
    });

    let resolution = resolver1.resolve(&f, "pck2").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck2/index.css")));
}

// Test coverage for check_restrictions at line 783 in load_index()
#[test]
fn should_check_restrictions_in_load_index_with_enforce_extension_disabled() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(css)$").unwrap();
    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".css".into()],
        main_files: vec!["index".into()],
        enforce_extension: crate::EnforceExtension::Disabled,
        restrictions: vec![Restriction::Fn(Arc::new(move |path| {
            path.as_os_str().to_str().is_some_and(|s| re.is_match(s).unwrap_or(false))
        }))],
        ..ResolveOptions::default()
    });

    // Should find index.css instead of index.js due to restriction
    let resolution = resolver.resolve(&f, "pck1").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck1/index.css")));
}

// Test coverage for check_restrictions at line 831 in load_alias_or_file()
#[test]
fn should_check_restrictions_in_load_alias_or_file() {
    let f = super::fixture().join("restrictions");

    // Restrict to only files outside the restrictions directory
    let restrictions_path = f.clone();
    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".js".into()],
        restrictions: vec![Restriction::Fn(Arc::new(move |path| {
            !path.starts_with(&restrictions_path)
        }))],
        ..ResolveOptions::default()
    });

    // Direct file access should fail due to restriction
    let resolution = resolver.resolve(&f, "./node_modules/pck1/index.js");
    assert!(resolution.is_err());
}

// Test coverage for check_restrictions at line 1148 in browser field/alias resolution
#[test]
fn should_check_restrictions_in_browser_field_alias() {
    let f = super::fixture().join("browser-module");

    let resolver = Resolver::new(ResolveOptions {
        alias_fields: vec![vec!["browser".into()]],
        restrictions: vec![Restriction::Fn(Arc::new(|path| {
            // Restrict files containing "browser" in their path
            !path.to_str().is_some_and(|s| s.contains("browser"))
        }))],
        ..ResolveOptions::default()
    });

    // Should fail to resolve due to restriction on browser field
    let resolution = resolver.resolve(&f, "./lib/self.js");
    assert!(resolution.is_err());
}

// Test coverage for check_restrictions at line 1326 in load_extension_alias()
#[test]
fn should_check_restrictions_in_extension_alias() {
    let f = super::fixture().join("extension-alias");

    let resolver = Resolver::new(ResolveOptions {
        extension_alias: vec![
            (".js".into(), vec![".ts".into(), ".js".into()]),
            (".mjs".into(), vec![".mts".into(), ".mjs".into()]),
        ],
        restrictions: vec![Restriction::Fn(Arc::new(|path| {
            // Only allow .js files, not .ts files
            path.extension().and_then(|e| e.to_str()) == Some("js")
        }))],
        ..ResolveOptions::default()
    });

    // Should resolve to .js file even though .ts exists, due to restriction
    let resolution = resolver.resolve(&f, "./index.js").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("index.js")));
}

// Test coverage for check_restrictions at line 1570 in package main field resolution
#[test]
fn should_check_restrictions_in_package_main_fields() {
    let f = super::fixture().join("restrictions");

    let resolver = Resolver::new(ResolveOptions {
        main_fields: vec!["module".into(), "main".into()],
        restrictions: vec![Restriction::Fn(Arc::new(|path| {
            // Restrict .js files
            path.extension().and_then(|e| e.to_str()) != Some("js")
        }))],
        ..ResolveOptions::default()
    });

    // Should skip module.js and main field due to restriction
    let resolution = resolver.resolve(&f, "pck2");
    assert_eq!(resolution, Err(ResolveError::NotFound("pck2".to_string())));
}

// Test multiple restrictions together (Path + Fn)
#[test]
fn should_apply_multiple_restrictions() {
    let f = super::fixture().join("restrictions");

    // Use two function restrictions to test that both are applied
    let re_css = Regex::new(r"\.(css)$").unwrap();
    let re_no_js = Regex::new(r"\.(js)$").unwrap();
    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".css".into()],
        main_files: vec!["index".into()],
        restrictions: vec![
            Restriction::Fn(Arc::new(move |path| {
                path.as_os_str().to_str().is_some_and(|s| re_css.is_match(s).unwrap_or(false))
            })),
            Restriction::Fn(Arc::new(move |path| {
                // Reject .js files
                path.as_os_str().to_str().is_some_and(|s| !re_no_js.is_match(s).unwrap_or(false))
            })),
        ],
        ..ResolveOptions::default()
    });

    // Should pass both restrictions and resolve to CSS file
    let resolution = resolver.resolve(&f, "pck1").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck1/index.css")));
}

// Test that all restrictions must pass
#[test]
fn should_fail_if_any_restriction_fails() {
    let f = super::fixture().join("restrictions");

    // Use two function restrictions where one will fail
    let re_css = Regex::new(r"\.(css)$").unwrap();
    let re_no_css = Regex::new(r"\.(css)$").unwrap();
    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".css".into()],
        main_files: vec!["index".into()],
        restrictions: vec![
            Restriction::Fn(Arc::new(move |path| {
                // First restriction: must be CSS
                path.as_os_str().to_str().is_some_and(|s| re_css.is_match(s).unwrap_or(false))
            })),
            Restriction::Fn(Arc::new(move |path| {
                // Second restriction: must NOT be CSS (contradicts first)
                path.as_os_str().to_str().is_some_and(|s| !re_no_css.is_match(s).unwrap_or(false))
            })),
        ],
        ..ResolveOptions::default()
    });

    // Should fail because restrictions contradict each other
    let resolution = resolver.resolve(&f, "pck1");
    assert_eq!(resolution, Err(ResolveError::NotFound("pck1".to_string())));
}

// Test is_inside() edge case: exact path match
#[test]
fn should_allow_exact_path_in_restriction() {
    let f = super::fixture().join("restrictions");
    let exact_file = f.join("node_modules/pck1/index.css");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".css".into()],
        main_files: vec!["index".into()],
        restrictions: vec![Restriction::Path(exact_file.clone())],
        ..ResolveOptions::default()
    });

    // Exact path should pass is_inside check
    let resolution = resolver.resolve(&f, "pck1").map(|r| r.full_path());
    assert_eq!(resolution, Ok(exact_file));
}

// Test is_inside() edge case: parent directory restriction
#[test]
fn should_respect_parent_directory_restriction() {
    let fixture = super::fixture();
    let f = fixture.join("restrictions");

    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".js".into()],
        restrictions: vec![Restriction::Path(fixture)],
        ..ResolveOptions::default()
    });

    // Files outside the fixture directory should be rejected
    // pck2's main field points to ../../../c.js which is outside restrictions dir
    let resolution = resolver.resolve(&f, "pck2");
    assert_eq!(resolution, Err(ResolveError::NotFound("pck2".to_string())));
}
