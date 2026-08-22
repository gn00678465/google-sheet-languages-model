const assert = require('node:assert/strict')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const test = require('node:test')

const { configSchema, loadConfig, SheetsClient } = require('../index.js')

function tempdir() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'gslm-config-'))
}

test('loadConfig resolves a config file into safe SDK data', () => {
  const directory = tempdir()
  fs.writeFileSync(
    path.join(directory, 'gslm.toml'),
    'version = 1\n' +
      'sheet = "sheet-id"\n' +
      'tab = "Main"\n' +
      'locales = ["en", "zh-TW"]\n' +
      'path = "locales/{locale}.json"\n' +
      '[credentials]\n' +
      'env = "GSLM_TEST_CONFIG_JSON"\n',
  )

  const config = loadConfig({
    cwd: directory,
    env: { GSLM_TEST_CONFIG_JSON: '{"type":"service_account"}' },
  })

  assert.equal(config.configPath, path.join(directory, 'gslm.toml'))
  assert.equal('credentialHandle' in config.targets[0], false)
  assert.equal(JSON.stringify(config).includes('gslm-credential-'), false)
  const target = config.targets[0]
  assert.deepEqual(target, {
      name: 'default',
      sheet: 'sheet-id',
      tab: 'Main',
      locales: ['en', 'zh-TW'],
      path: path.join(directory, 'locales/{locale}.json'),
      format: 'nest',
      keySeparator: '.',
      credentials: { kind: 'json', env: 'GSLM_TEST_CONFIG_JSON' },
  })
})

test('loadConfig surfaces stable ConfigError codes and configSchema has its id', () => {
  const directory = tempdir()
  fs.writeFileSync(path.join(directory, 'gslm.toml'), 'version = 1\nsheetId = "old"\n')

  assert.throws(
    () => loadConfig({ cwd: directory }),
    (error) => error && error.code === 'CONFIG_INVALID' && /sheet/.test(error.message),
  )
  assert.equal(
    configSchema().$id,
    'https://gn00678465.github.io/google-sheet-languages-model/schema/v1.json',
  )
})

test('SheetsClient.fromConfig retains loadConfig credential values without exposing them', async () => {
  const directory = tempdir()
  fs.writeFileSync(
    path.join(directory, 'gslm.toml'),
    'version = 1\n' +
      'sheet = "sheet-id"\n' +
      'tab = "Main"\n' +
      'locales = ["en"]\n' +
      'path = "locales/{locale}.json"\n' +
      '[credentials]\n' +
      'env = "GSLM_TEST_CONFIG_DOTENV_JSON"\n',
  )
  const config = loadConfig({
    cwd: directory,
    loadDotenv: false,
    env: { GSLM_TEST_CONFIG_DOTENV_JSON: '{"type":"service_account"}' },
  })
  assert.doesNotMatch(JSON.stringify(config), /service_account/)
  await assert.rejects(
    SheetsClient.fromConfig(config.targets[0]),
    (error) =>
      error &&
      error.code === 'CREDENTIALS' &&
      /missing client_email/.test(error.message),
  )
  await assert.rejects(
    SheetsClient.fromConfig(JSON.parse(JSON.stringify(config.targets[0]))),
    (error) => error && error.code === 'CREDENTIALS' && /直接使用 loadConfig/.test(error.message),
  )

  const overridden = loadConfig({
    cwd: directory,
    loadDotenv: false,
    overrides: { credentialsJson: '{"type":"service_account"}' },
  })
  await assert.rejects(
    SheetsClient.fromConfig(overridden.targets[0]),
    (error) =>
      error &&
      error.code === 'CREDENTIALS' &&
      /missing client_email/.test(error.message),
  )
})

test('loadConfig never echoes a malformed dotenv secret', () => {
  const directory = tempdir()
  fs.writeFileSync(
    path.join(directory, 'gslm.toml'),
    'version = 1\nsheet = "sheet-id"\ntab = "Main"\nlocales = ["en"]\npath = "locales/{locale}.json"\n',
  )
  const secret = 'never-show-this-dotenv-secret'
  fs.writeFileSync(path.join(directory, '.env'), `SERVICE_ACCOUNT="${secret}\n`)

  assert.throws(
    () => loadConfig({ cwd: directory }),
    (error) =>
      error && error.code === 'CONFIG_PARSE' && !error.message.includes(secret),
  )
})
