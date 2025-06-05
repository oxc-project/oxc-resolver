# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.7.11](https://github.com/unrs/unrs-resolver/compare/v1.7.10...v1.7.11) - 2025-06-05

### <!-- 1 -->Bug Fixes

- remove invalid file cache due to extension alias ([#136](https://github.com/unrs/unrs-resolver/pull/136))

## [1.7.10](https://github.com/unrs/unrs-resolver/compare/v1.7.9...v1.7.10) - 2025-06-05

### <!-- 1 -->Bug Fixes

- Revert "fix: custom `condition_names` should take higher priority than target in package.json ([#115](https://github.com/unrs/unrs-resolver/pull/115))" ([#131](https://github.com/unrs/unrs-resolver/pull/131))

### Other

- chore: bump napi v3.0.0-beta.7 ([#133](https://github.com/unrs/unrs-resolver/pull/133))

## [1.7.9](https://github.com/unrs/unrs-resolver/compare/v1.7.8...v1.7.9) - 2025-06-03

### <!-- 1 -->Bug Fixes

- pnp global cache should be supported ([#129](https://github.com/unrs/unrs-resolver/pull/129))

  Windows global cache is still unsupported due to upstream

  see also <https://github.com/yarnpkg/pnp-rs/pull/10>

## [1.7.8](https://github.com/unrs/unrs-resolver/compare/v1.7.7...v1.7.8) - 2025-05-29

### <!-- 1 -->Bug Fixes

- resolve symlink with nested `node_modules` ([#125](https://github.com/unrs/unrs-resolver/pull/125))

## [1.7.7](https://github.com/unrs/unrs-resolver/compare/v1.7.6...v1.7.7) - 2025-05-29

### <!-- 1 -->Bug Fixes

- resolve dir index with dot specifier correctly ([#123](https://github.com/unrs/unrs-resolver/pull/123))

## [1.7.6](https://github.com/unrs/unrs-resolver/compare/v1.7.5...v1.7.6) - 2025-05-28

### <!-- 1 -->Bug Fixes

- prefer index over current file for `.` and `./` ([#121](https://github.com/unrs/unrs-resolver/pull/121))

## [1.7.5](https://github.com/unrs/unrs-resolver/compare/v1.7.4...v1.7.5) - 2025-05-28

### <!-- 1 -->Bug Fixes

- should try package exports first per spec ([#118](https://github.com/unrs/unrs-resolver/pull/118))

## [1.7.4](https://github.com/unrs/unrs-resolver/compare/v1.7.3...v1.7.4) - 2025-05-28

### <!-- 1 -->Bug Fixes

- prefer file over package dir in `node_modules` ([#116](https://github.com/unrs/unrs-resolver/pull/116))

## [1.7.3](https://github.com/unrs/unrs-resolver/compare/v1.7.2...v1.7.3) - 2025-05-28

### <!-- 1 -->Bug Fixes

- custom `condition_names` should take higher priority than target in package.json ([#115](https://github.com/unrs/unrs-resolver/pull/115))

## [1.7.2](https://github.com/unrs/unrs-resolver/compare/v1.7.1...v1.7.2) - 2025-04-27

### <!-- 1 -->Bug Fixes

- bump `napi-postinstall` to fix `yarn` pnp compatibility issue ([#106](https://github.com/unrs/unrs-resolver/pull/106))

## [1.7.1](https://github.com/unrs/unrs-resolver/compare/v1.7.0...v1.7.1) - 2025-04-26

### Chore

- bump `napi-postinstall` to support `yarn`/`pnpm` on `webcontainer` ([#103](https://github.com/unrs/unrs-resolver/pull/103))

### <!-- 6 -->Testing

- add case for #65 ([#100](https://github.com/unrs/unrs-resolver/pull/100))

## [1.7.0](https://github.com/unrs/unrs-resolver/compare/v1.6.6...v1.7.0) - 2025-04-24

### <!-- 0 -->Features

- enable `no_opt_arch` flag for `mimalloc-safe` on `linux-aarch64` ([#98](https://github.com/unrs/unrs-resolver/pull/98))

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
- _(napi)_ add mimalloc ([#423](https://github.com/oxc-project/oxc-resolver/pull/423))
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
- _(napi)_ update buggy NAPI-RS versions ([#225](https://github.com/oxc-project/oxc-resolver/pull/225))
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
- _(README)_ should be `new ResolverFactory`

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

- _(pnp)_ support `pnpapi` core module and package deep link ([#24](https://github.com/unrs/unrs-resolver/pull/24))

## [1.2.0](https://github.com/unrs/unrs-resolver/compare/unrspack-resolver-v1.1.2...unrspack-resolver-v2.0.0) - 2025-03-18

### <!-- 0 -->Features

- _(napi)_ add mimalloc ([#423](https://github.com/unrs/unrs-resolver/pull/423)) ([#18](https://github.com/unrs/unrs-resolver/pull/18))
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

## Old Changelog for `oxc-resolver`

[CHANGELOG_OLD](CHANGELOG_OLD.md)
