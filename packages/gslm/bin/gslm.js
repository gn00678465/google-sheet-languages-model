#!/usr/bin/env node
// Thin launcher: the real CLI lives in Rust (ADR-0002). This file only forwards
// argv. Until the clap-based CLI lands, only --version is supported.
const { version } = require('../index.js')

const args = process.argv.slice(2)

if (args.length === 1 && (args[0] === '--version' || args[0] === '-v')) {
  // version() comes from the Rust binding, so this proves bin → napi → Rust.
  console.log(version())
  process.exitCode = 0
} else {
  console.error('gslm: commands are not implemented yet in this build (only --version).')
  process.exitCode = 2
}
