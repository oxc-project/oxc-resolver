# AI Agent Guidelines for oxc-resolver

This document provides guidance for AI coding assistants (like GitHub Copilot, Cursor, Claude, etc.) when working with the oxc-resolver codebase.

## Project Overview

oxc-resolver is a Rust port of webpack's enhanced-resolve, providing ESM and CommonJS module resolution. It offers both a Rust crate and Node.js bindings via NAPI.

### Key Technologies

- **Rust**: Core implementation using Rust 2024 edition (MSRV: 1.85.0)
- **NAPI**: Node.js bindings for JavaScript/TypeScript usage
- **WebAssembly**: Browser support
- **GitHub Actions**: CI/CD workflows

## Architecture

```
oxc-resolver/
├── src/                 # Core Rust implementation
├── napi/               # Node.js NAPI bindings
├── tests/              # Test fixtures and data
├── examples/           # Usage examples
├── benches/            # Performance benchmarks
└── .github/            # GitHub workflows and configs
```

## Development Workflow

`just init` has already been run, all tools (`typos-cli`, `cargo-shear`) are already installed, do not run `just init`.

Rust and `cargo` components `clippy`, `rust-docs` and `rustfmt` have already been installed, do not install them.

Always run `just ready` as the last step after code has been committed to the repository.

### Common Tasks

```bash
just ready     # Run all checks (format, lint, test, build)
just test      # Run all tests (Rust + Node.js)
just check     # Cargo check with all features
just lint      # Run clippy with strict settings
just fmt       # Format code (cargo fmt + oxfmt)
```

## Code Conventions

### Rust

- Use Rust 2024 edition features
- Follow standard Rust formatting (`cargo fmt`)
- All clippy warnings must be addressed (`cargo clippy -- --deny warnings`)
- Use `tracing` for logging/instrumentation
- Implement `FileSystem` trait for custom file systems

### Node.js/TypeScript

- Use TypeScript for type definitions (`index.d.ts`)
- Follow existing API patterns in NAPI bindings
- Use vitest for testing
- Support both ESM and CommonJS usage

### Documentation

- Use rustdoc for Rust APIs
- Maintain TypeScript definitions for Node.js API
- Update README.md for significant changes
- Add examples for new features

## Key APIs

### Rust

```rust
use oxc_resolver::{ResolveOptions, Resolver};

let options = ResolveOptions::default();
let resolver = Resolver::new(options);
let resolution = resolver.resolve("/path/to/project", "./module");
```

### Node.js

```javascript
import resolve, { ResolverFactory } from "oxc-resolver";

// Simple resolve
const result = resolve.sync(process.cwd(), "./module");

// Advanced usage
const resolver = new ResolverFactory({
  conditionNames: ["node", "import"],
  extensions: [".js", ".ts", ".json"],
});
```

## Testing Strategy

### Test Categories

1. **Enhanced-resolve compatibility**: Tests ported from webpack/enhanced-resolve
2. **TypeScript support**: tsconfig-paths functionality
3. **Yarn Plug'n'Play** resolution
4. **Node.js compatibility**: ESM/CJS resolution behavior
5. **Performance**: Benchmarks against enhanced-resolve

### Adding Tests

- Add Rust tests in `src/tests/`
- Add Node.js tests in `napi/`
- Use existing fixtures in `tests/` directory
- Ensure tests work on Windows, macOS, and Linux

## Common Patterns

### Error Handling

```rust
// Use proper error types
use oxc_resolver::{ResolveError, ResolveErrorKind};

match resolver.resolve(path, specifier) {
    Ok(resolution) => { /* handle success */ },
    Err(ResolveError { kind: ResolveErrorKind::NotFound, .. }) => { /* handle not found */ },
    Err(err) => { /* handle other errors */ }
}
```

### Configuration

```rust
// Build options incrementally
let options = ResolveOptions {
    condition_names: vec!["node".to_string(), "import".to_string()],
    extensions: vec![".js".to_string(), ".ts".to_string()],
    main_fields: vec!["module".to_string(), "main".to_string()],
    ..Default::default()
};
```

## Performance Considerations

- Use `FileSystem` trait for custom file systems (including in-memory)
- Cache `Resolver` instances when possible
- Consider `fullySpecified: true` for better performance when extensions are known
- Profile with `cargo bench` for performance-critical changes

## Debugging

### Rust

```bash
# Enable tracing
RUST_LOG=oxc_resolver=debug cargo test
```

### Node.js

```bash
# Enable tracing
OXC_LOG=DEBUG node your_program.js
```

## Contributing Guidelines

1. **Small, focused changes**: Make minimal modifications
2. **Test coverage**: Add tests for new functionality
3. **Documentation**: Update docs for API changes
4. **Performance**: Run benchmarks for core changes
5. **Compatibility**: Maintain enhanced-resolve compatibility where possible

## AI Assistant Tips

- **Context awareness**: This is a low-level resolver, performance matters
- **Test compatibility**: Changes should not break existing enhanced-resolve compatibility
- **Cross-platform**: Consider Windows path handling differences
- **Memory usage**: Be mindful of allocations in hot paths
- **Error messages**: Provide helpful error messages that match Node.js behavior

## References

- [Enhanced Resolve](https://github.com/webpack/enhanced-resolve) - Original implementation
- [Node.js Module Resolution](https://nodejs.org/api/modules.html) - CommonJS algorithm
- [Node.js ESM Resolution](https://nodejs.org/api/esm.html#resolution-algorithm) - ESM algorithm
- [TypeScript Module Resolution](https://www.typescriptlang.org/docs/handbook/module-resolution.html)
