import assert from 'assert';
import path from 'path';
import resolve, { ResolverFactory } from './index.js';

console.log(`Testing on ${process.platform}-${process.arch}`);

const dir = import.meta.dirname;

// `resolve`
assert.deepStrictEqual(
  resolve.sync(dir, './index.js').path,
  path.join(dir, 'index.js'),
);

// `ResolverFactory`
const resolver = new ResolverFactory();
assert.deepStrictEqual(
  resolver.sync(dir, './index.js').path,
  path.join(dir, 'index.js'),
);

assert.strict(resolver.sync(dir, './ts').error.length > 0);

resolver
  .async(dir, './ts')
  .then((result) => assert.strict(result.error.length > 0));

const newResolver = resolver.cloneWithOptions({});
newResolver.clearCache();

// custom constructor
const resolver2 = new ResolverFactory({
  extensions: ['.mjs'],
});

// After add `.ts` extension, resolver can resolve `ts` as `ts.ts` now
assert.deepStrictEqual(
  resolver2.sync(dir, './test.mjs').path,
  path.join(dir, 'test.mjs'),
);
