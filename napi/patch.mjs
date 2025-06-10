import fs from 'node:fs'

const fileUrl = new URL('index.js', import.meta.url)

let data = fs.readFileSync(fileUrl, 'utf-8')

data = data.replace(
  '\nif (!nativeBinding) {',
  (value) =>
    `
if (!nativeBinding && process.env.SKIP_UNRS_RESOLVER_FALLBACK !== '1') {
  try {
    nativeBinding = require('./fallback.js');
  } catch (err) {
    loadErrors.push(err)
  }
}
` + value,
)

fs.writeFileSync(fileUrl, data)
