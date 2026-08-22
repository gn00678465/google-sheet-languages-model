const { test } = require('node:test')
const assert = require('node:assert/strict')
const { execFile } = require('node:child_process')
const { join } = require('node:path')
const { promisify } = require('node:util')

const execFileAsync = promisify(execFile)

test('pull retains a loadConfig Target until its native promise settles', async () => {
  const { stderr } = await execFileAsync(
    process.execPath,
    ['--expose-gc', join(__dirname, 'fixtures', 'target-lifetime.cjs')],
    { cwd: join(__dirname, '..') },
  )
  assert.equal(stderr, '')
})
