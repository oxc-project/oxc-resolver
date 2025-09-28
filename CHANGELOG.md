# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [11.8.4](https://github.com/oxc-project/oxc-resolver/compare/v11.8.3...v11.8.4) - 2025-09-28

### <!-- 1 -->🐛 Bug Fixes

- ensure canonicalized paths remain accessible via strong references ([#733](https://github.com/oxc-project/oxc-resolver/pull/733)) (by @Boshen) - #733

### <!-- 4 -->⚡ Performance

- mark error path functions as #[cold] for better optimization ([#729](https://github.com/oxc-project/oxc-resolver/pull/729)) (by @Boshen) - #729

### Contributors

* @Boshen

## [11.8.3](https://github.com/oxc-project/oxc-resolver/compare/v11.8.2...v11.8.3) - 2025-09-23

### <!-- 1 -->🐛 Bug Fixes

- use `Weak` references for `CachedPath` to enable proper drop ([#727](https://github.com/oxc-project/oxc-resolver/pull/727)) (by @Boshen) - #727

### <!-- 2 -->🚜 Refactor

- remove a redundant path clone from PackageJson::parse ([#725](https://github.com/oxc-project/oxc-resolver/pull/725)) (by @Boshen) - #725
- split src/cache.rs into logical modules ([#714](https://github.com/oxc-project/oxc-resolver/pull/714)) (by @Boshen) - #714

### <!-- 6 -->🧪 Testing

- add memory leak test ([#726](https://github.com/oxc-project/oxc-resolver/pull/726)) (by @Boshen) - #726

### Contributors

* @Boshen
* @renovate[bot]

## [11.8.2](https://github.com/oxc-project/oxc-resolver/compare/v11.8.1...v11.8.2) - 2025-09-18

### <!-- 4 -->⚡ Performance

- bypass file system read cache if memory cache is available ([#707](https://github.com/oxc-project/oxc-resolver/pull/707)) (by @Brooooooklyn) - #707

### <!-- 6 -->🧪 Testing

- enable Windows global pnp case ([#703](https://github.com/oxc-project/oxc-resolver/pull/703)) (by @JounQin) - #703

### Contributors

* @Brooooooklyn
* @JounQin

## [11.8.1](https://github.com/oxc-project/oxc-resolver/compare/v11.8.0...v11.8.1) - 2025-09-16

### <!-- 9 -->💼 Other

- Revert "perf: use `memmap` to speed up file reading" ([#701](https://github.com/oxc-project/oxc-resolver/pull/701)) (by @Boshen) - #701

### Contributors

* @Boshen

## [11.8.0](https://github.com/oxc-project/oxc-resolver/compare/v11.7.2...v11.8.0) - 2025-09-15

### <!-- 0 -->🚀 Features

- add benchmark for package.json deserialization ([#698](https://github.com/oxc-project/oxc-resolver/pull/698)) (by @Boshen) - #698

### <!-- 4 -->⚡ Performance

- use `memmap` to speed up file reading ([#696](https://github.com/oxc-project/oxc-resolver/pull/696)) (by @Boshen) - #696

### Contributors

* @Boshen
* @renovate[bot]

## [11.7.2](https://github.com/oxc-project/oxc-resolver/compare/v11.7.1...v11.7.2) - 2025-09-12

### <!-- 4 -->⚡ Performance

- use `GetFileAttributesExW` for symlink metadata lookup on Windows ([#691](https://github.com/oxc-project/oxc-resolver/pull/691)) (by @sapphi-red) - #691

### Contributors

* @sapphi-red
* @renovate[bot]

## [11.7.0](https://github.com/oxc-project/oxc-resolver/compare/v11.6.2...v11.7.0) - 2025-08-25

### <!-- 0 -->🚀 Features

- *(tsconfig)* support `files` / `include` / `exclude` ([#659](https://github.com/oxc-project/oxc-resolver/pull/659)) (by @shulaoda)
- feat(tsconfig) support `allowJs` in `compilerOptions` ([#658](https://github.com/oxc-project/oxc-resolver/pull/658)) (by @shulaoda) - #658
- *(tsconfig)* complete inheritance of `compilerOptions` fields ([#657](https://github.com/oxc-project/oxc-resolver/pull/657)) (by @shulaoda)

### <!-- 1 -->🐛 Bug Fixes

- *(tsconfig)* respect Yarn PnP when resolving `extends` paths ([#656](https://github.com/oxc-project/oxc-resolver/pull/656)) (by @shulaoda)

### <!-- 6 -->🧪 Testing

- *(tsconfig)* tweak jsx `extends` tests ([#666](https://github.com/oxc-project/oxc-resolver/pull/666)) (by @shulaoda)

### <!-- 9 -->💼 Other

- Add comprehensive tests for tsconfig extends functionality ([#660](https://github.com/oxc-project/oxc-resolver/pull/660)) (by @Copilot) - #660

### Contributors

* @shulaoda
* @renovate[bot]
* @Copilot

## [11.6.2](https://github.com/oxc-project/oxc-resolver/compare/v11.6.1...v11.6.2) - 2025-08-20

### <!-- 1 -->🐛 Bug Fixes

- allow resolving `package?query#fragment` for packages with exports field ([#655](https://github.com/oxc-project/oxc-resolver/pull/655)) (by @sapphi-red) - #655

### <!-- 4 -->⚡ Performance

- improve `pattern_key_compare` ([#639](https://github.com/oxc-project/oxc-resolver/pull/639)) (by @Boshen) - #639
- most specifiers don't have escaped characters ([#636](https://github.com/oxc-project/oxc-resolver/pull/636)) (by @Boshen) - #636

### <!-- 6 -->🧪 Testing

- make tests pass on Windows ([#654](https://github.com/oxc-project/oxc-resolver/pull/654)) (by @sapphi-red) - #654

### Contributors

* @sapphi-red
* @renovate[bot]
* @Boshen

## [11.6.0](https://github.com/oxc-project/oxc-resolver/compare/v11.5.2...v11.6.0) - 2025-07-18

### <!-- 0 -->🚀 Features

- support pass closure to restriction ([#604](https://github.com/oxc-project/oxc-resolver/pull/604)) (by @JounQin) - #604

### <!-- 1 -->🐛 Bug Fixes

- support for resolving empty tsconfig file ([#602](https://github.com/oxc-project/oxc-resolver/pull/602)) (by @JounQin) - #602

### <!-- 9 -->💼 Other

- Expose the `ExtendsField` enum of TsConfig ([#607](https://github.com/oxc-project/oxc-resolver/pull/607)) (by @ostenbom) - #607

### Contributors

* @Boshen
* @JounQin
* @renovate[bot]
* @ostenbom

## [11.5.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v11.4.0...oxc_resolver-v11.5.0) - 2025-07-06

### <!-- 0 -->🚀 Features

- return proper errors when failed to find or read yarn pnp manifest ([#590](https://github.com/oxc-project/oxc-resolver/pull/590)) (by @Boshen) - #590
- add `yarn_pnp` logic to `FileSystem` ([#589](https://github.com/oxc-project/oxc-resolver/pull/589)) (by @Boshen) - #589
- *(resolver)* rework yarn manifest file look up ([#586](https://github.com/oxc-project/oxc-resolver/pull/586)) (by @Boshen)

### <!-- 2 -->🚜 Refactor

- remove `fs_cache` feature flag ([#588](https://github.com/oxc-project/oxc-resolver/pull/588)) (by @Boshen) - #588

### Contributors

* @Boshen

## [11.4.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v11.3.0...oxc_resolver-v11.4.0) - 2025-07-01

### <!-- 0 -->🚀 Features

- bump `pnp` to 0.11.0 ([#577](https://github.com/oxc-project/oxc-resolver/pull/577)) (by @Boshen) - #577

### Contributors

* @Boshen

## [11.3.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v11.2.1...oxc_resolver-v11.3.0) - 2025-06-26

### <!-- 0 -->🚀 Features

- align yarn pnp implementation ([#576](https://github.com/oxc-project/oxc-resolver/pull/576)) (by @Boshen) - #576
- *(resolver)* allow `exports` field in `require('../directory')` ([#572](https://github.com/oxc-project/oxc-resolver/pull/572)) (by @Boshen)
- *(napi)* add `ResolveResult::builtin` information ([#575](https://github.com/oxc-project/oxc-resolver/pull/575)) (by @Boshen)

### <!-- 3 -->📚 Documentation

- document `allowPackageExportsInDirectoryResolve` (by @Boshen)

### Contributors

* @Boshen

## [11.2.1](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v11.2.0...oxc_resolver-v11.2.1) - 2025-06-23

### <!-- 1 -->🐛 Bug Fixes

- avoid crash when encountering tsconfig circular extends ([#570](https://github.com/oxc-project/oxc-resolver/pull/570))
- *(napi)* ensure `pnp_manifest` is included with `yarn_pnp` feature ([#555](https://github.com/oxc-project/oxc-resolver/pull/555))

### <!-- 10 -->💼 Other

- *(rust)* `debug = false` in `[profile.dev]` and `[profile.test]` ([#554](https://github.com/oxc-project/oxc-resolver/pull/554))

### <!-- 3 -->📚 Documentation

- add ts config
- update `alias` and `fallback` options type and description ([#557](https://github.com/oxc-project/oxc-resolver/pull/557))

## [11.2.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v11.1.0...oxc_resolver-v11.2.0) - 2025-06-05

### <!-- 0 -->Features

- *(napi)* add `tracing-subscriber` feature; turned on by default ([#546](https://github.com/oxc-project/oxc-resolver/pull/546))

## [11.1.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v11.0.0...oxc_resolver-v11.1.0) - 2025-06-01

### <!-- 0 -->Features

- support module type for TS files ([#538](https://github.com/oxc-project/oxc-resolver/pull/538))

## [11.0.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v10.0.0...oxc_resolver-v11.0.0) - 2025-05-28

### <!-- 0 -->Features

- implement module type resolution algorithm `ESM_FILE_FORMAT` from the spec ([#535](https://github.com/oxc-project/oxc-resolver/pull/535))

### <!-- 3 -->Documentation

- *(README)* clarify algorithm specification

### <!-- 7 -->Chore

- *(deps)* lock file maintenance rust crates ([#530](https://github.com/oxc-project/oxc-resolver/pull/530))

## [10.0.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v9.0.2...oxc_resolver-v10.0.0) - 2025-05-20

### <!-- 0 -->Features

- *(napi)* upgrade NAPI-RS to 3.0.0-beta.1 ([#525](https://github.com/oxc-project/oxc-resolver/pull/525))

### <!-- 4 -->Refactor

- [**breaking**] set clippy `avoid-breaking-exported-api = false` ([#519](https://github.com/oxc-project/oxc-resolver/pull/519))

### <!-- 7 -->Chore

- *(deps)* lock file maintenance ([#523](https://github.com/oxc-project/oxc-resolver/pull/523))
- *(deps)* update dependency rust to v1.87.0 ([#520](https://github.com/oxc-project/oxc-resolver/pull/520))
- sync napi cfg on global_allocator
- *(napi)* adjust mimalloc features ([#515](https://github.com/oxc-project/oxc-resolver/pull/515))

## [9.0.1](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v9.0.0...oxc_resolver-v9.0.1) - 2025-05-09

### <!-- 1 -->Bug Fixes

- oxc_resolver_napi dependency version

## [9.0.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v8.0.0...oxc_resolver-v9.0.0) - 2025-05-09

### <!-- 1 -->Bug Fixes

- hash import does not need to load from node_modules ([#501](https://github.com/oxc-project/oxc-resolver/pull/501))

### <!-- 7 -->Chore

- add `--tsconfig` to example ([#505](https://github.com/oxc-project/oxc-resolver/pull/505))
- publish `oxc_napi_resolver` ([#496](https://github.com/oxc-project/oxc-resolver/pull/496))

## [8.0.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v7.0.1...oxc_resolver-v8.0.0) - 2025-05-01

### <!-- 1 -->Bug Fixes

- fix incorrect return of package.json when a workspace module has `node_modules` ([#482](https://github.com/oxc-project/oxc-resolver/pull/482))

### <!-- 2 -->Performance

- cache whether a path is `node_modules` or inside `node_modules` ([#490](https://github.com/oxc-project/oxc-resolver/pull/490))
- remove a useless `load_as_directory` call ([#487](https://github.com/oxc-project/oxc-resolver/pull/487))

### <!-- 4 -->Refactor

- [**breaking**] remove `description_files` option ([#488](https://github.com/oxc-project/oxc-resolver/pull/488))
- [**breaking**] remove `modules` options ([#484](https://github.com/oxc-project/oxc-resolver/pull/484))

## [7.0.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v6.0.2...oxc_resolver-v7.0.0) - 2025-04-29

### <!-- 1 -->Bug Fixes

- return the enclosing `package.json` if it is inside `node_modules` ([#476](https://github.com/oxc-project/oxc-resolver/pull/476))

### <!-- 4 -->Refactor

- add `Debug` to `FsCachedPath` ([#478](https://github.com/oxc-project/oxc-resolver/pull/478))

## [6.0.1](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v6.0.0...oxc_resolver-v6.0.1) - 2025-04-25

### <!-- 1 -->Bug Fixes

- handle package.json and tsconfig.json with BOM ([#463](https://github.com/oxc-project/oxc-resolver/pull/463))

### <!-- 2 -->Performance

- avoid double call to `parse_package_specifier` ([#465](https://github.com/oxc-project/oxc-resolver/pull/465))

## [6.0.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v5.3.0...oxc_resolver-v6.0.0) - 2025-04-22

### <!-- 1 -->Bug Fixes

- properly handle DOS device paths in strip_windows_prefix ([#455](https://github.com/oxc-project/oxc-resolver/pull/455))

## [5.3.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v5.2.0...oxc_resolver-v5.3.0) - 2025-04-19

### <!-- 0 -->Features

- deserialize `preserve_value_imports` and `imports_not_used_as_values` from `compilerOptions` ([#457](https://github.com/oxc-project/oxc-resolver/pull/457))
- deserialize `target` from `compilerOptions` ([#456](https://github.com/oxc-project/oxc-resolver/pull/456))

## [5.2.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v5.1.1...oxc_resolver-v5.2.0) - 2025-04-08

### <!-- 1 -->Bug Fixes

- resolve `${configDir}` in tsconfig `compilerOptions.baseUrl` ([#450](https://github.com/oxc-project/oxc-resolver/pull/450))

## [5.1.1](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v5.1.0...oxc_resolver-v5.1.1) - 2025-04-04

### <!-- 1 -->Bug Fixes

- handle query and fragment in pacakge.json `exports` and `imports` field ([#443](https://github.com/oxc-project/oxc-resolver/pull/443))

## [5.1.0](https://github.com/oxc-project/oxc-resolver/compare/oxc_resolver-v5.0.1...oxc_resolver-v5.1.0) - 2025-04-02

### <!-- 0 -->Features

- resolve emitDecoratorMetadata in tsconfig ([#439](https://github.com/oxc-project/oxc-resolver/pull/439))

### <!-- 3 -->Documentation

- fix an incorrect comment on `Context::missing_dependencies`

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
