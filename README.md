<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/Boshen/oxc-assets/main/preview-dark-transparent.png" width="600">
    <img alt="OXC Logo" src="https://raw.githubusercontent.com/Boshen/oxc-assets/main/preview-white.png" width="600">
  </picture>
</p>

<div align="center">

[![Crates.io][crates-badge]][crates-url]
[![Docs.rs][docs-badge]][docs-url]

[![MIT licensed][license-badge]][license-url]
[![Build Status][ci-badge]][ci-url]
[![Code Coverage][code-coverage-badge]][code-coverage-url]
[![CodSpeed Badge][codspeed-badge]][codspeed-url]
[![Sponsors][sponsors-badge]][sponsors-url]
[![Discord chat][discord-badge]][discord-url]

</div>

# Oxc Resolver

Rust port of [enhanced-resolve].

* built-in [tsconfig-paths-webpack-plugin]
  * support extending tsconfig defined in `tsconfig.extends`
  * support paths alias defined in `tsconfig.compilerOptions.paths`
  * support project references defined `tsconfig.references`
* supports in-memory file system via the `FileSystem` trait
* contains `tracing` instrumentation

## Options

The options are aligned with [enhanced-resolve].

| Field            | Default                     | Description                                                                                                                                               |
|------------------|-----------------------------| --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| alias            | []                          | A list of module alias configurations or an object which maps key to value                                                                                |
| aliasFields      | []                          | A list of alias fields in description files                                                                                                               |
| extensionAlias   | {}                          | An object which maps extension to extension aliases                                                                                                       |
| conditionNames   | []                          | A list of exports field condition names                                                                                                                   |
| descriptionFiles | ["package.json"]            | A list of description files to read from                                                                                                                  |
| enforceExtension | false                       | Enforce that a extension from extensions must be used                                                                                                     |
| exportsFields    | ["exports"]                 | A list of exports fields in description files                                                                                                             |
| extensions       | [".js", ".json", ".node"]   | A list of extensions which should be tried for files                                                                                                      |
| fallback         | []                          | Same as `alias`, but only used if default resolving fails                                                                                                 |
| fileSystem       |                             | The file system which should be used                                                                                                                      |
| fullySpecified   | false                       | Request passed to resolve is already fully specified and extensions or main files are not resolved for it (they are still resolved for internal requests) |
| mainFields       | ["main"]                    | A list of main fields in description files                                                                                                                |
| mainFiles        | ["index"]                   | A list of main files in directories                                                                                                                       |
| modules          | ["node_modules"]            | A list of directories to resolve modules from, can be absolute path or folder name                                                                        |
| resolveToContext | false                       | Resolve to a context instead of a file                                                                                                                    |
| preferRelative   | false                       | Prefer to resolve module requests as relative request and fallback to resolving as module                                                                 |
| preferAbsolute   | false                       | Prefer to resolve server-relative urls as absolute paths before falling back to resolve in roots                                                          |
| restrictions     | []                          | A list of resolve restrictions                                                                                                                            |
| roots            | []                          | A list of root paths                                                                                                                                      |
| symlinks         | true                        | Whether to resolve symlinks to their symlinked location                                                                                                   |

### Unimplemented Options

| Field            | Default                     | Description                                                                                                                                               |
|------------------|-----------------------------| --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| cachePredicate   | function() { return true }; | A function which decides whether a request should be cached or not. An object is passed to the function with `path` and `request` properties.             |
| cacheWithContext | true                        | If unsafe cache is enabled, includes `request.context` in the cache key                                                                                   |
| plugins          | []                          | A list of additional resolve plugins which should be applied                                                                                              |
| resolver         | undefined                   | A prepared Resolver to which the plugins are attached                                                                                                     |
| unsafeCache      | false                       | Use this cache object to unsafely cache the successful requests

## Test

Tests are ported from
* [enhanced-resolve](https://github.com/webpack/enhanced-resolve/tree/main/test)
* [tsconfig-path](https://github.com/dividab/tsconfig-paths/blob/master/src/__tests__/data/match-path-data.ts) and [parcel-resolver](https://github.com/parcel-bundler/parcel/tree/v2/packages/utils/node-resolver-core/test/fixture/tsconfig) for tsconfig-paths

Test cases are located in `./src/tests`, fixtures are located in `./tests`

- [x] alias.test.js
- [x] browserField.test.js
- [ ] dependencies.test.js
- [x] exportsField.test.js
- [x] extension-alias.test.js
- [x] extensions.test.js
- [x] fallback.test.js
- [x] fullSpecified.test.js
- [x] identifier.test.js (see unit test in `crates/oxc_resolver/src/request.rs`)
- [x] importsField.test.js
- [x] incorrect-description-file.test.js (need to add ctx.fileDependencies)
- [ ] missing.test.js
- [x] path.test.js (see unit test in `crates/oxc_resolver/src/path.rs`)
- [ ] plugins.test.js
- [ ] pnp.test.js
- [x] resolve.test.js
- [x] restrictions.test.js (partially done, regex is not supported yet)
- [x] roots.test.js
- [x] scoped-packages.test.js
- [x] simple.test.js
- [x] symlink.test.js

Irrelevant tests

- CachedInputFileSystem.test.js
- SyncAsyncFileSystemDecorator.test.js
- forEachBail.test.js
- getPaths.test.js
- pr-53.test.js
- unsafe-cache.test.js
- yield.test.js

[enhanced-resolve]: https://github.com/webpack/enhanced-resolve
[tsconfig-paths-webpack-plugin]: https://github.com/dividab/tsconfig-paths-webpack-plugin

[discord-badge]: https://img.shields.io/discord/1079625926024900739?logo=discord&label=Discord
[discord-url]: https://discord.gg/9uXCAwqQZW
[license-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[license-url]: https://github.com/oxc-project/oxc_resolver/blob/main/LICENSE
[ci-badge]: https://github.com/oxc-project/oxc_resolver/actions/workflows/ci.yml/badge.svg?event=push&branch=main
[ci-url]: https://github.com/oxc-project/oxc_resolver/actions/workflows/ci.yml?query=event%3Apush+branch%3Amain
[code-coverage-badge]: https://codecov.io/github/oxc-project/oxc_resolver/branch/main/graph/badge.svg
[code-coverage-url]: https://codecov.io/gh/oxc-project/oxc_resolver
[sponsors-badge]: https://img.shields.io/github/sponsors/Boshen
[sponsors-url]: https://github.com/sponsors/Boshen
[codspeed-badge]: https://img.shields.io/endpoint?url=https://codspeed.io/badge.json
[codspeed-url]: https://codspeed.io/oxc-project/oxc_resolver
[crates-badge]: https://img.shields.io/crates/d/oxc_resolver?label=crates.io
[crates-url]: https://crates.io/crates/oxc_resolver
[docs-badge]: https://img.shields.io/docsrs/oxc_resolver
[docs-url]: https://docs.rs/oxc_resolver/latest/oxc_resolver/
