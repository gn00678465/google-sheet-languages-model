const { test } = require('node:test')
const assert = require('node:assert/strict')
const { execFile } = require('node:child_process')
const { join } = require('node:path')
const { promisify } = require('node:util')

const execFileAsync = promisify(execFile)

// Windows has no POSIX signals: process.kill(pid, 'SIGINT') terminates
// unconditionally with a different exit status, so the assertion cannot hold.
test(
  'runCli leaves Node default Ctrl-C handling intact after a sync command',
  { skip: process.platform === 'win32' && 'POSIX signals unavailable on Windows' },
  async () => {
  await assert.rejects(
    execFileAsync(process.execPath, [join(__dirname, 'fixtures', 'run-cli-signal.cjs')], {
      cwd: join(__dirname, '..'),
    }),
    (error) => error && error.signal === 'SIGINT',
  )
},
)
