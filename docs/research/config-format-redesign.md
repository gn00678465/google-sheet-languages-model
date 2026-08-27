# gslm 設定檔格式重新設計研究（Rust CLI + Node SDK）

> 研究日期:2026-08-21。所有外部事實皆附主要來源(官方文件 / GitHub 原始碼 / crates.io API)連結;crates.io 版本與日期取自 `https://crates.io/api/v1/crates/<name>` 於研究當日的回應。

## TL;DR 建議

1. **格式:以 TOML 作為第一公民格式,同時接受 JSON/JSONC**(`gslm.toml` 優先、`gslm.json` / `gslm.jsonc` 為替代)。理由:Rust 生態系中 `toml`(1.1.4,TOML spec 1.1.0,8.3 億次下載)與 `serde_json`(1.0.151)是唯二「一線、持續維護、serde 原生」的設定解析 crate;YAML 在 Rust 端沒有穩定的 serde 實作(`serde_yaml` 已封存、`serde_yml` 自行標記 DEPRECATED、`yaml-rust2` 進入維護模式且不含 serde)。Node SDK 端兩者皆有成熟解析器(`smol-toml` / `@iarna/toml`、`jsonc-parser`)。**不要支援 YAML**,也不要再支援 `.js/.ts` 設定檔。
2. **Schema:用 `schemars` 從 Rust struct 產生 JSON Schema**,發佈到 `https://gn00678465.github.io/google-sheet-languages-model/schema/v1.json` 並提交 SchemaStore catalog(`fileMatch: ["gslm.toml", "gslm.json", "gslm.jsonc"]`)。TOML 用 `#:schema` 指令(Taplo / Even Better TOML)、JSON 用 `$schema` 取得編輯器自動完成。Node SDK 以同一份 schema 產生 TypeScript 型別(或以 zod/valibot 手寫並在 CI 比對)。
3. **憑證:設定檔內只允許「路徑」或「環境變數名稱」,禁止內嵌 service-account 物件**。預設走 Google ADC 順序(`GOOGLE_APPLICATION_CREDENTIALS` → gcloud ADC 檔 → metadata server);另支援 `credentials.file = "./sa.json"` 與 `credentials.env = "GSLM_CREDENTIALS_JSON"`(變數內容為 JSON 字串)。CLI 自動載入 `.env`(不覆寫既有環境變數,與 dotenv / dotenvy 預設一致)。
4. **結構:`[[targets]]` 陣列取代單一 sheet/directory**;每個 target 自帶 `sheet`、`tab`、`path = "src/locales/{locale}.json"`、`format = "nest"|"flat"`、`key_separator`、`locales`。頂層欄位作為所有 target 的預設值。`type` 改名 `format`,且 push 時若檔案既有結構與設定不符要警告(不再「自動偵測」靜默覆蓋)。
5. **探索與優先序:** `--config` > 由 cwd 向上尋找 `gslm.toml` / `gslm.jsonc` / `gslm.json`(停在含 `.git` 或檔案系統根目錄);值的優先序為 **CLI flag > 環境變數(`GSLM_*`)> 設定檔 > 內建預設**(figment 的 `merge` 語意)。加入 `version = 1` 欄位與 `gslm migrate` 子命令把舊 `gslm.config.js` 轉為 `gslm.toml`。

---

## 1. 格式選擇

### 1.1 Rust 解析 crate 現況(crates.io API,2026-08-21)

| 格式 | Crate | 最新版 | 最後更新 | 下載量 | 狀態 / 備註 |
|---|---|---|---|---|---|
| TOML | [`toml`](https://crates.io/crates/toml) | 1.1.4+spec-1.1.0 | 2026-07-28 | 8.3 億 | serde 原生;docs 說明支援 TOML spec 1.1.0,提供 `from_str` / `to_string_pretty`([docs.rs](https://docs.rs/toml/latest/toml/))。Cargo 本身的依賴。 |
| JSON | [`serde_json`](https://crates.io/crates/serde_json) | 1.0.151 | 2026-07-20 | 11.9 億 | 事實標準。**不接受註解 / trailing comma**。 |
| JSONC | [`jsonc-parser`](https://crates.io/crates/jsonc-parser) | 0.33.1 | 2026-07-26 | 940 萬 | dprint 作者維護;提供 `parse_to_serde_value`,可先剝註解再丟給 serde。 |
| JSON5 | [`json5`](https://crates.io/crates/json5) | 1.3.1 | 2026-02-07 | 7,300 萬 | serde 相容。JSON5 編輯器支援不如 JSONC 普及。 |
| YAML | [`serde_yaml`](https://crates.io/crates/serde_yaml) | 0.9.34+deprecated | 2024-03-25 | 3.75 億 | **已封存**。GitHub 頁面標示 "This repository was archived by the owner on Mar 25, 2024" 且 README 寫 "(This project is no longer maintained.)",未指定後繼者([github.com/dtolnay/serde-yaml](https://github.com/dtolnay/serde-yaml))。RustSec 已有討論([advisory-db#2132](https://github.com/rustsec/advisory-db/issues/2132))。 |
| YAML | [`serde_yml`](https://crates.io/crates/serde_yml) | 0.0.13 | 2026-05-27 | 2,170 萬 | crate 描述開頭即為 **"DEPRECATED — `serde_yml` is unmaintained. This release is a thin compatibility shim…"**(轉呼叫 `noyalib`)。不建議。 |
| YAML | [`serde_yaml_ng`](https://crates.io/crates/serde_yaml_ng) | 0.10.0 | 2024-05-26 | 900 萬 | 社群 fork,兩年未更新。 |
| YAML | [`yaml-rust2`](https://crates.io/crates/yaml-rust2) | 0.12.0 | 2026-08-18 | 5,200 萬 | YAML 1.2 完全相容,但 README 宣告 "This crate will receive only basic maintenance and keep a stable API. `saphyr` will accept new features",且**不提供 serde**([README](https://github.com/Ethiraric/yaml-rust2))。 |
| YAML | `serde_norway` / `serde-saphyr` / `noyalib` | — | — | — | Rust 論壇討論串列出的候選([users.rust-lang.org/t/108868](https://users.rust-lang.org/t/serde-yaml-deprecation-alternatives/108868)),皆屬年輕 fork 或未承諾長期維護。 |

**結論:** YAML 在 Rust 端「沒有一個既穩定又 serde 原生的選項」,這是決定性因素。TOML 與 JSON 都有一線 crate。

### 1.2 同類開發工具的選擇

| 工具 | 格式 | 編輯器支援機制 | 出處 |
|---|---|---|---|
| Cargo | `Cargo.toml`(TOML) | Taplo / rust-analyzer | FAQ 說明檔名尾碼 `.toml` 是為了「強調檔案使用 TOML 設定格式」([Cargo FAQ](https://doc.rust-lang.org/cargo/faq.html)) |
| Ruff(Rust 寫的 Python linter) | `pyproject.toml [tool.ruff]` / `ruff.toml` / `.ruff.toml` | SchemaStore 已收錄 ruff schema | 同目錄優先序 `.ruff.toml > ruff.toml > pyproject.toml`;「最近的設定檔」適用於每個檔案;CLI 提供的設定「override the settings in every resolved configuration file」([Ruff Configuration](https://docs.astral.sh/ruff/configuration/)) |
| Biome(Rust) | `biome.json` / `biome.jsonc` | `$schema`:`./node_modules/@biomejs/biome/configuration_schema.json` 或 `https://biomejs.dev/schemas/[VERSION]/schema.json` | 有 `extends`、`root`([Biome Configuration](https://biomejs.dev/reference/configuration/)) |
| Turborepo(Rust) | `turbo.json` / `turbo.jsonc` | `"$schema": "https://turborepo.dev/schema.json"` | 無 `version` 欄位;`extends: ["//"]` 做 package 層級設定([Turborepo Configuration](https://turborepo.dev/docs/reference/configuration)) |
| Wrangler | `wrangler.toml` → `wrangler.json` / `wrangler.jsonc` | `"$schema": "./node_modules/wrangler/config-schema.json"` | v3.91.0 起 JSON 正式支援([release](https://github.com/cloudflare/workers-sdk/releases/tag/wrangler%403.91.0));官方文件:「Cloudflare recommends using `wrangler.jsonc` for new projects, and some newer Wrangler features will only be available to projects using a JSON config file」([Wrangler Configuration](https://developers.cloudflare.com/workers/wrangler/configuration/));JSONC 支援源自 [PR #6276](https://github.com/cloudflare/workers-sdk/pull/6276) |
| Prettier | cosmiconfig 多格式(`package.json` key、`.prettierrc` JSON/YAML、`.json5`、`.js/.ts/.mjs/.cjs`、`.toml`) | — | 從被格式化檔案所在目錄向上搜尋([Prettier Configuration](https://prettier.io/docs/configuration)) |
| Lingui | `lingui.config.js/ts`(JS) | TS 型別 | `catalogs[].path` 使用 `{locale}` / `{name}` 佔位符([Lingui conf](https://lingui.dev/ref/conf)) |
| i18next-parser → i18next-cli | `i18next-parser.config.{js,mjs,json,ts,yaml,yml}` → `i18next.config.ts` | `defineConfig` | 已於 2025-09 棄用,改由 i18next-cli;輸出路徑 `{{language}}` / `{{namespace}}`,`keySeparator: '.'`、`nsSeparator: ':'`;locize apiKey 建議 `LOCIZE_API_KEY` 環境變數([i18next-parser](https://github.com/i18next/i18next-parser)、[i18next-cli](https://github.com/i18next/i18next-cli)) |

觀察:
- **Rust 寫的工具走兩條路**:Cargo / Ruff 用 TOML;Biome / Turborepo 用 JSON(C)+ `$schema`。兩者共同點是**靜態資料格式 + 發佈 JSON Schema**,沒有一個選 YAML 或可執行腳本。
- Wrangler 從 TOML 移向 JSONC 的官方理由只有「新功能僅 JSON 支援」與 `$schema` 自動完成;在 Node 生態中 JSONC 編輯器體驗最好,但 TOML 對「多個 target 的陣列 + 每個有多欄位」(`[[targets]]`)可讀性更佳,且本專案 CLI 主體是 Rust。
- 因此建議 **TOML 為主、JSON(C) 為輔**,兩者共用同一份 schema(SchemaStore 測試檔本身就支援 `.json / .toml / .yml`,見下)。

### 1.3 JSON Schema 發佈與編輯器自動完成

- **SchemaStore 貢獻流程**:schema 放在 `src/schemas/json/<name>.json`,並在 `src/api/json/catalog.json` 加入 `{ name, description, fileMatch, url }`;亦可「keep the content at a place you control」,catalog 的 `url` 指向自架位置;測試檔支援 `.json`、`.toml`、`.yml`([CONTRIBUTING.md](https://github.com/SchemaStore/schemastore/blob/master/CONTRIBUTING.md))。
- **TOML 端**:Taplo / Even Better TOML 支援檔首 `#:schema ./path-or-url.json` 指令覆寫 schema([Taplo directives](https://taplo.tamasfe.dev/configuration/directives.html));一旦進 SchemaStore 則依 `fileMatch` 自動套用。
- **Rust 端產生 schema**:`schemars` 1.2.2(2026-07-27)從 `#[derive(JsonSchema)]` 產生 JSON Schema 2020-12,並遵守 `#[serde(...)]` 屬性,保證 schema 與實際反序列化一致([docs.rs/schemars](https://docs.rs/schemars/latest/schemars/))。建議在 CI 以 `cargo run -- schema > schema/v1.json` 產生並 diff。

---

## 2. i18n 同步工具的設定檔先例

| 工具 | 檔案 / 格式 | 多檔案群組 | locale 佔位符 | key 分隔 / namespace | 憑證 | 出處 |
|---|---|---|---|---|---|---|
| Crowdin CLI | `crowdin.yml` | `files[]`,每項 `source` / `translation`(支援 `*`、`**`、`?`、`[set]`)、`ignore`、`dest`、`translation_replace`、`languages_mapping` | `%locale%`、`%two_letters_code%`、`%three_letters_code%`、`%android_code%`、`%osx_code%`、`%original_file_name%`、`%file_name%`、`%file_extension%` | 由檔案格式決定 | `api_token` / `project_id` 直寫(高優先),或 `api_token_env: CROWDIN_PERSONAL_TOKEN`、`project_id_env`、`base_path_env`、`base_url_env`(低優先)— 即「**在設定檔寫環境變數名稱**」 | [Configuration File](https://support.crowdin.com/developer/configuration-file/) |
| Phrase Strings CLI | `.phrase.yml` | `push.sources[]`(`file` glob + `params`)、`pull.targets[]` | `<locale_name>`、`<locale_code>`、`<tag>` | `file_format` | 文件明示把 token 放在 `PHRASE_ACCESS_TOKEN` 而非 `.phrase.yml`;優先序 flag → env → config | [Developer Hub](https://developers.phrase.com/en/developer-tools/strings-cli/)、[Create a CLI Configuration File](https://support.phrase.com/hc/en-us/articles/5784093898908-Create-a-CLI-Configuration-File-Strings) |
| Transifex CLI | `.tx/config`(INI) | 每個 `[o:org:p:proj:r:res]` 區段一個資源 | `file_filter = locale/<lang>.php` | `type` 決定 | `TX_TOKEN` env / `~/.transifexrc` / `--token`,**設定檔本身不含 token** | [Using the client](https://developers.transifex.com/docs/using-the-client) |
| Tolgee CLI | `.tolgeerc`(cosmiconfig:JSON/YAML/JS、`package.json#tolgee`),有 `$schema: https://docs.tolgee.io/cli-schema.json` | `push.files[] { path, language, namespace }` 或 `push.filesTemplate` | `{languageTag}`、`{namespace}` | `pull.delimiter`(預設 `.`,`null`/`""` 關閉巢狀)、`pull.namespaces` | `apiKey` 欄位存在但 schema 描述附安全警告,建議 `TOLGEE_API_KEY` 或 `tolgee login` | [Project configuration](https://docs.tolgee.io/tolgee-cli/project-configuration)、[cli-schema.json](https://docs.tolgee.io/cli-schema.json) |
| Lokalise CLI v2 | 選用 `config.yml`(`--config`) | 以 flag 為主 | `%LANG_ISO%` | — | `--token` 或 `LOKALISE_TOKEN` | [lokalise-cli-2-go](https://github.com/lokalise/lokalise-cli-2-go) |
| Lingui | `lingui.config.ts` | `catalogs[] { path, include, exclude }` | `{locale}`、`{name}` | `format` | N/A | [Lingui conf](https://lingui.dev/ref/conf) |
| i18next-cli | `i18next.config.ts` | `extract.output` | `{{language}}`、`{{namespace}}` | `keySeparator: '.'`、`nsSeparator: ':'` | `LOCIZE_API_KEY` env | [i18next-cli](https://github.com/i18next/i18next-cli) |

### Google Sheet 專用工具(npm,2026-08-21 查詢)

| 套件 | 最新版 / 日期 | 設定方式 | 憑證 | 出處 |
|---|---|---|---|---|
| `google-sheets-i18n` | 1.1.0 / 2026-07-29 | **純 `.env` / `.env.local`**:`GOOGLE_SHEET_ID`、`GOOGLE_SHEET_TITLE`、`TRANSLATIONS_DIR`、`TRANSLATIONS_KEY_SEPARATOR`(預設 `.`,設 `false` 為 flat) | `GOOGLE_SERVICE_ACCOUNT_EMAIL` + `GOOGLE_SERVICE_ACCOUNT_PRIVATE_KEY`、或 `GOOGLE_SERVICE_ACCOUNT_JSON`、或 `GOOGLE_APPLICATION_CREDENTIALS` | [README](https://github.com/vladislavbogomolov/google-sheets-i18n) |
| `i18n-google-sheets` | 0.3.0 / 2026-06-11 | `.env`:`SPREADSHEET_ID`、`BASE_LANGUAGE` | `GOOGLE_APPLICATION_CREDENTIALS='./credentials.json'` 或 `GOOGLE_API_KEY` | [README](https://github.com/PavelPivkin/i18n-google-sheets) |
| `gs-i18n` | 1.0.5 / 2025-12-01 | `gs-i18n.json`:`spreadsheet.docId`、`spreadsheet.sheetId` | `googleServiceAccount.email` + `privateKey` **直接寫在 JSON 設定檔**(反面教材) | [README](https://github.com/jgjgill/gs-i18n) |
| `sheet-i18n` | 1.10.10 / 2026-04-20 | 「configuration types and server-side importer」(npm 描述) | — | [npm](https://www.npmjs.com/package/sheet-i18n) |
| `google-spreadsheet-i18n` | 2.0.0 / 2019-07-13 | CLI flags | — | 已停更 |
| `gsheet-i18n`、`sheets-i18n` | 不存在 | — | — | — |

### 共同概念萃取

1. **多個檔案群組**(Crowdin `files[]`、Phrase `targets[]`、Tolgee `push.files[]`、Lingui `catalogs[]`)是標配;單一 directory 是少數。
2. **路徑中的 locale 佔位符**是所有工具的共同語彙,僅語法不同(`%locale%` / `<locale_code>` / `<lang>` / `{languageTag}` / `{locale}` / `{{language}}`)。JS 生態最常見的是 `{locale}`(Lingui)。
3. **key 分隔符 / namespace** 在 i18next 系工具是必要選項(`keySeparator`、`nsSeparator`);`google-sheets-i18n` 以 `TRANSLATIONS_KEY_SEPARATOR=false` 表示 flat — 與本專案 `nest`/`flat` 概念對應。
4. **憑證**:成熟工具一律 env var 優先(Phrase / Transifex / Tolgee / i18next-cli);Crowdin 的 `*_env` 欄位是「設定檔只寫變數名稱」的最佳範例。Google Sheet 專用小工具多半直接依賴 `GOOGLE_APPLICATION_CREDENTIALS`(即 ADC)。

---

## 3. 祕密(憑證)處理

- **Google ADC 搜尋順序**(官方文件):(1) `GOOGLE_APPLICATION_CREDENTIALS` 環境變數指向的 JSON;(2) `gcloud auth application-default login` 產生的 `$HOME/.config/gcloud/application_default_credentials.json`(Windows:`%APPDATA%\gcloud\…`);(3) 附掛在 GCP 資源上的 service account(metadata server)。文件並警告「Service account keys create a security risk and are not recommended… compromised service account keys can be used by a bad actor without any additional information」([ADC docs](https://docs.cloud.google.com/docs/authentication/application-default-credentials))。
- **金鑰管理最佳實務**:「Don't submit service account keys to source code repositories」、盡量「Avoid storing keys on a file system」、優先 Workload Identity Federation、定期輪替([Best practices](https://docs.cloud.google.com/iam/docs/best-practices-for-managing-service-account-keys))。→ 現行 `example/gslm.config.with-credentials-object.js` 把整個 key 物件 `import` 進設定檔,等於鼓勵把 key 與設定一起 commit,應移除。
- **`.env` 載入慣例**:Node `dotenv` 預設「we will never modify any environment variables that have already been set」,需 `override: true` 才覆寫;FAQ 明確回答「Should I commit my `.env` file? No.」([dotenv README](https://github.com/motdotla/dotenv))。Rust `dotenvy`(0.15.7)`dotenv()`「Loads the .env file from the current directory or parents」且不覆寫既有變數,另有 `dotenv_override()`([docs.rs/dotenvy](https://docs.rs/dotenvy/latest/dotenvy/))。兩端語意一致,可在 Rust CLI 與 Node SDK 都預設載入 `.env`(不覆寫)。

**建議規則:**
- 設定檔 `credentials` 只接受 table:`{ file = "path" }` 或 `{ env = "VAR_NAME" }`;schema 以 `oneOf` 禁止出現 `private_key` 等欄位,CLI 解析到 `private_key` 直接報錯並提示遷移。
- 不設 `credentials` 時走 ADC(Rust 端可用 `gcp_auth` / `google-cloud-auth` 實作;Node 端 `google-auth-library` 原生支援)。
- 支援 Crowdin 式「變數名稱」而非 `${VAR}` 字串插值:前者 schema 可檢查、不需自製模板引擎,也避免把插值語法套用到所有字串欄位。

---

## 4. 設定檔探索與優先序

- **cosmiconfig 預設搜尋位置**:`package.json`(讀 `moduleName` 屬性)、`.${moduleName}rc`、`.${moduleName}rc.{json,yaml,yml,js,ts,mjs,cjs}`、`.config/${moduleName}rc*`、`${moduleName}.config.{js,ts,mjs,cjs}`;搜尋策略 `none`(只看 cwd)/ `project`(向上直到找到 `package.json`)/ `global`(向上到 `stopDir`,預設 home,再查 OS 設定目錄)([cosmiconfig README](https://github.com/cosmiconfig/cosmiconfig))。
- **Ruff**:每個檔案用「最近的」設定檔、路徑相對於設定檔所在目錄、同目錄優先序固定、`--config` 可指定檔案或內嵌 TOML 片段、專用 flag 優先於 `--config` 覆寫([Ruff](https://docs.astral.sh/ruff/configuration/))。
- **Rust 實作選項**:
  - [`figment`](https://docs.rs/figment/latest/figment/)(0.10.19,2024-05):`Figment::new().merge(Toml::file("App.toml")).merge(Env::prefixed("APP_")).join(Json::file(...))`;文件定義「Values for duplicate keys from a *merged* provider replace those from previous providers, while no replacement occurs for *joined* providers」,並能追蹤每個值的來源(錯誤訊息可指出「此值來自 gslm.toml 第 N 行」)。
  - [`config`](https://docs.rs/config/latest/config/)(0.15.25,2026-06):支援 TOML/JSON/YAML/INI/RON/JSON5,`Config::builder().add_source(File…).add_source(Environment::with_prefix("GSLM"))`,後加入者覆寫先加入者。
  - 手寫:`std::env::current_dir()` 迴圈 `parent()`,每層檢查 `gslm.toml` → `gslm.jsonc` → `gslm.json`;停在含 `.git` 的目錄或根目錄。本專案欄位少,**建議手寫探索 + `toml`/`serde_json` 反序列化 + 自行合併**,避免 figment 1.5 年未更新的依賴風險;若想要 provenance 再引入 figment。
- **優先序(採 Ruff / Phrase / figment 慣例):** `CLI flag` > `GSLM_*` 環境變數 > 設定檔 > 內建預設。`package.json` 內的 `"gslm"` 區段**不建議**支援:Rust CLI 需額外解析 `package.json`,且 Ruff 的 `[tool.ruff]` 類比在 Node 世界沒有等價慣例(Biome/Turbo 都不讀 `package.json`)。

---

## 5. Schema 演進

- **Biome**:schema URL 帶版本(`https://biomejs.dev/schemas/[VERSION]/schema.json`),`biome migrate --write`「Previews configuration updates required by breaking changes」並實際改寫 `biome.json`;無法自動處理的部分「the migrate command will point them out to you」([CLI reference](https://biomejs.dev/reference/cli/#biome-migrate)、[Upgrade to v2](https://biomejs.dev/guides/upgrade-to-biome-v2/))。
- **Turborepo**:無 `version` 欄位,靠 `$schema` 固定 URL + 新版 CLI 內建遷移。
- **Wrangler**:TOML → JSONC 以「兩種並存 + 線上轉換器」漸進遷移([Wrangler Configuration](https://developers.cloudflare.com/workers/wrangler/configuration/))。

**建議:**
1. 頂層 `version = 1`(整數,必填)。Rust 端先只反序列化 `{ version }`,再依版本選 struct;未知版本給出「請升級 gslm」訊息。
2. Schema 以版本路徑發佈(`/schema/v1.json`),`latest.json` 指向最新。
3. `gslm migrate`:
   - `gslm migrate --from gslm.config.js`:**由 Node SDK 提供**(Rust 不能執行 JS),在 `@gslm/node` 匯出 `migrateLegacyConfig()`,並由 `npx gslm-migrate` 包裝;遇到 `credentials` 為物件時,寫出 `credentials.file = "./credentials.json"` 並在 stderr 警告把 key 另存、加入 `.gitignore`。
   - `gslm migrate`(無參數):Rust 端處理 v1 → v2 之類的純資料改寫(Biome 模式),`--write` 才落盤,預設 dry-run。
4. 棄用策略:欄位改名時同時接受舊名一個 minor 版本,載入時印 deprecation 警告並指向 `gslm migrate`(對應 `#[serde(alias = "type")]`)。

---

## 6. 草案 schema

### 6.1 最小版(等價於今日功能)

```toml
#:schema https://gn00678465.github.io/google-sheet-languages-model/schema/v1.json
version = 1

sheet = "1AbC...xyz"          # Google Sheet ID
tab = "i18n-demo"             # 工作表分頁名稱
locales = ["en", "zh", "ja", "fr", "es"]
path = "./i18n/{locale}.json" # 取代 directory;{locale} 必填佔位符
format = "nest"               # "nest" | "flat";pull 寫入格式、push 驗證格式

[credentials]
file = "./credentials.json"   # 或 env = "GSLM_CREDENTIALS_JSON";兩者皆無則走 ADC
```

等價 JSONC:

```jsonc
{
  "$schema": "https://gn00678465.github.io/google-sheet-languages-model/schema/v1.json",
  "version": 1,
  "sheet": "1AbC...xyz",
  "tab": "i18n-demo",
  "locales": ["en", "zh", "ja", "fr", "es"],
  "path": "./i18n/{locale}.json",
  "format": "nest",
  "credentials": { "env": "GSLM_CREDENTIALS_JSON" }
}
```

### 6.2 擴充版(多 sheet / 多分頁 → 多目錄)

```toml
#:schema https://gn00678465.github.io/google-sheet-languages-model/schema/v1.json
version = 1

# 頂層 = 所有 targets 的預設值
sheet = "1AbC...xyz"
locales = ["en", "zh-TW", "ja"]
format = "nest"
key_separator = "."           # nest 展開/摺疊用;flat 時僅做驗證

[credentials]
env = "GSLM_CREDENTIALS_JSON"

[[targets]]
name = "web"                  # 供 `gslm pull --target web` 篩選
tab = "web"
path = "apps/web/src/locales/{locale}.json"

[[targets]]
name = "mobile"
tab = "mobile"
path = "apps/mobile/i18n/{locale}/common.json"
format = "flat"

[[targets]]
name = "emails"
sheet = "9ZyX...abc"          # 覆寫預設 sheet
tab = "emails"
locales = ["en", "zh-TW"]     # 子集合
path = "packages/emails/locales/{locale}.json"
key_separator = "::"
```

### 6.3 進階(選配,供討論)

```toml
[[targets]]
name = "web"
tab = "web"
path = "apps/web/src/locales/{locale}/{namespace}.json"
namespaces = ["common", "auth"]          # 對應工作表中的 namespace 欄或多個分頁
source_locale = "en"                     # push 時以此語言為 key 基準
on_missing = "empty"                     # pull 時缺翻譯:"empty" | "fallback" | "skip"
```

### 6.4 語意規則

| 規則 | 說明 |
|---|---|
| `path` 必含 `{locale}` | 解析時驗證;`{namespace}` 僅在宣告 `namespaces` 時允許。相對路徑**相對於設定檔所在目錄**(Tolgee、Ruff 慣例),不是 cwd。 |
| `format` | 取代 `type`。pull:寫入格式;push:讀檔後若偵測結構不符印警告(`--strict` 時報錯)。`#[serde(alias = "type")]` 相容一個版本。 |
| `key_separator` | 預設 `"."`;只影響 nest ↔ flat 轉換,與 Sheet 中的 key 欄無關。 |
| 頂層 vs `targets` | 若無 `targets`,頂層本身就是唯一 target(6.1);有 `targets` 時頂層欄位是預設值,`tab` 與 `path` 必須在 target 層出現。 |
| `credentials` | `oneOf: [{file}, {env}]`,`additionalProperties: false`;出現 `private_key` / `client_email` 直接拒絕。 |
| `version` | 必填整數;目前僅接受 `1`。 |

### 6.5 CLI flag 覆寫

```
gslm pull [--config gslm.toml] [--target web,mobile]
          [--sheet ID] [--tab NAME] [--locales en,zh] [--path "src/locales/{locale}.json"]
          [--format nest|flat] [--credentials ./sa.json] [--dry-run]
gslm push [同上,少 --format 改為 --strict]
gslm init [--format toml|jsonc]      # 產生含 #:schema / $schema 的範本
gslm schema                          # 印出 JSON Schema(CI 用)
gslm migrate [--from gslm.config.js] [--write]
```

- 單 target 設定下,flag 直接覆寫頂層欄位(與今日 `mergeConfig` 相同)。
- 多 target 設定下,`--sheet/--tab/--path/--format` **只允許搭配單一 `--target`**,否則報錯(避免歧義)。
- 環境變數:`GSLM_SHEET`、`GSLM_TAB`、`GSLM_LOCALES`(逗號分隔)、`GSLM_PATH`、`GSLM_FORMAT`、`GSLM_CREDENTIALS`(路徑)、`GSLM_CREDENTIALS_JSON`(內容)。優先序 flag > env > file > default。
- CLI 啟動時以 `dotenvy::dotenv()` 載入 `.env`(不覆寫)。

### 6.6 Rust 端型別與 schema 產生(示意)

```rust
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1 {
    pub version: u8,                       // == 1
    #[serde(flatten)] pub defaults: TargetDefaults,
    #[serde(default)] pub targets: Vec<Target>,
    pub credentials: Option<Credentials>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum Credentials { File { file: PathBuf }, Env { env: String } }

#[derive(Deserialize, Serialize, JsonSchema, Default)]
pub struct TargetDefaults {
    pub sheet: Option<String>,
    pub tab: Option<String>,
    pub locales: Option<Vec<String>>,
    pub path: Option<String>,
    #[serde(alias = "type")] pub format: Option<Format>,
    pub key_separator: Option<String>,
}
// `cargo run -- schema` → schemars::schema_for!(ConfigV1) → schema/v1.json
```

### 6.7 Node SDK 消費同一份檔案

```ts
// @gslm/node  src/config.ts
import { parse as parseToml } from 'smol-toml'
import { parse as parseJsonc } from 'jsonc-parser'
import * as v from 'valibot'

const Credentials = v.union([
  v.strictObject({ file: v.string() }),
  v.strictObject({ env: v.string() }),
])
const TargetFields = {
  sheet: v.optional(v.string()),
  tab: v.optional(v.string()),
  locales: v.optional(v.array(v.string())),
  path: v.optional(v.pipe(v.string(), v.includes('{locale}'))),
  format: v.optional(v.picklist(['nest', 'flat'])),
  key_separator: v.optional(v.string()),
}
export const ConfigV1 = v.strictObject({
  $schema: v.optional(v.string()),
  version: v.literal(1),
  ...TargetFields,
  targets: v.optional(v.array(v.strictObject({ name: v.optional(v.string()), ...TargetFields }))),
  credentials: v.optional(Credentials),
})
export type Config = v.InferOutput<typeof ConfigV1>

export async function loadConfig(opts: { cwd?: string; configPath?: string } = {}) {
  const file = opts.configPath ?? findUp(['gslm.toml', 'gslm.jsonc', 'gslm.json'], opts.cwd)
  const raw = await readFile(file, 'utf8')
  const data = file.endsWith('.toml') ? parseToml(raw) : parseJsonc(raw)
  const config = v.parse(ConfigV1, data)
  return resolveTargets(config, dirname(file))   // 展開預設值、解析相對路徑
}
```

- 兩端一致性:CI 中以 `gslm schema` 產生 JSON Schema,再用 `ajv` 驗證 SDK 測試用的 fixture 檔;或改用 `json-schema-to-typescript` 從 schema 產生 `.d.ts`,valibot 只做執行期驗證。
- SDK 的 `GoogleSheetLanguagesModel` 可新增 `fromConfig(config, targetName?)` 工廠方法,內部走 `google-auth-library` 的 ADC / keyFile / credentials JSON。

---

## 7. 給維護者的待決問題

1. **是否保留 JSON(C) 作為第二格式?** 多一種格式 = 多一條測試矩陣;但 Node 使用者對 `$schema` 的熟悉度遠高於 `#:schema`。建議:保留,但文件與 `gslm init` 預設 TOML。
2. **佔位符語法**:`{locale}`(Lingui)vs `{{language}}`(i18next-cli)vs `%locale%`(Crowdin)。本文採 `{locale}`;若目標使用者多為 i18next 社群,可考慮同時接受 `{{language}}` 別名(不建議,會讓 schema 驗證變鬆)。
3. **push 的「自動偵測格式」要不要保留?** 建議改為「設定為準、偵測不符時警告」,否則同一 target 可能每次 push 行為不同。
4. **`key_separator` 要不要也影響 Sheet 端的 key 欄?** 目前只定義為本地 nest/flat 轉換用;若未來支援 namespace 欄,需決定 `nsSeparator`。
5. **ADC 在 Rust 端的實作 crate**(`gcp_auth` vs `google-cloud-auth` vs 自行用 `jsonwebtoken` 簽 JWT)— 超出本研究範圍,需另行評估。
6. **`gslm migrate --from *.js` 放在哪個套件?** 本文建議放 Node SDK(`npx gslm-migrate`),Rust CLI 偵測到 `gslm.config.{js,ts,mjs,cjs}` 時只印提示。
7. **設定檔探索的停止條件**:`.git` 目錄 vs `package.json` vs 檔案系統根。cosmiconfig `project` 策略停在 `package.json`;Ruff 一路向上。建議停在 `.git` 或根目錄,避免 monorepo 子套件讀不到根設定。
8. **多 target 時的 CLI 覆寫語意**:本文採「必須指定單一 `--target` 才能覆寫欄位」;另一選項是完全禁止欄位覆寫、只允許 `--target` 篩選(更簡單)。
