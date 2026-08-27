---
status: done
date: 2026-08-22
adrs: [0001, 0002, 0003]
depends_on: [0002, 0003]
---

# Spec 0004：TOML 設定檔與 `gslm migrate`

## Problem Statement

舊版 CLI 只接受可執行的 `gslm.config.{js,ts,mjs,cjs}`，由 Node 動態 `import` 取得設定物件。依 ADR-0001 CLI 核心改為 Rust 後無法再執行 JS；ADR-0003 已決定新版設定檔以 TOML 為主、JSON/JSONC 為輔，並禁止在設定檔內嵌 Service Account 物件。目前 Rust 端完全沒有設定檔的概念：`gslm-core` 只做轉換、`gslm-sheets` 只做 I/O，pull/push 需要的「Sheet、Tab、Locales、檔案路徑、Format、Credentials」都還沒有一個統一的來源與優先序。

舊版設定另有幾個沒寫成規格的問題：`directory` 只能對應「一個目錄、每個 Locale 一個 `<locale>.json`」；`type` 只影響 pull、push 則自動偵測，同一個專案每次 push 的行為可能不同；`credentials` 可以是物件，鼓勵把金鑰和設定一起 commit；沒有 `version` 欄位，schema 無法演進；相對路徑相對於 cwd 而非設定檔位置。

既有使用者升級時需要一條明確的遷移路徑：舊設定檔只有 Node 能執行，所以轉換工具必須放在 JS 端。

## Solution

新增 `gslm-config` crate，負責：在 cwd 往上探索 `gslm.toml` → `gslm.jsonc` → `gslm.json`（停在含 `.git` 的目錄或檔案系統根）；解析三種格式為同一個 schema；驗證語意規則；套用 `GSLM_*` 環境變數與 CLI flag 覆寫（優先序 flag > env > 檔案 > 預設）；把「頂層預設 + `[[targets]]`」展開成一串已解析的 Target（每個都含絕對路徑、完整 Locales、Format、Key separator、Credentials 來源）。同一個 crate 以 `schemars` 從型別產生 JSON Schema，供編輯器自動完成與 `gslm schema` 指令使用。

napi 層暴露 `loadConfig(options)` 與 `configSchema()`，讓 SDK 使用者和後續的 CLI（spec 0005）共用同一份載入邏輯。

JS 端提供 `migrateLegacyConfig(legacyObject) → string`（純函式，輸出 TOML 文字）以及 `gslm migrate` 子指令：動態 `import` 舊設定檔、轉換、預設 dry-run 印到 stdout，`--write` 才寫入 `gslm.toml`；遇到內嵌憑證物件時改寫成 `credentials.file` 並警告使用者另存金鑰、加入 `.gitignore`。

## User Stories

### 設定檔格式與探索

1. As a 使用者, I want 在專案根目錄放一個 `gslm.toml` 就能讓 `gslm pull` / `gslm push` 找到設定, so that 不必每次帶 `--config`。
2. As a 使用者, I want 也可以用 `gslm.json` 或 `gslm.jsonc`（允許註解與尾逗號）, so that 習慣 JSON 與 `$schema` 的團隊不必學 TOML。
3. As a 使用者, I want 在 monorepo 子目錄執行時自動往上找到根目錄的設定檔, so that 不必 `cd` 到根目錄。
4. As a 使用者, I want 探索停在含 `.git` 的目錄（或檔案系統根）, so that 不會誤讀到上層無關專案的設定。
5. As a 使用者, I want 同一目錄同時存在多種格式時以 `gslm.toml` > `gslm.jsonc` > `gslm.json` 的固定順序取用並印出警告, so that 行為可預期。
6. As a 使用者, I want 用 `--config <path>` 指定任意路徑的設定檔（副檔名決定格式）, so that 能在同一專案維護多份設定。
7. As a 使用者, I want 找不到任何設定檔時得到清楚的錯誤（列出搜尋過的檔名與起點）, so that 知道該在哪裡建立檔案。
8. As a 使用者, I want 偵測到目錄中有 `gslm.config.{js,ts,mjs,cjs}` 但沒有新格式設定檔時，錯誤訊息提示執行 `gslm migrate`, so that 升級後第一次執行就知道怎麼做。
9. As a 使用者, I want 設定檔內有 `.js/.ts` 設定檔或 YAML 時明確被拒絕（不支援）, so that 不會誤以為它們被讀取。

### Schema 與欄位

10. As a 使用者, I want 設定檔必填 `version = 1`, so that 日後 schema 變更時工具能辨識舊檔並引導遷移。
11. As a 使用者, I want `version` 缺少或不是 `1` 時得到「請升級 gslm」或「缺少 version」的明確錯誤, so that 不會靜默用錯誤的解讀執行。
12. As a 使用者, I want 以 `sheet`（Sheet ID）、`tab`（Tab 名稱）、`locales`（陣列，第一個為 Source locale）、`path`（含 `{locale}` 佔位符的檔案路徑樣板）、`format`（`nest` | `flat`）、`key_separator` 描述一個同步目標, so that 欄位名稱與 CONTEXT.md 的詞彙一致。
13. As a 使用者, I want `path` 取代舊的 `directory`, so that 能表達 `locales/{locale}/common.json` 這類不是「一目錄一檔」的佈局。
14. As a 使用者, I want `path` 沒有 `{locale}` 佔位符時在載入階段就報錯, so that 不會所有 Locale 寫到同一個檔案。
15. As a 使用者, I want 設定檔內的相對路徑（`path`、`credentials.file`）相對於**設定檔所在目錄**解析, so that 從任何子目錄執行結果都相同。
16. As a 使用者, I want `format` 預設 `nest`、`key_separator` 預設 `"."`, so that 最小設定只需要 `sheet`、`tab`、`locales`、`path`。
17. As a 使用者, I want `locales` 為空陣列或含重複項時報錯, so that Source locale 與欄位對應不會含糊。
18. As a 使用者, I want `key_separator` 為空字串時報錯, so that 與 `gslm-core` 的規則一致。
19. As a 使用者, I want 未知的欄位名稱被拒絕並指出是哪個欄位（含拼字相近的建議，例如 `sheetId` → `sheet`）, so that 打錯字不會被靜默忽略。
20. As a 使用者, I want 舊欄位名（`sheetId`、`sheetTitle`、`languages`、`directory`、`type`）被拒絕時錯誤訊息直接指出對應的新欄位並建議 `gslm migrate`, so that 手動遷移也容易。

### 多 Target

21. As a 使用者, I want 用 `[[targets]]` 陣列描述多個 Tab → 多個目錄的同步, so that 一個 Sheet 可同時服務 web、mobile、emails 等多個 app。
22. As a 使用者, I want 頂層欄位作為所有 target 的預設值、target 可覆寫任一欄位, so that 共同設定只寫一次。
23. As a 使用者, I want 每個 target 有唯一的 `name`, so that CLI 能以 `--target web,mobile` 篩選。
24. As a 使用者, I want 有 `targets` 時 `tab` 與 `path` 必須在 target 層（或由頂層提供預設）最終都有值，缺少時指出是哪個 target 缺什麼, so that 錯誤可直接定位。
25. As a 使用者, I want 沒有 `targets` 時頂層本身就是唯一的 target（名稱為 `default`）, so that 單 target 專案的設定檔最精簡。
26. As a 使用者, I want `targets` 內 `name` 重複時報錯, so that 篩選不會含糊。
27. As a 使用者, I want 同時寫了頂層 `tab`/`path` 又有 `targets` 時，頂層值只當作預設、不額外產生一個 target, so that 不會多同步一份。

### Credentials

28. As a 使用者, I want `[credentials] file = "./sa.json"` 指定 Service Account 檔案, so that 沿用既有的檔案流程。
29. As a 使用者, I want `[credentials] env = "GSLM_CREDENTIALS_JSON"` 指定「內容為 Service Account JSON 的環境變數名稱」, so that CI 可用 secret 注入而不落地。
30. As a 使用者, I want 不寫 `credentials` 時走 Google Application Default Credentials, so that 在 GCP 或 `gcloud auth application-default login` 後零設定可用。
31. As a 使用者, I want `credentials` 同時寫 `file` 與 `env`、或出現 `private_key` / `client_email` 等內嵌金鑰欄位時直接拒絕並說明原因, so that 金鑰不會被 commit 進設定檔。
32. As a 使用者, I want `credentials.env` 指向的環境變數不存在或為空時，在載入階段就報錯並指出變數名稱, so that 不會等到連線時才失敗。
33. As a 使用者, I want 載入的結果只描述「憑證來源」（檔案路徑／JSON 字串／ADC），交給 `gslm-sheets` 建立 client, so that `gslm-config` 不依賴網路與驗證邏輯。

### 環境變數與 CLI 覆寫

34. As a 使用者, I want `GSLM_SHEET`、`GSLM_TAB`、`GSLM_LOCALES`（逗號分隔）、`GSLM_PATH`、`GSLM_FORMAT`、`GSLM_KEY_SEPARATOR`、`GSLM_CREDENTIALS`（檔案路徑）、`GSLM_CREDENTIALS_JSON`（內容）覆寫設定檔, so that CI 可以不改檔案就換 Sheet。
35. As a 使用者, I want CLI flag 覆寫環境變數、環境變數覆寫設定檔、設定檔覆寫內建預設, so that 優先序固定且與 ADR-0003 一致。
36. As a 使用者, I want 覆寫的相對路徑（flag / env 的 `path`、`credentials`）相對於 **cwd** 解析, so that 命令列上的路徑行為符合直覺。
37. As a 使用者, I want 多 target 設定下，`--sheet/--tab/--path/--format/--locales/--key-separator` 只能搭配單一 `--target`，否則報錯, so that 覆寫不會含糊地套到多個 target。
38. As a 使用者, I want `--target` 指定不存在的名稱時報錯並列出可用名稱, so that 拼錯立刻發現。
39. As a 使用者, I want 沒有設定檔但 flag/env 已提供完整欄位時仍可執行（純命令列模式）, so that 一次性操作不必先建檔。
40. As a 使用者, I want 載入前自動讀取 cwd 的 `.env`（不覆寫既有環境變數）, so that 與舊版 `dotenv` 習慣一致；此行為可由選項關閉。

### 載入結果

41. As a SDK 使用者, I want `loadConfig({ cwd?, configPath?, env?, overrides?, targets? })` 回傳 `{ configPath, targets: [...] }`，每個 target 含 `name, sheet, tab, locales, path（絕對路徑樣板）, format, keySeparator, credentials`, so that SDK 與 CLI 拿到同一份已解析設定。
42. As a SDK 使用者, I want 載入結果可被序列化成 JSON（不含任何祕密內容；`credentials` 以 `{ kind: "file", path } | { kind: "json", env } | { kind: "adc" }` 表示）, so that 可以安全地印出除錯。
43. As a SDK 使用者, I want 載入錯誤帶有穩定的 `code`（例如 `CONFIG_NOT_FOUND`、`CONFIG_PARSE`、`CONFIG_INVALID`、`CONFIG_LEGACY`、`CONFIG_UNSUPPORTED_VERSION`）與檔案路徑、可能的話含行列, so that 程式可以分流處理。
44. As a 使用者, I want 解析錯誤（TOML/JSON 語法）的訊息含檔名與行列, so that 能直接跳到錯處。

### Schema 與編輯器支援

45. As a 使用者, I want `configSchema()` 回傳 JSON Schema（draft 2020-12）, so that CI 或編輯器外掛可直接使用。
46. As a 使用者, I want TOML 檔首行 `#:schema <url>`、JSON 檔 `$schema` 欄位被接受且不視為未知欄位, so that 編輯器自動完成可用。
47. As a 維護者, I want JSON Schema 由 Rust 型別產生（`schemars`）而非手寫, so that 型別與 schema 不會漂移。
48. As a 維護者, I want schema 以版本路徑（`schema/v1.json`）存放於 repo（`docs/schema/`），並有測試確保產生結果與存檔一致, so that 發佈到 GitHub Pages 時是同一份。

### 遷移

49. As a 既有使用者, I want 執行 `gslm migrate` 自動找到 cwd 的 `gslm.config.{js,mjs,cjs,ts}` 並輸出對應的 `gslm.toml` 內容到 stdout, so that 先預覽再決定。
50. As a 既有使用者, I want `gslm migrate --write` 才真正寫入 `gslm.toml`，且目標檔已存在時拒絕（除非 `--force`）, so that 不會覆蓋手動調整過的新設定。
51. As a 既有使用者, I want `gslm migrate --from <path>` 指定舊設定檔路徑, so that 非預設檔名也能遷移。
52. As a 既有使用者, I want 欄位對應固定：`sheetId → sheet`、`sheetTitle → tab`、`languages → locales`、`directory → path = "<directory>/{locale}.json"`、`type → format`、`credentials（字串） → credentials.file`, so that 輸出可預期。
53. As a 既有使用者, I want 舊設定的 `credentials` 是物件時，輸出 `credentials.file = "./credentials.json"`（不寫出金鑰內容），並在 stderr 警告「請把金鑰另存為該檔案並加入 `.gitignore`」, so that 遷移不會把金鑰寫進新檔。
54. As a 既有使用者, I want 舊設定缺少 `type` 時輸出 `format = "nest"`（舊版預設）, so that 行為不變。
55. As a 既有使用者, I want 舊設定以環境變數（如 `process.env.SHEET_ID`）計算出的值在遷移當下被「具體化」成字面值，並在 stderr 提示可改用 `GSLM_SHEET` 環境變數覆寫, so that 使用者知道動態值已固定。
56. As a 既有使用者, I want 輸出的 TOML 首行含 `#:schema` 指令與 `version = 1`，欄位順序固定、含簡短註解, so that 產出可讀、可直接 commit。
57. As a 既有使用者, I want 舊設定檔無法載入（語法錯、不存在、未匯出物件）時得到與舊版相同等級的錯誤訊息, so that 遷移失敗原因清楚。
58. As a 既有使用者, I want 遷移後立即用新設定檔執行 `gslm pull` 得到與舊版等價的行為（同 Sheet、同 Tab、同 Locales、同目錄、同 Format）, so that 遷移是零行為變更。
59. As a SDK 使用者, I want `migrateLegacyConfig(legacyObject)` 以純函式形式匯出（輸入物件、回傳 `{ toml, warnings }`）, so that 可在自己的腳本中批次遷移。

### 維護者

60. As a 維護者, I want 設定載入完全在 Rust（`gslm-config`），JS 端只有遷移邏輯, so that 規則不會在兩種語言各寫一份。
61. As a 維護者, I want `gslm-config` 不依賴 `gslm-sheets`（只依賴 `gslm-core` 的 `Format` 與常數）, so that 依賴圖保持單向：core ← config ← napi，core ← sheets ← napi。
62. As a 維護者, I want 載入邏輯以 tempdir 內的真實檔案測試（探索、相對路徑、三種格式、`.env`）, so that 測的是對外行為而非內部結構。

## Implementation Decisions

### 模組

- 新 crate `gslm-config`（workspace member）。公開介面：`load(LoadOptions) -> Result<ResolvedConfig, ConfigError>`、`schema() -> serde_json::Value`，以及 `ResolvedConfig` / `ResolvedTarget` / `CredentialsSource` / `ConfigError` 型別。`Format` 重用 `gslm-core`。
- `LoadOptions`：`cwd`、`config_path: Option`、`env: 環境變數來源（預設 `std::env`，測試可注入 map）`、`overrides: Overrides`（每個欄位皆為 `Option`，對應 CLI flag）、`targets: Option<Vec<String>>`（篩選）、`load_dotenv: bool`（預設 true）。
- 探索：從 `cwd` 往上，每層依序檢查 `gslm.toml`、`gslm.jsonc`、`gslm.json`；遇到含 `.git`（目錄或檔案，支援 worktree）的層級後不再往上；同層多檔時取第一個並回傳警告。`config_path` 指定時跳過探索、由副檔名決定格式（`.toml` / `.json` / `.jsonc`；其他副檔名 → `Unsupported`）。
- 找不到設定檔時，若 `overrides`（含 env）已足以組成完整 target 則以純命令列模式回傳（name `cli`、configPath 為 `None`），否則回傳 `NotFound`；`NotFound` 在 cwd 或探索路徑上發現 `gslm.config.{js,ts,mjs,cjs}` 時升級為 `Legacy { path }`。
- 解析：TOML 用 `toml` crate；JSON/JSONC 統一以寬鬆 JSON 解析（允許註解與尾逗號；`.json` 也接受，方便 `$schema` 與註解）。兩者先反序列化到只含 `version` 的結構，再依版本反序列化完整結構；目前只接受 `1`。
- `deny_unknown_fields`，但 `$schema` 欄位被明確宣告為可選字串並忽略。未知欄位錯誤含最接近的已知欄位名建議；舊欄位名另有專屬對應表（`sheetId→sheet` 等）在訊息中指出並建議 `gslm migrate`。
- 型別以 `serde(rename_all = "snake_case")`；`credentials` 為 enum（`File { file }` | `Env { env }`），多欄位或內嵌金鑰欄位經 `deny_unknown_fields` 拒絕，且對 `private_key` / `client_email` / `type = "service_account"` 給專屬訊息。
- 展開：`RawConfig { version, $schema, <target 欄位皆 Option>, credentials, targets: Option<Vec<RawTarget>> }`。無 `targets` → 一個名為 `default` 的 target；有 `targets` → 每個 target 以「target 欄位 ?? 頂層欄位 ?? 內建預設」解析，`sheet`、`tab`、`locales`、`path` 最終必須有值。
- 覆寫：先展開檔案，再套 env（`GSLM_*`），再套 `overrides`；多 target 且有欄位覆寫時必須 `targets` 篩選恰好一個，否則 `AmbiguousOverride`。`GSLM_CREDENTIALS` 與 `GSLM_CREDENTIALS_JSON` **兩者同時存在視為錯誤**（與檔案內 `file`+`env` 同規則）；env 層的 credentials 一旦出現即整個取代檔案層的 `credentials`，不做欄位合併。
- 路徑解析：檔案來源的相對路徑相對於設定檔目錄；env/flag 來源相對於 `cwd`。`path` 保留 `{locale}` 佔位符，但其餘部分轉為絕對路徑；必含 `{locale}`，暫不支援其他佔位符（出現 `{xxx}` 其他名稱 → 錯誤）。
- `CredentialsSource`：`File(PathBuf)` | `Json { env_name, value }` | `ApplicationDefault`。序列化給 JS 時 `Json` 只輸出 `env` 名稱，不輸出內容；napi 層另提供把 `ResolvedTarget.credentials` 轉成 `gslm-sheets` `Credentials` 的轉接（在 napi crate，不在 config crate）。`.env` 載入用 `dotenvy::from_path` 於 `cwd/.env`（不覆寫）。
- `ConfigError` 種類：`NotFound { searched, start }`、`Legacy { path }`、`Unsupported { path }`、`Parse { path, line, column, message }`、`UnsupportedVersion { found }`、`MissingVersion`、`Invalid { path, field, message }`（含 target name）、`UnknownField { field, suggestion }`、`AmbiguousOverride`、`UnknownTarget { name, available }`、`MissingEnv { name }`；每種有穩定 `code()`。
- Schema：`schemars` 從 `RawConfig` 產生 draft 2020-12，`$id` 為 `https://gn00678465.github.io/google-sheet-languages-model/schema/v1.json`。存檔於 `docs/schema/v1.json`，測試比對；`gslm schema` 指令留給 spec 0005，本票只暴露 `configSchema()`。

### napi 層

- `loadConfig(options?: LoadConfigOptions): ResolvedConfig`（同步；檔案 I/O 很小，不需 async）。錯誤以 `[CODE] ` 前綴 + `index.js` 轉 `code` 的既有機制。
- `configSchema(): object`。
- `ResolvedConfig` / `ResolvedTarget` 為 `#[napi(object)]` 純資料；`credentials` 為 `{ kind: 'file', path } | { kind: 'json', env } | { kind: 'adc' }`。
- 另加 `SheetsClient.fromConfig(target)`（或 `createFromTarget`）便利方法：把 target 的 credentials 轉成 `CredentialsOptions`；`kind: 'json'` 時由 Rust 端在轉接時再讀一次環境變數取得內容（不經 JS 傳遞祕密）。

### 遷移（JS 端）

- `migrateLegacyConfig(legacy: unknown): { toml: string; warnings: string[] }` 於 `index.js` 匯出、`index.d.ts` 宣告。純函式：驗證輸入是物件、欄位對應、產生 TOML 文字（手寫序列化，欄位固定、不引入 TOML 函式庫——輸出只有字串／字串陣列／table，需處理字串跳脫）。
- 對應：`sheetId→sheet`、`sheetTitle→tab`、`languages→locales`、`directory→path = join(directory, "{locale}.json")`（保留原字串、正規化為 `/` 分隔）、`type→format`（缺省 `nest`）、`credentials` 字串 → `[credentials] file`、物件 → `file = "./credentials.json"` + warning、缺省 → 不寫 `credentials`（ADC）+ warning 告知行為變更（舊版缺省讀 `GOOGLE_APPLICATION_CREDENTIALS`，ADC 首步相同，故實際相容）。未知欄位 → warning 並略過。
- `bin/gslm.js` 新增 `migrate [--from <path>] [--write] [--force]`：以動態 `import(pathToFileURL(...))` 載入（`.ts` 只在 Node 原生支援時可行，失敗時提示以 Node ≥ 22.18 執行或先轉 `.js`）；取 `default ?? module`；輸出到 stdout，warnings 到 stderr；`--write` 寫到舊檔同目錄的 `gslm.toml`，已存在且無 `--force` 時錯誤退出（exit code 1）。不使用 yargs/clap：此指令參數極少，手寫解析；完整 CLI 框架在 spec 0005 決定。
- 舊設定檔載入時會執行使用者程式碼（含 `dotenv.config`），這是遷移的本質，文件說明即可。

## Testing Decisions

- 好的測試只看對外行為：給定 tempdir 內的檔案 + 環境變數 map + overrides，斷言 `load` 回傳的 `ResolvedConfig` 或 `ConfigError` 種類與訊息關鍵字；不測內部的 Raw 結構。
- `gslm-config` 單一主縫 `load(LoadOptions)`：整合測試以 `tempfile` 建目錄樹（含 `.git` 標記、子目錄、`.env`、三種格式、舊 `gslm.config.js` 檔）；環境變數以注入 map 提供，避免測試間汙染。Schema 測試：`schema()` 與 `docs/schema/v1.json` 逐字相等（失敗訊息提示重新產生），並以 `jsonschema` crate（dev-dep）驗證範例設定通過、內嵌金鑰範例不通過。
- napi：`__tests__/config.test.cjs` 以 tempdir 走 `loadConfig` 正常與錯誤（`err.code`）各一至兩例、`configSchema()` 有 `$id`。
- JS 遷移：`__tests__/migrate.test.cjs` 以 `node:test` 測 `migrateLegacyConfig` 純函式（字串憑證、物件憑證、缺 `type`、未知欄位、非物件輸入、含引號／反斜線的字串跳脫），再一個端對端：tempdir 放 `gslm.config.mjs` → 執行 `bin/gslm.js migrate --write` → 用 `loadConfig` 載入產出的 `gslm.toml` 並比對欄位（對應 story 58）。
- 先例：`crates/gslm-sheets/tests/client.rs`（整合測試 + 注入）、`packages/gslm/__tests__/sheets.test.cjs`（node:test + fixture）。

## Out of Scope

- `gslm pull` / `gslm push` / `gslm init` / `gslm schema` 指令與 clap 整合（spec 0005）。
- 檔案 I/O 的 Catalog 讀寫（依 `path` 樣板讀寫 JSON）——spec 0005。
- 多 namespace（`{namespace}` 佔位符）、`source_locale`、`on_missing` 等進階欄位；schema 預留 `version` 演進即可。
- v1 → v2 的純資料遷移（Rust 端 `gslm migrate` 無參數模式）；目前只有 v1。
- 發佈 JSON Schema 到 GitHub Pages 與 SchemaStore 提交（檔案先放 `docs/schema/`）。
- `package.json` 內 `"gslm"` 區段。
- 在設定檔中支援 `${VAR}` 字串插值。

## Further Notes

- **`path` 語意是本票最大的行為變更**：舊版 `directory` 的隱含佈局是 `<dir>/<locale>.json`，遷移工具明確寫出等價樣板，之後使用者可自行改成其他佈局。
- **push 不再自動偵測 Format**：設定為準。偵測不符時的警告／`--strict` 在 spec 0005 實作，但本票的 `format` 欄位語意已定為「pull 寫入格式、push 預期格式」。
- **JSON 與 JSONC 同樣寬鬆解析**：避免使用者把 `.json` 加註解後被拒；代價是 `.json` 不再是嚴格 JSON，文件需註明。
- **`.env` 與祕密**：`credentials.env` 指向的變數內容絕不進入 `ResolvedConfig` 的 JS 序列化，也不進入錯誤訊息。
- **schemars 與 napi object 共存**：`RawConfig`（serde + schemars）與 napi 的 `ResolvedConfig`（`#[napi(object)]`）是不同型別，前者在 config crate、後者在 napi crate，由 `From` 轉換。
- **Node 版本與 `.ts` 舊設定**：舊版以 `import()` 載入 `.ts` 依賴執行環境（tsx/jiti 或 Node 原生 type stripping）；遷移沿用同一作法，不額外引入 loader。

## Comments

### 2026-08-22 實作備註（由 gslm-implementor 實作，orchestrator 驗證）

- 新 crate `gslm-config`：`load(LoadOptions) -> ResolvedConfig`、`schema()`；12 個 tempdir 整合測試 + schema 與 `docs/schema/v1.json` 逐字比對。napi：`loadConfig`、`configSchema`、`SheetsClient.fromConfig(target)`、`releaseConfigCredentials`。JS：`migrateLegacyConfig`（`migrate.js`，純 JS、延遲載入 native binding）與 `gslm migrate [--from] [--write] [--force]`。
- **憑證不經 JS**：`ResolvedTarget.credentials` 只含 `{kind, path|env}`；實際來源留在 Rust 端以隨機 handle 註冊，JS 端以不可列舉的 Symbol 屬性持有、`FinalizationRegistry` 釋放。展開／clone 過的 target 會失去 handle，`fromConfig` 給出明確錯誤。此設計取代原先「`kind: 'json'` 時重讀環境變數」的做法（code review F2/F3）。
- code review（11 項）全部修正：`.env` 解析錯誤不再含原始行（避免洩漏金鑰）；credentials 覆寫不觸發 `AmbiguousOverride`；純命令列模式優先於 `Legacy`；空 locale 拒絕；targets 去重、無設定檔時 `--target` 回 `NotFound`；空環境變數視為未設定；TOML basic string 正確跳脫、拒絕孤立 surrogate；`normalize_path` 不 pop `RootDir`/`Prefix`。
- 提交：7d4a32f、90ecdf4、209c811、b02d84d、9f4f277。
