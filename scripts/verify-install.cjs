#!/usr/bin/env node
// Acceptance check for spec 0001: run inside an EMPTY project after
//   npm install @gn00678465/google-sheet-languages-model@<version>
// Asserts that (1) exactly one platform sub-package was installed, (2) the
// binding loads from that very sub-package and `flatten` behaves, (3) the bin
// launcher reports the expected version.
const { readdirSync, existsSync } = require('node:fs')
const { join, sep } = require('node:path')
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

// (1) exactly one platform sub-package
const scopeDir = join(process.cwd(), 'node_modules', SCOPE)
if (!existsSync(scopeDir)) fail(`${scopeDir} does not exist`)
const platformPkgs = readdirSync(scopeDir).filter((d) => d.startsWith(`${NAME}-`))
if (platformPkgs.length !== 1) {
  fail(`expected exactly 1 platform package, found ${platformPkgs.length}: ${platformPkgs.join(', ')}`)
}
ok(`single platform package installed: ${platformPkgs[0]}`)

// (2) binding loads — from that sub-package — and behaves
const pkg = require(`${SCOPE}/${NAME}`)
const loadedNative = Object.keys(require.cache).find((k) => k.endsWith('.node'))
if (!loadedNative) fail('no .node module found in require.cache after loading the package')
const expectedDir = `${sep}${NAME}-${platformPkgs[0].slice(NAME.length + 1)}${sep}`
if (!loadedNative.includes(expectedDir)) {
  fail(`loaded ${loadedNative}, which is not inside the installed sub-package ${platformPkgs[0]}`)
}
ok(`binding loaded from ${loadedNative}`)

const out = pkg.flatten({ a: { b: 'x' }, c: 'y' })
if (JSON.stringify(out) !== JSON.stringify({ 'a.b': 'x', c: 'y' })) {
  fail(`flatten returned ${JSON.stringify(out)}`)
}
if (Object.keys(out).join(',') !== 'a.b,c') fail(`key order not preserved: ${Object.keys(out)}`)
ok('flatten() works and preserves key order')

const v = pkg.version()
if (v !== expectedVersion) fail(`version() returned "${v}", expected "${expectedVersion}"`)
ok(`version() = ${v}`)

// (3) bin launcher
const binOut = execFileSync('npx', ['--no-install', 'gslm', '--version'], {
  encoding: 'utf8',
  shell: process.platform === 'win32',
}).trim()
if (binOut !== expectedVersion) fail(`gslm --version printed "${binOut}", expected "${expectedVersion}"`)
ok(`npx gslm --version = ${binOut}`)

console.log('verify-install: all checks passed')
