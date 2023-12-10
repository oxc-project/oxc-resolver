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


// custom constructor
const resolver2 = new ResolverFactory(
  {
    extensions: ['.js', '.ts', '.node']
  }
);

// After add `.ts` extension, resolver can resolve `ts` as `ts.ts` now
assert.deepStrictEqual(resolver2.sync(cwd, "./ts").path, path.join(cwd, 'ts.ts'));

