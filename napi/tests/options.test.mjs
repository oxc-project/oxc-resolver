import * as path from 'node:path';
import { assert, describe, it } from 'vitest';

import { ResolverFactory } from '../index.js';
import { normalizePath } from './utils.mjs';

const fixtureDir = path.resolve(
  import.meta.dirname,
  '../../fixtures/enhanced-resolve/test/fixtures',
);

describe('option', () => {
  describe('aliasFields', () => {
    it('should allow field string ', () => {
      const resolver = new ResolverFactory({ aliasFields: ['browser'] });
      assert.match(
        normalizePath(resolver.sync(fixtureDir, './browser-module/lib/replaced.js').path),
        /browser-module\/lib\/browser\.js$/,
      );
    });
    it('should allow json path array', () => {
      const resolver = new ResolverFactory({
        aliasFields: [['innerBrowser1', 'field', 'browser']],
      });

      assert.match(
        normalizePath(resolver.sync(fixtureDir, './browser-module/lib/main1.js').path),
        /browser-module\/lib\/main\.js$/,
      );
    });
  });

  describe('exportsFields', () => {
    const createTest = (exportsFields) => {
      const resolver = new ResolverFactory({ exportsFields });
      assert.match(
        normalizePath(
          resolver.sync(
            path.resolve(fixtureDir, './exports-field3'),
            'exports-field',
          ).path,
        ),
        /\/exports-field\/src\/index\.js$/,
      );
    };
    it('should allow string as field item', createTest(['broken']));
    it('should allow json path array as field item', createTest([['broken']]));
  });

  describe('mainFields', () => {
    const createTest = (mainFields) => {
      const resolver = new ResolverFactory({ mainFields });
      assert.match(
        normalizePath(resolver.sync(fixtureDir, '../..').path),
        /\/lib\/index\.js$/,
      );
    };
    it("should use `'main'` as default", createTest(undefined));
    it('should allow field string', createTest('main'));
    it('should allow field array', createTest(['main']));
  });
});
