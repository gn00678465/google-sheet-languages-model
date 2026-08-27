# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 1.0.0

`gslm` 以 Rust 重寫，並改用 napi-rs 提供原生 binding。設定檔、CLI 旗標與
安裝方式都有破壞性變更；舊設定可用 `gslm migrate` 轉換。

### 💥 破壞性變更

- **設定檔改為非可執行格式。** 只讀取 `gslm.toml`、`gslm.jsonc`、`gslm.json`，
  且必須含 `version = 1`。`gslm.config.{js,mjs,cjs,ts}` 不再被載入 ——
  執行 `gslm migrate --write` 轉換既有設定。
- **設定欄位更名。** `sheetId` → `sheet`、`sheetTitle` → `tab`、
  `languages` → `locales`、`type` → `format`；`directory` 由 `path` 取代，
  且必須是含 `{locale}` 的路徑樣板（例如 `locales/{locale}.json`）。
  `locales` 的第一項是來源語言，決定 Sheet 的 key 順序。
- **憑證不得內嵌於設定檔。** 只接受 `credentials.file`、`credentials.env`，
  或省略以使用 Application Default Credentials。舊版可直接把 credentials
  物件寫進設定檔，該用法已移除。
- **CLI 旗標更名。** `--sheet-id` → `--sheet`、`--sheet-title` → `--tab`、
  `--languages` → `--locales`、`--directory` → `--path`。
- **改為原生套件。** 安裝時依平台解析 optionalDependencies，支援
  darwin-x64/arm64、win32-x64-msvc、linux-x64/arm64 的 gnu 與 musl。
- **Node 需求提升至 >= 20。**
- **移除 `googleapis` 相依。** Sheets REST 呼叫由 Rust 直接發出，TLS 根憑證
  隨套件內建。

### 🚀 新增

- Rust 核心：Catalog / Model / Sheet Table 轉換、設定解析、檔案 I/O 與
  Sheets I/O 全部在 Rust 實作，JavaScript 只保留薄的 binding 與 `migrate`。
- 多 Target 設定（`[[targets]]`）與 `--target` 選取，可重複或以逗號分隔。
- `gslm init` 產生含註解的設定範本（`--format toml|jsonc`）。
- `gslm schema` 輸出 JSON Schema；設定檔可用 `#:schema` / `$schema`
  取得編輯器自動完成。
- `.env` 支援。優先序為 CLI 旗標 > 環境變數 > `.env` > 設定檔，
  `--no-dotenv` 可完全停用。
- Node API：`loadConfig`、`pull`、`push`、`runCli`、`SheetsClient`
  與轉換函式，`require()` 與 ESM 具名匯入皆可用。

### 🔒 安全與護欄

- 憑證內容留在 Rust 的不透明 handle 中，`loadConfig()` 只回傳安全中繼資料
  （來源種類與路徑／變數名），憑證本身不會進入 JavaScript。
- `pull` 不會用空 Sheet 覆蓋非空的本地 Catalog；`push` 不會把全空的本地
  Catalog 寫回 Sheet。兩者都必須明確加上 `--force`。
- `push` 會警告只存在於非來源 Locale 的 Orphan key；`--strict` 將該警告與
  檔案格式漂移升級為錯誤，供 CI 使用。
- `--dry-run` 一律不寫入檔案與 Sheet。進度輸出走 stderr，Schema 與 dry-run
  摘要走 stdout。

## v0.5.1

[compare changes](https://github.com/gn00678465/google-sheet-languages-model/compare/v0.5.0...v0.5.1)

### 🏡 Chore

- 移除 credentials.json 的 TypeScript 錯誤註解 ([b7b71a8](https://github.com/gn00678465/google-sheet-languages-model/commit/b7b71a8))
- 移除 pnpm 設定中的版本指定 ([7307f15](https://github.com/gn00678465/google-sheet-languages-model/commit/7307f15))
- 更新 tsconfig.json 的 exclude 設定 ([2dcea5a](https://github.com/gn00678465/google-sheet-languages-model/commit/2dcea5a))
- **ci:** 移除 CI 流程中的 Node.js 版本矩陣設定 ([71d14b9](https://github.com/gn00678465/google-sheet-languages-model/commit/71d14b9))
- 更新 credentials.json 的 TypeScript 錯誤註解以提供更清晰的說明 ([c16950e](https://github.com/gn00678465/google-sheet-languages-model/commit/c16950e))

### ❤️ Contributors

- Madao <gn00678465@gmail.com>

## v0.5.0

### 🚀 Features
- Add CLI interface with pull/push commands
- Support config file for CLI (gslm.config.js)
- Support credentials as object or file path
- Add comprehensive validation for config and inputs

### 🐛 Bug Fixes
- Fix configuration object validation logic
- Fix language code validation regex
- Update path imports to use node:path and node:url

### 📚 Documentation
- Update README with CLI usage examples
- Add commit message guidelines

### ♻️ Refactors
- Refactor project structure and integrate Copilot settings
- Refactor config loading and merging logic

### 📦 Build System
- Migrate from tsup to tsdown

## v0.4.0

### 🚀 Features
- Initial release with programmatic API
- Support for Google Sheets integration
- Bidirectional sync (pull/push)
- Support for nested and flat i18n structures
