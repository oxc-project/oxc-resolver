use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rayon::prelude::*;

fn data() -> Vec<(PathBuf, &'static str)> {
    let cwd = env::current_dir().unwrap();
    let f1 = cwd.join("fixtures/enhanced_resolve");
    let f2 = f1.join("test/fixtures");
    vec![
        // real packages
        (cwd.clone(), "@napi-rs/cli"),
        (cwd.clone(), "@napi-rs/wasm-runtime"),
        (cwd.clone(), "vitest"),
        (cwd.clone(), "emnapi"),
        (cwd, "typescript"),
        // relative path
        (f1.clone(), "./"),
        (f1.clone(), "./lib/index"),
        // absolute path
        (f1.clone(), "/absolute/path"),
        // query fragment
        (f2.clone(), "./main1.js#fragment?query"),
        (f2.clone(), "m1/a.js?query#fragment"),
        // browserField
        (f2.join("browser-module"), "./lib/replaced"),
        (f2.join("browser-module/lib"), "./replaced"),
        // exportsField
        (f2.join("exports-field"), "exports-field"),
        (f2.join("exports-field"), "exports-field/dist/main.js"),
        (f2.join("exports-field"), "exports-field/dist/main.js?foo"),
        (f2.join("exports-field"), "exports-field/dist/main.js#foo"),
        (f2.join("exports-field"), "@exports-field/core"),
        (f2.join("imports-exports-wildcard"), "m/features/f.js"),
        // extensionAlias
        (f2.join("extension-alias"), "./index.js"),
        (f2.join("extension-alias"), "./dir2/index.mjs"),
        // extensions
        (f2.join("extensions"), "./foo"),
        (f2.join("extensions"), "."),
        (f2.join("extensions"), "./dir"),
        (f2.join("extensions"), "module/"),
        // importsField
        (f2.join("imports-field"), "#imports-field"),
        (f2.join("imports-exports-wildcard/node_modules/m/"), "#internal/i.js"),
        // scoped
        (f2.join("scoped"), "@scope/pack1"),
        (f2.join("scoped"), "@scope/pack2/lib"),
        // dashed name
        (f2.clone(), "dash"),
        (f2.clone(), "dash-name"),
        (f2.join("node_modules/dash"), "dash"),
        (f2.join("node_modules/dash"), "dash-name"),
        (f2.join("node_modules/dash-name"), "dash"),
        (f2.join("node_modules/dash-name"), "dash-name"),
        // alias
        (f1.clone(), "aaa"),
        (f1.clone(), "ggg"),
        (f1.clone(), "rrr"),
        (f1.clone(), "@"),
        (f1, "@@@"),
    ]
}

fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    #[cfg(target_family = "unix")]
    {
        std::os::unix::fs::symlink(original, link)
    }

    #[cfg(target_family = "windows")]
    {
        std::os::windows::fs::symlink_file(original, link)
    }
}

fn create_symlinks() -> io::Result<PathBuf> {
    let root = env::current_dir()?.join("fixtures/enhanced_resolve");
    let dirname = root.join("test");
    let temp_path = dirname.join("temp_symlinks");
    let create_symlink_fixtures = || -> io::Result<()> {
        fs::create_dir(&temp_path)?;
        let mut index = fs::File::create(temp_path.join("index.js"))?;
        index.write_all(b"console.log('Hello, World!')")?;
        // create 10000 symlink files pointing to the index.js
        for i in 0..10000 {
            symlink(temp_path.join("index.js"), temp_path.join(format!("file{i}.js")))?;
        }
        Ok(())
    };
    if !temp_path.exists() {
        if let Err(err) = create_symlink_fixtures() {
            let _ = fs::remove_dir_all(&temp_path);
            return Err(err);
        }
    }
    Ok(temp_path)
}

fn unrs_resolver() -> unrs_resolver::Resolver {
    use unrs_resolver::{AliasValue, ResolveOptions, Resolver};
    let alias_value = AliasValue::from("./");
    Resolver::new(ResolveOptions {
        extensions: vec![".ts".into(), ".js".into()],
        condition_names: vec!["webpack".into(), "require".into()],
        alias_fields: vec![vec!["browser".into()]],
        extension_alias: vec![
            (".js".into(), vec![".ts".into(), ".js".into()]),
            (".mjs".into(), vec![".mts".into()]),
        ],
        // Real projects LOVE setting these many aliases.
        // I saw them with my own eyes.
        alias: vec![
            ("/absolute/path".into(), vec![alias_value.clone()]),
            ("aaa".into(), vec![alias_value.clone()]),
            ("bbb".into(), vec![alias_value.clone()]),
            ("ccc".into(), vec![alias_value.clone()]),
            ("ddd".into(), vec![alias_value.clone()]),
            ("eee".into(), vec![alias_value.clone()]),
            ("fff".into(), vec![alias_value.clone()]),
            ("ggg".into(), vec![alias_value.clone()]),
            ("hhh".into(), vec![alias_value.clone()]),
            ("iii".into(), vec![alias_value.clone()]),
            ("jjj".into(), vec![alias_value.clone()]),
            ("kkk".into(), vec![alias_value.clone()]),
            ("lll".into(), vec![alias_value.clone()]),
            ("mmm".into(), vec![alias_value.clone()]),
            ("nnn".into(), vec![alias_value.clone()]),
            ("ooo".into(), vec![alias_value.clone()]),
            ("ppp".into(), vec![alias_value.clone()]),
            ("qqq".into(), vec![alias_value.clone()]),
            ("rrr".into(), vec![alias_value.clone()]),
            ("sss".into(), vec![alias_value.clone()]),
            ("@".into(), vec![alias_value.clone()]),
            ("@@".into(), vec![alias_value.clone()]),
            ("@@@".into(), vec![alias_value]),
        ],
        ..ResolveOptions::default()
    })
}

fn bench_resolver(c: &mut Criterion) {
    let data = data();

    // check validity
    for (path, request) in &data {
        assert!(unrs_resolver().resolve(path, request).is_ok(), "{:?} {request}", path.display());
    }

    let symlink_test_dir = create_symlinks().expect("Create symlink fixtures failed");

    let symlinks_range = 0u32..10000;

    for i in symlinks_range.clone() {
        assert!(
            unrs_resolver().resolve(&symlink_test_dir, &format!("./file{i}")).is_ok(),
            "file{i}.js"
        );
    }

    let mut group = c.benchmark_group("resolver");

    group.bench_with_input(BenchmarkId::from_parameter("single-thread"), &data, |b, data| {
        let unrs_resolver = unrs_resolver();
        b.iter(|| {
            for (path, request) in data {
                _ = unrs_resolver.resolve(path, request);
            }
        });
    });

    group.bench_with_input(BenchmarkId::from_parameter("multi-thread"), &data, |b, data| {
        let unrs_resolver = unrs_resolver();
        b.iter(|| {
            data.par_iter().for_each(|(path, request)| {
                _ = unrs_resolver.resolve(path, request);
            });
        });
    });

    group.bench_with_input(
        BenchmarkId::from_parameter("resolve from symlinks"),
        &symlinks_range,
        |b, data| {
            let unrs_resolver = unrs_resolver();
            b.iter(|| {
                for i in data.clone() {
                    assert!(
                        unrs_resolver.resolve(&symlink_test_dir, &format!("./file{i}")).is_ok(),
                        "file{i}.js"
                    );
                }
            });
        },
    );
}

criterion_group!(resolver, bench_resolver);
criterion_main!(resolver);
