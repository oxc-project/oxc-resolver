# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.1](https://github.com/web-infra-dev/rspack-resolver/compare/rspack_resolver-v0.5.0...rspack_resolver-v0.5.1) - 2025-02-11

### Fixed

- 🐛 pnp feat respect options.enable_pnp (#47)

## [0.4.0](https://github.com/web-infra-dev/rspack-resolver/compare/rspack_resolver-v0.3.6...rspack_resolver-v0.4.0) - 2024-12-26

### Feat

- Implements the PnP manifest lookup within the resolver ([#39](https://github.com/web-infra-dev/rspack-resolver/pull/39))

## [0.3.6](https://github.com/web-infra-dev/rspack-resolver/compare/rspack_resolver-v0.3.5...rspack_resolver-v0.3.6) - 2024-12-13

### Fixed

- alias match request end with slash (#35)

## [0.3.5](https://github.com/web-infra-dev/rspack-resolver/compare/rspack_resolver-v0.3.4...rspack_resolver-v0.3.5) - 2024-10-21

### Fixed

- resolve mathjs error when using `extensionAlias` ([#31](https://github.com/web-infra-dev/rspack-resolver/pull/31))

## [0.3.4](https://github.com/web-infra-dev/rspack-resolver/compare/rspack_resolver-v0.3.3...rspack_resolver-v0.3.4) - 2024-10-21

### Added

- rebase and refine extension-alias error format ([#30](https://github.com/web-infra-dev/rspack-resolver/pull/30))

## [1.12.0](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.11.0...oxc_resolver-v1.12.0) - 2024-09-25

### Added

- [**breaking**] mark `ResolveError` #[non_exhaustive] ([#252](https://github.com/oxc-project/oxc_resolver/pull/252))
- show tried extension aliases in `ResolveError::ExtensionAlias` ([#251](https://github.com/oxc-project/oxc_resolver/pull/251))
- give a specific error for matched alias not found ([#238](https://github.com/oxc-project/oxc_resolver/pull/238))

## [1.11.0](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.10.2...oxc_resolver-v1.11.0) - 2024-08-26

### Added
- use simdutf8 to validate UTF-8 when reading files ([#237](https://github.com/oxc-project/oxc_resolver/pull/237))
- Yarn PnP (behind a feature flag) ([#217](https://github.com/oxc-project/oxc_resolver/pull/217))

## [1.10.2](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.10.1...oxc_resolver-v1.10.2) - 2024-07-16

### Chore
- Release FreeBSD

## [1.10.1](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.10.0...oxc_resolver-v1.10.1) - 2024-07-15

### Fixed
- resolve module `ipaddr.js` correctly when `extensionAlias` is provided ([#228](https://github.com/oxc-project/oxc_resolver/pull/228))

## [1.10.0](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.9.4...oxc_resolver-v1.10.0) - 2024-07-11

### Added
- *(napi)* expose module type info in ResolveResult ([#223](https://github.com/oxc-project/oxc_resolver/pull/223))

### Fixed
- remove `#[cfg(target_os = "windows")]` logic in `canonicalize` ([#221](https://github.com/oxc-project/oxc_resolver/pull/221))

### Other
- update `cargo deny` ([#222](https://github.com/oxc-project/oxc_resolver/pull/222))
- pin crate-ci/typos version

## [1.9.4](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.9.3...oxc_resolver-v1.9.4) - 2024-07-10

### Other
- use custom canonicalize impl to avoid useless syscall ([#220](https://github.com/oxc-project/oxc_resolver/pull/220))
- add symlink fixtures ([#219](https://github.com/oxc-project/oxc_resolver/pull/219))

## [1.9.3](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v1.9.2...oxc_resolver-v1.9.3) - 2024-07-03

### Fixed
- tsconfig project reference it self should throw error ([#211](https://github.com/oxc-project/oxc-resolver/pull/211))

### Other
- *(napi)* make napi binary smaller with minimal tracing features ([#213](https://github.com/oxc-project/oxc-resolver/pull/213))
- *(napi)* remove tokio ([#212](https://github.com/oxc-project/oxc-resolver/pull/212))
- *(deps)* update rust crate dashmap to v6 ([#209](https://github.com/oxc-project/oxc-resolver/pull/209))

## [1.9.2](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.9.1...oxc_resolver-v1.9.2) - 2024-06-30

### Added
- *(napi)* add tracing via `OXC_LOG:DEBUG` ([#202](https://github.com/oxc-project/oxc_resolver/pull/202))

### Other
- document directory is an absolute path for `resolve(directory, specifier)` ([#206](https://github.com/oxc-project/oxc_resolver/pull/206))
- add a broken tsconfig test ([#205](https://github.com/oxc-project/oxc_resolver/pull/205))
- improve code coverage for src/error.rs ([#204](https://github.com/oxc-project/oxc_resolver/pull/204))
- skip resolving extension alias when `options.extension_alias` is empty ([#203](https://github.com/oxc-project/oxc_resolver/pull/203))
- add npm badge to crates.io

## [1.9.1](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.9.0...oxc_resolver-v1.9.1) - 2024-06-29

### Added
- strip symbols and enable LTO ([#197](https://github.com/oxc-project/oxc_resolver/pull/197))

### Other
- improve call to `Path::ends_with` ([#199](https://github.com/oxc-project/oxc_resolver/pull/199))
- list [profile.release] explicitly ([#198](https://github.com/oxc-project/oxc_resolver/pull/198))

## [1.9.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v1.8.4...oxc_resolver-v1.9.0) - 2024-06-28

### Added
- export package.json `type` and `sideEffects` field by default for bundlers ([#196](https://github.com/oxc-project/oxc-resolver/pull/196))

## [1.8.4](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.8.3...oxc_resolver-v1.8.4) - 2024-06-27

### Other
- skip searching for package.json when `alias_fields` is not provided ([#194](https://github.com/oxc-project/oxc_resolver/pull/194))

## [1.8.3](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.8.3...oxc_resolver-v1.8.2) - 2024-06-26

* *(napi*) release wasi build

## [1.8.2](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.8.2...oxc_resolver-v1.8.1) - 2024-06-24

### Added
- *(napi)* add async API ([#191](https://github.com/oxc-project/oxc_resolver/pull/191))

## [1.8.1](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.8.0...oxc_resolver-v1.8.1) - 2024-05-31

### Fixed
- alias value should try fragment as path ([#172](https://github.com/oxc-project/oxc_resolver/pull/172))

## [1.8.0](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.7.0...oxc_resolver-v1.8.0) - 2024-05-27

### Added
- [**breaking**] remove the constraint on packages exports `default` must be the last one ([#171](https://github.com/oxc-project/oxc_resolver/pull/171))
- [**breaking**] return `ResolveError:Builtin("node:{specifier}")` from package imports and exports ([#165](https://github.com/oxc-project/oxc_resolver/pull/165))

### Fixed
- alias not found should return error ([#168](https://github.com/oxc-project/oxc_resolver/pull/168))

### Other
- add panic test for extensions without a leading dot ([#150](https://github.com/oxc-project/oxc_resolver/pull/150))
- add test case for empty alias fields ([#149](https://github.com/oxc-project/oxc_resolver/pull/149))

## [1.7.0](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.6.7...oxc_resolver-v1.7.0) - 2024-04-24

### Added
- add `imports_fields` option ([#138](https://github.com/oxc-project/oxc_resolver/pull/138))
- substitute path that starts with `${configDir}/` in tsconfig.compilerOptions.paths ([#136](https://github.com/oxc-project/oxc_resolver/pull/136))

### Fixed
- RootsPlugin debug_assert on windows ([#145](https://github.com/oxc-project/oxc_resolver/pull/145))
- RootsPlugin should fall through if it fails to resolve the roots ([#144](https://github.com/oxc-project/oxc_resolver/pull/144))
- lazily read package.json.exports for shared resolvers ([#137](https://github.com/oxc-project/oxc_resolver/pull/137))

### Other
- remove `PartialEq` and `Eq` from `Specifier` ([#148](https://github.com/oxc-project/oxc_resolver/pull/148))
- add test case for tsconfig paths alias fall through ([#147](https://github.com/oxc-project/oxc_resolver/pull/147))
- use `cargo shear`
- fix test not failing the jobs property ([#146](https://github.com/oxc-project/oxc_resolver/pull/146))
- lazily read package.json.browser_fields for shared resolvers ([#142](https://github.com/oxc-project/oxc_resolver/pull/142))
- avoid an extra allocation in `load_extensions`
- ignore code coverage for `Display` on `ResolveOptions` ([#140](https://github.com/oxc-project/oxc_resolver/pull/140))
- remove the browser field lookup in `resolve_esm_match` ([#141](https://github.com/oxc-project/oxc_resolver/pull/141))
- remove the extra `condition_names` from `package_exports_resolve`

## [1.6.7](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.6.6...oxc_resolver-v1.6.7) - 2024-04-22

### Fixed
- incorrect resolution when using shared resolvers with different `main_fields` ([#134](https://github.com/oxc-project/oxc_resolver/pull/134))

## [1.6.6](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.6.5...oxc_resolver-v1.6.6) - 2024-04-22

### Other
- print resolve options while debug tracing ([#133](https://github.com/oxc-project/oxc_resolver/pull/133))

## [1.6.5](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.6.4...oxc_resolver-v1.6.5) - 2024-04-10

### Fixed
- canonicalize is not supported on wasi target ([#124](https://github.com/oxc-project/oxc_resolver/pull/124))

### Other
- document feature flags

## [1.6.4](https://github.com/oxc-project/oxc_resolver/compare/oxc_resolver-v1.6.3...oxc_resolver-v1.6.4) - 2024-03-29

### Docs

* improve terminology and clarify contexts
