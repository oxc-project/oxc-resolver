# Benchmarks

`just benchmark` runs native resolver benchmarks. `just install-bench-fixtures`
followed by `just benchmark-pm` also measures the eight package-manager layouts.
Native runs include OS filesystem and four-worker parallel throughput benchmarks;
compare these on the same idle machine.

CI selects the `resolver` and `package_managers` targets for CodSpeed CPU simulation.
The `resolver_native` and `package_managers_native` targets contain OS-backed resolver
and parallel workloads, and are included by the local commands above. Thread scheduling
changes cache insertion order even with a fixed worker count, and simulation does
not measure filesystem syscall time, so `resolver_real` and parallel workloads are
kept out of simulation. Package-manager layouts still exercise OS-backed cold
resolution, but requests run in a fixed order; their simulated CPU cost does not
represent I/O latency.

- `resolver_memory/single-thread`, symlink resolution, and tsconfig discovery use
  explicitly warmed resolver caches. The tsconfig alias benchmark also warms each
  request before measurement.
- `resolver_memory/cold-resolve-and-drop` includes resolver construction, cold
  resolution, and destruction. Fixture contents are shared through an immutable
  snapshot loaded in sorted order, outside measurement.
- `pm-cold-single-thread/*` includes resolver construction, cold resolution of the
  16 requests, and destruction, including PnP parsing and package-store discovery.
- JSON parsing prepares input bytes outside measurement and includes parsing and
  destruction of the parsed result.

CI uses `ubuntu-latest`, pins the CodSpeed CLI, and runs on one CPU using `taskset`.
This also fixes `available_parallelism()`, which DashMap uses to choose its shard
count.

These workload changes require a new baseline on `main`. Compare subsequent runs
with that baseline, and rerun the same commit to check repeatability.
Dependency, compiler, runner-image, or CPU-feature changes can still affect the
results. Changes to benchmark fixtures should be assessed separately from resolver
performance changes.
