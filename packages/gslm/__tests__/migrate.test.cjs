const assert = require('node:assert/strict')
const { execFileSync, spawnSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const test = require('node:test')

const { loadConfig, migrateLegacyConfig } = require('../index.js')

function tempdir() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'gslm-migrate-'))
}

test('migrateLegacyConfig maps legacy values into deterministic TOML', () => {
  const result = migrateLegacyConfig({
    sheetId: 'sheet-id',
    sheetTitle: 'Main',
    credentials: './secrets/service-account.json',
    languages: ['en', 'zh-TW'],
    directory: 'locales\\common',
    type: 'flat',
  })

  assert.deepEqual(result.warnings, [])
  assert.match(result.toml, /^#:schema https:\/\/raw\.githubusercontent\.com\/gn00678465\/google-sheet-languages-model\/main\/docs\/schema\/v1\.json\n/)
  assert.match(result.toml, /version = 1\n/)
  assert.match(result.toml, /sheet = "sheet-id"\n/)
  assert.match(result.toml, /tab = "Main"\n/)
  assert.match(result.toml, /locales = \["en", "zh-TW"\]\n/)
  assert.ok(result.toml.includes('path = "locales/common/{locale}.json"\n'))
  assert.match(result.toml, /format = "flat"\n/)
  assert.match(result.toml, /\[credentials\]\nfile = "\.\/secrets\/service-account\.json"\n$/)
})

test('migrateLegacyConfig protects inline credentials and documents defaults', () => {
  const objectCredentials = migrateLegacyConfig({
    sheetId: 'id',
    sheetTitle: 'Main',
    credentials: { private_key: 'never print me' },
    languages: ['en'],
    directory: './i18n',
    extra: true,
  })
  assert.match(objectCredentials.toml, /file = "\.\/credentials\.json"/)
  assert.doesNotMatch(objectCredentials.toml, /never print me/)
  assert.deepEqual(objectCredentials.warnings, [
    '舊設定內嵌憑證；請把金鑰另存為 ./credentials.json 並加入 .gitignore。',
    '略過不支援的舊欄位：extra。',
  ])

  const defaults = migrateLegacyConfig({
    sheetId: 'id',
    sheetTitle: 'He said "hello" \\ yes',
    languages: ['en'],
    directory: 'C:\\i18n',
  })
  assert.match(defaults.toml, /tab = "He said \\"hello\\" \\\\ yes"/)
  assert.ok(defaults.toml.includes('path = "C:/i18n/{locale}.json"'))
  assert.match(defaults.toml, /format = "nest"/)
  assert.deepEqual(defaults.warnings, [
    '未提供 credentials；新版將使用 Google Application Default Credentials（會先讀 GOOGLE_APPLICATION_CREDENTIALS）。',
  ])

  assert.throws(() => migrateLegacyConfig(null), /物件/)
  assert.throws(
    () =>
      migrateLegacyConfig({
        sheetId: 'id',
        sheetTitle: 'Main',
        languages: [''],
        directory: './i18n',
      }),
    /非空字串陣列/,
  )
  assert.throws(
    () =>
      migrateLegacyConfig({
        sheetId: 'id',
        sheetTitle: 'Main',
        languages: 'en',
        directory: './i18n',
      }),
    /非空字串陣列/,
  )
})

test('migrateLegacyConfig emits valid TOML basic strings for controls and rejects lone surrogates', () => {
  const value = 'line\u0000tab\tbreak\nquote"slash\\delete\u007f'
  const result = migrateLegacyConfig({
    sheetId: value,
    sheetTitle: value,
    languages: ['en'],
    directory: './i18n',
  })
  assert.match(result.toml, /\\u0000/)
  assert.match(result.toml, /\\t/)
  assert.match(result.toml, /\\n/)
  assert.match(result.toml, /\\"/)
  assert.match(result.toml, /\\\\/)
  assert.match(result.toml, /\\u007f/)

  const directory = tempdir()
  fs.writeFileSync(path.join(directory, 'gslm.toml'), result.toml)
  const config = loadConfig({ cwd: directory, loadDotenv: false })
  assert.equal(config.targets[0].sheet, value)
  assert.equal(config.targets[0].tab, value)

  assert.throws(
    () =>
      migrateLegacyConfig({
        sheetId: 'id',
        sheetTitle: '\ud800',
        languages: ['en'],
        directory: './i18n',
      }),
    /無法表示/,
  )
})

test('migrate command runs without a native binding', () => {
  const packageRoot = tempdir()
  const binDirectory = path.join(packageRoot, 'bin')
  fs.mkdirSync(binDirectory)
  fs.copyFileSync(path.join(__dirname, '..', 'bin', 'gslm.js'), path.join(binDirectory, 'gslm.js'))
  fs.copyFileSync(path.join(__dirname, '..', 'migrate.js'), path.join(packageRoot, 'migrate.js'))
  fs.copyFileSync(path.join(__dirname, '..', 'index.js'), path.join(packageRoot, 'index.js'))
  fs.writeFileSync(
    path.join(packageRoot, 'gslm.config.cjs'),
    "module.exports = { sheetId: 'id', sheetTitle: 'Main', languages: ['en'], directory: 'i18n' }\n",
  )

  const output = execFileSync(process.execPath, [path.join(binDirectory, 'gslm.js'), 'migrate'], {
    cwd: packageRoot,
    encoding: 'utf8',
  })

  assert.match(output, /sheet = "id"/)

  const { migrateLegacyConfig: sdkMigrate } = require(path.join(packageRoot, 'index.js'))
  assert.match(
    sdkMigrate({ sheetId: 'id', sheetTitle: 'Main', languages: ['en'], directory: 'i18n' }).toml,
    /sheet = "id"/,
  )
})

test('gslm migrate writes a loadable TOML config only when requested', () => {
  const directory = tempdir()
  fs.writeFileSync(
    path.join(directory, 'gslm.config.mjs'),
    `export default {
      sheetId: 'sheet-id',
      sheetTitle: 'Main',
      credentials: './credentials.json',
      languages: ['en', 'ja'],
      directory: './translations',
      type: 'flat',
    }
`,
  )
  const bin = path.join(__dirname, '..', 'bin', 'gslm.js')
  const preview = execFileSync(process.execPath, [bin, 'migrate'], {
    cwd: directory,
    encoding: 'utf8',
  })
  assert.match(preview, /sheet = "sheet-id"/)
  assert.equal(fs.existsSync(path.join(directory, 'gslm.toml')), false)

  execFileSync(process.execPath, [bin, 'migrate', '--write'], {
    cwd: directory,
    encoding: 'utf8',
  })
  const config = loadConfig({ cwd: directory, loadDotenv: false })
  assert.equal('credentialHandle' in config.targets[0], false)
  const target = config.targets[0]
  assert.deepEqual(target, {
    name: 'default',
    sheet: 'sheet-id',
    tab: 'Main',
    locales: ['en', 'ja'],
    path: path.join(directory, 'translations/{locale}.json'),
    format: 'flat',
    keySeparator: '.',
    credentials: { kind: 'file', path: path.join(directory, 'credentials.json') },
  })
})

test('gslm migrate reports actionable argument, discovery, and overwrite errors', () => {
  const directory = tempdir()
  const bin = path.join(__dirname, '..', 'bin', 'gslm.js')
  const run = (...args) =>
    spawnSync(process.execPath, [bin, 'migrate', ...args], {
      cwd: directory,
      encoding: 'utf8',
    })

  let result = run()
  assert.equal(result.status, 1)
  assert.match(result.stderr, /找不到舊設定檔/)

  result = run('--from')
  assert.equal(result.status, 1)
  assert.match(result.stderr, /--from 需要一個設定檔路徑/)

  result = run('--unknown')
  assert.equal(result.status, 1)
  assert.match(result.stderr, /不支援的 migrate 參數：--unknown/)

  result = run('--from', 'missing.cjs')
  assert.equal(result.status, 1)
  assert.match(result.stderr, /找不到舊設定檔：/)

  fs.writeFileSync(
    path.join(directory, 'gslm.config.cjs'),
    "module.exports = { sheetId: process.env.GSLM_MIGRATE_TEST_SHEET || 'id', sheetTitle: 'Main', languages: ['en'], directory: 'i18n' }\n",
  )
  fs.writeFileSync(path.join(directory, 'gslm.toml'), 'existing = true\n')

  result = run('--write')
  assert.equal(result.status, 1)
  assert.match(result.stderr, /已存在；使用 --force 才會覆寫/)

  result = run('--write', '--force')
  assert.equal(result.status, 0, result.stderr)
  assert.match(result.stdout, /已寫入 /)
  assert.match(result.stderr, /舊設定使用環境變數；值已在遷移時具體化/)
})
