# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [11.17.1](https://github.com/oxc-project/oxc-resolver/compare/v11.17.0...v11.17.1) - 2026-02-08

### <!-- 4 -->⚡ Performance

- *(resolve)* reuse dot-prefixed subpath in exports/imports ([#1004](https://github.com/oxc-project/oxc-resolver/pull/1004)) (by @Boshen)
- *(cache)* remove package.json index arena indirection ([#1003](https://github.com/oxc-project/oxc-resolver/pull/1003)) (by @Boshen)
- *(tsconfig)* precompile wildcard path alias matcher ([#1001](https://github.com/oxc-project/oxc-resolver/pull/1001)) (by @Boshen)
- precompile alias match metadata in resolver hot path ([#999](https://github.com/oxc-project/oxc-resolver/pull/999)) (by @Boshen)

### <!-- 9 -->💼 Other

- add tsconfig paths alias scalability benchmark ([#1002](https://github.com/oxc-project/oxc-resolver/pull/1002)) (by @Boshen)

### Contributors

* @Boshen

## [11.17.0](https://github.com/oxc-project/oxc-resolver/compare/v11.16.4...v11.17.0) - 2026-01-29

### <!-- 0 -->🚀 Features

- allow access to inner data in error related types ([#990](https://github.com/oxc-project/oxc-resolver/pull/990)) (by @sapphi-red)

### Contributors

* @sapphi-red

## [11.16.4](https://github.com/oxc-project/oxc-resolver/compare/v11.16.3...v11.16.4) - 2026-01-23

### <!-- 1 -->🐛 Bug Fixes

- resolve clippy unnecessary_unwrap warning ([#980](https://github.com/oxc-project/oxc-resolver/pull/980)) (by @Boshen)

### Contributors

* @leegeunhyeok
* @Boshen

## [11.16.3](https://github.com/oxc-project/oxc-resolver/compare/v11.16.2...v11.16.3) - 2026-01-14

### <!-- 1 -->🐛 Bug Fixes

- prevent tsconfig cache pollution with separate raw and built caches ([#970](https://github.com/oxc-project/oxc-resolver/pull/970)) (by @Boshen)
- use `/fixtures` path for WASI target (by @Boshen)

### <!-- 3 -->📚 Documentation

- *(README.md)* update logo ([#968](https://github.com/oxc-project/oxc-resolver/pull/968)) (by @sapphi-red)

### <!-- 6 -->🧪 Testing

- use CARGO_MANIFEST_DIR instead `env::current_dir` (by @Boshen)
- fix skipped tests in options.test.mjs (by @Boshen)

### Contributors

* @Boshen
* @renovate[bot]
* @sapphi-red

## [11.16.1](https://github.com/oxc-project/oxc-resolver/compare/v11.16.0...v11.16.1) - 2025-12-24

### <!-- 1 -->🐛 Bug Fixes

- only recreate the cache when `yarn_pnp` is toggled ([#943](https://github.com/oxc-project/oxc-resolver/pull/943)) (by @Boshen)
- resolve absolute path to package with trailing slash ([#942](https://github.com/oxc-project/oxc-resolver/pull/942)) (by @Boshen)
- show yarn pnp errors as-is instead of NotFound error ([#939](https://github.com/oxc-project/oxc-resolver/pull/939)) (by @sapphi-red)

### <!-- 2 -->🚜 Refactor

- replace builtins.rs with nodejs-built-in-modules crate ([#940](https://github.com/oxc-project/oxc-resolver/pull/940)) (by @Boshen)

### Contributors

* @Boshen
* @renovate[bot]
* @sapphi-red

## [11.16.0](https://github.com/oxc-project/oxc-resolver/compare/v11.15.0...v11.16.0) - 2025-12-18

### <!-- 0 -->🚀 Features

- allow subpath imports that start with #/ ([#907](https://github.com/oxc-project/oxc-resolver/pull/907)) (by @Boshen)

### <!-- 1 -->🐛 Bug Fixes

- resolve solution tsconfig for auto discovered tsconfig ([#927](https://github.com/oxc-project/oxc-resolver/pull/927)) (by @Boshen)
- fix `clone_with_options` + `yarn_pnp: true` not working ([#916](https://github.com/oxc-project/oxc-resolver/pull/916)) (by @sapphi-red)

### <!-- 2 -->🚜 Refactor

- s/self.cache.as_ref()/&self.cache ([#910](https://github.com/oxc-project/oxc-resolver/pull/910)) (by @Boshen)

### <!-- 6 -->🧪 Testing

- Add a test to ensure NODEJS_BUILTINS is alphabetized. ([#926](https://github.com/oxc-project/oxc-resolver/pull/926)) (by @connorshea)
- add test cases for package imports starting with `#` or `#/` ([#905](https://github.com/oxc-project/oxc-resolver/pull/905)) (by @Boshen)

### Contributors

* @Boshen
* @connorshea
* @renovate[bot]
* @sapphi-red

## [11.15.0](https://github.com/oxc-project/oxc-resolver/compare/v11.14.2...v11.15.0) - 2025-12-04

### <!-- 0 -->🚀 Features

- [**breaking**] disallow manually passing a list of references to `TsconfigOptions::references` ([#902](https://github.com/oxc-project/oxc-resolver/pull/902)) (by @Boshen)
- support tsconfig `rootDirs` ([#885](https://github.com/oxc-project/oxc-resolver/pull/885)) (by @Boshen)
- *(napi)* expose resolve_file API as resolveFileSync and resolveFileAsync ([#900](https://github.com/oxc-project/oxc-resolver/pull/900)) (by @Boshen)

### <!-- 1 -->🐛 Bug Fixes

- fix resolution when resolving non-relative specifier on tsconfig baseUrl ([#903](https://github.com/oxc-project/oxc-resolver/pull/903)) (by @Boshen)
- add fallback for statx NOSYS error on old kernels ([#901](https://github.com/oxc-project/oxc-resolver/pull/901)) (by @Boshen)
- correct grammatical errors in documentation and comments ([#897](https://github.com/oxc-project/oxc-resolver/pull/897)) (by @Boshen)

### Contributors

* @Boshen
* @renovate[bot]

## [11.14.2](https://github.com/oxc-project/oxc-resolver/compare/v11.14.1...v11.14.2) - 2025-12-02

### <!-- 1 -->🐛 Bug Fixes

- resolve `node_modules/package/dir/foo.js` if `node_modules/package/dir/foo/` exists ([#896](https://github.com/oxc-project/oxc-resolver/pull/896)) (by @Boshen)

### <!-- 2 -->🚜 Refactor

- remove redundant PathBuf storage in CachedPath ([#891](https://github.com/oxc-project/oxc-resolver/pull/891)) (by @Boshen)

### Contributors

* @Boshen

## [11.14.1](https://github.com/oxc-project/oxc-resolver/compare/v11.14.0...v11.14.1) - 2025-11-28

### <!-- 1 -->🐛 Bug Fixes

- incorrect resolution when project reference extends a tsconfig without baseUrl ([#882](https://github.com/oxc-project/oxc-resolver/pull/882)) (by @Boshen) - #882
- store PathBuf with weak pointers to handle cache clearing ([#879](https://github.com/oxc-project/oxc-resolver/pull/879)) (by @Boshen) - #879

### <!-- 4 -->⚡ Performance

- remove an allocation from `CachedPath::module_directory` ([#880](https://github.com/oxc-project/oxc-resolver/pull/880)) (by @Boshen) - #880
- skip searching for node_modules/@scope/package.json ([#876](https://github.com/oxc-project/oxc-resolver/pull/876)) (by @Boshen) - #876
- remove the redundant `node_modules/package/index` cache value ([#875](https://github.com/oxc-project/oxc-resolver/pull/875)) (by @Boshen) - #875

### <!-- 6 -->🧪 Testing

- change all fixture directory names to dashed case ([#884](https://github.com/oxc-project/oxc-resolver/pull/884)) (by @Boshen) - #884

### Contributors

* @Boshen
* @sapphi-red

## [11.14.0](https://github.com/oxc-project/oxc-resolver/compare/v11.13.2...v11.14.0) - 2025-11-24

### <!-- 0 -->🚀 Features

- add `resolve_file` API for tsconfig auto discovery to work ([#860](https://github.com/oxc-project/oxc-resolver/pull/860)) (by @Boshen) - #860
- port tsconfck (find tsconfig files) ([#854](https://github.com/oxc-project/oxc-resolver/pull/854)) (by @Boshen) - #854
- add many.rs example for profiling resolver with many packages ([#836](https://github.com/oxc-project/oxc-resolver/pull/836)) (by @Boshen) - #836

### <!-- 1 -->🐛 Bug Fixes

- apply `conditionNames: ['node', 'import']` when resolving tsconfig extends ([#869](https://github.com/oxc-project/oxc-resolver/pull/869)) (by @Boshen) - #869
- do not resolve to `node_modules/pacakge/index` ([#849](https://github.com/oxc-project/oxc-resolver/pull/849)) (by @Boshen) - #849
- use std::fs::canonicalize as a fallback when canonicalize fails ([#835](https://github.com/oxc-project/oxc-resolver/pull/835)) (by @Boshen) - #835

### <!-- 2 -->🚜 Refactor

- remove the redundant `inner_resolver` from `load_pnp` ([#862](https://github.com/oxc-project/oxc-resolver/pull/862)) (by @Boshen) - #862
- improve `Debug` and `Display` for `CachedPath` ([#861](https://github.com/oxc-project/oxc-resolver/pull/861)) (by @Boshen) - #861
- too_many_arguments = "allow" ([#863](https://github.com/oxc-project/oxc-resolver/pull/863)) (by @Boshen) - #863
- change Tsconfig::parse to accept owned string; add replace_bom_with_whitespace ([#859](https://github.com/oxc-project/oxc-resolver/pull/859)) (by @Boshen) - #859
- remove the useless getters and setters from `CompilerOptions` ([#858](https://github.com/oxc-project/oxc-resolver/pull/858)) (by @Boshen) - #858
- add `Tsconfig:references_resolved` ([#856](https://github.com/oxc-project/oxc-resolver/pull/856)) (by @Boshen) - #856
- move tsconfig resolution related code to its own file ([#855](https://github.com/oxc-project/oxc-resolver/pull/855)) (by @Boshen) - #855
- use RwLock<Vec<Arc<PackageJson>> for package.json storage ([#838](https://github.com/oxc-project/oxc-resolver/pull/838)) (by @Boshen) - #838
- do not store is_symlink in CachedPathImpl ([#850](https://github.com/oxc-project/oxc-resolver/pull/850)) (by @Boshen) - #850
- remove CachedPathImpl::canonicaling ([#834](https://github.com/oxc-project/oxc-resolver/pull/834)) (by @Boshen) - #834

### <!-- 4 -->⚡ Performance

- cache all package.json resolutions for faster package.json lookup ([#853](https://github.com/oxc-project/oxc-resolver/pull/853)) (by @Boshen) - #853
- do not canonicalize the entry path ([#848](https://github.com/oxc-project/oxc-resolver/pull/848)) (by @Boshen) - #848
- remove Result from `CachedPathImpl::canonicalized` ([#847](https://github.com/oxc-project/oxc-resolver/pull/847)) (by @Boshen) - #847
- fast path for node_modules/package ([#839](https://github.com/oxc-project/oxc-resolver/pull/839)) (by @Boshen) - #839
- cache canonicalization results at every recursion level ([#843](https://github.com/oxc-project/oxc-resolver/pull/843)) (by @Boshen) - #843
- use IdentityHasher for visited set to avoid double hashing ([#837](https://github.com/oxc-project/oxc-resolver/pull/837)) (by @Boshen) - #837

### Contributors

* @Boshen
* @renovate[bot]

## [11.13.2](https://github.com/oxc-project/oxc-resolver/compare/v11.13.1...v11.13.2) - 2025-11-12

### <!-- 1 -->🐛 Bug Fixes

- remove AT_STATX_DONT_SYNC from statx calls ([#828](https://github.com/oxc-project/oxc-resolver/pull/828)) (by @Boshen) - #828

### <!-- 2 -->🚜 Refactor

- *(file_system)* deduplicate read methods and use Vec<u8> ([#816](https://github.com/oxc-project/oxc-resolver/pull/816)) (by @Boshen)

### Contributors

* @renovate[bot]
* @Boshen

## [11.13.1](https://github.com/oxc-project/oxc-resolver/compare/v11.13.0...v11.13.1) - 2025-11-04

### <!-- 1 -->🐛 Bug Fixes

- *(package_json)* re-read file for serde_json fallback in simd implementation ([#808](https://github.com/oxc-project/oxc-resolver/pull/808)) (by @Boshen)
- revert system file cache optimization on Linux ([#810](https://github.com/oxc-project/oxc-resolver/pull/810)) (by @Brooooooklyn) - #810
- skip loading tsconfig.json from virtual module paths ([#809](https://github.com/oxc-project/oxc-resolver/pull/809)) (by @sapphi-red) - #809

### <!-- 2 -->🚜 Refactor

- use cfg_if and rustix in read_to_string_bypass_system_cache ([#802](https://github.com/oxc-project/oxc-resolver/pull/802)) (by @Boshen) - #802

### <!-- 4 -->⚡ Performance

- optimize FileSystem metadata operations with rustix ([#800](https://github.com/oxc-project/oxc-resolver/pull/800)) (by @Boshen) - #800

### Contributors

* @Boshen
* @Brooooooklyn
* @sapphi-red
* @renovate[bot]

## [11.13.0](https://github.com/oxc-project/oxc-resolver/compare/v11.12.0...v11.13.0) - 2025-11-02

### <!-- 0 -->🚀 Features

- improve error message for empty package.json files ([#793](https://github.com/oxc-project/oxc-resolver/pull/793)) (by @Boshen) - #793

### <!-- 1 -->🐛 Bug Fixes

- don't drop canonicalized path by cache clear ([#791](https://github.com/oxc-project/oxc-resolver/pull/791)) (by @sapphi-red) - #791

### Contributors

* @sapphi-red
* @Boshen

## [11.12.0](https://github.com/oxc-project/oxc-resolver/compare/v11.11.1...v11.12.0) - 2025-10-27

### <!-- 0 -->🚀 Features

- improve PackagePathNotExported error message with condition names (by @Boshen)

### Contributors

* @Boshen

## [11.11.1](https://github.com/oxc-project/oxc-resolver/compare/v11.11.0...v11.11.1) - 2025-10-21

### <!-- 1 -->🐛 Bug Fixes

- derive Error for JSONError ([#779](https://github.com/oxc-project/oxc-resolver/pull/779)) (by @Boshen) - #779

### Contributors

* @Boshen

## [11.11.0](https://github.com/oxc-project/oxc-resolver/compare/v11.10.0...v11.11.0) - 2025-10-20

### <!-- 0 -->🚀 Features

- add big-endian support for package.json parsing ([#768](https://github.com/oxc-project/oxc-resolver/pull/768)) (by @Boshen) - #768
- add tsconfig discovery ([#758](https://github.com/oxc-project/oxc-resolver/pull/758)) (by @Boshen) - #758

### <!-- 1 -->🐛 Bug Fixes

- tsconfig paths should not be applied to paths inside node_modules ([#760](https://github.com/oxc-project/oxc-resolver/pull/760)) (by @Boshen) - #760

### <!-- 6 -->🧪 Testing

- add a tsconfig extend not found case ([#763](https://github.com/oxc-project/oxc-resolver/pull/763)) (by @Boshen) - #763

### Contributors

* @renovate[bot]
* @Boshen

## [11.10.0](https://github.com/oxc-project/oxc-resolver/compare/v11.9.0...v11.10.0) - 2025-10-17

### <!-- 0 -->🚀 Features

- add ESM file:// protocol support with comprehensive tests ([#746](https://github.com/oxc-project/oxc-resolver/pull/746)) (by @Boshen) - #746

### <!-- 2 -->🚜 Refactor

- remove normalize-path dependency, use internal PathUtil ([#742](https://github.com/oxc-project/oxc-resolver/pull/742)) (by @Boshen) - #742

### <!-- 4 -->⚡ Performance

- use simd-json for package.json parsing ([#761](https://github.com/oxc-project/oxc-resolver/pull/761)) (by @Boshen) - #761
- make url crate optional for wasm32 targets (by @Boshen)

### Contributors

* @Boshen
* @renovate[bot]

## [11.9.0](https://github.com/oxc-project/oxc-resolver/compare/v11.8.4...v11.9.0) - 2025-10-01

### <!-- 0 -->🚀 Features

- only resolve file:// protocol on windows ([#737](https://github.com/oxc-project/oxc-resolver/pull/737)) (by @Boshen) - #737

### <!-- 6 -->🧪 Testing

- improve test coverage for edge cases ([#740](https://github.com/oxc-project/oxc-resolver/pull/740)) (by @Boshen) - #740
- improve coverage for check_restrictions ([#739](https://github.com/oxc-project/oxc-resolver/pull/739)) (by @Boshen) - #739

### Contributors

* @Boshen

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
