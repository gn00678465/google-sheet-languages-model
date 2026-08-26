#!/usr/bin/env node
// Acceptance check for spec 0001: run inside an EMPTY project after
//   npm install @gn00678465/google-sheet-languages-model@<version>
// Asserts that (1) exactly one platform sub-package was installed, (2) the
// binding loads from that very sub-package and `flatten` behaves, (3) the bin
// launcher reports the expected version.
const { readdirSync, existsSync } = require('node:fs')
const { join, sep } = require('node:path')
const { createRequire } = require('node:module')
const { execFileSync } = require('node:child_process')

// This script lives in the repo but inspects a throwaway project in `cwd`.
// A bare require() would resolve against the script's own directory, so every
// lookup of the installed package has to go through the project's root.
const fromProject = createRequire(join(process.cwd(), 'package.json'))

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

// The sub-package this host must end up running.
function expectedSuffix() {
  const { platform, arch } = process
  if (platform === 'win32') return `${arch}-msvc`
  if (platform === 'darwin') return arch
  if (platform !== 'linux') fail(`unsupported platform ${platform}`)
  // `process.report` names the runtime glibc only when linked against it.
  const glibc = process.report?.getReport()?.header?.glibcVersionRuntime
  return `${arch}-${glibc ? 'gnu' : 'musl'}`
}

// (1) the sub-package for this host is installed.
// npm only filters optionalDependencies by the `libc` field from v11 on, so
// older npm also unpacks the sibling libc build. That is inert — the loader
// still has to pick the right one, which (2) asserts — but anything beyond a
// libc sibling means the os/cpu filtering itself is broken.
const scopeDir = join(process.cwd(), 'node_modules', SCOPE)
if (!existsSync(scopeDir)) fail(`${scopeDir} does not exist`)
const platformPkgs = readdirSync(scopeDir).filter((d) => d.startsWith(`${NAME}-`))
const wanted = `${NAME}-${expectedSuffix()}`
if (!platformPkgs.includes(wanted)) {
  fail(`expected ${wanted} to be installed, found: ${platformPkgs.join(', ') || 'none'}`)
}
const foreign = platformPkgs.filter((d) => d !== wanted && d.replace(/-(gnu|musl)$/, '') !== wanted.replace(/-(gnu|musl)$/, ''))
if (foreign.length) {
  fail(`packages for other platforms were installed: ${foreign.join(', ')}`)
}
ok(`platform package installed: ${wanted}${platformPkgs.length > 1 ? ` (plus libc sibling ${platformPkgs.filter((d) => d !== wanted).join(', ')})` : ''}`)

// (2) binding loads — from that sub-package — and behaves
const pkg = fromProject(`${SCOPE}/${NAME}`)
const loadedNative = Object.keys(require.cache).find((k) => k.endsWith('.node'))
if (!loadedNative) fail('no .node module found in require.cache after loading the package')
if (!loadedNative.includes(`${sep}${wanted}${sep}`)) {
  fail(`loaded ${loadedNative}, which is not inside the expected sub-package ${wanted}`)
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
