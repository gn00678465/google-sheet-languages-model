const { mkdtempSync, rmSync, writeFileSync } = require('node:fs')
const http = require('node:http')
const { tmpdir } = require('node:os')
const { join } = require('node:path')
const { runCli } = require('../../index.js')

async function main() {
  const server = http.createServer((_request, response) => {
    response.writeHead(200, { 'content-type': 'application/json' })
    response.end(JSON.stringify({ values: [['key', 'en'], ['title', 'Title']] }))
  })
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
  const cwd = mkdtempSync(join(tmpdir(), 'gslm-run-cli-signal-'))
  try {
    writeFileSync(
      join(cwd, 'gslm.toml'),
      'version = 1\nsheet = "sheet-id"\ntab = "i18n"\nlocales = ["en"]\npath = "locales/{locale}.json"\nformat = "nest"\n',
    )
    await runCli(['gslm', 'pull'], {
      cwd,
      baseUrl: `http://127.0.0.1:${server.address().port}`,
      accessToken: 'fixture-token',
    })
  } finally {
    server.close()
    rmSync(cwd, { recursive: true, force: true })
  }
  process.kill(process.pid, 'SIGINT')
}

main().catch((error) => {
  process.stderr.write(`${error.stack || error}\n`)
  process.exitCode = 1
})
