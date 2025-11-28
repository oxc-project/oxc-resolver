import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { assert, describe, test } from 'vitest';

let ResolverFactory;

if (process.env.WASI_TEST) {
  const wasi = await import('../resolver.wasi.cjs');
  ResolverFactory = wasi.ResolverFactory;
} else {
  const napi = await import('../index.js');
  ResolverFactory = napi.ResolverFactory;
}

const currentDir = join(fileURLToPath(import.meta.url), '..');
const rootDir = join(currentDir, '..', '..');
const fixturesDir = join(rootDir, 'fixtures');
const enhancedResolveRoot = join(
  fixturesDir,
  'enhanced-resolve',
  'test',
  'fixtures',
);

// ESM allows file:// protocol URLs for module specifiers
// See: https://nodejs.org/api/esm.html#urls

describe.skipIf(process.env.WASI_TEST)('file:// protocol', () => {
  test('with absolute path', () => {
    const resolver = new ResolverFactory({
      extensions: ['.js', '.jsx', '.ts', '.tsx'],
    });

    const main1Path = join(enhancedResolveRoot, 'main1.js');
    const fileUrl = pathToFileURL(main1Path).href;

    const result = resolver.sync(enhancedResolveRoot, fileUrl);

    assert.equal(result.path, main1Path);
  });

  test('with query string', () => {
    const resolver = new ResolverFactory({
      extensions: ['.js', '.jsx', '.ts', '.tsx'],
    });

    const main1Path = join(enhancedResolveRoot, 'main1.js');
    const fileUrl = pathToFileURL(main1Path).href + '?query=value';

    const result = resolver.sync(enhancedResolveRoot, fileUrl);

    assert.ok(result.path.includes('main1.js'));
    assert.ok(result.path.includes('?query=value'));
  });

  test('with fragment', () => {
    const resolver = new ResolverFactory({
      extensions: ['.js', '.jsx', '.ts', '.tsx'],
    });

    const main1Path = join(enhancedResolveRoot, 'main1.js');
    const fileUrl = pathToFileURL(main1Path).href + '#fragment';

    const result = resolver.sync(enhancedResolveRoot, fileUrl);

    assert.ok(result.path.includes('main1.js'));
    assert.ok(result.path.includes('#fragment'));
  });

  test('with query and fragment', () => {
    const resolver = new ResolverFactory({
      extensions: ['.js', '.jsx', '.ts', '.tsx'],
    });

    const main1Path = join(enhancedResolveRoot, 'main1.js');
    const fileUrl = pathToFileURL(main1Path).href + '?query=value#fragment';

    const result = resolver.sync(enhancedResolveRoot, fileUrl);

    assert.ok(result.path.includes('main1.js'));
    assert.ok(result.path.includes('?query=value#fragment'));
  });

  test('with unicode path', () => {
    const resolver = new ResolverFactory({
      extensions: ['.js', '.jsx', '.ts', '.tsx'],
    });

    const unicodePath = join(enhancedResolveRoot, '测试.js');
    const fileUrl = pathToFileURL(unicodePath).href;

    const result = resolver.sync(enhancedResolveRoot, fileUrl);

    assert.equal(result.path, unicodePath);
  });

  test('with percent-encoded special characters', () => {
    const resolver = new ResolverFactory({
      extensions: ['.js', '.jsx', '.ts', '.tsx'],
    });

    // Test that percent-encoded characters in file URLs are handled
    // Node.js requires # to be encoded as %23 in file URLs
    const main1Path = join(enhancedResolveRoot, 'main1.js');
    const fileUrl = pathToFileURL(main1Path).href;

    // Manually create a URL with encoded characters
    const encodedUrl = fileUrl.replace('main1.js', 'main%231.js');

    const result = resolver.sync(enhancedResolveRoot, encodedUrl);

    // This file doesn't exist, so we expect an error
    assert.ok(result.error);
  });

  test('with directory path', () => {
    const resolver = new ResolverFactory({
      extensions: ['.js', '.jsx', '.ts', '.tsx'],
    });

    const dirPath = join(enhancedResolveRoot, 'dirOrFile/');
    const fileUrl = pathToFileURL(dirPath).href;

    const result = resolver.sync(enhancedResolveRoot, fileUrl);

    // Should resolve to index.js in the directory
    assert.ok(result.path.includes('dirOrFile'));
    assert.ok(result.path.includes('index.js'));
  });

  test('with relative path segments (should error)', () => {
    const resolver = new ResolverFactory({
      extensions: ['.js', '.jsx', '.ts', '.tsx'],
    });

    // file:// URLs with relative path segments like './main.js' are invalid
    // This should error
    const result = resolver.sync(enhancedResolveRoot, 'file://./main.js');

    // Should return an error for malformed file URLs
    assert.ok(result.error);
  });

  test('async resolution', async () => {
    const resolver = new ResolverFactory({
      extensions: ['.js', '.jsx', '.ts', '.tsx'],
    });

    const main1Path = join(enhancedResolveRoot, 'main1.js');
    const fileUrl = pathToFileURL(main1Path).href;

    const result = await resolver.async(enhancedResolveRoot, fileUrl);

    assert.equal(result.path, main1Path);
  });
});
