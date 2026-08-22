// Public entry point. Everything comes from the napi binding; this file only
// lifts the `[CODE] ` message prefix that Rust attaches to Sheets errors onto
// `error.code` (napi async functions cannot set a custom code themselves).
const binding = require('./binding.js')
const { migrateLegacyConfig } = require('./migrate.js')
const { join, resolve } = require('node:path')

const CODE_PREFIX = /^\[([A-Z_]+)\] /
const targetContexts = new WeakMap()

function liftCode(err) {
  if (err instanceof Error) {
    const m = CODE_PREFIX.exec(err.message)
    if (m) {
      err.code = m[1]
      err.message = err.message.slice(m[0].length)
    }
  }
  return err
}

async function lifted(promise) {
  try {
    return await promise
  } catch (err) {
    throw liftCode(err)
  }
}

function liftedSync(callback) {
  try {
    return callback()
  } catch (err) {
    throw liftCode(err)
  }
}

class SheetsClient {
  #inner
  constructor(inner) {
    this.#inner = inner
  }
  static async create(options) {
    return new SheetsClient(await lifted(binding.SheetsClient.create(options)))
  }
  static async fromConfig(target) {
    const context = targetContexts.get(target)
    const cwd = context?.cwd ?? process.cwd()
    const nativeTarget = {
      credentials: target.credentials,
      dotenvPath: context?.loadDotenv === false ? undefined : join(cwd, '.env'),
    }
    return new SheetsClient(await lifted(binding.SheetsClient.fromConfig(nativeTarget)))
  }
  readTab(sheetId, tab) {
    return lifted(this.#inner.readTab(sheetId, tab))
  }
  writeTab(sheetId, tab, rows) {
    return lifted(this.#inner.writeTab(sheetId, tab, rows))
  }
}

function loadConfig(options) {
  const config = liftedSync(() => binding.loadConfig(options))
  const context = {
    cwd: resolve(options?.cwd ?? process.cwd()),
    loadDotenv: options?.loadDotenv ?? true,
  }
  for (const target of config.targets) targetContexts.set(target, context)
  return config
}

function configSchema() {
  return liftedSync(() => binding.configSchema())
}

module.exports = {
  ...binding,
  SheetsClient,
  loadConfig,
  configSchema,
  migrateLegacyConfig,
}
