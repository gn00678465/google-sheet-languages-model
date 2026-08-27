# Google Sheet Languages Model

`@gn00678465/google-sheet-languages-model` 同步 Google Sheets 與本地 i18n
JSON Catalog。它提供 `gslm` 指令，以及可嵌入的 Node API；轉換、設定、檔案
處理與 Sheets I/O 都由 Rust 實作。

## 安裝

```bash
pnpm add -D @gn00678465/google-sheet-languages-model
```

請先啟用 Google Sheets API、建立 service account，並把該帳號的 email 以編輯者
身分分享給目標 Spreadsheet。

## 快速開始

在專案根目錄建立 `gslm.toml`：

```toml
#:schema https://raw.githubusercontent.com/gn00678465/google-sheet-languages-model/main/docs/schema/v1.json
version = 1
sheet = "YOUR_GOOGLE_SHEET_ID"
tab = "i18n"
locales = ["en", "zh-TW"]
path = "locales/{locale}.json"
format = "nest"

[credentials]
file = "./service-account.json"
```

第一個 Locale 是來源語言，並決定 Sheet 的 key 順序。`path` 必須含有
`{locale}`。可使用 `credentials.env = "GSLM_CREDENTIALS_JSON"` 讓憑證來自
環境變數，或省略 `credentials` 使用 Application Default Credentials。

```bash
# 先建立可註解的設定範本
gslm init

# 從 Sheet 寫入本地 JSON
gslm pull

# 先預覽，確認後再寫回 Sheet
gslm push --dry-run
gslm push
```

`pull` 不會用空 Sheet 覆蓋非空本地 Catalog，`push` 不會把全空本地檔案寫回
Sheet；兩種情況都必須明確加上 `--force`。`push` 會警告只在非來源 Locale 存在的
Orphan key，CI 可用 `--strict` 將此警告和檔案格式漂移視為錯誤。

## 指令

```text
gslm pull [--dry-run] [--force] [--config path] [--target names]
gslm push [--dry-run] [--force] [--strict] [--config path] [--target names]
gslm init [--format toml|jsonc] [--force]
gslm schema > gslm.schema.json
gslm migrate [--from legacy-config] [--write] [--force]
```

`--target` 可重複或以逗號分隔。`--sheet`、`--tab`、`--locales`、`--path`、
`--format`、`--key-separator`、`--credentials` 是對設定檔的暫時覆寫；多 Target
時欄位覆寫必須同時指定恰好一個 Target。可用 `--no-dotenv` 停用 `.env`。

設定檔只支援 `gslm.toml`、`gslm.jsonc`、`gslm.json`；舊的可執行
`gslm.config.*` 可透過 `gslm migrate` 轉換。migration 保留在 JavaScript，故即使
系統沒有對應的原生 binding 仍可使用。

## Node API

```js
const { loadConfig, pull, push, runCli } = require('@gn00678465/google-sheet-languages-model')

const target = loadConfig({ cwd: process.cwd() }).targets[0]
await pull(target)
await push(target, { dryRun: true })
const code = await runCli(['gslm', 'schema'])
```

請直接傳遞 `loadConfig()` 回傳的 Target。憑證內容保留在 Rust 的不透明 handle
中，序列化後重新建立的物件無法用於 `pull`、`push` 或 `SheetsClient.fromConfig`。

## 開發

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm -C packages/gslm build:debug
pnpm -C packages/gslm test
```

範例設定和指令見 [example](example/README.md)。
