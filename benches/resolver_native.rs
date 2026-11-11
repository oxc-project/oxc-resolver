use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rayon::prelude::*;
use resolver_workload::{data, oxc_resolver_memory, resolve_options};

#[path = "support/resolver.rs"]
mod resolver_workload;

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
    let root = env::current_dir()?.join("fixtures/enhanced-resolve");
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
    if !temp_path.exists()
        && let Err(err) = create_symlink_fixtures()
    {
        let _ = fs::remove_dir_all(&temp_path);
        return Err(err);
    }
    Ok(temp_path)
}

fn oxc_resolver_real() -> oxc_resolver::Resolver {
    use oxc_resolver::Resolver;
    Resolver::new(resolve_options())
}

/// Use four workers for local throughput comparisons. These benchmarks are excluded from CPU
/// simulation because a fixed pool size does not make thread scheduling deterministic.
fn pin_rayon_threads() {
    let _ = rayon::ThreadPoolBuilder::new().num_threads(4).build_global();
}

fn bench_resolver_memory(c: &mut Criterion) {
    pin_rayon_threads();
    let data = data();
    let mut group = c.benchmark_group("resolver_memory");

    group.bench_with_input(BenchmarkId::from_parameter("multi-thread"), &data, |b, data| {
        let oxc_resolver = oxc_resolver_memory();
        for (path, request) in data {
            oxc_resolver.resolve(path, request).unwrap();
        }
        b.iter(|| {
            data.par_iter().for_each(|(path, request)| {
                _ = oxc_resolver.resolve(path, request);
            });
        });
    });
}

fn bench_resolver_real(c: &mut Criterion) {
    pin_rayon_threads();
    let data = data();
    let symlink_test_dir = create_symlinks().expect("Create symlink fixtures failed");

    // check validity
    for (path, request) in &data {
        assert!(oxc_resolver_real().resolve(path, request).is_ok(), "{} {request}", path.display());
    }

    let symlink_specifiers = (0u32..10000).map(|i| format!("./file{i}")).collect::<Vec<_>>();

    {
        let oxc_resolver = oxc_resolver_real();
        for specifier in &symlink_specifiers {
            assert!(oxc_resolver.resolve(&symlink_test_dir, specifier).is_ok(), "{specifier}");
        }
    }

    let mut group = c.benchmark_group("resolver_real");

    group.bench_with_input(BenchmarkId::from_parameter("single-thread"), &data, |b, data| {
        let oxc_resolver = oxc_resolver_real();
        b.iter(|| {
            for (path, request) in data {
                _ = oxc_resolver.resolve(path, request);
            }
        });
    });

    group.bench_with_input(BenchmarkId::from_parameter("multi-thread"), &data, |b, data| {
        let oxc_resolver = oxc_resolver_real();
        b.iter(|| {
            data.par_iter().for_each(|(path, request)| {
                _ = oxc_resolver.resolve(path, request);
            });
        });
    });

    group.bench_with_input(
        BenchmarkId::from_parameter("resolve from symlinks"),
        &symlink_specifiers,
        |b, specifiers| {
            let oxc_resolver = oxc_resolver_real();
            b.iter(|| {
                for specifier in specifiers {
                    _ = oxc_resolver.resolve(&symlink_test_dir, specifier);
                }
            });
        },
    );
}

criterion_group!(resolver_native, bench_resolver_memory, bench_resolver_real);
criterion_main!(resolver_native);
