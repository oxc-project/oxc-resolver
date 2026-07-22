use std::path::PathBuf;

use walkdir::WalkDir;

use crate::{ResolveOptions, Resolver, TsconfigDiscovery};

fn walk(dir: &PathBuf) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

// https://github.com/dominikg/tsconfck/blob/main/packages/tsconfck/tests/parse.js
#[test]
fn parse_valid() {
    let dir = super::fixture_root().join("tsconfck").join("parse").join("valid");
    let resolver = Resolver::default();
    for path in walk(&dir).into_iter().filter(|path| path.file_name().unwrap() == "tsconfig.json") {
        let tsconfig = resolver.resolve_tsconfig(&path);
        assert_eq!(tsconfig.map(|t| t.path().to_path_buf()), Ok(path));
    }
}

#[test]
fn parse_invalid() {
    let dir = super::fixture_root().join("tsconfck").join("parse").join("invalid");
    let resolver = Resolver::default();
    for path in walk(&dir).into_iter().filter(|path| path.file_name().unwrap() == "tsconfig.json") {
        // A missing `extends` target is non-fatal in oxc (TS6053), matching
        // tsc/tsgo — unlike tsconfck, which classifies it as invalid.
        if path.parent().is_some_and(|p| p.ends_with("extends-not-found")) {
            continue;
        }
        let tsconfig = resolver.resolve_tsconfig(&path);
        assert!(tsconfig.is_err(), "{} {tsconfig:?}", path.display());
    }
}

#[test]
fn config_dir() {
    let dir = super::fixture_root().join("tsconfck").join("parse").join("valid").join("configDir");
    let resolver = Resolver::default();
    for path in walk(&dir).into_iter().filter(|path| path.file_name().unwrap() == "tsconfig.json") {
        let tsconfig = resolver.resolve_tsconfig(&path).unwrap();
        let base_url = tsconfig.compiler_options.base_url.clone();
        assert_eq!(base_url, Some(path.parent().unwrap().join("src")));
    }
}

#[test]
fn solution() {
    let dir = super::fixture_root().join("tsconfck").join("parse").join("solution");
    let resolver = Resolver::default();
    for path in walk(&dir).into_iter().filter(|path| path.file_name().unwrap() == "tsconfig.json") {
        let tsconfig = resolver.resolve_tsconfig(&path);
        assert_eq!(tsconfig.map(|t| t.path().to_path_buf()), Ok(path));
    }
}

#[test]
fn part_of_solution() {
    let root = super::fixture_root().join("tsconfck").join("parse").join("solution");

    let pass = [
        ("simple", "src/foo.ts", "src/tsconfig.json"),
        ("simple", "tests/foo.ts", "tests/tsconfig.json"),
        ("mixed", "src/bar.mts", "tsconfig.src.json"),
        ("mixed", "src/baz.cts", "tsconfig.src.json"),
        ("mixed", "src/foo.ts", "tsconfig.src.json"),
        ("mixed", "src/foo.spec.ts", "tsconfig.test.json"),
        ("referenced-extends-original", "src/foo.ts", "src/tsconfig.src.json"),
        ("referenced-extends-original", "tests/foo.test.ts", "tests/tsconfig.test.json"),
        ("referenced-with-configDir", "src/foo.ts", "tsconfig.src.json"),
        (
            "referenced-with-configDir-and-extends",
            "packages/foo/src/foo.ts",
            "packages/foo/tsconfig.foo.json",
        ),
        ("referenced-with-implicit-globs", "src/foo.ts", "tsconfig.src.json"),
        ("referenced-with-implicit-globs", "tests/foo.test.ts", "tsconfig.test.json"),
        // not part of tsconfck
        ("referenced-files", "src/foo.ts", "tsconfig.foo.json"),
        ("referenced-include", "src/foo.ts", "tsconfig.foo.json"),
        ("referenced-include", "src/bar.ts", "tsconfig.bar.json"),
        ("referenced-exclude", "src/foo.ts", "tsconfig.foo.json"),
        ("referenced-exclude", "src/bar.ts", "tsconfig.bar.json"),
    ];

    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });
    for (dir, specifier, expected) in pass {
        let dir = root.join(dir);
        let tsconfig = resolver.find_tsconfig(dir.join(specifier)).unwrap().unwrap();
        assert_eq!(tsconfig.path.clone(), dir.join(expected), "{dir:?} {specifier}");
    }

    // `src/bar.ts` is excluded from the only referenced project
    // (`tsconfig.foo.json` lists `files: ["src/foo.ts"]`) and owned by no other,
    // so discovery finds no config for it — matching `tsserver` / `typescript-go`,
    // which leave such a file in an inferred project rather than the solution root.
    let unowned = root.join("referenced-files").join("src/bar.ts");
    assert!(resolver.find_tsconfig(unowned).unwrap().is_none());
}

// https://github.com/dominikg/tsconfck/blob/main/packages/tsconfck/tests/find.js
#[test]
fn find() {
    let dir = super::fixture_root().join("tsconfck").join("find").join("a");
    let resolver = Resolver::new(ResolveOptions {
        tsconfig: Some(TsconfigDiscovery::Auto),
        ..ResolveOptions::default()
    });

    let result = resolver.find_tsconfig(dir.join("foo.ts"));
    let path = result.map(|tsconfig| tsconfig.unwrap().path().to_path_buf());
    assert_eq!(path, Ok(dir.join("tsconfig.json")));

    let result = resolver.find_tsconfig(dir.join("b").join("foo.ts"));
    let path = result.map(|tsconfig| tsconfig.unwrap().path().to_path_buf());
    assert_eq!(path, Ok(dir.join("tsconfig.json")));
}
