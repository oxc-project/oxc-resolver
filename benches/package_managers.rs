use criterion::{Criterion, criterion_group, criterion_main};

#[path = "support/package_managers.rs"]
mod package_manager_bench;

fn bench_package_managers(c: &mut Criterion) {
    // Fixed request order keeps cold-cache insertion deterministic in CPU simulation.
    package_manager_bench::bench_package_managers(
        c,
        "pm-cold-single-thread",
        |resolver, requests| {
            for request in requests {
                _ = resolver.resolve(&request.importer, request.specifier);
            }
        },
    );
}

criterion_group!(package_managers, bench_package_managers);
criterion_main!(package_managers);
