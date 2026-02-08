# Tsconfig Paths Benchmark Fixtures

These fixtures are used by `benches/resolver.rs` for the
`tsconfig_paths_aliases_memory/*` criterion benchmarks.

- `200/`: 200 wildcard `compilerOptions.paths` aliases

All aliases map to `src/shared/*` intentionally. The benchmark focuses on alias
matching/scanning cost rather than filesystem shape.
