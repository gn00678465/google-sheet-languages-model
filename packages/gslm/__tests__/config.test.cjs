const assert = require('node:assert/strict')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const test = require('node:test')

const { configSchema, loadConfig } = require('../index.js')

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
