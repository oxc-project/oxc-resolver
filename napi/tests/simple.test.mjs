import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { assert, test } from 'vitest';

import resolve, { ModuleType, ResolverFactory } from '../index.js';

const cwd = join(fileURLToPath(import.meta.url), '..', '..');

test('simple', () => {
  // `resolve`
  assert.equal(resolve.sync(cwd, './index.js').path, join(cwd, 'index.js'));

  // `ResolverFactory`
  const resolver = new ResolverFactory();
  assert.equal(resolver.sync(cwd, './index.js').path, join(cwd, 'index.js'));

  assert.isAbove(resolver.sync(cwd, './ts').error.length, 0);

  resolver
    .async(cwd, './ts')
    .then((result) => assert.isAbove(result.error.length, 0));

  // Test API
  const newResolver = resolver.cloneWithOptions({});
  newResolver.clearCache();
});

test('module_type', () => {
  const dir = join(cwd, '..', 'fixtures', 'pnpm');

  const esmResolver = new ResolverFactory({
    conditionNames: ['node', 'import'],
    moduleType: true,
  });

  assert.equal(
    esmResolver.sync(dir, 'minimatch').moduleType,
    ModuleType.Module,
  );

  const cjsResolver = esmResolver.cloneWithOptions({
    conditionNames: ['node', 'require'],
    moduleType: true,
  });

  assert.equal(
    cjsResolver.sync(dir, 'minimatch').moduleType,
    ModuleType.CommonJs,
  );
});
