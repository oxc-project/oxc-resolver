# Tsconfig Extends Benchmark Fixtures

These fixtures are used by `benches/resolver.rs` for the
`tsconfig_extends_shared_paths_memory/*` criterion benchmarks.

- `tsconfig.base.json`: 512 wildcard `compilerOptions.paths` aliases, all mapping to
  `src/shared/*`.
- `packages/pkg-00` ... `packages/pkg-31`: tsconfigs that only `extends` the base,
  like the per-package tsconfigs of a large monorepo.

The benchmark resolves an alias from every package with a cold cache, so it measures loading
tsconfigs that inherit one large `paths` map.
