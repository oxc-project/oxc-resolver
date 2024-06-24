# Oxc Resolver Napi Binding

See

* index.d.ts for `resolveSync` and `ResolverFactory` API.
* [README.md](https://github.com/oxc-project/oxc-resolver?tab=readme-ov-file#oxc-resolver) for options.

## ESM

```javascript
import path from 'path';
import resolve, { ResolverFactory } from './index.js';
import assert from 'assert';

// `resolve`
assert(resolve.sync(process.cwd(), "./index.js").path, path.join(cwd, 'index.js'));

// `ResolverFactory`
const resolver = new ResolverFactory();
assert(resolver.sync(process.cwd(), "./index.js").path, path.join(cwd, 'index.js'));
```
