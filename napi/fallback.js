const { execFileSync } = require('node:child_process')

const pkg = require('unrs-resolver/package.json')

const userAgent =
  (process.env.npm_config_user_agent || '').split('/')[0] || 'npm'

const EXECUTORS = {
  npm: 'npx',
  pnpm: 'pnpm',
  yarn: 'yarn',
  bun: 'bun',
  deno: (args) => ['deno', 'run', `npm:${args[0]}`, ...args.slice(1)],
}

const executor = EXECUTORS[userAgent]

if (!executor) {
  console.error(
    `Unsupported package manager: ${userAgent}. Supported managers are: ${Object.keys(
      EXECUTORS,
    ).join(', ')}.`,
  )
  process.exitCode = 1
  return
}

function constructCommand(value, args) {
  const list = typeof value === 'function' ? value(args) : [value].concat(args)
  return {
    command: list[0],
    args: list.slice(1),
  }
}

const { command, args } = constructCommand(executor, [
  'napi-postinstall',
  'unrs-resolver',
  pkg.version,
  'check',
])

execFileSync(command, args, {
  cwd: __dirname,
  stdio: 'inherit',
})

process.env.SKIP_UNRS_RESOLVER_FALLBACK = '1'

const UNRS_RESOLVER_PATH = require.resolve('unrs-resolver')

delete require.cache[UNRS_RESOLVER_PATH]

module.exports = require('unrs-resolver')
