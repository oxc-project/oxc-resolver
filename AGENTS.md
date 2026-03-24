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
│   └── tests/           # Unit tests (one file per feature area)
├── tests/               # Integration tests
├── fixtures/            # Test fixtures (real files on disk, not generated)
├── napi/               # Node.js NAPI bindings
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

Tests must use **fixture directories** with real files on disk. Do not dynamically create files, directories, or temp folders in tests — always add fixture files and commit them to the repository.

#### Where to put test code

- **Unit tests** (`src/tests/`): Test individual resolution features (aliases, extensions, exports, etc.). Each file maps to a feature area. Use `super::fixture_root()` to access `fixtures/`.
- **Integration tests** (`tests/`): Test end-to-end resolution behavior. Use `env::current_dir().unwrap().join("fixtures/integration")` to access fixtures.
- **Node.js tests** (`napi/`): Test the NAPI bindings with vitest.

#### Where to put fixtures

```
fixtures/
├── enhanced-resolve/      # Ported from webpack/enhanced-resolve (shared by many unit tests)
├── integration/           # Integration test fixtures (tests/ directory)
│   ├── misc/              # Unicode paths, BOM handling, package.json edge cases
│   ├── dot/               # Dot-path resolution
│   └── ...
├── dts_resolver/          # .d.ts resolution fixtures
├── invalid/               # Invalid configuration scenarios
├── pnp/                   # Yarn Plug'n'Play fixtures
├── pnpm/                  # pnpm node_modules structure
├── tsconfck/              # tsconfck compatibility
├── tsconfig/              # TypeScript config resolution
└── yarn/                  # Yarn monorepo fixtures
```

- Add new fixtures under the directory that matches the test file or feature area.
- For integration tests, add fixtures under `fixtures/integration/`.
- Ensure tests work on Windows, macOS, and Linux.

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

<!--VITE PLUS START-->

# Using Vite+, the Unified Toolchain for the Web

This project is using Vite+, a unified toolchain built on top of Vite, Rolldown, Vitest, tsdown, Oxlint, Oxfmt, and Vite Task. Vite+ wraps runtime management, package management, and frontend tooling in a single global CLI called `vp`. Vite+ is distinct from Vite, but it invokes Vite through `vp dev` and `vp build`.

## Vite+ Workflow

`vp` is a global binary that handles the full development lifecycle. Run `vp help` to print a list of commands and `vp <command> --help` for information about a specific command.

### Start

- create - Create a new project from a template
- migrate - Migrate an existing project to Vite+
- config - Configure hooks and agent integration
- staged - Run linters on staged files
- install (`i`) - Install dependencies
- env - Manage Node.js versions

### Develop

- dev - Run the development server
- check - Run format, lint, and TypeScript type checks
- lint - Lint code
- fmt - Format code
- test - Run tests

### Execute

- run - Run monorepo tasks
- exec - Execute a command from local `node_modules/.bin`
- dlx - Execute a package binary without installing it as a dependency
- cache - Manage the task cache

### Build

- build - Build for production
- pack - Build libraries
- preview - Preview production build

### Manage Dependencies

Vite+ automatically detects and wraps the underlying package manager such as pnpm, npm, or Yarn through the `packageManager` field in `package.json` or package manager-specific lockfiles.

- add - Add packages to dependencies
- remove (`rm`, `un`, `uninstall`) - Remove packages from dependencies
- update (`up`) - Update packages to latest versions
- dedupe - Deduplicate dependencies
- outdated - Check for outdated packages
- list (`ls`) - List installed packages
- why (`explain`) - Show why a package is installed
- info (`view`, `show`) - View package information from the registry
- link (`ln`) / unlink - Manage local package links
- pm - Forward a command to the package manager

### Maintain

- upgrade - Update `vp` itself to the latest version

These commands map to their corresponding tools. For example, `vp dev --port 3000` runs Vite's dev server and works the same as Vite. `vp test` runs JavaScript tests through the bundled Vitest. The version of all tools can be checked using `vp --version`. This is useful when researching documentation, features, and bugs.

## Common Pitfalls

- **Using the package manager directly:** Do not use pnpm, npm, or Yarn directly. Vite+ can handle all package manager operations.
- **Always use Vite commands to run tools:** Don't attempt to run `vp vitest` or `vp oxlint`. They do not exist. Use `vp test` and `vp lint` instead.
- **Running scripts:** Vite+ commands take precedence over `package.json` scripts. If there is a `test` script defined in `scripts` that conflicts with the built-in `vp test` command, run it using `vp run test`.
- **Do not install Vitest, Oxlint, Oxfmt, or tsdown directly:** Vite+ wraps these tools. They must not be installed directly. You cannot upgrade these tools by installing their latest versions. Always use Vite+ commands.
- **Use Vite+ wrappers for one-off binaries:** Use `vp dlx` instead of package-manager-specific `dlx`/`npx` commands.
- **Import JavaScript modules from `vite-plus`:** Instead of importing from `vite` or `vitest`, all modules should be imported from the project's `vite-plus` dependency. For example, `import { defineConfig } from 'vite-plus';` or `import { expect, test, vi } from 'vite-plus/test';`. You must not install `vitest` to import test utilities.
- **Type-Aware Linting:** There is no need to install `oxlint-tsgolint`, `vp lint --type-aware` works out of the box.

## Review Checklist for Agents

- [ ] Run `vp install` after pulling remote changes and before getting started.
- [ ] Run `vp check` and `vp test` to validate changes.
<!--VITE PLUS END-->
