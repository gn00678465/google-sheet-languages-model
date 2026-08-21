#!/usr/bin/env node
// Thin launcher: the real CLI lives in Rust (ADR-0002). This file only forwards
// argv. Until the clap-based CLI lands, only --version is supported.
const { version: coreVersion } = require('../index.js')
const { version: packageVersion } = require('../package.json')

const args = process.argv.slice(2)

if (args.length === 1 && (args[0] === '--version' || args[0] === '-v')) {
  // Loading the binding above proves the bin → napi → Rust chain works.
  coreVersion()
  console.log(packageVersion)
  process.exit(0)
}

console.error('gslm: commands are not implemented yet in this build (only --version).')
process.exit(2)
