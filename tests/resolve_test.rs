use std::{env, path::PathBuf};

use oxc_resolver::{ResolveOptions, Resolver};

fn dir() -> PathBuf {
    env::current_dir().unwrap()
}

#[test]
fn chinese_dir() {
    let dir = dir();
    let specifier = "./fixtures/misc/中文/中文.js";
    let resolution = Resolver::new(ResolveOptions::default()).resolve(&dir, specifier);
    assert_eq!(resolution.map(|r| r.into_path_buf()), Ok(dir.join("fixtures/misc/中文/中文.js")))
}
