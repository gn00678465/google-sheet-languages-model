---
status: ready-for-agent
date: 2026-08-22
adrs: [0001, 0002, 0003, 0004]
depends_on: [0002, 0003, 0004]
---

# Spec 0005：CLI `pull` / `push` / `init` / `schema` 與 Catalog 檔案 I/O

## Problem Statement

重寫至今的四個 spec 把轉換（0002）、Sheets I/O（0003）、設定載入（0004）各自做成可獨立測試的元件，但使用者真正要的指令——`gslm pull` 把 Sheet 同步到本地 JSON、`gslm push` 反向——還不存在。`bin/gslm.js` 目前只有 `--version` 與 `migrate`。舊版 CLI（yargs + `LanguagesModel.loadFromFolder/saveToFolder`）仍在根目錄的 TS 程式碼中，但它讀的是舊設定格式、走的是 `googleapis`，與新元件完全沒有接線。

舊版 CLI 有幾個需要趁重寫修正的行為：push 自動偵測本地檔案的 Format，同一個專案可能每次行為不同；pull 遇到空 Sheet 會把本地所有 Catalog 清成空物件，沒有任何保護；push 時只存在於非 Source locale 的 Orphan key 會被靜默寫上去；本地 JSON 若有非字串葉節點、數字 key 片段，錯誤只在深處的 lodash 呼叫中冒出；輸出訊息全是英文且沒有統一的 exit code；沒有 `--dry-run`。

## Solution

新增 `gslm-cli` crate（library），提供一個入口 `run(argv, options) → exit code`：以 clap 解析 `pull` / `push` / `init` / `schema` 四個子指令（`migrate` 留在 JS，`--version` 改由 clap 提供），透過 `gslm-config` 取得 Target 清單，對每個 Target 依 `path` 樣板讀寫 Catalog JSON 檔，經 `gslm-core` 在 Catalog、Model、Table 之間轉換，用 `gslm-sheets` 讀寫 Tab。所有輸出經注入的 writer，所有檔案與網路端點可注入，讓整條流程在 tempdir + mock server 內完整測試。

napi 暴露 `runCli(argv)`；`bin/gslm.js` 只把 `process.argv` 交給它並設定 exit code。Pull 在「Sheet 解析出的 Model 為空、但本地已有非空 Catalog」時拒絕覆寫（除非 `--force`）；push 列出 Orphan key 警告、檢查本地 Format 與設定相符（`--strict` 時視為錯誤）；兩者皆支援 `--dry-run` 與多 Target。

## User Stories

### 指令與入口

1. As a 使用者, I want `gslm pull`、`gslm push`、`gslm init`、`gslm schema`、`gslm migrate` 五個子指令與 `gslm --version` / `gslm --help`, so that 一個指令涵蓋整個工作流程。
2. As a 使用者, I want `gslm`（無參數）與 `gslm <未知子指令>` 印出 help 並以 exit code 2 結束, so that 誤用立即可見。
3. As a 使用者, I want 參數錯誤（缺值、不合法的 `--format`）印出 clap 的錯誤與用法、exit code 2, so that 與一般 CLI 慣例一致。
4. As a 使用者, I want 執行失敗（設定、檔案、Sheets 錯誤）印出一行以 `error:` 開頭、含錯誤碼（例如 `[PERMISSION_DENIED]`）的訊息到 stderr、exit code 1, so that CI log 可辨識失敗原因。
5. As a 使用者, I want 成功時 exit code 0，且一般進度訊息走 stderr、資料輸出（`schema`、`--dry-run` 的摘要）走 stdout, so that 可以安全地 pipe。
6. As a 使用者, I want `--quiet` 隱藏進度訊息（錯誤仍顯示）、`--verbose` 顯示每個檔案與請求的細節, so that 在 CI 與除錯間切換。
7. As a 使用者, I want 在 TTY 時有顏色、非 TTY 或 `NO_COLOR` 設定時無顏色，並可用 `--color always|never|auto` 覆寫, so that log 檔乾淨、終端機可讀。
8. As a 使用者, I want 所有訊息為繁體中文（與專案其他輸出一致）, so that 與 `migrate` 與設定錯誤訊息風格統一。
9. As a 使用者, I want `Ctrl-C` 中斷時以非零 exit code 結束且不留下寫到一半的檔案, so that 中斷是安全的。

### 設定與覆寫（接 spec 0004）

10. As a 使用者, I want `pull`/`push` 接受 `--config <path>`、`--target <names>`（逗號分隔或重複）、`--sheet`、`--tab`、`--locales`、`--path`、`--format`、`--key-separator`、`--credentials <file>`、`--no-dotenv`, so that 所有 spec 0004 定義的覆寫都能從命令列給。
11. As a 使用者, I want 覆寫規則完全委託 `gslm-config`（flag > env > 檔案 > 預設；多 Target 時欄位覆寫必須配單一 `--target`）, so that SDK 與 CLI 行為一致。
12. As a 使用者, I want 設定載入的警告（例如同目錄多種格式）在執行前印到 stderr, so that 不會被忽略。
13. As a 使用者, I want 執行前印出每個 Target 的摘要（名稱、Sheet、Tab、Locales、path 樣板、Format、憑證種類——不含祕密）, so that 確認對的目標。

### Catalog 檔案 I/O

14. As a 使用者, I want 每個 Locale 的 Catalog 檔案路徑由 `path` 樣板以 `{locale}` 代入決定, so that `locales/{locale}.json` 與 `locales/{locale}/common.json` 都可用。
15. As a 使用者, I want pull 寫檔時自動建立不存在的父目錄, so that 新專案第一次 pull 就能成功。
16. As a 使用者, I want 寫出的 JSON 為 2 空白縮排、key 順序依 Sheet 順序、UTF-8、結尾換行, so that diff 穩定且與舊版輸出相容（舊版無結尾換行，這是刻意的小改動）。
17. As a 使用者, I want pull 以「先寫暫存檔再 rename」的方式寫入, so that 中斷或錯誤不會留下半個檔案。
18. As a 使用者, I want 內容沒有變化的檔案不被重寫（mtime 不動）, so that watch 工具與 git 狀態不會被無意義觸發。
19. As a 使用者, I want push 讀檔時缺少某 Locale 的檔案視為「該 Locale 無任何翻譯」並印警告, so that 新增 Locale 時不必先手動建空檔。
20. As a 使用者, I want push 讀到的 JSON 不是物件、含陣列、含非字串葉節點或數字 key 片段時，錯誤訊息含檔案路徑與 `gslm-core` 的原因, so that 能直接修檔。
21. As a 使用者, I want push 讀檔依設定的 `format` 解讀（`nest` 攤平、`flat` 直接取用；兩者皆透過 `Catalog::from_value`）, so that 行為由設定決定。
22. As a 使用者, I want push 時偵測本地檔案的實際形狀與設定 `format` 不符（`nest` 設定但檔案沒有任何巢狀、或 `flat` 設定但檔案有巢狀物件）時印警告，`--strict` 時改為錯誤, so that 設定與檔案漂移時能發現。
23. As a 使用者, I want pull 時 `flat` 格式輸出 key 含 separator 的一層物件、`nest` 格式輸出巢狀物件, so that 與 spec 0002 的 `Catalog::to_value` 一致。

### Pull

24. As a 使用者, I want `gslm pull` 對每個 Target：讀 Tab → `Model::from_table` → 依 Locale 寫 Catalog 檔, so that 一次同步所有 Target。
25. As a 使用者, I want Sheet 中不存在於設定 `locales` 的欄被忽略、設定中有但 Sheet 沒有的 Locale 報錯（由 core 的 `LocaleNotInHeader`）, so that 欄位對應明確。
26. As a 使用者, I want Sheet 解析後的 Model 為空（無任何 key）而本地已存在任一非空 Catalog 檔時，pull 拒絕執行並說明「Sheet 為空，本地有 N 個 key；若確定要清空請加 `--force`」, so that 誤指向空 Tab 不會毀掉翻譯。
27. As a 使用者, I want `--force` 跳過上述保護, so that 刻意清空可行。
28. As a 使用者, I want pull 完成後印出每個 Locale 寫入的檔案路徑、key 數、以及新增／變更／未變動的檔案數, so that 知道發生了什麼。
29. As a 使用者, I want `pull --dry-run` 不寫任何檔案，只列出「會寫入的檔案與 key 數」與會被觸發的保護, so that 可以先預覽。
30. As a 使用者, I want 缺譯（Sheet 上空儲存格）在 pull 後的 Catalog 中**不出現該 key**（而非寫入空字串）, so that 與 spec 0002 的「缺譯 = 缺 key」規則一致。

### Push

31. As a 使用者, I want `gslm push` 對每個 Target：讀所有 Locale 的 Catalog 檔 → `Model` → `to_table` → `write_tab`, so that 本地成為 Sheet 的真實來源。
32. As a 使用者, I want push 前列出 Orphan key（只存在於非 Source locale 的 key）到 stderr 作為警告，並照 spec 0002 規則附在表格尾端, so that 譯者看得到這些 key，維護者也知道它們存在。
33. As a 使用者, I want `--strict` 時 Orphan key 視為錯誤（不 push）, so that CI 可以強制 Source locale 完整。
34. As a 使用者, I want push 前若所有 Locale 的 Catalog 都為空（或都不存在），拒絕執行並提示（除非 `--force`）, so that 路徑設錯不會把 Sheet 清空。
35. As a 使用者, I want `push --dry-run` 不碰 Sheet，只印出會寫入的列數、欄數、Orphan key 清單, so that 可以先預覽。
36. As a 使用者, I want push 完成後印出寫入的 Tab、列數（含標題列）、各 Locale 的 key 數, so that 知道寫了什麼。
37. As a 使用者, I want push 的 `WriteAfterClearFailed` 錯誤訊息特別說明「Tab 已被清空但寫入失敗，請重試 push」, so that 使用者知道 Sheet 現在是空的。

### Init 與 Schema

38. As a 新使用者, I want `gslm init` 在 cwd 產生 `gslm.toml` 範本（含 `#:schema`、`version = 1`、註解說明每個欄位、`credentials.file` 範例）, so that 不必從文件複製。
39. As a 新使用者, I want `gslm init --format jsonc` 產生 `gslm.jsonc`（含 `$schema`）, so that 選擇 JSON 的團隊也有範本。
40. As a 新使用者, I want `init` 在目標檔已存在時拒絕（除非 `--force`），並在發現舊 `gslm.config.*` 時建議改用 `gslm migrate`, so that 不會覆蓋或重複設定。
41. As a 新使用者, I want `init` 產生的範本能直接被 `loadConfig` 解析（填入佔位值即可執行）, so that 範本永遠不過期。
42. As a 使用者, I want `gslm schema` 把 JSON Schema 印到 stdout, so that 可以 `gslm schema > schema.json` 供 CI 或編輯器使用。

### 多 Target 與錯誤處理

43. As a 使用者, I want 多 Target 時依設定順序逐一執行，單一 Target 失敗即停止並回報是哪個 Target、exit code 1，已完成的 Target 不回滾, so that 行為簡單可預期。
44. As a 使用者, I want `--target` 篩選後只執行指定者，且摘要只列指定者, so that 不會誤動其他 Target。
45. As a 使用者, I want 同一次執行中同一個 Sheet 的 client 被重用（同憑證只換一次 token）, so that 多 Target 不會重複驗證。

### SDK 與維護者

46. As a SDK 使用者, I want `runCli(argv: string[]): Promise<number>` 從套件匯出, so that 可在自己的工具中嵌入 CLI 而不 spawn 子程序。
47. As a SDK 使用者, I want 同時匯出高階函式 `pull(target, options)` / `push(target, options)`（接受 `loadConfig` 回傳的 Target，回傳摘要物件）, so that 不經 argv 也能用同一條流程。
48. As a 維護者, I want clap 解析、config、Catalog 檔案 I/O、pull/push 流程全在 `gslm-cli` crate 並以單一 `run` 入口測試, so that 未來若需獨立二進位（ADR-0002 後果）只要加一個 `main` 包裝。
49. As a 維護者, I want 根目錄舊的 TS CLI（`src/`）在本票中刪除，並把 `example/` 改為新設定與新指令, so that repo 不再有兩套 CLI。
50. As a 維護者, I want CI 的 JS 測試經 `bin/gslm.js` 跑一個完整 pull → push 的端對端（對 fixture server）, so that napi 邊界（argv、exit code、stdout/stderr、tokio runtime）在每個平台被驗證。

## Implementation Decisions

### 模組

- 新 crate `gslm-cli`（library）：依賴 `gslm-core`、`gslm-config`、`gslm-sheets`、`clap`（derive）、`tokio`、`serde_json`、`tempfile`（寫檔原子性可用 `tempfile::NamedTempFile::persist` 或手寫 `.tmp` + rename）。公開介面：`run(argv: Vec<String>, options: RunOptions) -> i32`（async 版本 `run_async`），`RunOptions { cwd, env: Option<map>, stdout: Box<dyn Write>, stderr: Box<dyn Write>, sheets: SheetsOverride { base_url: Option, access_token: Option }, color: Option<ColorChoice>, is_tty: Option<bool> }`；另公開 `pull(target, PullOptions) -> Result<PullSummary>` 與 `push(target, PushOptions) -> Result<PushSummary>` 供 napi 高階函式使用。
- 子模組：`args`（clap 型別）、`catalog_fs`（`read_catalog(path, format, separator)`、`write_catalog(path, &Catalog, format, separator) -> WriteOutcome{Created|Updated|Unchanged}`、`detect_shape(value) -> Shape{Nested|Flat|Empty}`、`render_path(template, locale)`）、`pull`、`push`、`init`、`report`（訊息格式、顏色）。
- `argv[0]` 為程式名，由 bin 傳入 `process.argv.slice(1)`（即 `[bin 路徑, ...args]`）；clap 以 `gslm` 作為顯示名稱。
- `--version` 印 `gslm <version>`，版本沿用 build.rs 注入的套件版本（與 `version()` 相同來源）。

### 指令與參數

- 全域：`--config`、`--target`（`-t`，可重複或逗號分隔）、`--quiet`（`-q`）、`--verbose`（`-v`）、`--color`、`--no-dotenv`。
- `pull`：欄位覆寫 flag、`--dry-run`、`--force`。
- `push`：欄位覆寫 flag（含 `--format`，因為它決定讀檔的解讀方式）、`--dry-run`、`--force`、`--strict`。
- `init`：`--format toml|jsonc`（預設 toml）、`--force`。
- `schema`：無參數。
- `migrate`：由 bin 在呼叫 Rust 之前攔截（argv[2] === 'migrate'），維持 JS 實作；Rust 端的 clap 也登記 `migrate` 子指令，但執行時回傳錯誤「請經由 gslm 的 JS 入口執行」，讓 `--help` 列出完整清單。
- 覆寫 flag 對應 `gslm-config` 的 `Overrides`；`--credentials` 對應 `credentials`（檔案）；`GSLM_CREDENTIALS_JSON` 只能由環境變數提供（無 flag，避免祕密進入 shell history）。

### Catalog 檔案 I/O

- `render_path`：把 `{locale}` 全部替換；Target 的 `path` 已是絕對樣板（spec 0004）。
- 讀：不存在 → `None`（push 印警告、視為空 Catalog）；存在 → `serde_json::from_str`（保序）→ `Catalog::from_value(value, separator)`；錯誤包成 `CliError::Catalog { path, source }`。
- 形狀偵測：物件中任一值為物件 → `Nested`；全部為字串 → `Flat`；空物件 → `Empty`（不警告）。`nest` 設定 + `Flat` 且任一 key 含 separator → 警告／strict 錯誤；`flat` 設定 + `Nested` → 警告／strict 錯誤。`nest` 設定 + 全部 key 不含 separator 的 `Flat` 視為相符（單層巢狀本來就長這樣）。
- 寫：`Catalog::to_value(format, separator)` → `serde_json::to_string_pretty`（2 空白）+ `\n`；先讀既有內容比對，相同則 `Unchanged`；否則寫到同目錄暫存檔再 rename。
- Locale 缺譯：`Model` 的 Catalog 本來就不含缺譯 key，直接寫出即可。

### Pull 流程

1. 載入設定（含 `targets` 篩選）。
2. 對每個 Target：以 client cache（key = 憑證來源 + sheet）取得 `SheetsClient`；`read_tab` → `Model::from_table(rows, locales)`。
3. 空 Model 保護：`model.ordered_keys().is_empty()` 且任一 Locale 的既有檔案讀出非空 Catalog（讀不出來也算「有內容」）→ 無 `--force` 時錯誤 `PULL_EMPTY_SHEET`。
4. 逐 Locale 寫檔（或 dry-run 列出），累計摘要。

### Push 流程

1. 載入設定。
2. 對每個 Target：逐 Locale 讀檔 → `Model::new(locales)` + `set_catalog`；形狀檢查；Orphan key 計算（`model.orphan_keys()`）。
3. 全空保護：所有 Catalog 為空 → 無 `--force` 時錯誤 `PUSH_EMPTY_LOCAL`。
4. `--strict` 且 Orphan 非空或形狀不符 → 錯誤 `PUSH_STRICT`。
5. `to_table` → `write_tab`（或 dry-run 印列數）。

### 錯誤與輸出

- `CliError` 種類：`Usage`（exit 2）、`Config(ConfigError)`、`Catalog { path, source }`、`Sheets(SheetsError)`、`PullEmptySheet { local_keys }`、`PushEmptyLocal`、`PushStrict { reasons }`、`Io { path, source }`、`Interrupted`；皆 exit 1（`Usage` 除外），訊息格式 `error: [CODE] 訊息`，`code()` 對 Config/Sheets 直接沿用底層 code。
- 進度訊息走 stderr（`report::Reporter` 依 quiet/verbose 過濾）；`schema` 與 dry-run 摘要（人類可讀）走 stdout。
- 顏色：`--color auto` 時 `is_tty && NO_COLOR 未設` 才上色；`is_tty` 由 napi 層傳入（`process.stderr.isTTY`），Rust 端不呼叫 isatty（避免 Node 與 Rust 對 fd 的判斷不一致）。
- Ctrl-C：tokio `signal::ctrl_c` 競速；中斷時回傳 `Interrupted`（exit 130），寫檔因原子寫入不會留半檔。

### napi 層

- `runCli(argv: string[], options?: { cwd?, isTty?, color? }): Promise<number>`：`async fn`，stdout/stderr 直接用 Rust 的 `std::io::stdout/stderr`（與 Node 共用 fd；呼叫前 flush）。
- `pull(target: ResolvedTarget, options?: { dryRun?, force? }): Promise<PullSummary>`、`push(target, options?: { dryRun?, force?, strict? }): Promise<PushSummary>`：以 spec 0004 的憑證 handle 取得 client；摘要為純資料物件。
- `bin/gslm.js`：`migrate` 攔截後，其餘 `runCli(process.argv.slice(1), { isTty: process.stderr.isTTY })` → `process.exitCode = code`。

### 清理

- 刪除根目錄 `src/`、`vitest.config.ts`、`tsdown` 相關 build、根 `package.json` 中的舊 scripts 與 `googleapis`/`yargs`/`lodash-es` 依賴；`example/` 改為 `gslm.toml` + 新指令；`AGENTS.md` 的 Commands/Architecture 段落更新為新結構。`pnpm typecheck` 改為只檢查 `packages/gslm/index.d.ts`（或移除）。

## Testing Decisions

- 好的測試：以 `run(argv, options)` 為唯一入口，斷言 exit code、stderr/stdout 內容關鍵字、tempdir 內檔案的存在與內容、mock server 收到的請求；不測 `catalog_fs` 等內部函式（唯一例外：`detect_shape` 與 `render_path` 為純函式可加少量單元測試）。
- `gslm-cli` 整合測試（`tests/cli.rs`）：以 `tempfile` 建專案（`gslm.toml`、既有 Catalog 檔）、`wiremock` 模擬 Sheets（沿用 spec 0003 測試的 matcher 寫法）、`RunOptions` 注入 base URL + access token + `Vec<u8>` writer。案例至少涵蓋：pull 正常（nest/flat、建目錄、Unchanged 偵測）、pull 空 Sheet 保護與 `--force`、pull `--dry-run` 不寫檔、push 正常（含 Orphan 警告與表格尾端）、push 全空保護、push `--strict`、形狀不符警告／錯誤、壞 JSON 錯誤含路徑、多 Target 與 `--target` 篩選、欄位覆寫含糊錯誤、`init` 產出可被 `load` 解析、`schema` 輸出等於 `gslm_config::schema()`、未知子指令 exit 2、`--version`。
- Round-trip：pull 寫出的檔案再 push，mock server 收到的表格等於原始表格（非 Source 欄順序不敏感，依 spec 0002）。
- napi / JS：`__tests__/cli.test.cjs` 以 `node:child_process` 執行 `bin/gslm.js`（fixture server 同 `sheets.test.cjs`）跑 `init` → 改寫設定 → `pull` → `push`，斷言 exit code 與檔案；另測 `runCli` 直接呼叫與 `pull()` 高階函式各一例。
- 先例：`crates/gslm-sheets/tests/client.rs`、`crates/gslm-config/tests/load.rs`、`packages/gslm/__tests__/sheets.test.cjs`。

## Out of Scope

- 獨立二進位 `crates/gslm-cli` 的 `main` 與 GitHub Releases（ADR-0002 後果，待需求）。
- 增量／差異更新、衝突偵測、雙向合併——永遠整表覆寫。
- 自動重試與退避（429/5xx 直接報錯，訊息建議重試）。
- `watch` 模式、互動式提示、shell completion。
- 多 namespace、`source_locale`、`on_missing` 等設定擴充。
- 發佈、`verify-install`、beta tag（重構完成後另行處理）。

## Further Notes

- **`migrate` 的雙入口**：bin 先攔截是為了讓 migrate 在沒有原生 binding 的平台也能跑（spec 0004 F6）；clap 登記同名子指令只為 help 完整性。
- **stdout/stderr 經 Rust 直接寫 fd**：在 napi async 中寫 Rust 的 stdout 與 Node 的 `process.stdout` 共用 fd 1，但 Node 端可能有未 flush 的緩衝；bin 在呼叫 `runCli` 前不輸出任何東西、之後只設 exitCode，避免交錯。測試時 `RunOptions` 注入 writer，不依賴 fd。
- **空 Model 保護的邊界**：header-only 的 Sheet 是「合法的空 Model」（spec 0002），所以保護條件看 key 數而非 `EmptySheet` 錯誤；完全空白的 Sheet 則由 core 回 `EmptySheet` 錯誤，不進入保護邏輯。
- **Unchanged 偵測靠內容比對**而非 hash，檔案小，成本可忽略；比對的是寫出字串與既有檔案位元組，因此舊版無結尾換行的檔案第一次 pull 會被標為 Updated（僅換行差異）。
- **Exit code 130** 給中斷，與 shell 慣例一致；Node 端 `process.exitCode = 130`。
- 刪除舊 TS 程式碼是 breaking change 的最後一步；在此之前 root `pnpm test`（vitest）仍在跑，本票完成後 CI 的 legacy 測試步驟一併移除。
