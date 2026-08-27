#!/usr/bin/env node
// Acceptance check for spec 0001: run inside an EMPTY project after
//   npm install @gn00678465/google-sheet-languages-model@<version>
// Asserts that (1) the platform sub-package for this host was installed and no
// foreign one was, (2) the public API behaves, (3) the addon that served it
// came from that sub-package, and (4) the bin launcher reports the version.
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
const BIN = 'gslm'
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
  if (platform === 'win32') return `win32-${arch}-msvc`
  if (platform === 'darwin') return `darwin-${arch}`
  if (platform !== 'linux') fail(`unsupported platform ${platform}`)
  // `process.report` names the runtime glibc only when linked against it.
  const glibc = process.report?.getReport()?.header?.glibcVersionRuntime
  return `linux-${arch}-${glibc ? 'gnu' : 'musl'}`
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

// (2) the package behaves. index.js loads the addon lazily, so the API has to
// be exercised before the module cache can say anything about the addon.
const pkg = fromProject(`${SCOPE}/${NAME}`)

const out = pkg.flatten({ a: { b: 'x' }, c: 'y' })
if (JSON.stringify(out) !== JSON.stringify({ 'a.b': 'x', c: 'y' })) {
  fail(`flatten returned ${JSON.stringify(out)}`)
}
if (Object.keys(out).join(',') !== 'a.b,c') fail(`key order not preserved: ${Object.keys(out)}`)
ok('flatten() works and preserves key order')

const v = pkg.version()
if (v !== expectedVersion) fail(`version() returned "${v}", expected "${expectedVersion}"`)
ok(`version() = ${v}`)

// (3) that behaviour came from the sub-package for this host, not a stray build
const loadedNative = Object.keys(require.cache).find((k) => k.endsWith('.node'))
if (!loadedNative) fail('no .node module in require.cache after exercising the API')
if (!loadedNative.includes(`${sep}${wanted}${sep}`)) {
  fail(`loaded ${loadedNative}, which is not inside the expected sub-package ${wanted}`)
}
ok(`binding loaded from ${loadedNative}`)

// (4) bin launcher. clap prints `<program> <version>`.
const expectedBinOut = `${BIN} ${expectedVersion}`
const binOut = execFileSync('npx', ['--no-install', BIN, '--version'], {
  encoding: 'utf8',
  shell: process.platform === 'win32',
}).trim()
if (binOut !== expectedBinOut) fail(`${BIN} --version printed "${binOut}", expected "${expectedBinOut}"`)
ok(`npx ${BIN} --version = ${binOut}`)

console.log('verify-install: all checks passed')
