use std::{env, path::PathBuf};

use unrs_resolver::{Resolution, ResolveError, ResolveOptions, Resolver};

fn dir() -> PathBuf {
    env::current_dir().unwrap()
}

#[test]
fn chinese() {
    let dir = dir();
    let specifier = "./fixtures/misc/中文/中文.js";
    let resolution = Resolver::new(ResolveOptions::default()).resolve(&dir, specifier);
    assert_eq!(
        resolution.map(Resolution::into_path_buf),
        Ok(dir.join("fixtures/misc/中文/中文.js"))
    );
}

#[test]
fn styled_components() {
    let dir = dir();
    let path = dir.join("fixtures/pnpm");
    let module_path = dir
        .join("node_modules")
        .join(".pnpm")
        .join("styled-components@6.1.17_react-dom@19.1.0_react@19.1.0__react@19.1.0")
        .join("node_modules")
        .join("styled-components");
    let specifier = "styled-components";

    // cjs
    let options =
        ResolveOptions { alias_fields: vec![vec!["browser".into()]], ..ResolveOptions::default() };
    let resolution = Resolver::new(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(Resolution::into_path_buf),
        Ok(module_path.join("dist").join("styled-components.browser.cjs.js"))
    );

    // esm
    let options = ResolveOptions {
        alias_fields: vec![vec!["browser".into()]],
        main_fields: vec!["module".into()],
        ..ResolveOptions::default()
    };
    let resolution = Resolver::new(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(unrs_resolver::Resolution::into_path_buf),
        Ok(module_path.join("dist").join("styled-components.browser.esm.js"))
    );
}

#[test]
fn axios() {
    let dir = dir();
    let path = dir.join("fixtures/pnpm");
    let module_path = dir.join("node_modules/.pnpm/axios@1.8.4/node_modules/axios");
    let specifier = "axios";

    // default
    let options = ResolveOptions::default();
    let resolution = Resolver::new(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(unrs_resolver::Resolution::into_path_buf),
        Ok(module_path.join("index.js"))
    );

    // browser
    let options = ResolveOptions {
        condition_names: vec!["browser".into(), "require".into()],
        ..ResolveOptions::default()
    };
    let resolution = Resolver::new(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(unrs_resolver::Resolution::into_path_buf),
        Ok(module_path.join("dist/browser/axios.cjs"))
    );

    // cjs
    let options = ResolveOptions {
        condition_names: vec!["node".into(), "require".into()],
        ..ResolveOptions::default()
    };
    let resolution = Resolver::new(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(unrs_resolver::Resolution::into_path_buf),
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
        symlinks: false,
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
        dir.join("node_modules/.pnpm/decimal.js@10.5.0/node_modules/decimal.js/decimal.mjs");

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
        dir.join("node_modules/.pnpm/decimal.js@10.5.0/node_modules/decimal.js/decimal.mjs");

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
// regression: https://github.com/NicholasLYang/oxc-repro
fn nested_symlinks() {
    let dir = dir();
    let dir = dir.join("fixtures/nested-symlink");
    assert_eq!(
        Resolver::new(ResolveOptions::default())
            // ./apps/web/nm/@repo/typescript-config is a symlink
            .resolve(&dir, "./apps/web/nm/@repo/typescript-config/index.js")
            .map(unrs_resolver::Resolution::into_path_buf),
        Ok(dir.join("nm/index.js"))
    );
    assert_eq!(
        Resolver::new(ResolveOptions::default())
            // ./apps/tooling is a symlink
            .resolve(&dir, "./apps/tooling/typescript-config/index.js")
            .map(unrs_resolver::Resolution::into_path_buf),
        Ok(dir.join("nm/index.js"))
    );
}

// NOTE: pnpm v10 shortens windows directory path.
// `virtualStoreDirMaxLength: 1024` is set in pnpm-workspace.yaml to keep the long name.
#[test]
fn windows_symlinked_longfilename() {
    let dir = dir();
    let path = dir.join("fixtures/pnpm");
    let module_path = dir.join("node_modules")
        .join(".pnpm")
        .join("@oxc-resolver+test-longfilename-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@file+fixtures+pnpm+longfilename")
        .join("node_modules")
        .join("@oxc-resolver")
        .join("test-longfilename-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        .join("index.js");

    // Note: dunce::canonicalize seems to only trim \\?\ when the path is shorter than 260 chars.
    // Our implementation should mitigate that.
    // <https://gitlab.com/kornelski/dunce/-/blob/1ee29a83526c9f4c3618e1335f0454c878a54dcf/src/lib.rs#L176-180>
    assert!(module_path.as_os_str().len() > 260, "Windows path must be super long.");

    let specifier = "@oxc-resolver/test-longfilename-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let resolution =
        Resolver::new(ResolveOptions::default()).resolve(&path, specifier).map(|r| r.full_path());
    assert_eq!(resolution, Ok(module_path));
}

#[test]
fn package_json_with_bom() {
    let dir = dir();
    let dir = dir.join("fixtures/misc");
    assert_eq!(
        Resolver::new(ResolveOptions::default())
            .resolve(&dir, "./package-json-with-bom")
            .map(Resolution::into_path_buf),
        Ok(dir.join("package-json-with-bom/index.js"))
    );
}

#[test]
fn dual_condition_names() {
    let dir: PathBuf = dir();
    let path = dir.join("fixtures/dual-condition-names");
    let resolver = Resolver::new(ResolveOptions {
        condition_names: vec!["import".into(), "require".into()],
        ..ResolveOptions::default()
    });
    assert_eq!(
        resolver.resolve(path, "zod").map(|r| r.full_path()),
        Ok(dir.join("node_modules/.pnpm/zod@3.24.4/node_modules/zod/lib/index.mjs"))
    );
}
