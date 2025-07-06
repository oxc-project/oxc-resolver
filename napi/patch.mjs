import fs from 'node:fs'

const filename = new URL('index.js', import.meta.url)

let data = fs.readFileSync(filename, 'utf-8')

data = data.replace(
  '\nif (!nativeBinding) {',
  (value) =>
    `
if (!nativeBinding && process.env.SKIP_OXC_RESOLVER_FALLBACK !== '1') {
  try {
    nativeBinding = require('napi-postinstall/fallback')(require.resolve('./package.json'), true)
  } catch (err) {
    loadErrors.push(err)
  }
}
` + value,
)

data = data + `
if (process.versions.pnp) {
  process.env.OXC_RESOLVER_YARN_PNP = '1'
}
`

fs.writeFileSync(filename, data)
