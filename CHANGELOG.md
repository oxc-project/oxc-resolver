# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0](https://github.com/unrs/rspack-resolver/releases/tag/unrspack-resolver-v1.0.0) - 2025-03-15

### Added

- *(pnp)* support link ([#49](https://github.com/unrs/rspack-resolver/pull/49))
- Revert  vfs logic ([#44](https://github.com/unrs/rspack-resolver/pull/44))
- expose PnpFileSystem ([#43](https://github.com/unrs/rspack-resolver/pull/43))
- pub read api ([#41](https://github.com/unrs/rspack-resolver/pull/41))
- rebase and refine extension-alias error format ([#30](https://github.com/unrs/rspack-resolver/pull/30))
- pub all FileMetadata field ([#27](https://github.com/unrs/rspack-resolver/pull/27))
- remove send + sync constraint ([#25](https://github.com/unrs/rspack-resolver/pull/25))
- rebase latest oxc-resolver and support pnp ([#12](https://github.com/unrs/rspack-resolver/pull/12))
- *(napi)* add tracing via `OXC_LOG:DEBUG` ([#202](https://github.com/unrs/rspack-resolver/pull/202))
- strip symbols and enable LTO ([#197](https://github.com/unrs/rspack-resolver/pull/197))
- export package.json `type` and `sideEffects` field by default for bundlers ([#196](https://github.com/unrs/rspack-resolver/pull/196))
- *(napi)* add async API ([#191](https://github.com/unrs/rspack-resolver/pull/191))
- [**breaking**] remove the constraint on packages exports `default` must be the last one ([#171](https://github.com/unrs/rspack-resolver/pull/171))
- [**breaking**] return `ResolveError:Builtin("node:{specifier}")` from package imports and exports ([#165](https://github.com/unrs/rspack-resolver/pull/165))
- add `imports_fields` option ([#138](https://github.com/unrs/rspack-resolver/pull/138))
- substitute path that starts with `${configDir}/` in tsconfig.compilerOptions.paths ([#136](https://github.com/unrs/rspack-resolver/pull/136))
- allow `Resolver<Box<dyn FileSystem>>` by removing unnecessary `Default` constraint ([#116](https://github.com/unrs/rspack-resolver/pull/116))
- add more builder functions for options ([#110](https://github.com/unrs/rspack-resolver/pull/110))
- add feature `package_json_raw_json_api` for returning package's raw json ([#104](https://github.com/unrs/rspack-resolver/pull/104))
- support tsconfig#extends array ([#102](https://github.com/unrs/rspack-resolver/pull/102))
- more builder pattern options ([#84](https://github.com/unrs/rspack-resolver/pull/84))
- functions to add more options using builder pattern ([#81](https://github.com/unrs/rspack-resolver/pull/81))
- *(napi)* support wasi target ([#31](https://github.com/unrs/rspack-resolver/pull/31))
- add file_dependencies and missing_dependencies API ([#50](https://github.com/unrs/rspack-resolver/pull/50))
- add context to PackageImportNotDefined error
- improve errors by adding more contexts
- *(napi)* expose cloneWithOptions and clearCache methods ([#40](https://github.com/unrs/rspack-resolver/pull/40))
- clean up the error message todos ([#38](https://github.com/unrs/rspack-resolver/pull/38))
- return not found when recursing non-existent file ([#36](https://github.com/unrs/rspack-resolver/pull/36))
- *(napi)* update the doc and type for tsconfig references ([#24](https://github.com/unrs/rspack-resolver/pull/24))
- *(napi)* add options ([#19](https://github.com/unrs/rspack-resolver/pull/19))
- *(resolver)* add a `realpath` to package.json ([#1634](https://github.com/unrs/rspack-resolver/pull/1634))
- *(resovler)* impl Into for IOError ([#1223](https://github.com/unrs/rspack-resolver/pull/1223))
- *(resolver)* strip trailling commas from tsconfig.json ([#1198](https://github.com/unrs/rspack-resolver/pull/1198))
- *(resolver)* configurable tsconfig project references ([#965](https://github.com/unrs/rspack-resolver/pull/965))
- *(resolver)* add more tracing events to resolver ([#907](https://github.com/unrs/rspack-resolver/pull/907))
- *(resolver)* add TsconfigNotFound error ([#905](https://github.com/unrs/rspack-resolver/pull/905))
- *(resolver)* add tracing-subscriber feature ([#904](https://github.com/unrs/rspack-resolver/pull/904))
- *(resolver)* tsconfig project references ([#862](https://github.com/unrs/rspack-resolver/pull/862))
- *(resolver)* add thiserror ([#847](https://github.com/unrs/rspack-resolver/pull/847))
- *(resolver)* add tracing example
- *(resolver)* add an option to turn off builtin_modules ([#833](https://github.com/unrs/rspack-resolver/pull/833))
- *(resolver)* check for node.js core modules ([#794](https://github.com/unrs/rspack-resolver/pull/794))
- *(resolver)* implement nested alias field ([#795](https://github.com/unrs/rspack-resolver/pull/795))
- *(resolver)* implement tsconfig-paths ([#750](https://github.com/unrs/rspack-resolver/pull/750))
- *(resolver)* handle path alias with `#` ([#739](https://github.com/unrs/rspack-resolver/pull/739))
- *(resolver)* expose raw package_json value; improve print debug ([#738](https://github.com/unrs/rspack-resolver/pull/738))
- *(resolver)* implement configurable `exports_fields` option ([#733](https://github.com/unrs/rspack-resolver/pull/733))
- *(resolver)* resolve `#` as path instead of a fragment ([#727](https://github.com/unrs/rspack-resolver/pull/727))
- *(resolver)* pass on query string from alias fields
- *(resolver)* complete browser_field implementation
- *(resolver)* check for infinite recursion ([#714](https://github.com/unrs/rspack-resolver/pull/714))
- *(resolver)* implement `main_fields`
- *(resolver)* add `exports_fields` and `main_fields` for logging purposes.
- *(resolver)* add tracing ([#710](https://github.com/unrs/rspack-resolver/pull/710))
- *(resolver)* implement recursive alias, file as alias and exports field with query / fragment ([#695](https://github.com/unrs/rspack-resolver/pull/695))
- *(resolver)* implement resolveToContext ([#694](https://github.com/unrs/rspack-resolver/pull/694))
- *(resolver)* implement restrictions (path only) ([#693](https://github.com/unrs/rspack-resolver/pull/693))
- *(resolver)* implement the basics of ESM ([#691](https://github.com/unrs/rspack-resolver/pull/691))
- *(resolver)* implement fully specified ([#687](https://github.com/unrs/rspack-resolver/pull/687))
- *(resolver)* imports field ([#681](https://github.com/unrs/rspack-resolver/pull/681))
- *(resolver)* finish most of exports field ([#674](https://github.com/unrs/rspack-resolver/pull/674))
- *(resolver)* port the rest of the exports field tests ([#659](https://github.com/unrs/rspack-resolver/pull/659))
- *(resolver)* implement more of exports field ([#648](https://github.com/unrs/rspack-resolver/pull/648))
- *(resolver)* initialize implementation of package.json exports field ([#630](https://github.com/unrs/rspack-resolver/pull/630))
- *(resolver)* check for directory before loading a directory ([#590](https://github.com/unrs/rspack-resolver/pull/590))
- *(resolver)* implement symlinks ([#582](https://github.com/unrs/rspack-resolver/pull/582))
- *(resolver)* complete query and fragment parsing ([#579](https://github.com/unrs/rspack-resolver/pull/579))
- *(resolver)* add preferRelative and preferAbsolute ([#577](https://github.com/unrs/rspack-resolver/pull/577))
- *(resolver)* implement roots ([#576](https://github.com/unrs/rspack-resolver/pull/576))
- *(resolver)* implement fallback ([#572](https://github.com/unrs/rspack-resolver/pull/572))
- *(resolver)* implement enforceExtension ([#567](https://github.com/unrs/rspack-resolver/pull/567))
- *(resolver)* implement enforceExtension ([#566](https://github.com/unrs/rspack-resolver/pull/566))
- *(resolver)* implement descriptionFiles option ([#565](https://github.com/unrs/rspack-resolver/pull/565))
- *(resolver)* implement the basics of path alias ([#564](https://github.com/unrs/rspack-resolver/pull/564))
- *(resolver)* accept different file system implementations ([#562](https://github.com/unrs/rspack-resolver/pull/562))
- *(resolver)* implement browser field ([#561](https://github.com/unrs/rspack-resolver/pull/561))
- *(resolver)* implement scoped packages ([#558](https://github.com/unrs/rspack-resolver/pull/558))
- *(resolver)* port incorrect description file test ([#557](https://github.com/unrs/rspack-resolver/pull/557))
- *(resolver)* implement extension_alias ([#556](https://github.com/unrs/rspack-resolver/pull/556))
- *(resolver)* port resolve tests ([#555](https://github.com/unrs/rspack-resolver/pull/555))
- *(resolver)* resolve extensions ([#549](https://github.com/unrs/rspack-resolver/pull/549))
- *(resolver)* resolve as module ([#544](https://github.com/unrs/rspack-resolver/pull/544))
- *(resolver)* resolve js file ([#543](https://github.com/unrs/rspack-resolver/pull/543))
- *(resolver)* add resolver test fixtures ([#542](https://github.com/unrs/rspack-resolver/pull/542))

### Fixed

- abs path in main fields ([#52](https://github.com/unrs/rspack-resolver/pull/52))
- 🐛 pnp feat respect options.enable_pnp ([#47](https://github.com/unrs/rspack-resolver/pull/47))
- alias match request end with slash ([#35](https://github.com/unrs/rspack-resolver/pull/35))
- resolve mathjs error when using `extensionAlias` ([#31](https://github.com/unrs/rspack-resolver/pull/31))
- fix symbol link support in pnpm workspace ([#21](https://github.com/unrs/rspack-resolver/pull/21))
- fallback to next main field when resolve failed ([#17](https://github.com/unrs/rspack-resolver/pull/17))
- tsconfig project reference it self should throw error ([#211](https://github.com/unrs/rspack-resolver/pull/211))
- comment ([#179](https://github.com/unrs/rspack-resolver/pull/179))
- alias value should try fragment as path ([#172](https://github.com/unrs/rspack-resolver/pull/172))
- alias not found should return error ([#168](https://github.com/unrs/rspack-resolver/pull/168))
- RootsPlugin debug_assert on windows ([#145](https://github.com/unrs/rspack-resolver/pull/145))
- RootsPlugin should fall through if it fails to resolve the roots ([#144](https://github.com/unrs/rspack-resolver/pull/144))
- lazily read package.json.exports for shared resolvers ([#137](https://github.com/unrs/rspack-resolver/pull/137))
- incorrect resolution when using shared resolvers with different `main_fields` ([#134](https://github.com/unrs/rspack-resolver/pull/134))
- canonicalize is not supported on wasi target ([#124](https://github.com/unrs/rspack-resolver/pull/124))
- missing `Debug` for `Specifier`
- windows path with C:// like prefixes ([#92](https://github.com/unrs/rspack-resolver/pull/92))
- extending tsconfig paths with baseUrl in original tsconfig file ([#91](https://github.com/unrs/rspack-resolver/pull/91))
- specifier with multiple `?` ([#83](https://github.com/unrs/rspack-resolver/pull/83))
- tsconfig#extends must be a string ([#80](https://github.com/unrs/rspack-resolver/pull/80))
- normalize aliased path ([#78](https://github.com/unrs/rspack-resolver/pull/78))
- panic when `?` is passed in ([#70](https://github.com/unrs/rspack-resolver/pull/70))
- resolve "browser" field when "exports" is present ([#59](https://github.com/unrs/rspack-resolver/pull/59))
- returning broken missing dependencies when alias and extensions are provided ([#54](https://github.com/unrs/rspack-resolver/pull/54))
- change ResolveError::NotFound(PathBuf) to report specifier
- make raw_json return `&Arc<serde_json::Value>`
- browser field resolving relative to path to itself ([#34](https://github.com/unrs/rspack-resolver/pull/34))
- *(justfile)* fix example command
- *(resolver)* resolve query and fragments with unicode filenames ([#1591](https://github.com/unrs/rspack-resolver/pull/1591))
- *(resolver)* make sure package.json path is inside the resolved path ([#1481](https://github.com/unrs/rspack-resolver/pull/1481))
- *(resolver)* resolve tsconfig extend that are extensionless ([#971](https://github.com/unrs/rspack-resolver/pull/971))
- *(resolver)* log error as debug so it does not print the error by default
- *(resolver)* fix tsconfig lookup when a directory is provided ([#900](https://github.com/unrs/rspack-resolver/pull/900))
- *(resolver)* fix collision on hash entries ([#850](https://github.com/unrs/rspack-resolver/pull/850))
- *(resolver)* fix a case where ignored package has a fallback ([#837](https://github.com/unrs/rspack-resolver/pull/837))
- *(resolver)* fix a case where an alias is part of a dashed package name ([#836](https://github.com/unrs/rspack-resolver/pull/836))
- *(resolver)* fix cases with conflicting node_modules path ([#835](https://github.com/unrs/rspack-resolver/pull/835))
- *(resolver)* add test case for resolve_to_context ([#834](https://github.com/unrs/rspack-resolver/pull/834))
- *(resolver)* resolve exports field that are directories ([#820](https://github.com/unrs/rspack-resolver/pull/820))
- *(resolver)* fix resolving package_self with the correct subpath
- *(resolver)* correct behavior for enforceExtension
- *(resolver)* do not resolve browser field that are strings ([#816](https://github.com/unrs/rspack-resolver/pull/816))
- *(resolver)* make sure package name is valid when loading package self ([#810](https://github.com/unrs/rspack-resolver/pull/810))
- *(resolver)* fix a case where package name and specifier is the wrong order
- *(resolver)* add a case with multi-dot filename
- *(resolver)* add `derive` to serde
- *(resolver)* fix a case with multi-dot file extensions ([#704](https://github.com/unrs/rspack-resolver/pull/704))
- *(resolver)* fix resolver benchmark

### Other

- let's release to npm
- release  0.5.2 ([#51](https://github.com/unrs/rspack-resolver/pull/51))
- bump pnp 0.9.1 ([#50](https://github.com/unrs/rspack-resolver/pull/50))
- release 0.5.1 ([#48](https://github.com/unrs/rspack-resolver/pull/48))
- Release
- Fixes "exports" field w/ PnP ([#46](https://github.com/unrs/rspack-resolver/pull/46))
- Implements a virtual filesystem layer ([#42](https://github.com/unrs/rspack-resolver/pull/42))
- release 0.4.0
- Implements the PnP manifest lookup within the resolver ([#39](https://github.com/unrs/rspack-resolver/pull/39))
- release v0.3.6 ([#36](https://github.com/unrs/rspack-resolver/pull/36))
- release v0.3.5 ([#33](https://github.com/unrs/rspack-resolver/pull/33))
- release v0.3.4 ([#32](https://github.com/unrs/rspack-resolver/pull/32))
- release 0.3.3 ([#28](https://github.com/unrs/rspack-resolver/pull/28))
- release 0.3.2 ([#26](https://github.com/unrs/rspack-resolver/pull/26))
- *(release)* bump to 0.3.1 ([#22](https://github.com/unrs/rspack-resolver/pull/22))
- *(release)* 0.3.0 ([#18](https://github.com/unrs/rspack-resolver/pull/18))
- *(release)* release 0.2.0 ([#15](https://github.com/unrs/rspack-resolver/pull/15))
- *(rebase)* rebase the latest change ([#13](https://github.com/unrs/rspack-resolver/pull/13))
- *(rebase)* rebase the commits of oxc-resolver ([#11](https://github.com/unrs/rspack-resolver/pull/11))
- change crate name & update license ([#3](https://github.com/unrs/rspack-resolver/pull/3))
- clean readme
- add fork reason
- Release v1.9.3
- *(napi)* make napi binary smaller with minimal tracing features ([#213](https://github.com/unrs/rspack-resolver/pull/213))
- *(napi)* remove tokio ([#212](https://github.com/unrs/rspack-resolver/pull/212))
- *(deps)* update rust crate dashmap to v6 ([#209](https://github.com/unrs/rspack-resolver/pull/209))
- *(deps)* update rust crate serde_json to v1.0.119 ([#208](https://github.com/unrs/rspack-resolver/pull/208))
- Release v1.9.2
- document directory is an absolute path for `resolve(directory, specifier)` ([#206](https://github.com/unrs/rspack-resolver/pull/206))
- add a broken tsconfig test ([#205](https://github.com/unrs/rspack-resolver/pull/205))
- improve code coverage for src/error.rs ([#204](https://github.com/unrs/rspack-resolver/pull/204))
- skip resolving extension alias when `options.extension_alias` is empty ([#203](https://github.com/unrs/rspack-resolver/pull/203))
- add npm badge to crates.io
- Release v1.9.1
- improve call to `Path::ends_with` ([#199](https://github.com/unrs/rspack-resolver/pull/199))
- list [profile.release] explicitly ([#198](https://github.com/unrs/rspack-resolver/pull/198))
- Release v1.9.0
- Release v1.8.4
- skip searching for package.json when `alias_fields` is not provided ([#194](https://github.com/unrs/rspack-resolver/pull/194))
- Release v1.8.3
- re-enable the wasi build ([#193](https://github.com/unrs/rspack-resolver/pull/193))
- Release v1.8.2
- *(deps)* update dependency rust to v1.79.0 ([#180](https://github.com/unrs/rspack-resolver/pull/180))
- document release channels (crates.io + npm)
- *(deps)* update rust crate rustc-hash to v2 ([#186](https://github.com/unrs/rspack-resolver/pull/186))
- *(deps)* update rust crate criterion2 to 0.11.0 ([#184](https://github.com/unrs/rspack-resolver/pull/184))
- *(README)* explain tracing information; add debug Rolldown
- *(deps)* update rust crates ([#176](https://github.com/unrs/rspack-resolver/pull/176))
- Update README.md
- specify `criterion2` directly in Cargo.toml
- *(deps)* lock file maintenance rust crates ([#174](https://github.com/unrs/rspack-resolver/pull/174))
- Release v1.8.1
- Release v1.8.0
- add test cases for resolve alias value with fragment ([#170](https://github.com/unrs/rspack-resolver/pull/170))
- *(deps)* lock file maintenance rust crates ([#167](https://github.com/unrs/rspack-resolver/pull/167))
- *(deps)* lock file maintenance rust crates ([#163](https://github.com/unrs/rspack-resolver/pull/163))
- *(deps)* lock file maintenance rust crates ([#158](https://github.com/unrs/rspack-resolver/pull/158))
- *(deps)* lock file maintenance rust crates ([#155](https://github.com/unrs/rspack-resolver/pull/155))
- *(deps)* update dependency rust to v1.78.0 ([#154](https://github.com/unrs/rspack-resolver/pull/154))
- ignore invalid browser field
- test more `IOError` methods
- improve `normalize_with` function ([#153](https://github.com/unrs/rspack-resolver/pull/153))
- *(deps)* lock file maintenance rust crates ([#152](https://github.com/unrs/rspack-resolver/pull/152))
- code covererage on `FileMetadata`
- add panic test for extensions without a leading dot ([#150](https://github.com/unrs/rspack-resolver/pull/150))
- add test case for empty alias fields ([#149](https://github.com/unrs/rspack-resolver/pull/149))
- Release v1.7.0
- remove `PartialEq` and `Eq` from `Specifier` ([#148](https://github.com/unrs/rspack-resolver/pull/148))
- add test case for tsconfig paths alias fall through ([#147](https://github.com/unrs/rspack-resolver/pull/147))
- lazily read package.json.browser_fields for shared resolvers ([#142](https://github.com/unrs/rspack-resolver/pull/142))
- avoid an extra allocation in `load_extensions`
- ignore code coverage for `Display` on `ResolveOptions` ([#140](https://github.com/unrs/rspack-resolver/pull/140))
- remove the browser field lookup in `resolve_esm_match` ([#141](https://github.com/unrs/rspack-resolver/pull/141))
- remove the extra `condition_names` from `package_exports_resolve`
- Release v1.6.7
- Release v1.6.6
- print resolve options while debug tracing ([#133](https://github.com/unrs/rspack-resolver/pull/133))
- move tsconfig test fixtures around
- *(deps)* lock file maintenance rust crates ([#132](https://github.com/unrs/rspack-resolver/pull/132))
- switch to criterion2 to reduce dependencies ([#128](https://github.com/unrs/rspack-resolver/pull/128))
- *(deps)* lock file maintenance rust crates ([#127](https://github.com/unrs/rspack-resolver/pull/127))
- Release v1.6.5
- *(deps)* lock file maintenance rust crates ([#123](https://github.com/unrs/rspack-resolver/pull/123))
- document feature flags
- Release v1.6.4
- add resolver to Cargo.toml workspace
- try allow_dirty again
- configure release-plz
- improve terminology and clarify contexts ([#118](https://github.com/unrs/rspack-resolver/pull/118))
- Publish crate and napi v1.6.3
- *(deps)* update rust crate rayon to 1.10.0 ([#117](https://github.com/unrs/rspack-resolver/pull/117))
- Publish crate and napi v1.6.2
- *(deps)* update rust crate thiserror to 1.0.58 ([#114](https://github.com/unrs/rspack-resolver/pull/114))
- enable `clippy::cargo`
- Publish crate and napi v1.6.1
- deserialize less values in tsconfig ([#109](https://github.com/unrs/rspack-resolver/pull/109))
- *(README.md)* document "main" field
- *(deps)* update rust crates ([#106](https://github.com/unrs/rspack-resolver/pull/106))
- *(README)* add errors section
- Publish crate and napi v1.6.0
- selectively parse package_json fields instead of parsing everything ([#103](https://github.com/unrs/rspack-resolver/pull/103))
- bump deps
- Migrate to package.json/tsconfig.json crates ([#99](https://github.com/unrs/rspack-resolver/pull/99))
- *(deps)* update rust crates ([#100](https://github.com/unrs/rspack-resolver/pull/100))
- *(deps)* update rust crates ([#96](https://github.com/unrs/rspack-resolver/pull/96))
- Update README.md passing "require" instead of "import" to CJS resolver ([#97](https://github.com/unrs/rspack-resolver/pull/97))
- Publish crate v1.5.4
- export nodejs builtins
- add precautionary measures to top level require methods ([#94](https://github.com/unrs/rspack-resolver/pull/94))
- Publish crate v1.5.3
- fix unused code warning on windows
- fix unused code warning on windows
- fix unused code warning on windows
- fix unused code warning on windows
- increase some codecov
- use the same const for slash start '/' and '\\'
- Publish crate v1.5.2
- document `specifier` and `path` for `resolve`
- *(deps)* update rust crate vfs to 0.11.0 ([#89](https://github.com/unrs/rspack-resolver/pull/89))
- *(deps)* update rust crates ([#86](https://github.com/unrs/rspack-resolver/pull/86))
- update documentation for `enforce_extension` ([#85](https://github.com/unrs/rspack-resolver/pull/85))
- Publish crate v1.5.1
- Publish crate v1.5.0
- Publish crate v1.3.0 and napi v0.2.0
- *(deps)* update rust crate json-strip-comments to 1.0.2 ([#74](https://github.com/unrs/rspack-resolver/pull/74))
- update README
- update README regarding ESM
- Publish v1.3.0
- *(deps)* update rust crates ([#64](https://github.com/unrs/rspack-resolver/pull/64))
- improve code code coverage ([#67](https://github.com/unrs/rspack-resolver/pull/67))
- *(deps)* update crates ([#62](https://github.com/unrs/rspack-resolver/pull/62))
- *(renovate)* update
- Publish v1.2.2
- update project github url
- clean up .gitignore
- Release napi v0.1.1
- Release napi v0.1.0
- use json-strip-comments crate ([#56](https://github.com/unrs/rspack-resolver/pull/56))
- Publish v1.2.1
- *(deps)* update rust crate rayon to 1.8.1 ([#55](https://github.com/unrs/rspack-resolver/pull/55))
- *(deps)* update rust crate serde to 1.0.195 ([#46](https://github.com/unrs/rspack-resolver/pull/46))
- Publish v1.2.0
- *(deps)* update rust crate serde_json to 1.0.111 ([#47](https://github.com/unrs/rspack-resolver/pull/47))
- Publish v1.1.0
- s/ResolveContext/Ctx for inner usage
- add a `resolve_tracing` method
- Rust v1.75.0
- move ResolveContext to its own file
- s/ResolveState/ResolveResult
- bump dependencies
- *(deps)* update rust crate thiserror to 1.0.56 ([#45](https://github.com/unrs/rspack-resolver/pull/45))
- *(deps)* update rust crate thiserror to 1.0.55 ([#44](https://github.com/unrs/rspack-resolver/pull/44))
- *(deps)* update rust crate thiserror to 1.0.53 ([#41](https://github.com/unrs/rspack-resolver/pull/41))
- *(deps)* update rust crate serde_json to 1.0.109 ([#43](https://github.com/unrs/rspack-resolver/pull/43))
- Publish v1.0.1
- remove some periods
- Publish v1.0.0
- clean up docs and remove some `pub` fields
- update how examples should be run
- include benches
- include examples
- include the src only
- Publish v0.6.2
- add publish script
- use FxHashMap instead of FxIndexMap for BrowserField ([#33](https://github.com/unrs/rspack-resolver/pull/33))
- tweak the load_browser_field API
- *(deps)* update rust crate thiserror to 1.0.51 ([#30](https://github.com/unrs/rspack-resolver/pull/30))
- Release napi v0.0.3
- fix upload / download
- Release napi v0.0.3
- fix download artifacts
- Release napi v0.0.3
- *(deps)* update actions/download-artifact action to v4 ([#27](https://github.com/unrs/rspack-resolver/pull/27))
- *(deps)* update actions/upload-artifact action to v4 ([#28](https://github.com/unrs/rspack-resolver/pull/28))
- setup napi release script
- *(deps)* update actions/download-artifact action to v4 ([#25](https://github.com/unrs/rspack-resolver/pull/25))
- *(deps)* update actions/upload-artifact action to v4 ([#26](https://github.com/unrs/rspack-resolver/pull/26))
- *(resolver)* remove extra large fields from raw package json ([#23](https://github.com/unrs/rspack-resolver/pull/23))
- Release v0.6.1
- throw recursion error when resolving cursed browser fields ([#17](https://github.com/unrs/rspack-resolver/pull/17))
- *(deps)* update rust crate once_cell to 1.19.0 ([#13](https://github.com/unrs/rspack-resolver/pull/13))
- remove some unnecessary trace information
- add tests for axios
- *(README)* adding debugging command from Rspack
- Update README.md
- Update README.md
- Update README.md
- Release 0.6.0
- Update README.md
- add test for styled-components in real pnpm 8 ([#8](https://github.com/unrs/rspack-resolver/pull/8))
- Update README.md
- add integration_test ([#7](https://github.com/unrs/rspack-resolver/pull/7))
- add codecov ([#6](https://github.com/unrs/rspack-resolver/pull/6))
- add renovate.json
- remove unused `impl<T> FileSystem for Arc<T>` ([#5](https://github.com/unrs/rspack-resolver/pull/5))
- Update README.md
- add benchmark ([#4](https://github.com/unrs/rspack-resolver/pull/4))
- update README
- add CI ([#3](https://github.com/unrs/rspack-resolver/pull/3))
- add SECURITY.md
- add CODE_OF_CONDUCT
- add deny.toml
- add license
- update Cargo.toml and dot files
- Release oxc_resolver v0.5.5
- *(resolver)* add a path alias test ([#1549](https://github.com/unrs/rspack-resolver/pull/1549))
- Release oxc_resolver v0.5.4
- *(resolver)* do not search for package.json inside non-existing directories ([#1482](https://github.com/unrs/rspack-resolver/pull/1482))
- *(resolver)* do not search for package.json inside non-existing directories ([#1480](https://github.com/unrs/rspack-resolver/pull/1480))
- *(rust)* move to workspace lint table ([#1444](https://github.com/unrs/rspack-resolver/pull/1444))
- *(resolver)* remove tracing_subscriber ([#1362](https://github.com/unrs/rspack-resolver/pull/1362))
- Rust v1.74.0 ([#1357](https://github.com/unrs/rspack-resolver/pull/1357))
- Release oxc_resolver v0.5.3
- Release oxc_resolver v0.5.2
- Release oxc_resolver v0.5.1
- 🤖 impl fileSystem for `Arc<T>` ([#1166](https://github.com/unrs/rspack-resolver/pull/1166))
- Release oxc_resolver v0.5.0
- 🤖 remove generic in FileSystem trait ([#1163](https://github.com/unrs/rspack-resolver/pull/1163))
- Release oxc_resolver v0.4.0
- 🤖 make FileSystem trait object safe ([#1157](https://github.com/unrs/rspack-resolver/pull/1157))
- *(deps)* bump the dependencies group with 5 updates ([#1002](https://github.com/unrs/rspack-resolver/pull/1002))
- *(clippy)* enable undocumented_unsafe_blocks
- *(clippy)* allow clippy::too_many_lines
- *(clippy)* allow struct_excessive_bools
- *(resolver)* remove accidentally committed temp files
- Release oxc_resolver v0.3.1
- Release oxc_resolver v0.3.0
- *(resolver)* move tests folder to fixtures ([#964](https://github.com/unrs/rspack-resolver/pull/964))
- Release oxc_resolver v0.2.0
- *(resolver)* use system canonicalize to reduce total number of path hashes ([#902](https://github.com/unrs/rspack-resolver/pull/902))
- *(resolver)* used cached node_modules in `package_resolve` ([#901](https://github.com/unrs/rspack-resolver/pull/901))
- *(resolver)* do not search inside non-existent directories for scoped packages ([#899](https://github.com/unrs/rspack-resolver/pull/899))
- *(resolver)* clean up `load_alias` ([#875](https://github.com/unrs/rspack-resolver/pull/875))
- *(resolver)* avoid double hashing by memoizing the hash ([#871](https://github.com/unrs/rspack-resolver/pull/871))
- *(resolver)* optimize canonicalize ([#870](https://github.com/unrs/rspack-resolver/pull/870))
- *(resolver)* cache `node_modules` lookup ([#869](https://github.com/unrs/rspack-resolver/pull/869))
- Release oxc_resolver v0.1.0
- *(resolver)* remove unnecessary `RefCell` ([#849](https://github.com/unrs/rspack-resolver/pull/849))
- clean up deps ([#840](https://github.com/unrs/rspack-resolver/pull/840))
- *(benchmark)* use codspeed for all benchmarks ([#839](https://github.com/unrs/rspack-resolver/pull/839))
- *(resolver)* benchmark with codspeed ([#838](https://github.com/unrs/rspack-resolver/pull/838))
- *(resolver)* remove nodejs_resolver comparison
- *(deps)* bump the dependencies group with 10 updates ([#831](https://github.com/unrs/rspack-resolver/pull/831))
- *(resolver)* stop descending into node_modules when possible ([#821](https://github.com/unrs/rspack-resolver/pull/821))
- improve code coverage a little bit
- Rust 1.72.0 ([#784](https://github.com/unrs/rspack-resolver/pull/784))
- Revert "fix(resolver): fix a case where package name and specifier is the wrong order"
- improve code coverage in various places ([#721](https://github.com/unrs/rspack-resolver/pull/721))
- *(resolver)* remove the leading dot trim on extensions
- *(resolver)* clean up some code and tests
- *(resolver)* clean up the tests a little bit
- *(resolver)* remove the identity-hash crate
- *(resolver)* add a EnforceExtension tri state
- *(resolver)* make Resolution::full_path not owned
- *(resolver)* return package json error immediately instead of saving it ([#702](https://github.com/unrs/rspack-resolver/pull/702))
- *(resolver)* improve code by looking at the code coverage ([#697](https://github.com/unrs/rspack-resolver/pull/697))
- *(resolver)* clean some code ([#692](https://github.com/unrs/rspack-resolver/pull/692))
- *(resolver)* change internal funcs to non-pub by moving to unit tests ([#682](https://github.com/unrs/rspack-resolver/pull/682))
- *(rust)* update crate info, add minimal rust-version, add categories
- reformat
- *(resolver)* reduce memory allocation when resolving node_modules ([#608](https://github.com/unrs/rspack-resolver/pull/608))
- *(resolver)* hash once for the `get` + `insert` case ([#606](https://github.com/unrs/rspack-resolver/pull/606))
- *(resolver)* use DashSet for the cache ([#605](https://github.com/unrs/rspack-resolver/pull/605))
- *(resolver)* allocate less when resolving extensions ([#603](https://github.com/unrs/rspack-resolver/pull/603))
- *(resolver)* reduce the total number of hashes by passing the cached value around ([#602](https://github.com/unrs/rspack-resolver/pull/602))
- *(resolver)* do not read package_json of a file ([#601](https://github.com/unrs/rspack-resolver/pull/601))
- *(resolver)* add a alias test and check resolution is the same in benchmark ([#600](https://github.com/unrs/rspack-resolver/pull/600))
- *(resolver)* make the global cache hold less memory ([#593](https://github.com/unrs/rspack-resolver/pull/593))
- *(resolver)* improve browser_field lookup ([#592](https://github.com/unrs/rspack-resolver/pull/592))
- *(resolver)* s/request_str/request
- *(resolver)* improve documentation ([#591](https://github.com/unrs/rspack-resolver/pull/591))
- *(resolver)* improve how browser field is resolved ([#589](https://github.com/unrs/rspack-resolver/pull/589))
- *(resolver)* add multi-threaded benchmark ([#588](https://github.com/unrs/rspack-resolver/pull/588))
- *(resolver)* add more data to benchmark ([#586](https://github.com/unrs/rspack-resolver/pull/586))
- *(resolver)* improve cache hit for package.json ([#585](https://github.com/unrs/rspack-resolver/pull/585))
- *(resolver)* cache canonicalized path ([#584](https://github.com/unrs/rspack-resolver/pull/584))
- *(resolver)* use `fs::symlink_metadata`, which doesn't traverse symlinks ([#581](https://github.com/unrs/rspack-resolver/pull/581))
- *(rust)* bump dependencies
- *(resolver)* check against Result for better assertion message ([#573](https://github.com/unrs/rspack-resolver/pull/573))
- *(resolver)* cache all package.json queries ([#569](https://github.com/unrs/rspack-resolver/pull/569))
- *(resolver)* use rustc_hash::FxHasher for DashMap ([#568](https://github.com/unrs/rspack-resolver/pull/568))
- *(resolver)* add example
- *(resolver)* add file system cache ([#547](https://github.com/unrs/rspack-resolver/pull/547))
- *(resolver)* add our own path util for normalization
- *(rust)* rust cargo fmt and fix clippy warnings
- *(resolver)* add oxc_resolver crate

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
