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
  assert.deepEqual(config.targets, [
    {
      name: 'default',
      sheet: 'sheet-id',
      tab: 'Main',
      locales: ['en', 'zh-TW'],
      path: path.join(directory, 'locales/{locale}.json'),
      format: 'nest',
      keySeparator: '.',
      credentials: { kind: 'json', env: 'GSLM_TEST_CONFIG_JSON' },
    },
  ])
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

test('SheetsClient.fromConfig can re-read JSON credentials loaded from cwd .env', async () => {
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
  fs.writeFileSync(
    path.join(directory, '.env'),
    "GSLM_TEST_CONFIG_DOTENV_JSON='{\"type\":\"service_account\"}'\n",
  )

  const config = loadConfig({ cwd: directory })
  await assert.rejects(
    SheetsClient.fromConfig(config.targets[0]),
    (error) =>
      error &&
      error.code === 'CREDENTIALS' &&
      /missing client_email/.test(error.message),
  )

  const unrelated = tempdir()
  fs.writeFileSync(
    path.join(unrelated, 'gslm.toml'),
    'version = 1\n' +
      'sheet = "sheet-id"\n' +
      'tab = "Main"\n' +
      'locales = ["en"]\n' +
      'path = "locales/{locale}.json"\n' +
      '[credentials]\n' +
      'env = "GSLM_TEST_CONFIG_DOTENV_JSON"\n',
  )
  assert.throws(
    () => loadConfig({ cwd: unrelated }),
    (error) =>
      error &&
      error.code === 'CONFIG_INVALID' &&
      /GSLM_TEST_CONFIG_DOTENV_JSON/.test(error.message),
  )

  const withoutDotenv = loadConfig({
    cwd: directory,
    loadDotenv: false,
    env: { GSLM_TEST_CONFIG_DOTENV_JSON: '{"type":"service_account"}' },
  })
  await assert.rejects(
    SheetsClient.fromConfig(withoutDotenv.targets[0]),
    (error) =>
      error &&
      error.code === 'CREDENTIALS' &&
      /GSLM_TEST_CONFIG_DOTENV_JSON/.test(error.message),
  )
})
