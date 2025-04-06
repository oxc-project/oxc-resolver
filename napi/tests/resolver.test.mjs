import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { assert, test } from 'vitest';

let ResolverFactory;

if (process.env.WASI_TEST) {
  const wasi = await import('../resolver.wasi.cjs');
  ResolverFactory = wasi.ResolverFactory;
} else {
  const napi = await import('../index.js');
  ResolverFactory = napi.ResolverFactory;
}

const currentDir = join(fileURLToPath(import.meta.url), '..');

const enhancedResolveRoot = join(
  currentDir,
  '..',
  '..',
  'fixtures',
  'enhanced_resolve',
  'test',
  'fixtures',
);

// https://github.com/webpack/enhanced-resolve/blob/main/test/resolve.test.js

for (
  const [title, context, request, expected] of [
    [
      'absolute path',
      enhancedResolveRoot,
      join(enhancedResolveRoot, 'main1.js'),
      join(enhancedResolveRoot, 'main1.js'),
    ],
    [
      'file with .js',
      enhancedResolveRoot,
      './main1.js',
      join(enhancedResolveRoot, 'main1.js'),
    ],
    [
      'file without extension',
      enhancedResolveRoot,
      './main1',
      join(enhancedResolveRoot, 'main1.js'),
    ],
    [
      'another file with .js',
      enhancedResolveRoot,
      './a.js',
      join(enhancedResolveRoot, 'a.js'),
    ],
    [
      'another file without extension',
      enhancedResolveRoot,
      './a',
      join(enhancedResolveRoot, 'a.js'),
    ],
    [
      'file in module with .js',
      enhancedResolveRoot,
      'm1/a.js',
      join(enhancedResolveRoot, 'node_modules/m1/a.js'),
    ],
    [
      'file in module without extension',
      enhancedResolveRoot,
      'm1/a',
      join(enhancedResolveRoot, 'node_modules/m1/a.js'),
    ],
    [
      'another file in module without extension',
      enhancedResolveRoot,
      'complexm/step1',
      join(enhancedResolveRoot, 'node_modules/complexm/step1.js'),
    ],
    [
      'from submodule to file in sibling module',
      join(enhancedResolveRoot, 'node_modules/complexm'),
      'm2/b.js',
      join(enhancedResolveRoot, 'node_modules/m2/b.js'),
    ],
    [
      'from nested directory to overwritten file in module',
      join(enhancedResolveRoot, 'multiple_modules'),
      'm1/a.js',
      join(enhancedResolveRoot, 'multiple_modules/node_modules/m1/a.js'),
    ],
    [
      'from nested directory to not overwritten file in module',
      join(enhancedResolveRoot, 'multiple_modules'),
      'm1/b.js',
      join(enhancedResolveRoot, 'node_modules/m1/b.js'),
    ],
    [
      'file with query',
      enhancedResolveRoot,
      './main1.js?query',
      join(enhancedResolveRoot, 'main1.js?query'),
    ],
    [
      'file with fragment',
      enhancedResolveRoot,
      './main1.js#fragment',
      join(enhancedResolveRoot, 'main1.js#fragment'),
    ],
    [
      'file with fragment and query',
      enhancedResolveRoot,
      './main1.js#fragment?query',
      join(enhancedResolveRoot, 'main1.js#fragment?query'),
    ],
    [
      'file with query and fragment',
      enhancedResolveRoot,
      './main1.js?#fragment',
      join(enhancedResolveRoot, 'main1.js?#fragment'),
    ],

    [
      'file with query (unicode)',
      enhancedResolveRoot,
      './测试.js?query',
      join(enhancedResolveRoot, '测试.js?query'),
    ],
    [
      'file with fragment (unicode)',
      enhancedResolveRoot,
      './测试.js#fragment',
      join(enhancedResolveRoot, '测试.js#fragment'),
    ],
    [
      'file with fragment and query (unicode)',
      enhancedResolveRoot,
      './测试.js#fragment?query',
      join(enhancedResolveRoot, '测试.js#fragment?query'),
    ],
    [
      'file with query and fragment (unicode)',
      enhancedResolveRoot,
      './测试.js?#fragment',
      join(enhancedResolveRoot, '测试.js?#fragment'),
    ],

    [
      'file in module with query',
      enhancedResolveRoot,
      'm1/a?query',
      join(enhancedResolveRoot, 'node_modules/m1/a.js?query'),
    ],
    [
      'file in module with fragment',
      enhancedResolveRoot,
      'm1/a#fragment',
      join(enhancedResolveRoot, 'node_modules/m1/a.js#fragment'),
    ],
    [
      'file in module with fragment and query',
      enhancedResolveRoot,
      'm1/a#fragment?query',
      join(enhancedResolveRoot, 'node_modules/m1/a.js#fragment?query'),
    ],
    [
      'file in module with query and fragment',
      enhancedResolveRoot,
      'm1/a?#fragment',
      join(enhancedResolveRoot, 'node_modules/m1/a.js?#fragment'),
    ],
    [
      'differ between directory and file, resolve file',
      enhancedResolveRoot,
      './dirOrFile',
      join(enhancedResolveRoot, 'dirOrFile.js'),
    ],
    [
      'differ between directory and file, resolve directory',
      enhancedResolveRoot,
      './dirOrFile/',
      join(enhancedResolveRoot, 'dirOrFile/index.js'),
    ],
    [
      'find node_modules outside of node_modules',
      join(enhancedResolveRoot, 'browser-module/node_modules'),
      'm1/a',
      join(enhancedResolveRoot, 'node_modules/m1/a.js'),
    ],
    [
      "don't crash on main field pointing to self",
      enhancedResolveRoot,
      './main-field-self',
      join(enhancedResolveRoot, './main-field-self/index.js'),
    ],
    [
      "don't crash on main field pointing to self (2)",
      enhancedResolveRoot,
      './main-field-self2',
      join(enhancedResolveRoot, './main-field-self2/index.js'),
    ],
    // enhanced-resolve has `#` prepended with a `\0`, they are removed from the
    // following 3 expected test results.
    // See https://github.com/webpack/enhanced-resolve#escaping
    [
      'handle fragment edge case (no fragment)',
      enhancedResolveRoot,
      './no#fragment/#/#',
      join(enhancedResolveRoot, 'no#fragment', '#', '#.js'),
    ],
    [
      'handle fragment edge case (fragment)',
      enhancedResolveRoot,
      './no#fragment/#/',
      join(enhancedResolveRoot, 'no.js#fragment') + '/#/',
    ],
    [
      'handle fragment escaping',
      enhancedResolveRoot,
      './no\0#fragment/\0#/\0##fragment',
      join(enhancedResolveRoot, 'no#fragment', '#', '#.js#fragment'),
    ],
  ]
) {
  test(title, () => {
    const resolver = new ResolverFactory({
      modules: ['src/a', 'src/b', 'src/common', 'node_modules'],
      extensions: ['.js', '.jsx', '.ts', '.tsx'],
    });

    assert.equal(resolver.sync(context, request).path, expected);
  });
}

test('resolve pnpm package', () => {
  const rootDir = join(currentDir, '..', '..');
  const pnpmProjectPath = join(rootDir, 'fixtures', 'pnpm');
  const resolver = new ResolverFactory({
    aliasFields: ['browser'],
  });

  const styledComponents = resolver.sync(pnpmProjectPath, 'styled-components');
  assert.deepEqual(
    styledComponents.path,
    join(
      rootDir,
      'node_modules/.pnpm/styled-components@6.1.17_react-dom@19.1.0_react@19.1.0__react@19.1.0/node_modules/styled-components/dist/styled-components.browser.cjs.js',
    ),
  );

  const react = resolver.sync(
    join(
      rootDir,
      'node_modules/.pnpm/styled-components@6.1.17_react-dom@19.1.0_react@19.1.0__react@19.1.0/node_modules/styled-components',
    ),
    'react',
  );
  assert.deepEqual(
    react.path,
    join(
      rootDir,
      'node_modules/.pnpm/react@19.1.0/node_modules/react/index.js',
    ),
  );
});
