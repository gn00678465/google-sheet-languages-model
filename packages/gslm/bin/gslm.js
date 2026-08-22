#!/usr/bin/env node
// `migrate` is intentionally JavaScript: importing an old executable config is
// the one compatibility task Rust cannot perform (ADR-0003). The remaining
// CLI will move to Rust in spec 0005.
const { existsSync, readFileSync, writeFileSync } = require('node:fs')
const { dirname, extname, resolve } = require('node:path')
const { pathToFileURL } = require('node:url')
const { migrateLegacyConfig } = require('../migrate.js')

const args = process.argv.slice(2)

function findLegacyConfig(cwd) {
  for (const name of [
    'gslm.config.js',
    'gslm.config.mjs',
    'gslm.config.cjs',
    'gslm.config.ts',
  ]) {
    const candidate = resolve(cwd, name)
    if (existsSync(candidate)) return candidate
  }
  throw new Error(
    '找不到舊設定檔；請在目前目錄建立 gslm.config.{js,mjs,cjs,ts}，或使用 --from <path>。',
  )
}

function parseMigrateArgs(argv) {
  const options = { from: undefined, write: false, force: false }
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index]
    if (arg === '--from') {
      const value = argv[index + 1]
      if (!value) throw new Error('--from 需要一個設定檔路徑')
      options.from = value
      index += 1
    } else if (arg === '--write') {
      options.write = true
    } else if (arg === '--force') {
      options.force = true
    } else {
      throw new Error(`不支援的 migrate 參數：${arg}`)
    }
  }
  return options
}

async function loadLegacyConfig(path) {
  try {
    const module = await import(pathToFileURL(path).href)
    return module.default ?? module
  } catch (error) {
    const hint =
      extname(path) === '.ts'
        ? '；.ts 設定檔需要 Node ≥ 22.18 的原生 type stripping，或請先轉成 .js'
        : ''
    throw new Error(`無法載入舊設定檔 ${path}: ${error.message}${hint}`)
  }
}

async function migrate(argv) {
  const options = parseMigrateArgs(argv)
  const legacyPath = options.from
    ? resolve(process.cwd(), options.from)
    : findLegacyConfig(process.cwd())
  if (!existsSync(legacyPath)) {
    throw new Error(`找不到舊設定檔：${legacyPath}`)
  }
  const legacy = await loadLegacyConfig(legacyPath)
  const result = migrateLegacyConfig(legacy)
  const warnings = [...result.warnings]
  if (/process\.env(?:\.[A-Za-z0-9_]+|\[[^\]]+\])/.test(readFileSync(legacyPath, 'utf8'))) {
    warnings.push(
      '舊設定使用環境變數；值已在遷移時具體化，可改用 GSLM_SHEET 等環境變數覆寫。',
    )
  }

  if (options.write) {
    const output = resolve(dirname(legacyPath), 'gslm.toml')
    if (existsSync(output) && !options.force) {
      throw new Error(`${output} 已存在；使用 --force 才會覆寫。`)
    }
    writeFileSync(output, result.toml, 'utf8')
    process.stdout.write(`已寫入 ${output}\n`)
  } else {
    process.stdout.write(result.toml)
  }
  for (const warning of warnings) {
    process.stderr.write(`gslm migrate: ${warning}\n`)
  }
}

async function main() {
  if (args[0] === 'migrate') {
    await migrate(args.slice(1))
    return
  }
  // Keep native loading after the migrate branch: migration must still work on
  // platforms where no matching native binding has been installed.
  const { runCli } = require('../index.js')
  process.exitCode = await runCli(process.argv.slice(1), { isTty: process.stderr.isTTY })
}

main().catch((error) => {
  process.stderr.write(`gslm: ${error.message}\n`)
  process.exitCode = 1
})
