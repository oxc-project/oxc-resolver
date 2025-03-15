use std::{env, path::PathBuf};

use unrspack_resolver::{ResolveError, ResolveOptions, Resolver};

fn dir() -> PathBuf {
    env::current_dir().unwrap()
}

#[test]
fn chinese() {
    let dir = dir();
    let specifier = "./fixtures/misc/中文/中文.js";
    let resolution = Resolver::new(ResolveOptions::default()).resolve(&dir, specifier);
    assert_eq!(
        resolution.map(unrspack_resolver::Resolution::into_path_buf),
        Ok(dir.join("fixtures/misc/中文/中文.js"))
    );
}

#[test]
fn styled_components() {
    let dir = dir();
    let path = dir.join("fixtures/pnpm");
    let module_path = dir.join("node_modules/.pnpm/styled-components@6.1.1_react-dom@18.3.1_react@18.3.1__react@18.3.1/node_modules/styled-components");
    let specifier = "styled-components";

    // cjs
    let options =
        ResolveOptions { alias_fields: vec![vec!["browser".into()]], ..ResolveOptions::default() };
    let resolution = Resolver::new(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(unrspack_resolver::Resolution::into_path_buf),
        Ok(module_path.join("dist/styled-components.browser.cjs.js"))
    );

    // esm
    let options = ResolveOptions {
        alias_fields: vec![vec!["browser".into()]],
        main_fields: vec!["module".into()],
        ..ResolveOptions::default()
    };
    let resolution = Resolver::new(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(unrspack_resolver::Resolution::into_path_buf),
        Ok(module_path.join("dist/styled-components.browser.esm.js"))
    );
}

#[test]
fn axios() {
    let dir = dir();
    let path = dir.join("fixtures/pnpm");
    let module_path = dir.join("node_modules/.pnpm/axios@1.6.2/node_modules/axios");
    let specifier = "axios";

    // default
    let options = ResolveOptions::default();
    let resolution = Resolver::new(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(unrspack_resolver::Resolution::into_path_buf),
        Ok(module_path.join("index.js"))
    );

    // browser
    let options = ResolveOptions {
        condition_names: vec!["browser".into(), "require".into()],
        ..ResolveOptions::default()
    };
    let resolution = Resolver::new(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(unrspack_resolver::Resolution::into_path_buf),
        Ok(module_path.join("dist/browser/axios.cjs"))
    );

    // cjs
    let options = ResolveOptions {
        condition_names: vec!["node".into(), "require".into()],
        ..ResolveOptions::default()
    };
    let resolution = Resolver::new(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(unrspack_resolver::Resolution::into_path_buf),
        Ok(module_path.join("dist/node/axios.cjs"))
    );
}

#[test]
fn postcss() {
    let dir = dir();
    let path = dir.join("fixtures/pnpm");
    let module_path = path.join("node_modules/postcss");
    let resolver = Resolver::new(ResolveOptions {
        alias_fields: vec![vec!["browser".into()]],
        ..ResolveOptions::default()
    });

    // should ignore "path"
    let resolution = resolver.resolve(&module_path, "path");
    assert_eq!(resolution, Err(ResolveError::Ignored(module_path.clone())));

    // should ignore "./lib/terminal-highlight"
    let resolution = resolver.resolve(&module_path, "./lib/terminal-highlight");
    assert_eq!(resolution, Err(ResolveError::Ignored(module_path.join("lib/terminal-highlight"))));
}

#[test]
fn ipaddr_js() {
    let dir = dir();
    let path = dir.join("fixtures/pnpm");
    let module_path =
        dir.join("node_modules/.pnpm/ipaddr.js@2.2.0/node_modules/ipaddr.js/lib/ipaddr.js");

    let resolvers = [
        // with `extension_alias`
        Resolver::new(ResolveOptions {
            extension_alias: vec![(".js".into(), vec![".js".into(), ".ts".into(), ".tsx".into()])],
            ..ResolveOptions::default()
        }),
        // with `extensions` should still resolve to module main
        Resolver::new(ResolveOptions {
            extensions: vec![(".ts".into())],
            ..ResolveOptions::default()
        }),
        // default
        Resolver::default(),
    ];

    for resolver in resolvers {
        let resolution = resolver.resolve(&path, "ipaddr.js").map(|r| r.full_path());
        assert_eq!(resolution, Ok(module_path.clone()));
    }
}

#[test]
fn decimal_js() {
    let dir = dir();
    let path = dir.join("fixtures/pnpm");
    let module_path =
        dir.join("node_modules/.pnpm/decimal.js@10.4.3/node_modules/decimal.js/decimal.mjs");

    let resolvers = [
        // with `extension_alias`
        Resolver::new(ResolveOptions {
            extension_alias: vec![(".js".into(), vec![".js".into(), ".ts".into(), ".tsx".into()])],
            condition_names: vec!["import".into()],
            ..ResolveOptions::default()
        }),
        // default
        Resolver::new(ResolveOptions {
            condition_names: vec!["import".into()],
            ..ResolveOptions::default()
        }),
    ];

    for resolver in resolvers {
        let resolution = resolver.resolve(&path, "decimal.js").map(|r| r.full_path());
        assert_eq!(resolution, Ok(module_path.clone()));
    }
}

#[test]
fn decimal_js_from_mathjs() {
    let dir = dir();
    let path = dir.join("node_modules/.pnpm/mathjs@13.2.0/node_modules/mathjs/lib/esm");
    let module_path =
        dir.join("node_modules/.pnpm/decimal.js@10.4.3/node_modules/decimal.js/decimal.mjs");

    let resolvers = [
        // with `extension_alias`
        Resolver::new(ResolveOptions {
            extension_alias: vec![(".js".into(), vec![".js".into(), ".ts".into(), ".tsx".into()])],
            condition_names: vec!["import".into()],
            ..ResolveOptions::default()
        }),
        // default
        Resolver::new(ResolveOptions {
            condition_names: vec!["import".into()],
            ..ResolveOptions::default()
        }),
    ];

    for resolver in resolvers {
        let resolution = resolver.resolve(&path, "decimal.js").map(|r| r.full_path());
        assert_eq!(resolution, Ok(module_path.clone()));
    }
}
