use std::{env, path::PathBuf};

use oxc_resolver::{ResolveOptions, Resolver};

fn dir() -> PathBuf {
    env::current_dir().unwrap()
}

#[test]
fn chinese() {
    let dir = dir();
    let specifier = "./fixtures/misc/中文/中文.js";
    let resolution = Resolver::new(ResolveOptions::default()).resolve(&dir, specifier);
    assert_eq!(resolution.map(|r| r.into_path_buf()), Ok(dir.join("fixtures/misc/中文/中文.js")));
}

#[test]
fn styled_components() {
    let dir = dir();
    let path = dir.join("fixtures/pnpm8");
    let specifier = "styled-components";

    // cjs
    let options =
        ResolveOptions { alias_fields: vec![vec!["browser".into()]], ..ResolveOptions::default() };
    let resolution = Resolver::new(options).resolve(&path, specifier);
    assert_eq!(resolution.map(|r| r.into_path_buf()), Ok(path.join("node_modules/.pnpm/styled-components@6.1.1_react-dom@18.2.0_react@18.2.0/node_modules/styled-components/dist/styled-components.browser.cjs.js")));

    // esm
    let options = ResolveOptions {
        alias_fields: vec![vec!["browser".into()]],
        main_fields: vec!["module".into()],
        ..ResolveOptions::default()
    };
    let resolution = Resolver::new(options).resolve(&path, specifier);
    assert_eq!(resolution.map(|r| r.into_path_buf()), Ok(path.join("node_modules/.pnpm/styled-components@6.1.1_react-dom@18.2.0_react@18.2.0/node_modules/styled-components/dist/styled-components.browser.esm.js")));
}
