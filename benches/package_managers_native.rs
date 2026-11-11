use criterion::{Criterion, criterion_group, criterion_main};
use rayon::prelude::*;

#[path = "support/package_managers.rs"]
mod package_manager_bench;

fn bench_package_managers(c: &mut Criterion) {
    let _ = rayon::ThreadPoolBuilder::new().num_threads(4).build_global();
    package_manager_bench::bench_package_managers(c, "pm", |resolver, requests| {
        requests.par_iter().for_each(|request| {
            _ = resolver.resolve(&request.importer, request.specifier);
        });
    });
}

criterion_group!(package_managers_native, bench_package_managers);
criterion_main!(package_managers_native);
