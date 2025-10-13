#[cfg(feature = "typescript")]
mod tests {
    use crate::{
        ResolveOptions,
        typescript::{
            TypeResolutionMode, TypeScriptOptions, TypesVersions, VersionRange,
            get_types_package_name, mangle_scoped_package_name,
        },
    };
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_mangle_scoped_package_name() {
        assert_eq!(mangle_scoped_package_name("@foo/bar"), "foo__bar");
        assert_eq!(mangle_scoped_package_name("@angular/core"), "angular__core");
        assert_eq!(mangle_scoped_package_name("react"), "react");
        assert_eq!(mangle_scoped_package_name("lodash"), "lodash");
        assert_eq!(mangle_scoped_package_name("@types/node"), "types__node");
    }

    #[test]
    fn test_resolve_at_types_package() {
        assert_eq!(get_types_package_name("@foo/bar"), "@types/foo__bar");
        assert_eq!(get_types_package_name("@angular/core"), "@types/angular__core");
        assert_eq!(get_types_package_name("react"), "@types/react");
        assert_eq!(get_types_package_name("lodash"), "@types/lodash");
    }

    #[test]
    fn test_version_range_exact_match() {
        let range = VersionRange::new("4.2.0".to_string());
        assert!(range.matches("4.2.0"));
        assert!(!range.matches("4.2.1"));
        assert!(!range.matches("4.3.0"));
    }

    #[test]
    fn test_version_range_greater_or_equal() {
        let range = VersionRange::new(">=4.2".to_string());
        assert!(range.matches("4.2"));
        assert!(range.matches("4.2.0"));
        assert!(range.matches("4.3"));
        assert!(range.matches("5.0"));
        assert!(!range.matches("4.1"));
        assert!(!range.matches("3.9"));
    }

    #[test]
    fn test_version_range_wildcard() {
        let range = VersionRange::new("*".to_string());
        assert!(range.matches("1.0"));
        assert!(range.matches("3.0"));
        assert!(range.matches("4.2"));
        assert!(range.matches("5.0"));
        assert!(range.matches("100.0.0"));
    }

    #[test]
    fn test_types_versions_resolve() {
        let mut mappings_42 = HashMap::new();
        mappings_42.insert("*".to_string(), vec!["ts4.2/*".to_string()]);

        let mut mappings_37 = HashMap::new();
        mappings_37.insert("*".to_string(), vec!["ts3.7/*".to_string()]);

        let mut mappings_default = HashMap::new();
        mappings_default.insert("*".to_string(), vec!["ts3.0/*".to_string()]);

        let types_versions = TypesVersions::new(vec![
            (VersionRange::new(">=4.2".to_string()), mappings_42),
            (VersionRange::new(">=3.7".to_string()), mappings_37),
            (VersionRange::new("*".to_string()), mappings_default),
        ]);

        let result = types_versions.resolve_for_version("4.3", "index");
        assert_eq!(result, Some(vec!["ts4.2/index".to_string()]));

        let result = types_versions.resolve_for_version("3.8", "index");
        assert_eq!(result, Some(vec!["ts3.7/index".to_string()]));

        let result = types_versions.resolve_for_version("3.0", "index");
        assert_eq!(result, Some(vec!["ts3.0/index".to_string()]));

        let result = types_versions.resolve_for_version("2.0", "index");
        assert_eq!(result, Some(vec!["ts3.0/index".to_string()]));
    }

    #[test]
    fn test_types_versions_specific_paths() {
        let mut mappings = HashMap::new();
        mappings.insert("utils".to_string(), vec!["dist/utils.d.ts".to_string()]);
        mappings.insert("core".to_string(), vec!["dist/core.d.ts".to_string()]);
        mappings.insert("*".to_string(), vec!["dist/*.d.ts".to_string()]);

        let types_versions =
            TypesVersions::new(vec![(VersionRange::new(">=4.0".to_string()), mappings)]);

        let result = types_versions.resolve_for_version("4.5", "utils");
        assert_eq!(result, Some(vec!["dist/utils.d.ts".to_string()]));

        let result = types_versions.resolve_for_version("4.5", "core");
        assert_eq!(result, Some(vec!["dist/core.d.ts".to_string()]));

        let result = types_versions.resolve_for_version("4.5", "helpers");
        assert_eq!(result, Some(vec!["dist/helpers.d.ts".to_string()]));
    }

    #[test]
    fn test_typescript_options_builder() {
        let options = TypeScriptOptions::new()
            .with_typescript_version("5.0.0".to_string())
            .with_type_roots(vec![PathBuf::from("./custom-types")])
            .with_type_resolution_mode(TypeResolutionMode::Declaration)
            .with_resolve_type_references(true);

        assert_eq!(options.typescript_version, Some("5.0.0".to_string()));
        assert_eq!(options.type_roots, Some(vec![PathBuf::from("./custom-types")]));
        assert_eq!(options.type_resolution_mode, TypeResolutionMode::Declaration);
        assert!(options.resolve_type_references);
    }

    #[test]
    fn test_typescript_options_default() {
        let options = TypeScriptOptions::default();
        assert_eq!(options.typescript_version, None);
        assert_eq!(options.type_resolution_mode, TypeResolutionMode::Full);
        assert!(!options.resolve_type_references);
    }

    #[test]
    fn test_resolve_options_with_typescript() {
        let ts_options = TypeScriptOptions::new().with_typescript_version("5.0.0".to_string());

        let resolve_options = ResolveOptions::default().with_typescript_options(ts_options);

        assert!(resolve_options.typescript_options.is_some());
        let stored_options = resolve_options.typescript_options.unwrap();
        assert_eq!(stored_options.typescript_version, Some("5.0.0".to_string()));
    }

    #[test]
    fn test_types_versions_from_json() {
        let json_str = r#"{
            ">=4.2": {
                "*": ["ts4.2/*"]
            },
            ">=3.7": {
                "*": ["ts3.7/*"]
            },
            "*": {
                "*": ["ts3.0/*"]
            }
        }"#;

        let json: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let types_versions = TypesVersions::from_json(json.as_object().unwrap());

        assert!(types_versions.is_some());
        let types_versions = types_versions.unwrap();

        let result = types_versions.resolve_for_version("4.3", "index");
        assert!(result.is_some());
        assert_eq!(result.unwrap()[0], "ts4.2/index");
    }

    #[test]
    fn test_types_versions_from_json_complex() {
        let json_str = r#"{
            ">=4.0": {
                "utils": ["dist/v4/utils.d.ts"],
                "core/*": ["dist/v4/core/*.d.ts"],
                "*": ["dist/v4/*.d.ts"]
            },
            "*": {
                "*": ["dist/legacy/*.d.ts"]
            }
        }"#;

        let json: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let types_versions = TypesVersions::from_json(json.as_object().unwrap());

        assert!(types_versions.is_some());
        let types_versions = types_versions.unwrap();

        let result = types_versions.resolve_for_version("4.5", "utils");
        assert_eq!(result, Some(vec!["dist/v4/utils.d.ts".to_string()]));

        let result = types_versions.resolve_for_version("3.0", "anything");
        assert_eq!(result, Some(vec!["dist/legacy/anything.d.ts".to_string()]));
    }

    #[test]
    fn test_version_comparison() {
        let range = VersionRange::new(">=4.2.3".to_string());
        assert!(range.matches("4.2.3"));
        assert!(range.matches("4.2.4"));
        assert!(range.matches("4.3.0"));
        assert!(range.matches("5.0.0"));
        assert!(!range.matches("4.2.2"));
        assert!(!range.matches("4.2.0"));
        assert!(!range.matches("4.1.9"));
    }

    #[test]
    fn test_type_resolution_mode() {
        assert_eq!(TypeResolutionMode::default(), TypeResolutionMode::Full);

        let mode = TypeResolutionMode::Declaration;
        assert_eq!(mode, TypeResolutionMode::Declaration);

        let mode = TypeResolutionMode::None;
        assert_eq!(mode, TypeResolutionMode::None);
    }
}
