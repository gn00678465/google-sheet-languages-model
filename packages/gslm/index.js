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
    return new SheetsClient(
      await lifted(binding().SheetsClient.fromConfig({ credentialHandle: credentialHandleFor(target) })),
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

function credentialHandleFor(target) {
  const credentialHandle = target?.[CREDENTIAL_HANDLE]
  if (typeof credentialHandle !== 'string') {
    const error = new Error('Target 不含可用的憑證 handle；請直接使用 loadConfig 回傳的 Target')
    error.code = 'CREDENTIALS'
    throw error
  }
  return credentialHandle
}

function targetForNative(target) {
  return { ...target, credentialHandle: credentialHandleFor(target) }
}

function runCli(argv, options) {
  return lifted(binding().runCli(argv, options))
}

async function retainTarget(target, operation) {
  const keepAlive = { target }
  try {
    return await operation
  } finally {
    // The native credential registry belongs to this target. Keep the actual
    // JS object alive until N-API has completed its asynchronous operation.
    keepAlive.target = undefined
  }
}

function pull(target, options) {
  return retainTarget(target, lifted(binding().pull(targetForNative(target), options)))
}

function push(target, options) {
  return retainTarget(target, lifted(binding().push(targetForNative(target), options)))
}

const exported = {
  SheetsClient,
  loadConfig,
  configSchema,
  migrateLegacyConfig,
  runCli,
  pull,
  push,
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

// `module.exports = <variable>` plus dynamic getters is invisible to
// cjs-module-lexer, so Node finds no named exports and
// `import { pull } from '@gn00678465/google-sheet-languages-model'` throws.
// This never-executed assignment is the annotation the lexer does read; it
// must list every name above.
0 &&
  (module.exports = {
    SheetsClient,
    loadConfig,
    configSchema,
    migrateLegacyConfig,
    runCli,
    pull,
    push,
    flatten,
    unflatten,
    sheetToModel,
    modelToSheet,
    orphanKeys,
    version,
  })
