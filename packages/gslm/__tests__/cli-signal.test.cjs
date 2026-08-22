const { test } = require('node:test')
const assert = require('node:assert/strict')
const { execFile } = require('node:child_process')
const { join } = require('node:path')
const { promisify } = require('node:util')

const execFileAsync = promisify(execFile)

test('runCli leaves Node default Ctrl-C handling intact after a sync command', async () => {
  await assert.rejects(
    execFileAsync(process.execPath, [join(__dirname, 'fixtures', 'run-cli-signal.cjs')], {
      cwd: join(__dirname, '..'),
    }),
    (error) => error && error.signal === 'SIGINT',
  )
})
