
# Rspack Resolver

> [!NOTE]  
>  This is a fork of [oxc-resolver](https://github.com/oxc-project/oxc-resolver), and will be used in Rspack cause 100% compatible with enhanced-resolve it the non-goal of oxc-resolver itself, we may add enhanced-resolve specific features like [`pnp support`](https://github.com/web-infra-dev/rspack/issues/2236) and [`alternative support`](https://github.com/web-infra-dev/rspack/issues/5052) in the future.

Rust port of [enhanced-resolve].

* built-in [tsconfig-paths-webpack-plugin]
  * support extending tsconfig defined in `tsconfig.extends`
  * support paths alias defined in `tsconfig.compilerOptions.paths`
  * support project references defined `tsconfig.references`
  * support [template variable ${configDir} for substitution of config files directory path](https://github.com/microsoft/TypeScript/pull/58042)
* supports in-memory file system via the `FileSystem` trait
* contains `tracing` instrumentation

## Usage

The following usages apply to both Rust and Node.js; the code snippets are written in JavaScript.

To handle the `exports` field in `package.json`, ESM and CJS need to be differentiated.

### ESM

Per [ESM Resolution algorithm](https://nodejs.org/api/esm.html#resolution-and-loading-algorithm)

> defaultConditions is the conditional environment name array, ["node", "import"].

This means when the caller is an ESM import (`import "module"`), resolve options should be

```javascript
{
  "conditionNames": ["node", "import"]
}
```

### CJS

Per [CJS Resolution algorithm](https://nodejs.org/api/modules.html#all-together)

> LOAD_PACKAGE_EXPORTS(X, DIR)
>
> 5. let MATCH = PACKAGE_EXPORTS_RESOLVE(pathToFileURL(DIR/NAME), "." + SUBPATH,
>   `package.json` "exports", ["node", "require"]) defined in the ESM resolver.

This means when the caller is a CJS require (`require("module")`), resolve options should be

```javascript
{
  "conditionNames": ["node", "require"]
}
```

### Cache

To support both CJS and ESM with the same cache:

```javascript
const esmResolver = ResolverFactory({
  conditionNames: ["node", "import"]
});

const cjsResolver = esmResolver.cloneWithOptions({
  conditionNames: ["node", "require"]
});
```

### Browser Field

From this [non-standard spec](https://github.com/defunctzombie/package-browser-field-spec):

> The `browser` field is provided to JavaScript bundlers or component tools when packaging modules for client side use.

The option is

```javascript
{
  "aliasFields": ["browser"]
}
```

### Main Field

```javascript
{
  "mainFields": ["module", "main"]
}
```

Quoting esbuild's documentation:

* `main` - This is [the standard field](https://docs.npmjs.com/files/package.json#main) for all packages that are meant to be used with node. The name main is hard-coded in to node's module resolution logic itself. Because it's intended for use with node, it's reasonable to expect that the file path in this field is a CommonJS-style module.
* `module` - This field came from a [proposal](https://github.com/dherman/defense-of-dot-js/blob/f31319be735b21739756b87d551f6711bd7aa283/proposal.md) for how to integrate ECMAScript modules into node. Because of this, it's reasonable to expect that the file path in this field is an ECMAScript-style module. This proposal wasn't adopted by node (node uses "type": "module" instead) but it was adopted by major bundlers because ECMAScript-style modules lead to better tree shaking, or dead code removal.
* `browser` - This field came from a [proposal](https://gist.github.com/defunctzombie/4339901/49493836fb873ddaa4b8a7aa0ef2352119f69211) that allows bundlers to replace node-specific files or modules with their browser-friendly versions. It lets you specify an alternate browser-specific entry point. Note that it is possible for a package to use both the browser and module field together (see the note below).

### Errors

* `Error: Package subpath '.' is not defined by "exports" in` - occurs when resolving without `conditionNames`.

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

