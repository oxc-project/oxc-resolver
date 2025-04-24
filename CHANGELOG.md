# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.6.6](https://github.com/unrs/unrs-resolver/compare/v1.6.5...v1.6.6) - 2025-04-23

### <!-- 0 -->Features

- add new target `riscv64gc-unknown-linux-musl` ([#96](https://github.com/unrs/unrs-resolver/pull/96))

## [1.6.5](https://github.com/unrs/unrs-resolver/compare/v1.6.4...v1.6.5) - 2025-04-23

### <!-- 1 -->Bug Fixes

- rework on handling DOS device paths on Windows ([#84](https://github.com/unrs/unrs-resolver/pull/84))
- handle package.json and tsconfig.json with BOM ([#463](https://github.com/oxc-project/oxc-resolver/pull/463))

### <!-- 2 -->Performance

- avoid double call to `parse_package_specifier` ([#465](https://github.com/oxc-project/oxc-resolver/pull/465))

### <!-- 3 -->Documentation

- add more details about the changes in this fork ([#92](https://github.com/unrs/unrs-resolver/pull/92))

## [1.6.4](https://github.com/unrs/unrs-resolver/compare/v1.6.3...v1.6.4) - 2025-04-22

### <!-- 1 -->Bug Fixes

- properly handle DOS device paths in strip_windows_prefix ([#455](https://github.com/oxc-project/oxc-resolver/pull/455))

## [1.6.3](https://github.com/unrs/unrs-resolver/compare/v1.6.2...v1.6.3) - 2025-04-21

### <!-- 1 -->Bug Fixes

- support `load_as_directory` for pnp mode ([#75](https://github.com/unrs/unrs-resolver/pull/75))

### <!-- 6 -->Testing

- add case for import-js/eslint-import-resolver-typescript#429 ([#76](https://github.com/unrs/unrs-resolver/pull/76))

## [1.6.2](https://github.com/unrs/unrs-resolver/compare/v1.6.1...v1.6.2) - 2025-04-21

### <!-- 1 -->Bug Fixes

- resolve parent base url correctly by normalizing as absolute path ([#72](https://github.com/unrs/unrs-resolver/pull/72))

## [1.6.1](https://github.com/unrs/unrs-resolver/compare/v1.6.0...v1.6.1) - 2025-04-20

### <!-- 1 -->Bug Fixes

- disable `mimalloc` on linux with aarch64 ([#69](https://github.com/unrs/unrs-resolver/pull/69))

## [1.6.0](https://github.com/unrs/unrs-resolver/compare/v1.5.0...v1.6.0) - 2025-04-20

### <!-- 0 -->Features

- deserialize `preserve_value_imports` and `imports_not_used_as_values` from `compilerOptions` ([#457](https://github.com/oxc-project/oxc-resolver/pull/457))
- deserialize `target` from `compilerOptions` ([#456](https://github.com/oxc-project/oxc-resolver/pull/456))

### <!-- 1 -->Bug Fixes

- add `napi-postinstall` dep for workaround `npm`'s bug ([#66](https://github.com/unrs/unrs-resolver/pull/66))

## [1.5.0](https://github.com/unrs/unrs-resolver/compare/v1.4.1...v1.5.0) - 2025-04-11

### <!-- 1 -->Bug Fixes

- resolve `${configDir}` in tsconfig `compilerOptions.baseUrl` ([#450](https://github.com/oxc-project/oxc-resolver/pull/450))

## [1.4.1](https://github.com/unrs/unrs-resolver/compare/v1.4.0...v1.4.1) - 2025-04-07

### <!-- 4 -->Refactor

- remove unnecessary checks for query ([#53](https://github.com/unrs/unrs-resolver/pull/53))

## [1.4.0](https://github.com/unrs/unrs-resolver/compare/v1.3.3...v1.4.0) - 2025-04-06

### <!-- 0 -->Features

- handle query and fragment in pacakge.json `exports` and `imports` field ([#443](https://github.com/oxc-project/oxc-resolver/pull/443))
- resolve emitDecoratorMetadata in tsconfig ([#439](https://github.com/oxc-project/oxc-resolver/pull/439))
- *(napi)* add mimalloc ([#423](https://github.com/oxc-project/oxc-resolver/pull/423))
- [**breaking**] Rust Edition 2024 ([#402](https://github.com/oxc-project/oxc-resolver/pull/402))
- deserialize `verbatim_module_syntax` from compilerOptions ([#411](https://github.com/oxc-project/oxc-resolver/pull/411))
- support wildcard `*` in alias plugin ([#388](https://github.com/oxc-project/oxc-resolver/pull/388))
- merge options from extends tsconfig.json ([#375](https://github.com/oxc-project/oxc-resolver/pull/375))
- add more fields in tsconfig#CompilerOptionsSerde ([#374](https://github.com/oxc-project/oxc-resolver/pull/374))
- [**breaking**] generic fs cache `type Resolver = ResolverGeneric<FsCache<FileSystemOs>>` ([#358](https://github.com/oxc-project/oxc-resolver/pull/358))
- [**breaking**] replace `FileSystem::canonicalize` with `FileSystem::read_link` ([#331](https://github.com/oxc-project/oxc-resolver/pull/331))
- faster and stable path hash for the cache ([#328](https://github.com/oxc-project/oxc-resolver/pull/328))
- add `Resolver::resolve_tsconfig` API ([#312](https://github.com/oxc-project/oxc-resolver/pull/312))
- [**breaking**] add `ResolveError::Builtin::prefixed_with_node_colon` ([#272](https://github.com/oxc-project/oxc-resolver/pull/272))
- [**breaking**] mark `ResolveError` #[non_exhaustive] ([#252](https://github.com/oxc-project/oxc-resolver/pull/252))
- show tried extension aliases in `ResolveError::ExtensionAlias` ([#251](https://github.com/oxc-project/oxc-resolver/pull/251))
- give a specific error for matched alias not found ([#238](https://github.com/oxc-project/oxc-resolver/pull/238))
- Yarn PnP ([#217](https://github.com/oxc-project/oxc-resolver/pull/217))

### <!-- 1 -->Bug Fixes

- handle query and fragment in package.json `exports` and `imports` field ([#443](https://github.com/oxc-project/oxc-resolver/pull/443))
- fix bench
- try browsers field and alias before resolving directory in node_modules ([#349](https://github.com/oxc-project/oxc-resolver/pull/349))
- special case for aliasing `@/` ([#348](https://github.com/oxc-project/oxc-resolver/pull/348))
- normalize resolved result on Windows for root ([#345](https://github.com/oxc-project/oxc-resolver/pull/345))
- don't panic when resolving `/` with `roots` ([#310](https://github.com/oxc-project/oxc-resolver/pull/310))
- use same UNC path normalization logic with libuv ([#306](https://github.com/oxc-project/oxc-resolver/pull/306))
- use `fs::canonicalize` to cover symlink edge cases ([#284](https://github.com/oxc-project/oxc-resolver/pull/284))
- extensionAlias cannot resolve mathjs ([#273](https://github.com/oxc-project/oxc-resolver/pull/273))
- resolve module `ipaddr.js` correctly when `extensionAlias` is provided ([#228](https://github.com/oxc-project/oxc-resolver/pull/228))
- *(napi)* update buggy NAPI-RS versions ([#225](https://github.com/oxc-project/oxc-resolver/pull/225))
- remove `#[cfg(target_os = "windows")]` logic in `canonicalize` ([#221](https://github.com/oxc-project/oxc-resolver/pull/221))

### <!-- 2 -->Performance

- use papaya instead of dashmap ([#356](https://github.com/oxc-project/oxc-resolver/pull/356))
- try directory first in `node_modules` ([#340](https://github.com/oxc-project/oxc-resolver/pull/340))
- guard `load_alias` on hot path ([#339](https://github.com/oxc-project/oxc-resolver/pull/339))
- use `as_os_str` for `Hash` and `PartialEq` operations ([#338](https://github.com/oxc-project/oxc-resolver/pull/338))
- reduce hash while resolving package.json ([#319](https://github.com/oxc-project/oxc-resolver/pull/319))
- reduce memory allocation while normalizing package path ([#318](https://github.com/oxc-project/oxc-resolver/pull/318))
- reduce memory allocation while resolving package.json ([#317](https://github.com/oxc-project/oxc-resolver/pull/317))
- use `path.as_os_str().hash()` instead of `path.hash()` ([#316](https://github.com/oxc-project/oxc-resolver/pull/316))
- reduce memory allocation by using a thread_local path for path methods ([#315](https://github.com/oxc-project/oxc-resolver/pull/315))
- bring back the symlink optimization ([#298](https://github.com/oxc-project/oxc-resolver/pull/298))
- use simdutf8 to validate UTF-8 when reading files ([#237](https://github.com/oxc-project/oxc-resolver/pull/237))
- use custom canonicalize impl to avoid useless syscall ([#220](https://github.com/oxc-project/oxc-resolver/pull/220))

### <!-- 3 -->Documentation

- fix an incorrect comment on `Context::missing_dependencies`
- mention extension must start with a `.` in `with_extension` ([#313](https://github.com/oxc-project/oxc-resolver/pull/313))
- *(README)* should be `new ResolverFactory`

### <!-- 4 -->Refactor

- remove papaya `.collector(seize::Collector::new())` call ([#393](https://github.com/oxc-project/oxc-resolver/pull/393))
- replace UnsafeCell with RefCell ([#346](https://github.com/oxc-project/oxc-resolver/pull/346))
- run clippy with `--all-targets` ([#333](https://github.com/oxc-project/oxc-resolver/pull/333))
- apply latest `cargo +nightly fmt` ([#281](https://github.com/oxc-project/oxc-resolver/pull/281))
- add more clippy fixes ([#279](https://github.com/oxc-project/oxc-resolver/pull/279))
- clean up elided lifetimes ([#277](https://github.com/oxc-project/oxc-resolver/pull/277))

### <!-- 6 -->Testing

- fix warning on Windows
- fix symlink test init on Windows ([#307](https://github.com/oxc-project/oxc-resolver/pull/307))

## [1.3.3](https://github.com/unrs/unrs-resolver/compare/v1.3.2...v1.3.3) - 2025-03-29

### Build

- build: remove `--strip` flag ([#44](https://github.com/unrs/unrs-resolver/pull/44))

### <!-- 6 -->Testing

- add nested package json case ([#40](https://github.com/unrs/unrs-resolver/pull/40))

## [1.3.2](https://github.com/unrs/unrs-resolver/compare/unrs_resolver-v1.3.1...v1.3.2) - 2025-03-26

### <!-- 1 -->Bug Fixes

- absolute path aliasing should not be skipped ([#37](https://github.com/unrs/unrs-resolver/pull/37))

## [1.3.1](https://github.com/unrs/unrs-resolver/compare/unrs_resolver-v1.3.0...unrs_resolver-v1.3.1) - 2025-03-26

### Other

- bump all (dev) deps ([#34](https://github.com/unrs/unrs-resolver/pull/34))

## [1.3.0](https://github.com/unrs/unrs-resolver/compare/unrspack-resolver-v1.2.2...unrs_resolver-v1.3.0) - 2025-03-26

### <!-- 0 -->Features

- enable more targets ([#29](https://github.com/unrs/unrs-resolver/pull/29) and [#32](https://github.com/unrs/unrs-resolver/pull/32))

## [1.2.2](https://github.com/unrs/unrs-resolver/compare/unrspack-resolver-v1.2.1...unrspack-resolver-v1.2.2) - 2025-03-19

### <!-- 1 -->Bug Fixes

- *(pnp)* support `pnpapi` core module and package deep link ([#24](https://github.com/unrs/unrs-resolver/pull/24))

## [1.2.0](https://github.com/unrs/unrs-resolver/compare/unrspack-resolver-v1.1.2...unrspack-resolver-v2.0.0) - 2025-03-18

### <!-- 0 -->Features

- *(napi)* add mimalloc ([#423](https://github.com/unrs/unrs-resolver/pull/423)) ([#18](https://github.com/unrs/unrs-resolver/pull/18))
- merge from upstream `oxc-project/oxc-resolver` ([#15](https://github.com/unrs/unrs-resolver/pull/15))

## [1.1.2](https://github.com/unrs/unrs-resolver/compare/unrspack-resolver-v1.1.1...unrspack-resolver-v1.1.2) - 2025-03-16

### Fixed

- references should take higher priority ([#13](https://github.com/unrs/unrs-resolver/pull/13))
- takes paths and references into account at the same time
- should always try resolve_path_alias

## [1.1.1](https://github.com/unrs/unrs-resolver/compare/unrspack-resolver-v1.1.0...unrspack-resolver-v1.1.1) - 2025-03-16

### Other

- bump all (dev) deps
- bump to edition 2024

## [1.1.0](https://github.com/unrs/unrs-resolver/compare/unrspack-resolver-v1.0.0...unrspack-resolver-v1.1.0) - 2025-03-15

### Added

- support resolving path with extra query ([#7](https://github.com/unrs/unrs-resolver/pull/7))

## [1.0.0](https://github.com/unrs/unrs-resolver/releases/tag/unrspack-resolver-v1.0.0) - 2025-03-15

---

## [5.0.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v4.2.0...oxc_resolver-v5.0.0) - 2025-03-07

### <!-- 0 -->Features

- [**breaking**] Rust Edition 2024 ([#402](https://github.com/oxc-project/oxc-resolver/pull/402))
- deserialize `verbatim_module_syntax` from compilerOptions ([#411](https://github.com/oxc-project/oxc-resolver/pull/411))

### <!-- 4 -->Refactor

- remove papaya `.collector(seize::Collector::new())` call ([#393](https://github.com/oxc-project/oxc-resolver/pull/393))

## [4.2.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v4.1.0...oxc_resolver-v4.2.0) - 2025-02-19

### <!-- 0 -->Features

- support wildcard `*` in alias plugin (#388)

## [4.1.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v4.0.1...oxc_resolver-v4.1.0) - 2025-02-14

### <!-- 0 -->Features

- merge options from extends tsconfig.json (#375)
- add more fields in tsconfig#CompilerOptionsSerde (#374)

### <!-- 1 -->Bug Fixes

- fix bench

## [4.0.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v3.0.3...oxc_resolver-v4.0.0) - 2025-01-20

### <!-- 0 -->Features

- [**breaking**] generic fs cache `type Resolver = ResolverGeneric<FsCache<FileSystemOs>>` (#358)
- [**breaking**] `PackageJson` and `TsConfig` traits (#360)

### <!-- 2 -->Performance

- use papaya instead of dashmap (#356)

## [3.0.3](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v3.0.2...oxc_resolver-v3.0.3) - 2024-12-14

### <!-- 1 -->Bug Fixes

- try browsers field and alias before resolving directory in node_modules (#349)

## [3.0.2](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v3.0.1...oxc_resolver-v3.0.2) - 2024-12-13

### <!-- 1 -->Bug Fixes

- special case for aliasing `@/` (#348)
- normalize resolved result on Windows for root (#345)

### <!-- 4 -->Refactor

- replace UnsafeCell with RefCell (#346)

## [3.0.1](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v3.0.0...oxc_resolver-v3.0.1) - 2024-12-12

### <!-- 2 -->Performance

- try directory first in `node_modules` (#340)

## [3.0.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v2.1.1...oxc_resolver-v3.0.0) - 2024-12-11

### Added

- [**breaking**] replace `FileSystem::canonicalize` with `FileSystem::read_link` (#331)

### Other

- guard `load_alias` on hot path (#339)

## [2.1.1](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v2.1.0...oxc_resolver-v2.1.1) - 2024-11-22

### Performance

- reduce hash while resolving package.json ([#319](https://github.com/oxc-project/oxc-resolver/pull/319))
- reduce memory allocation while normalizing package path ([#318](https://github.com/oxc-project/oxc-resolver/pull/318))
- reduce memory allocation while resolving package.json ([#317](https://github.com/oxc-project/oxc-resolver/pull/317))
- use `path.as_os_str().hash()` instead of `path.hash()` ([#316](https://github.com/oxc-project/oxc-resolver/pull/316))
- reduce memory allocation by using a thread_local path for path methods ([#315](https://github.com/oxc-project/oxc-resolver/pull/315))

### Other

- remove the deprecated simdutf8 aarch64_neon feature
- mention extension must start with a `.` in `with_extension` ([#313](https://github.com/oxc-project/oxc-resolver/pull/313))

## [2.1.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v2.0.1...oxc_resolver-v2.1.0) - 2024-11-20

### Added

- add `Resolver::resolve_tsconfig` API ([#312](https://github.com/oxc-project/oxc-resolver/pull/312))

### Fixed

- don't panic when resolving `/` with `roots` ([#310](https://github.com/oxc-project/oxc-resolver/pull/310))
- use same UNC path normalization logic with libuv ([#306](https://github.com/oxc-project/oxc-resolver/pull/306))

### Other

- *(deps)* update rust crates to v1.0.215
- fix symlink test init on windows ([#307](https://github.com/oxc-project/oxc-resolver/pull/307))

## [2.0.1](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v2.0.0...oxc_resolver-v2.0.1) - 2024-11-08

### Other

- `cargo upgrade` && `pnpm upgrade`
- bring back the symlink optimization ([#298](https://github.com/oxc-project/oxc-resolver/pull/298))
- *(deps)* update rust crate criterion2 to v2

## [2.0.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v1.12.0...oxc_resolver-v2.0.0) - 2024-10-22

### Added

- [**breaking**] add `add ResolveError::Builtin::is_runtime_module` ([#272](https://github.com/oxc-project/oxc-resolver/pull/272))

### Fixed

- use `fs::canonicalize` to cover symlink edge cases ([#284](https://github.com/oxc-project/oxc-resolver/pull/284))
- extensionAlias cannot resolve mathjs ([#273](https://github.com/oxc-project/oxc-resolver/pull/273))

---

## [0.5.2](https://github.com/web-infra-dev/rspack-resolver/compare/rspack_resolver-v0.5.1...rspack_resolver-v0.5.2) - 2025-02-28

### Added

- *(pnp)* support link (#49)

### Other

- bump pnp 0.9.1 (#50)

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
