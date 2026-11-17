# Benchmarks

`just benchmark` runs the resolver benchmarks. `just install-bench-fixtures`
followed by `just benchmark-pm` runs the eight package-manager layouts. CI runs the
same two targets with CodSpeed CPU simulation on `ubuntu-latest`.

The workloads control setup and execution order to reduce variance:

- Warm-cache benchmarks resolve their inputs before measurement, including the
  OS-backed and symlink cases.
- Four-worker benchmarks assign each worker a fixed subset of requests and warm
  each worker before measurement. Each measured dispatch resolves 32 rounds of
  the complete input set to amortize scheduling overhead.
- Cold package-manager requests run in a fixed serial order so cache insertion
  and allocation order do not depend on worker scheduling.
- The in-memory filesystem loads fixtures in sorted order and shares an immutable
  snapshot. `cold-resolve-and-drop` measures resolver creation, resolution, and
  destruction without copying the fixture filesystem.
- Package-manager fixtures install from frozen lockfiles. CI also pins the
  CodSpeed CLI and uses one CPU via `taskset`, fixing the CPU count DashMap uses
  to size the resolver's caches.

The changed workloads need a fresh baseline on `main`. Compare repeated runs of
the same commit when checking variance. CPU simulation excludes filesystem syscall
time; use local timings on the same idle machine to assess I/O and parallel
throughput. Compiler, runner-image, dependency, and CPU-feature changes can still
affect results.
