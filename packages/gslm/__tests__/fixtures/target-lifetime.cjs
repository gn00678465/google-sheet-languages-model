const assert = require('node:assert/strict')
const { mkdtempSync, rmSync, writeFileSync } = require('node:fs')
const http = require('node:http')
const { tmpdir } = require('node:os')
const { join } = require('node:path')
const { loadConfig, pull } = require('../../index.js')

async function main() {
  let releaseResponse
  const responseGate = new Promise((resolve) => {
    releaseResponse = resolve
  })
  const server = http.createServer(async (_request, response) => {
    await responseGate
    response.writeHead(200, { 'content-type': 'application/json' })
    response.end(JSON.stringify({ values: [['key', 'en'], ['app.title', 'Title']] }))
  })
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
  const cwd = mkdtempSync(join(tmpdir(), 'gslm-target-lifetime-'))
  try {
    writeFileSync(
      join(cwd, 'gslm.toml'),
      'version = 1\nsheet = "sheet-id"\ntab = "i18n"\nlocales = ["en"]\npath = "locales/{locale}.json"\nformat = "nest"\n',
    )
    const started = (() => {
      let target = loadConfig({ cwd, env: {} }).targets[0]
      const weak = new WeakRef(target)
      const operation = pull(target, {
        baseUrl: `http://127.0.0.1:${server.address().port}`,
        accessToken: 'fixture-token',
      })
      target = undefined
      return { weak, operation }
    })()

    for (let index = 0; index < 8; index += 1) {
      global.gc()
      await new Promise((resolve) => setImmediate(resolve))
    }
    assert.notEqual(started.weak.deref(), undefined, 'Target 被 promise 完成前的 GC 回收')

    releaseResponse()
    await started.operation
  } finally {
    server.close()
    rmSync(cwd, { recursive: true, force: true })
  }
}

main().catch((error) => {
  process.stderr.write(`${error.stack || error}\n`)
  process.exitCode = 1
})
