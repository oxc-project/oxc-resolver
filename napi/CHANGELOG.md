# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [8.0.0](https://github.com/oxc-project/oxc-resolver/releases/tag/oxc_napi_resolver-v8.0.0) - 2025-05-09

### <!-- 0 -->Features

- *(napi)* add mimalloc ([#423](https://github.com/oxc-project/oxc-resolver/pull/423))
- [**breaking**] Rust Edition 2024 ([#402](https://github.com/oxc-project/oxc-resolver/pull/402))
- expose `package_json_path` ([#376](https://github.com/oxc-project/oxc-resolver/pull/376))
- *(napi)* expose module type info in ResolveResult ([#223](https://github.com/oxc-project/oxc-resolver/pull/223))
- *(napi)* add tracing via `OXC_LOG:DEBUG` ([#202](https://github.com/oxc-project/oxc-resolver/pull/202))
- *(napi)* add async API ([#191](https://github.com/oxc-project/oxc-resolver/pull/191))
- add `imports_fields` option ([#138](https://github.com/oxc-project/oxc-resolver/pull/138))
- add more builder functions for options ([#110](https://github.com/oxc-project/oxc-resolver/pull/110))
- *(napi)* support wasi target ([#31](https://github.com/oxc-project/oxc-resolver/pull/31))
- add file_dependencies and missing_dependencies API ([#50](https://github.com/oxc-project/oxc-resolver/pull/50))
- *(napi)* expose cloneWithOptions and clearCache methods ([#40](https://github.com/oxc-project/oxc-resolver/pull/40))
- *(napi)* update the doc and type for tsconfig references ([#24](https://github.com/oxc-project/oxc-resolver/pull/24))
- *(napi)* add options ([#19](https://github.com/oxc-project/oxc-resolver/pull/19))
- *(resolver)* add tracing-subscriber feature ([#904](https://github.com/oxc-project/oxc-resolver/pull/904))
- *(resolver)* tsconfig project references ([#862](https://github.com/oxc-project/oxc-resolver/pull/862))
- *(resolver)* add thiserror ([#847](https://github.com/oxc-project/oxc-resolver/pull/847))
- *(resolver)* implement nested alias field ([#795](https://github.com/oxc-project/oxc-resolver/pull/795))
- *(resolver)* implement tsconfig-paths ([#750](https://github.com/oxc-project/oxc-resolver/pull/750))
- *(resolver)* implement configurable `exports_fields` option ([#733](https://github.com/oxc-project/oxc-resolver/pull/733))
- *(resolver)* implement `main_fields`
- *(resolver)* implement resolveToContext ([#694](https://github.com/oxc-project/oxc-resolver/pull/694))
- *(resolver)* implement restrictions (path only) ([#693](https://github.com/oxc-project/oxc-resolver/pull/693))
- *(resolver)* implement fully specified ([#687](https://github.com/oxc-project/oxc-resolver/pull/687))
- *(resolver)* imports field ([#681](https://github.com/oxc-project/oxc-resolver/pull/681))
- *(resolver)* finish most of exports field ([#674](https://github.com/oxc-project/oxc-resolver/pull/674))
- *(resolver)* port the rest of the exports field tests ([#659](https://github.com/oxc-project/oxc-resolver/pull/659))
- *(resolver)* implement symlinks ([#582](https://github.com/oxc-project/oxc-resolver/pull/582))
- *(resolver)* complete query and fragment parsing ([#579](https://github.com/oxc-project/oxc-resolver/pull/579))
- *(resolver)* add preferRelative and preferAbsolute ([#577](https://github.com/oxc-project/oxc-resolver/pull/577))
- *(resolver)* implement roots ([#576](https://github.com/oxc-project/oxc-resolver/pull/576))
- *(resolver)* implement fallback ([#572](https://github.com/oxc-project/oxc-resolver/pull/572))
- *(resolver)* implement enforceExtension ([#566](https://github.com/oxc-project/oxc-resolver/pull/566))
- *(resolver)* implement descriptionFiles option ([#565](https://github.com/oxc-project/oxc-resolver/pull/565))
- *(resolver)* implement the basics of path alias ([#564](https://github.com/oxc-project/oxc-resolver/pull/564))
- *(resolver)* accept different file system implementations ([#562](https://github.com/oxc-project/oxc-resolver/pull/562))
- *(resolver)* implement browser field ([#561](https://github.com/oxc-project/oxc-resolver/pull/561))
- *(resolver)* implement scoped packages ([#558](https://github.com/oxc-project/oxc-resolver/pull/558))
- *(resolver)* port incorrect description file test ([#557](https://github.com/oxc-project/oxc-resolver/pull/557))
- *(resolver)* implement extension_alias ([#556](https://github.com/oxc-project/oxc-resolver/pull/556))
- *(resolver)* port resolve tests ([#555](https://github.com/oxc-project/oxc-resolver/pull/555))
- *(resolver)* resolve extensions ([#549](https://github.com/oxc-project/oxc-resolver/pull/549))
- *(resolver)* add resolver test fixtures ([#542](https://github.com/oxc-project/oxc-resolver/pull/542))

### <!-- 1 -->Bug Fixes

- hash import does not need to load from node_modules ([#501](https://github.com/oxc-project/oxc-resolver/pull/501))
- *(napi)* `new ResolverFactory()` options should be optional ([#256](https://github.com/oxc-project/oxc-resolver/pull/256))
- *(napi)* update buggy NAPI-RS versions ([#225](https://github.com/oxc-project/oxc-resolver/pull/225))
- canonicalize is not supported on wasi target ([#124](https://github.com/oxc-project/oxc-resolver/pull/124))
- resolve "browser" field when "exports" is present ([#59](https://github.com/oxc-project/oxc-resolver/pull/59))

### <!-- 4 -->Refactor

- [**breaking**] remove `description_files` option ([#488](https://github.com/oxc-project/oxc-resolver/pull/488))
- [**breaking**] remove `modules` options ([#484](https://github.com/oxc-project/oxc-resolver/pull/484))
- vitest ([#380](https://github.com/oxc-project/oxc-resolver/pull/380))
- apply latest `cargo +nightly fmt` ([#281](https://github.com/oxc-project/oxc-resolver/pull/281))
- selectively parse package_json fields instead of parsing everything ([#103](https://github.com/oxc-project/oxc-resolver/pull/103))
- *(resolver)* clean up some code and tests
- *(resolver)* change internal funcs to non-pub by moving to unit tests ([#682](https://github.com/oxc-project/oxc-resolver/pull/682))

### <!-- 7 -->Chore

- publish `oxc_napi_resolver` ([#496](https://github.com/oxc-project/oxc-resolver/pull/496))
- *(napi)* make mimalloc optional to build ([#495](https://github.com/oxc-project/oxc-resolver/pull/495))
- *(README)* add wasm usage example
- *(README)* crates.io badge use recent downloads
- *(napi)* auto download wasm binding on webcontainer ([#471](https://github.com/oxc-project/oxc-resolver/pull/471))
- use root package.json for napi build ([#469](https://github.com/oxc-project/oxc-resolver/pull/469))
- *(deps)* update github-actions ([#444](https://github.com/oxc-project/oxc-resolver/pull/444))
- *(deps)* lock file maintenance npm packages ([#436](https://github.com/oxc-project/oxc-resolver/pull/436))
- bump napi ([#404](https://github.com/oxc-project/oxc-resolver/pull/404))
- *(deps)* lock file maintenance npm packages ([#391](https://github.com/oxc-project/oxc-resolver/pull/391))
- *(deps)* lock file maintenance rust crates ([#390](https://github.com/oxc-project/oxc-resolver/pull/390))
- *(README)* clarify Rust and node.js usages
- add dprint ([#326](https://github.com/oxc-project/oxc-resolver/pull/326))
- *(deps)* update napi-rs to 3.0.0-alpha
- `cargo upgrade` && `pnpm upgrade`
- *(deps)* update napi-rs to 3.0.0-alpha
- update napi changes
- *(deps)* update rust crate napi-derive to 3.0.0-alpha
- *(deps)* update rust crate napi to 3.0.0-alpha
- *(deps)* update napi-rs to 2.16.8
- *(napi)* make napi binary smaller with minimal tracing features ([#213](https://github.com/oxc-project/oxc-resolver/pull/213))
- *(napi)* remove tokio ([#212](https://github.com/oxc-project/oxc-resolver/pull/212))
- document directory is an absolute path for `resolve(directory, specifier)` ([#206](https://github.com/oxc-project/oxc-resolver/pull/206))
- re-enable the wasi build ([#193](https://github.com/oxc-project/oxc-resolver/pull/193))
- use pnpm workspace ([#182](https://github.com/oxc-project/oxc-resolver/pull/182))
- *(deps)* update rust crates ([#176](https://github.com/oxc-project/oxc-resolver/pull/176))
- *(napi)* update NAPI-RS cli version and binding template ([#111](https://github.com/oxc-project/oxc-resolver/pull/111))
- update project github url
- *(deps)* update pnpm to v8.14.1 ([#52](https://github.com/oxc-project/oxc-resolver/pull/52))
- *(deps)* update pnpm to v8.14.0 ([#48](https://github.com/oxc-project/oxc-resolver/pull/48))
- *(deps)* update pnpm to v8.13.1 ([#42](https://github.com/oxc-project/oxc-resolver/pull/42))
- remove FIXME comments
- *(napi)* align `*Fields` user options with enhanced-resolve ([#35](https://github.com/oxc-project/oxc-resolver/pull/35))
- *(deps)* update pnpm to v8.12.1 ([#21](https://github.com/oxc-project/oxc-resolver/pull/21))
- add some doc for napi TsconfigOptions ([#20](https://github.com/oxc-project/oxc-resolver/pull/20))
- *(deps)* update pnpm to v8.12.0 ([#18](https://github.com/oxc-project/oxc-resolver/pull/18))
- *(README)* adding debugging command from Rspack
- *(deps)* update pnpm to v8.11.0 ([#9](https://github.com/oxc-project/oxc-resolver/pull/9))
- *(resolver)* remove tracing_subscriber ([#1362](https://github.com/oxc-project/oxc-resolver/pull/1362))
- *(resolver)* improve documentation ([#591](https://github.com/oxc-project/oxc-resolver/pull/591))

### <!-- 8 -->CI

- check for napi .d.index changes ([#491](https://github.com/oxc-project/oxc-resolver/pull/491))
- *(release-napi)* support `riscv64gc-unknown-linux-gnu` and `s390x-unknown-linux-gnu` ([#451](https://github.com/oxc-project/oxc-resolver/pull/451))
