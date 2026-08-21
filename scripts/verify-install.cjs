#!/usr/bin/env node
// Acceptance check for spec 0001: run inside an EMPTY directory after
//   npm install @gn00678465/google-sheet-languages-model@<version>
// Asserts that (1) exactly one platform sub-package was installed and it
// matches the current platform, (2) the binding loads and `flatten` behaves,
// (3) the bin launcher reports the expected version.
const { readdirSync, existsSync } = require('node:fs')
const { join } = require('node:path')
const { execFileSync } = require('node:child_process')

const SCOPE = '@gn00678465'
const NAME = 'google-sheet-languages-model'
const expectedVersion = process.argv[2]
if (!expectedVersion) {
  console.error('usage: verify-install.cjs <expected-version>')
  process.exit(2)
}

const fail = (msg) => {
  console.error(`✗ ${msg}`)
  process.exit(1)
}
const ok = (msg) => console.log(`✓ ${msg}`)

// (1) exactly one platform sub-package, matching this platform
const scopeDir = join(process.cwd(), 'node_modules', SCOPE)
if (!existsSync(scopeDir)) fail(`${scopeDir} does not exist`)
const platformPkgs = readdirSync(scopeDir).filter((d) => d.startsWith(`${NAME}-`))
if (platformPkgs.length !== 1) {
  fail(`expected exactly 1 platform package, found ${platformPkgs.length}: ${platformPkgs.join(', ')}`)
}
const libc = (() => {
  if (process.platform !== 'linux') return ''
  const report = process.report?.getReport?.()
  const isMusl = report?.header?.glibcVersionRuntime === undefined
  return isMusl ? '-musl' : '-gnu'
})()
const expectedSuffix =
  process.platform === 'win32'
    ? `win32-${process.arch}-msvc`
    : `${process.platform}-${process.arch}${libc}`
if (platformPkgs[0] !== `${NAME}-${expectedSuffix}`) {
  fail(`platform package ${platformPkgs[0]} does not match expected ${NAME}-${expectedSuffix}`)
}
ok(`single platform package installed: ${platformPkgs[0]}`)

// (2) binding loads and behaves
const pkg = require(`${SCOPE}/${NAME}`)
const out = pkg.flatten({ a: { b: 'x' }, c: 'y' })
if (JSON.stringify(out) !== JSON.stringify({ 'a.b': 'x', c: 'y' })) {
  fail(`flatten returned ${JSON.stringify(out)}`)
}
if (Object.keys(out).join(',') !== 'a.b,c') fail(`key order not preserved: ${Object.keys(out)}`)
ok('flatten() works and preserves key order')

const v = pkg.version()
if (typeof v !== 'string' || v.length === 0) fail('version() returned empty')
ok(`version() = ${v}`)

// (3) bin launcher
const binOut = execFileSync('npx', ['--no-install', 'gslm', '--version'], {
  encoding: 'utf8',
  shell: process.platform === 'win32',
}).trim()
if (binOut !== expectedVersion) fail(`gslm --version printed "${binOut}", expected "${expectedVersion}"`)
ok(`npx gslm --version = ${binOut}`)

console.log('verify-install: all checks passed')
