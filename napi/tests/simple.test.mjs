import path from 'path';
import { assert, test } from 'vitest';

import resolve, { ResolverFactory } from '../index.js';

const cwd = path.join(__dirname, '..');

test('simple', () => {
  // `resolve`
  assert.equal(resolve.sync(cwd, './index.js').path, path.join(cwd, 'index.js'));

  // `ResolverFactory`
  const resolver = new ResolverFactory();
  assert.equal(resolver.sync(cwd, './index.js').path, path.join(cwd, 'index.js'));

  assert.isAbove(resolver.sync(cwd, './ts').error.length, 0);

  resolver.async(cwd, './ts')
    .then((result) => assert.isAbove(result.error.length, 0));

  // Test API
  const newResolver = resolver.cloneWithOptions({});
  newResolver.clearCache();
});
