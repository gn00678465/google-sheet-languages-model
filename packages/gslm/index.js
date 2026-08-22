// Public entry point. Everything comes from the napi binding; this file only
// lifts the `[CODE] ` message prefix that Rust attaches to Sheets errors onto
// `error.code` (napi async functions cannot set a custom code themselves).
const binding = require('./binding.js')
const { migrateLegacyConfig } = require('./migrate.js')

const CODE_PREFIX = /^\[([A-Z_]+)\] /

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
    return new SheetsClient(await lifted(binding.SheetsClient.fromConfig(target)))
  }
  readTab(sheetId, tab) {
    return lifted(this.#inner.readTab(sheetId, tab))
  }
  writeTab(sheetId, tab, rows) {
    return lifted(this.#inner.writeTab(sheetId, tab, rows))
  }
}

function loadConfig(options) {
  return liftedSync(() => binding.loadConfig(options))
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
