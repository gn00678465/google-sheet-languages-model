// Public entry point. Native exports load on first use so the pure JS
// migration helper remains usable on systems without a matching binary.
const { migrateLegacyConfig } = require('./migrate.js')

const CODE_PREFIX = /^\[([A-Z_]+)\] /
const CREDENTIAL_HANDLE = Symbol('gslm credential handle')
let nativeBinding

const credentialFinalizer = new FinalizationRegistry((handle) => {
  nativeBinding?.releaseConfigCredentials(handle)
})

function binding() {
  nativeBinding ??= require('./binding.js')
  return nativeBinding
}

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
    return new SheetsClient(await lifted(binding().SheetsClient.create(options)))
  }
  static async fromConfig(target) {
    const credentialHandle = target?.[CREDENTIAL_HANDLE]
    if (typeof credentialHandle !== 'string') {
      const error = new Error('Target 不含可用的憑證 handle；請直接使用 loadConfig 回傳的 Target')
      error.code = 'CREDENTIALS'
      throw error
    }
    return new SheetsClient(
      await lifted(binding().SheetsClient.fromConfig({ credentialHandle })),
    )
  }
  readTab(sheetId, tab) {
    return lifted(this.#inner.readTab(sheetId, tab))
  }
  writeTab(sheetId, tab, rows) {
    return lifted(this.#inner.writeTab(sheetId, tab, rows))
  }
}

function loadConfig(options) {
  const config = liftedSync(() => binding().loadConfig(options))
  for (const target of config.targets) {
    const credentialHandle = target.credentialHandle
    delete target.credentialHandle
    Object.defineProperty(target, CREDENTIAL_HANDLE, { value: credentialHandle })
    credentialFinalizer.register(target, credentialHandle)
  }
  return config
}

function configSchema() {
  return liftedSync(() => binding().configSchema())
}

const exported = {
  SheetsClient,
  loadConfig,
  configSchema,
  migrateLegacyConfig,
}

for (const name of [
  'flatten',
  'unflatten',
  'sheetToModel',
  'modelToSheet',
  'orphanKeys',
  'version',
]) {
  Object.defineProperty(exported, name, {
    enumerable: true,
    get() {
      return binding()[name]
    },
  })
}

module.exports = exported
