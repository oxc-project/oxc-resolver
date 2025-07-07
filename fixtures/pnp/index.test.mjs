import { test } from 'node:test'
import assert from 'node:assert'
import pnpapi from 'pnpapi'

import { ResolverFactory } from '../../napi/index.js'

test('resolver', () => {
  const resolver = new ResolverFactory()
  const directory = import.meta.dirname
  const resolution = resolver.sync(directory, 'is-even')
  assert.strictEqual(resolution.path, pnpapi.resolveRequest('is-even', directory))
})
