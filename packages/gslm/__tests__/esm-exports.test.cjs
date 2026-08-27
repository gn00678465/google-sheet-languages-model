const { test } = require('node:test')
const assert = require('node:assert/strict')
const { execFile } = require('node:child_process')
const { join } = require('node:path')
const { pathToFileURL } = require('node:url')
const { promisify } = require('node:util')

const execFileAsync = promisify(execFile)

// index.js is CommonJS, so ESM consumers only get named imports if
// cjs-module-lexer can statically see every export. `module.exports = <var>`
// and the lazy `Object.defineProperty` getters are both invisible to it — the
// `0 && (module.exports = { ... })` annotation at the end of index.js is what
// makes this work. Without it `import { pull } from '...'` throws at link time
// even though index.d.ts declares the export.
test('every documented export is importable as an ESM named import', async () => {
  const entry = pathToFileURL(join(__dirname, '..', 'index.js')).href
  const names = [
    'SheetsClient',
    'loadConfig',
    'configSchema',
    'migrateLegacyConfig',
    'runCli',
    'pull',
    'push',
    'flatten',
    'unflatten',
    'sheetToModel',
    'modelToSheet',
    'orphanKeys',
    'version',
  ]
  const source = `import { ${names.join(', ')} } from ${JSON.stringify(entry)}
const imported = { ${names.join(', ')} }
for (const [name, value] of Object.entries(imported)) {
  if (typeof value !== 'function') throw new Error(\`\${name} is \${typeof value}\`)
}
process.stdout.write(String(Object.keys(imported).length))`

  const { stdout } = await execFileAsync(process.execPath, ['--input-type=module', '-e', source])
  assert.equal(stdout, String(names.length))
})
