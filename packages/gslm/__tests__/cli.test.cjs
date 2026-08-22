// End-to-end coverage for bin → index.js → napi → gslm-cli. The fixture is a
// normal node:http server, so argv and exit codes run through the real bridge.
const { after, before, describe, it } = require('node:test')
const assert = require('node:assert/strict')
const { mkdtempSync, readFileSync, rmSync, writeFileSync } = require('node:fs')
const { tmpdir } = require('node:os')
const { join } = require('node:path')
const { spawn } = require('node:child_process')
const http = require('node:http')
const { loadConfig, pull, runCli } = require('../index.js')

let server
let baseUrl
let requests = []

before(async () => {
  server = http.createServer((req, res) => {
    let body = ''
    req.on('data', (chunk) => (body += chunk))
    req.on('end', () => {
      requests.push({ method: req.method, url: req.url, body })
      res.writeHead(200, { 'content-type': 'application/json' })
      if (req.method === 'GET') {
        res.end(JSON.stringify({ values: [['key', 'en', 'zh-TW'], ['app.title', 'Title', '標題']] }))
      } else {
        res.end(JSON.stringify({}))
      }
    })
  })
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
  baseUrl = `http://127.0.0.1:${server.address().port}`
})

after(() => server.close())

function makeProject() {
  return mkdtempSync(join(tmpdir(), 'gslm-cli-'))
}

function config() {
  return 'version = 1\nsheet = "sheet-id"\ntab = "i18n"\nlocales = ["en", "zh-TW"]\npath = "locales/{locale}.json"\nformat = "nest"\n'
}

function bin(cwd, ...args) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [join(__dirname, '..', 'bin', 'gslm.js'), ...args], {
      cwd,
      env: {
        ...process.env,
        GSLM_CLI_BASE_URL: baseUrl,
        GSLM_CLI_ACCESS_TOKEN: 'fixture-token',
      },
    })
    let stdout = ''
    let stderr = ''
    child.stdout.on('data', (chunk) => (stdout += chunk))
    child.stderr.on('data', (chunk) => (stderr += chunk))
    child.once('error', reject)
    child.once('close', (status) => resolve({ status, stdout, stderr }))
  })
}

describe('gslm bin CLI', () => {
  it('runs init, pull and push end-to-end', async () => {
    const cwd = makeProject()
    try {
      const initialized = await bin(cwd, 'init')
      assert.equal(initialized.status, 0, initialized.stderr)
      writeFileSync(join(cwd, 'gslm.toml'), config())

      requests = []
      const pulled = await bin(cwd, 'pull')
      assert.equal(pulled.status, 0, pulled.stderr)
      assert.match(readFileSync(join(cwd, 'locales/en.json'), 'utf8'), /"app"/)
      assert.deepEqual(requests.map((request) => request.method), ['GET'])

      requests = []
      const pushed = await bin(cwd, 'push')
      assert.equal(pushed.status, 0, pushed.stderr)
      assert.deepEqual(requests.map((request) => request.method), ['POST', 'PUT'])
      assert.deepEqual(JSON.parse(requests[1].body).values, [
        ['key', 'en', 'zh-TW'],
        ['app.title', 'Title', '標題'],
      ])
    } finally {
      rmSync(cwd, { recursive: true, force: true })
    }
  })

  it('exposes direct runCli and high-level pull', async () => {
    const cwd = makeProject()
    try {
      assert.equal(await runCli(['gslm', 'init'], { cwd }), 0)
      writeFileSync(join(cwd, 'gslm.toml'), config())
      const target = loadConfig({ cwd, env: {} }).targets[0]
      requests = []
      const summary = await pull(target, { baseUrl, accessToken: 'fixture-token' })
      assert.equal(summary.created, 2)
      assert.deepEqual(requests.map((request) => request.method), ['GET'])
    } finally {
      rmSync(cwd, { recursive: true, force: true })
    }
  })
})
