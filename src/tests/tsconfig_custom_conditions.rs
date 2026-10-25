use crate::{ResolveOptions, Resolver, TsconfigDiscovery};

#[test]
fn custom_conditions_are_inherited_and_combined_with_global_conditions() {
    let f = super::fixture_root().join("tsconfig/cases/custom-conditions");
    let resolver = Resolver::new(ResolveOptions {
        condition_names: vec!["global".into()],
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });

    let cases = [
        ("inherited", Some("tsconfig"), "tsconfig.js"),
        ("referenced/app", Some("tsconfig"), "tsconfig.js"),
        ("overridden", Some("override"), "override.js"),
        ("cleared", None, "global.js"),
    ];

    for (project, expected_conditions, expected_target) in cases {
        let importer = f.join(project).join("main.ts");
        let tsconfig = resolver.find_tsconfig(&importer).unwrap().unwrap();
        let conditions = tsconfig
            .compiler_options
            .custom_conditions
            .as_deref()
            .map(|conditions| conditions.iter().map(String::as_str).collect::<Vec<_>>());
        assert_eq!(
            conditions,
            expected_conditions.map(|condition| vec![condition]),
            "{project} compiler options"
        );

        for specifier in ["custom-conditions-pkg", "#fixture"] {
            let resolution = resolver.resolve_file(&importer, specifier).unwrap();
            assert_eq!(
                resolution.path().file_name().and_then(|name| name.to_str()),
                Some(expected_target),
                "{project} resolving {specifier}"
            );
        }
    }
}
