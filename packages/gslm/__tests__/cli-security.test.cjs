const { after, before, test } = require('node:test')
const assert = require('node:assert/strict')
const { mkdtempSync, readFileSync, rmSync, writeFileSync } = require('node:fs')
const { tmpdir } = require('node:os')
const { join } = require('node:path')
const http = require('node:http')
const { runCli } = require('../index.js')

let server
let baseUrl
let requests = 0

before(async () => {
  server = http.createServer((req, res) => {
    requests += 1
    res.writeHead(200, { 'content-type': 'application/json' })
    res.end(JSON.stringify({ values: [['key', 'en'], ['app.title', 'Title']] }))
  })
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
  baseUrl = `http://127.0.0.1:${server.address().port}`
})

after(() => server.close())

function project() {
  const cwd = mkdtempSync(join(tmpdir(), 'gslm-cli-security-'))
  writeFileSync(
    join(cwd, 'gslm.toml'),
    'version = 1\nsheet = "sheet-id"\ntab = "i18n"\nlocales = ["en"]\npath = "locales/{locale}.json"\nformat = "nest"\n[credentials]\nfile = "./missing-service-account.json"\n',
  )
  return cwd
}

test('lint workflow builds the native binding before package tests', () => {
  const workflow = readFileSync(join(__dirname, '..', '..', '..', '.github', 'workflows', 'napi.yml'), 'utf8')
  const lint = workflow.slice(workflow.indexOf('  lint:'), workflow.indexOf('\n  build:'))
  const build = lint.indexOf('pnpm -C packages/gslm build:debug')
  const test = lint.indexOf('pnpm -C packages/gslm test')
  assert.ok(build >= 0 && build < test)
})

test('runCli ignores hidden Sheets environment overrides and accepts explicit options', async () => {
  const cwd = project()
  const oldBaseUrl = process.env.GSLM_CLI_BASE_URL
  const oldToken = process.env.GSLM_CLI_ACCESS_TOKEN
  try {
    process.env.GSLM_CLI_BASE_URL = baseUrl
    process.env.GSLM_CLI_ACCESS_TOKEN = 'test-only-token'
    requests = 0

    assert.equal(await runCli(['gslm', 'pull'], { cwd }), 1)
    assert.equal(requests, 0)

    assert.equal(
      await runCli(['gslm', 'pull'], { cwd, baseUrl, accessToken: 'test-only-token' }),
      0,
    )
    assert.equal(requests, 1)
  } finally {
    if (oldBaseUrl === undefined) delete process.env.GSLM_CLI_BASE_URL
    else process.env.GSLM_CLI_BASE_URL = oldBaseUrl
    if (oldToken === undefined) delete process.env.GSLM_CLI_ACCESS_TOKEN
    else process.env.GSLM_CLI_ACCESS_TOKEN = oldToken
    rmSync(cwd, { recursive: true, force: true })
  }
})
