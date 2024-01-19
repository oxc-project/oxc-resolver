import path from 'path';
import resolve, { ResolverFactory } from './index.js';
import assert from 'assert';

console.log(`Testing on ${process.platform}-${process.arch}`)

const cwd = process.cwd();

// `resolve`
assert.deepStrictEqual(resolve.sync(cwd, "./index.js").path, path.join(cwd, 'index.js'));

// `ResolverFactory`
const resolver = ResolverFactory.default();
assert.deepStrictEqual(resolver.sync(cwd, "./index.js").path, path.join(cwd, 'index.js'));

assert.strict(resolver.sync(cwd, "./ts").error.length > 0);

const newResolver = resolver.cloneWithOptions({});
newResolver.clearCache();


// custom constructor
const resolver2 = new ResolverFactory(
  {
    extensions: ['.mjs']
  }
);

// After add `.ts` extension, resolver can resolve `ts` as `ts.ts` now
assert.deepStrictEqual(resolver2.sync(cwd, "./test.mjs").path, path.join(cwd, 'test.mjs'));
