# Node.js SDK 架構研究：Rust CLI + Node SDK 該怎麼拆

> 研究日期：2026-08-21。所有結論皆以一手來源（官方文件、GitHub 原始碼、npm registry manifest）為依據，連結附在各段落。
> 本文只處理「CLI 改寫為 Rust 後，Node SDK 應採何種架構」這個問題；不討論 Rust CLI 本身的實作。

## 0. 現況盤點（本 repo）

| 項目 | 現況 | 來源 |
| --- | --- | --- |
| 單一套件同時出 CLI 與 API | `bin.gslm = ./dist/cli.js`；`exports["."]` 提供 `types/import/require` | [`package.json`](../../package.json) |
| 建置 | `tsdown`，`format: ['esm','cjs']`、`dts: true`，`googleapis`/`yargs` external，`lodash-es` bundled | [`tsdown.config.ts`](../../tsdown.config.ts) |
| 核心邏輯量 | `LanguagesModel.ts` 138 行 + `GoogleSheetLanguagesModel.ts` 120 行 + `cli.ts` 22 行 = **280 行** | `wc -l src/core/*.ts src/cli.ts` |
| 邏輯內容 | (1) `flatten` / `flatToNest`（用 `lodash-es/set`）；(2) `languagesModelToSheetValue` / `sheetValueToLanguageModel`（二維陣列 ⇄ model）；(3) 資料夾 JSON 讀寫；(4) Sheets API `values.get` / `values.update` 兩個呼叫 | [`src/core/LanguagesModel.ts`](../../src/core/LanguagesModel.ts)、[`src/core/GoogleSheetLanguagesModel.ts`](../../src/core/GoogleSheetLanguagesModel.ts) |
| 對外 API 面 | `GoogleSheetLanguagesModel({ sheetId, auth })`、`loadFromGoogleSheet(title, languages)`、`saveToGoogleSheet(title, model)`、`LanguagesModel.loadFromFolder()` / `saveToFolder(path, 'nest'|'flat')` | [`example/pull.ts`](../../example/pull.ts)、[`example/push.ts`](../../example/push.ts) |
| 依賴 | `googleapis ^164`（整包，見 §4） | `package.json` |

關鍵觀察：**純邏輯（不含 I/O）大約 100 行**，而且是純資料轉換（無效能壓力、無原生依賴需求）。這是後面所有判斷的前提。

---

## 1. TL;DR 與建議

**建議：B（純 TypeScript SDK）+ 共用黃金測試夾具（conformance fixtures），SDK 以 `@googleapis/sheets`（或 `google-auth-library` + `fetch`）取代整包 `googleapis`，ESM-only 發佈；Rust CLI 與 TS SDK 是同一 monorepo 內兩個獨立可發佈產物，透過 `fixtures/` 下的 JSON 黃金檔保證行為一致。**

理由濃縮：

1. 這個領域的邏輯約 100 行純資料轉換。napi-rs 帶來的成本（每平台一個 `optionalDependencies` 套件、十幾個 CI target、musl/glibc 偵測、版本不一致錯誤）是為了「重算力」或「大量 Rust 既有程式碼」而存在的——Rolldown / SWC / oxc 的 Rust 程式碼都是數十萬行等級，本專案不是。
2. A 方案中 **Google Sheets 的 HTTP + OAuth/JWT 簽章也得在 Rust 端做**，否則 TS 仍要自帶 auth 與 HTTP，等於兩邊各做一半，napi 只省下 100 行。而且 Rust 端的 HTTP 在 napi 中做 async 需要 tokio runtime + `napi::bindgen_prelude::AsyncTask` 等額外複雜度。
3. 成熟 SDK（Octokit、Stripe、Anthropic）共通特徵是：**可注入 `fetch`、明確錯誤類別、`exports` 條件映射、零或極少 runtime 依賴**。這些在純 TS 下最容易做到。
4. Node 20.19+/22.12+ 已支援 `require(esm)`，Node 25.4 起不再是實驗功能；tsdown 官方已宣告「CJS 為維護模式，新函式庫建議 ESM-only」。

### 比較表

| 面向 | A-napi（napi-rs） | A-wasm（wasm-bindgen / napi wasi） | **B-pure-TS** | B+shell-out（SDK spawn CLI） |
| --- | --- | --- | --- | --- |
| 邏輯來源 | 單一（Rust） | 單一（Rust） | 兩份（Rust CLI + TS SDK），以黃金夾具同步 | 單一（Rust CLI） |
| 安裝體驗 | 主套件 + N 個 `optionalDependencies` 平台套件（rolldown 15 個、oxc-parser 20 個） | 一個 `.wasm`（Biome 分 3 個 peer 套件） | `npm i` 即用，無原生檔 | 需要 CLI 平台套件（Biome CLI 8 個 optional deps） |
| 建置/CI 複雜度 | 高：13+ target 矩陣、docker/zig/QEMU（napi 範本 CI.yml） | 中：單一 `wasm32-wasip1-threads` 或 wasm-bindgen 目標 | 低：`tsdown` 一行 | 高：同 A-napi 的平台發佈，加上 stdin/stdout 協定設計 |
| TypeScript 型別 | napi-rs 自動產 `.d.ts`（`--dts`、`dtsHeader`） | wasm-bindgen 產 `.d.ts` | 手寫/原生 | 手寫（協定型別） |
| 需在 Rust 實作 HTTP/OAuth | 是（否則 napi 無意義） | 是，且 wasm 內 HTTP 需橋接 host fetch | 否 | 是（CLI 本來就有） |
| 可測試性（注入 fetch / mock） | 差（跨語言邊界） | 差 | **佳**（Octokit/Anthropic 模式） | 中（mock 子程序） |
| 瀏覽器/Edge runtime | 否 | 可（Biome wasm-web） | 可（若只用 `fetch` + JWT 簽章） | 否 |
| 啟動開銷 | 低 | wasm 編譯開銷（esbuild 文件：wasm 版慢約 10x） | 無 | 每次呼叫 spawn 子程序 |
| 適合本專案（~100 行邏輯）？ | 否（殺雞用牛刀） | 否 | **是** | 否（協定設計 > 邏輯本身） |

---

## 2. 方案 A 證據：Rust core + Node bindings

### 2.1 Rolldown（napi-rs）

- 套件 `rolldown` 的 `napi` 設定：`binaryName: "rolldown-binding"`、`packageName: "@rolldown/binding"`、16 個 targets（含 `wasm32-wasip1-threads`）。
  來源：<https://raw.githubusercontent.com/rolldown/rolldown/main/packages/rolldown/package.json>
- npm 上的 `rolldown@1.2.5` 有 **15 個 `optionalDependencies`**（`@rolldown/binding-darwin-arm64`…`@rolldown/binding-openharmony-arm64`），`engines.node = "^20.19.0 || >=22.12.0"`，`type: "module"`，`exports` 全為 `.mjs`（ESM-only）。
  來源：<https://registry.npmjs.org/rolldown/latest>
- 型別：`napi.dtsHeader` 內嵌自訂 header（`type MaybePromise<T> …`），由 napi-rs 自動產出 `binding.d.ts`。
- 發佈腳本：`napi artifacts --npm-dir ../npm`、`prepublishOnly: napi pre-publish -t npm --no-gh-release`。
- Monorepo 佈局：根目錄 `crates/`（Rust）、`packages/`（`rolldown`, `browser`, `bench`, `debug`…）、`pnpm-workspace.yaml`、`Cargo.toml` workspace、`justfile`。
  來源：<https://api.github.com/repos/rolldown/rolldown/contents/>

### 2.2 oxc（`oxc-parser`，napi-rs + wasm fallback）

- 位置在 `napi/parser/`（非 `npm/`；`npm/` 放的是 `oxlint`、`oxfmt` 等純 launcher）。`repository.directory = "napi/parser"`。
  來源：<https://raw.githubusercontent.com/oxc-project/oxc/main/napi/parser/package.json>
- npm 上 `oxc-parser@0.146.0`：20 個 targets、**20 個 `optionalDependencies`**，`napi.wasm.browser.fs = false`，`dtsHeaderFile: "src-js/header.d.ts"`，`browser: "src-js/wasm.js"`，`engines` 同 Rolldown。
  來源：<https://registry.npmjs.org/oxc-parser/latest>
- napi-rs 產生的 loader `src-js/bindings.js`：先用 `readFileSync('/usr/bin/ldd')` / `process.report` / `ldd --version` 三段式偵測 musl，逐平台 `require('@oxc-parser/binding-xxx')`，版本不符時 throw `Native binding package version mismatch, expected 0.146.0 but got …`；最後依 `NAPI_RS_FORCE_WASI`（tri-state）退回 `wasm32-wasi`。
  來源：<https://raw.githubusercontent.com/oxc-project/oxc/main/napi/parser/src-js/bindings.js>

### 2.3 SWC（`@swc/core`，napi-rs）

- 12 個 targets、12 個 `optionalDependencies`（`@swc/core-darwin-arm64` 等），`main: ./index.js`、`types: ./index.d.ts`（CJS、非 exports map），`engines.node >= 10`。
  來源：<https://registry.npmjs.org/@swc/core/latest>
- 建置腳本：`tsc -d && napi build --manifest-path ../../Cargo.toml --platform …`、`napi artifacts --npm-dir scripts/npm`、`prepack: napi prepublish -p scripts/npm`。
  來源：<https://raw.githubusercontent.com/swc-project/swc/main/packages/core/package.json>

### 2.4 Biome（wasm-bindgen；CLI 走 binary launcher）

- `@biomejs/js-api@6.0.0`：**不含任何 wasm**，以 `peerDependencies`（皆 optional）要求使用者自行安裝 `@biomejs/wasm-bundler` / `wasm-nodejs` / `wasm-web` 其中之一；`exports` 提供 `.`、`./nodejs`、`./web`、`./bundler` 四個入口。
  來源：<https://raw.githubusercontent.com/biomejs/biome/main/packages/%40biomejs/js-api/package.json>、<https://registry.npmjs.org/@biomejs/js-api/latest>
- `src/index.ts` 以 `await import("@biomejs/wasm-nodejs")` 動態載入對應 distribution。
  來源：<https://raw.githubusercontent.com/biomejs/biome/main/packages/%40biomejs/js-api/src/index.ts>
- `@biomejs/wasm-nodejs` 只含 `biome_wasm_bg.wasm`、`biome_wasm.js`、`biome_wasm.d.ts`（典型 wasm-bindgen 輸出）。
  來源：<https://raw.githubusercontent.com/biomejs/biome/main/packages/%40biomejs/wasm-nodejs/package.json>
- README 明言：「The API is currently in alpha. It is not yet ready for production use.」
  來源：<https://github.com/biomejs/biome/blob/main/packages/@biomejs/js-api/README.md>
- **CLI 的做法完全不同**：`@biomejs/biome` 只含 `bin/biome`（一支 Node launcher），`optionalDependencies` 列 8 個 `@biomejs/cli-<platform>`；launcher 以 `ldd --version` 偵測 musl，`require.resolve('@biomejs/cli-linux-x64/biome')` 後 `spawnSync(..., { stdio: 'inherit' })`，並支援 `BIOME_BINARY` 環境變數覆寫。
  來源：<https://raw.githubusercontent.com/biomejs/biome/main/packages/%40biomejs/biome/package.json>、<https://raw.githubusercontent.com/biomejs/biome/main/packages/%40biomejs/biome/bin/biome>
- 啟示：Biome **不讓 JS API 去 spawn CLI**，而是把核心編成 wasm 讓 JS 直接呼叫；CLI 與 JS API 是兩條獨立的發佈線。

### 2.5 Turborepo

- `turbo@2.10.11` 是純 launcher：`bin/turbo` + 6 個 `@turbo/<os>-<arch>` `optionalDependencies`，**沒有 JS SDK**。
  來源：<https://registry.npmjs.org/turbo/latest>、<https://raw.githubusercontent.com/vercel/turborepo/main/packages/turbo/package.json>
- 內部有 `packages/turbo-repository`（`@turbo/repository`，`private: true`）用 `@napi-rs/cli 2.16.3`（v2 舊式 `napi.triples` 設定）做實驗性 binding。
  來源：<https://raw.githubusercontent.com/vercel/turborepo/main/packages/turbo-repository/package.json>
- Rust 在 `crates/`（60+ crates），JS 在 `packages/`。
  來源：<https://api.github.com/repos/vercel/turborepo/contents/crates>

### 2.6 napi-rs v3 狀態

- `napi new` 會複製官方範本、套上套件名稱與 target，並可選擇產生 GitHub Actions workflow；範本不提交 `npm/` 目錄，CI 以 `napi create-npm-dirs` 生成各平台子套件目錄。本地 `napi build` 預設只編 host 平台，產出 `.node`、`index.js` loader、可選的 `index.d.ts`。
  來源：<https://napi.rs/docs/introduction/getting-started>
- `napi build --platform`：「Add platform triple to the generated nodejs binding file, eg: [name].linux-x64-gnu.node」；`--esm` 可產 ESM loader；`--dts` / `--dts-header` 控制型別輸出。
  來源：<https://napi.rs/docs/cli/build>
- WASI：`wasm32-wasip1-threads` 作為「no prebuilt native addon matches the host」時的 fallback；文件警告「Treat the WASI addon as trusted native application code, not as a security sandbox」且「A WASI build is not automatically equivalent to a native addon」。
  來源：<https://napi.rs/docs/concepts/webassembly>
- 官方範本 CI：13 個 target 矩陣（macOS ×2、Windows ×3、Linux gnu ×3、musl ×2 via `cargo-zigbuild`、Android ×2、wasm32-wasip1-threads）、FreeBSD 額外 job、Linux 測試用 `node:*-alpine`/`-slim` docker + QEMU、WASI 測試設 `NAPI_RS_FORCE_WASI=true`、發佈前 `napi create-npm-dirs` + `napi artifacts` + `npm config set provenance true`。
  來源：<https://raw.githubusercontent.com/napi-rs/package-template/main/.github/workflows/CI.yml>

### 2.7 方案 A 小結

| | napi-rs | wasm |
| --- | --- | --- |
| 選用者 | Rolldown、oxc、SWC、Turborepo（實驗） | Biome（JS API）、oxc（fallback）、Rolldown（fallback） |
| 何時合理 | 核心本來就是大量 Rust、效能敏感、需要 Node 專屬 API（fs、threads） | 需要跑在瀏覽器 / StackBlitz、想避免 N 平台發佈 |
| 對本專案 | 邏輯 100 行，沒有效能需求；napi 的固定成本（CI 矩陣、平台套件、版本鎖定）不會因為邏輯小而變小 | 若只是 flat⇄nest，wasm 的啟動成本比邏輯本身還貴；Sheets HTTP 仍得留在 JS 端 |

---

## 3. 方案 B 證據：純 TypeScript SDK

### 3.1 `@octokit/core`

- `@octokit/core@7.0.7`：`type: "module"`、`engines.node >= 20`、`exports["."] = { types, import, default }`（**無 `require` 條件 → ESM-only**）、`sideEffects: false`、unpacked 僅 23 KB。
  來源：<https://registry.npmjs.org/@octokit/core/latest>
- 可注入：`options.auth` / `options.authStrategy`（可換掉整個認證策略）、`options.baseUrl`、`options.request`；擴充點 `octokit.hook.before/after/error/wrap('request', …)` 與 `Octokit.plugin()`。
  來源：<https://github.com/octokit/core.js/blob/main/README.md>
- `@octokit/request` 提供 `request.fetch`：「Custom replacement for fetch. Useful for testing or request hooks.」；非 2xx 以 `RequestError` 拒絕，帶 `status` / `request` / `response`。`@octokit/request-error` 是獨立套件（6.5 KB）。
  來源：<https://github.com/octokit/request.js/blob/main/README.md>、<https://registry.npmjs.org/@octokit/request-error/latest>

### 3.2 Stripe Node

- `stripe@22.5.0`：`dependencies: {}`（零 runtime 依賴）、`exports` 依 `bun`/`deno`/`worker`/`browser`/`workerd`/`default` 條件各給 `import`/`require`，`default.import.types = ./esm/stripe.esm.node.d.ts`、`default.require.types = ./cjs/stripe.cjs.node.d.ts`（雙格式、型別分開）。
  來源：<https://registry.npmjs.org/stripe/latest>
- 設定項：`timeout`、`maxNetworkRetries`、`httpAgent`（proxy）、`host/port/protocol`、`telemetry`；事件 `stripe.on('request' | 'response')`。
  來源：<https://github.com/stripe/stripe-node/blob/master/README.md>

### 3.3 `@anthropic-ai/sdk`（Stainless 產生）

- `exports["."] = { types: ./index.d.mts, default: ./index.mjs, require: { types: ./index.d.ts, default: ./index.js } }`，子路徑 `./error`、`./client`、`./core/*`、`./lib/*` 都各給 `import`/`require`。
  來源：<https://registry.npmjs.org/@anthropic-ai/sdk/latest>
- 錯誤階層：非成功狀態碼丟 `APIError` 子類，表格對應 400 `BadRequestError`、401 `AuthenticationError`、403 `PermissionDeniedError`、404 `NotFoundError`、409 `ConflictError`、422 `UnprocessableEntityError`、429 `RateLimitError`、≥500 `InternalServerError`、N/A `APIConnectionError`；逾時丟 `APIConnectionTimeoutError`。
- 注入點：「By default, this library expects a global `fetch` function is defined」；可 `new Anthropic({ fetch })` 或 `fetchOptions`（含 undici `dispatcher` 做 proxy）；另有 `maxRetries`、`timeout`、`logger`/`logLevel`、`client.get/post` 供未文件化端點。
- 支援 Node 20 LTS+、Deno、Bun、Cloudflare Workers、Vercel Edge；瀏覽器需 `dangerouslyAllowBrowser`。
  來源：<https://platform.claude.com/docs/en/api/sdks/typescript>

### 3.4 `@aws-sdk`（modular）

- `@aws-sdk/client-s3@3.1115.0`：一個服務一個套件，底層 `@smithy/*`（`@smithy/node-http-handler`、`@smithy/fetch-http-handler` 兩種 HTTP handler 可換）；`sideEffects: false`；`main: dist-cjs`、`types: dist-types`。
  來源：<https://registry.npmjs.org/@aws-sdk/client-s3/latest>、<https://registry.npmjs.org/@smithy/smithy-client/latest>
- 啟示：HTTP handler 以介面抽象、可替換，是「依賴注入」的大型範例；但對本專案而言 Octokit/Anthropic 的「注入 `fetch`」已足夠。

### 3.5 ESM-only 還是 dual？（Node 官方文件）

- Node `modules.md`「Loading ECMAScript modules using require()」：v20.19.0 / v22.12.0 / v23.0.0 起**不需 `--experimental-require-module` flag**；v20.19.0 / v22.13.0 / v23.5.0 起不再印實驗警告；**v25.4.0 起「This feature is no longer experimental」**。限制：被 `require` 的 ESM 圖中若有 top-level `await` 會丟 `ERR_REQUIRE_ASYNC_MODULE`。解析 exports 時條件為 `["node", "require", "module-sync"]`。
  來源：<https://nodejs.org/api/modules.html#loading-ecmascript-modules-using-require>
- Node `packages.md` 的 dual package 章節已整段移到 `nodejs/package-examples`，而該文件開頭寫著：「A lot of the information below been outdated since Node.js started to support `require(esm)`. Do not follow the documentation below for new packages for the time being.」
  來源：<https://nodejs.org/api/packages.html#dual-commonjses-module-packages>、<https://raw.githubusercontent.com/nodejs/package-examples/main/guide/07-dual-packages/README.md>
- tsdown 官方文件：「**CJS is in maintenance-only mode.** Since the ecosystem is transitioning to ESM and Node.js now supports `require(esm)`, … New libraries are encouraged to publish ESM-only.」
  來源：<https://raw.githubusercontent.com/rolldown/tsdown/main/docs/options/output-format.md>
- 實務佐證：`rolldown`、`oxc-parser`、`@octokit/core` 皆 ESM-only 且 `engines.node = "^20.19.0 || >=22.12.0"`（正是 `require(esm)` 免 flag 的最低版本）。

結論：**SDK 發 ESM-only，`engines.node` 設 `^20.19.0 || >=22.12.0`**，CJS 使用者靠 `require(esm)`；只需確保 SDK 入口沒有 top-level `await`。

### 3.6 建置與驗證工具

- `tsdown` 可自動產生 `exports`（`exports: true`；`exports.legacy` 在 ESM-only 預設 false），並內建 `publint: true` / `attw: true`（`@arethetypeswrong/core` 為 optional peer）。
  來源：<https://tsdown.dev/options/package-exports>、<https://raw.githubusercontent.com/rolldown/tsdown/main/docs/options/lint.md>、<https://registry.npmjs.org/tsdown/latest>
- `publint@0.3.24`、`@arethetypeswrong/cli@0.18.5`。
  來源：<https://registry.npmjs.org/publint/latest>、<https://registry.npmjs.org/@arethetypeswrong/cli/latest>
- 版本管理：`changesets` 定位是「manage versioning and changelogs with a focus on monorepos」，Rust CLI + TS SDK 同 repo 時比目前的 `bumpp` 更合適（可對不同套件獨立 bump）。
  來源：<https://github.com/changesets/changesets/blob/main/README.md>

---

## 4. 依賴：`googleapis` vs `@googleapis/sheets` vs `google-auth-library` + `fetch`

| 套件 | 版本 | unpackedSize | 檔案數 | engines | 備註 | 來源 |
| --- | --- | --- | --- | --- | --- | --- |
| `googleapis` | 176.0.0 | **212,514,827 bytes（約 203 MB）** | 1,893 | >=18 | 含全部 Google API；依賴 `googleapis-common ^8`、`google-auth-library 10.5.0` | <https://registry.npmjs.org/googleapis/latest> |
| `@googleapis/sheets` | 14.0.0 | 755,905 bytes（約 0.7 MB） | 14 | >=12 | 只依賴 `googleapis-common ^8`（其內部拉 `google-auth-library`） | <https://registry.npmjs.org/@googleapis/sheets/latest> |
| `google-auth-library` | 11.0.2 | 601,781 bytes | 95 | **>=22** | 依賴 `gaxios ^7`、`jws`、`gcp-metadata` 等 | <https://registry.npmjs.org/google-auth-library/latest> |
| `gaxios` | 7.3.1 | 679,009 bytes | 78 | >=18 | 依賴 `node-fetch@3`、`https-proxy-agent`、`extend` | <https://registry.npmjs.org/gaxios/latest> |

- 本專案目前安裝 ~203 MB 的 `googleapis` 只為了 `values.get` 與 `values.update` 兩個端點；改用 `@googleapis/sheets` 可縮小約 280 倍，API 形狀相同（同樣是 `sheets({version:'v4', auth})`）。
- 更輕：`google-auth-library` 的 `JWT` client 支援 `client.fetch(url)` 直接發已簽章的請求（README 範例：`new JWT({ email, key, scopes })` → `await client.fetch(url)`），亦支援 `GOOGLE_APPLICATION_CREDENTIALS`。
  來源：<https://github.com/googleapis/google-auth-library-nodejs/blob/main/README.md>
- REST 端點很單純：`PUT https://sheets.googleapis.com/v4/spreadsheets/{spreadsheetId}/values/{range}?valueInputOption=USER_ENTERED`，body 為 `ValueRange`，scope `https://www.googleapis.com/auth/spreadsheets`。
  來源：<https://developers.google.com/workspace/sheets/api/reference/rest/v4/spreadsheets.values/update>
- 注意：`google-auth-library@11` 的 `engines.node >= 22`，高於 SDK 擬定的 `^20.19.0`；`googleapis@176` 仍鎖 `google-auth-library 10.5.0`。若要支援 Node 20，需鎖 `google-auth-library@^10` 或自行用 `jose`/`node:crypto` 做 RS256 JWT 換 token（約 40 行）。
- **建議**：SDK 依賴 `@googleapis/sheets` 或直接 `google-auth-library`（peer/optional）+ 全域 `fetch`；並在建構子開放 `auth` 注入（接受任何有 `getRequestHeaders()` / `fetch()` 的物件）與 `fetch` 注入，以便測試時不碰網路。

---

## 5. 共用邏輯問題：兩份實作怎麼不分叉

### 5.1 三個選項

| 選項 | 做法 | 代表案例 | 本專案適用度 |
| --- | --- | --- | --- |
| (1) 接受重複 + 黃金夾具 | `fixtures/*.json` 描述輸入/輸出（nest、flat、sheet 二維陣列），Rust 與 TS 各自跑同一組 | 常見於 parser/formatter 的 conformance tests（Biome 的 prettier tests、oxc 的 test262） | **高**：邏輯 100 行，夾具可以完整覆蓋 |
| (2) napi / wasm | TS 直接呼叫 Rust | Rolldown、oxc、SWC、Biome js-api | 低：詳 §2.7 |
| (3) SDK spawn CLI | JS API 起子程序，經 stdin/stdout 協定溝通 | esbuild：`lib/shared/stdio_protocol.ts` 開頭註明「The JavaScript API communicates with the Go child process over stdin/stdout using this protocol. It's a very simple binary protocol … basically JSON with UTF-8 encoding and an additional byte array primitive」；`lib/npm/node.ts` 以 `child_process.spawn(command, [..., '--service=<ver>', '--ping'], { stdio: ['pipe','pipe','inherit'] })` 長駐服務 | 低：需設計協定、處理子程序生命週期、CLI 平台套件仍要發佈；而 esbuild 這麼做是因為核心是 Go 且需要長駐 watch/serve |

來源：<https://raw.githubusercontent.com/evanw/esbuild/main/lib/shared/stdio_protocol.ts>、<https://raw.githubusercontent.com/evanw/esbuild/main/lib/npm/node.ts>、esbuild 安裝說明（optionalDependencies 與 wasm 慢 10x）<https://esbuild.github.io/getting-started/#wasm>

### 5.2 黃金夾具的具體規格（建議）

```
fixtures/
  conformance/
    001-basic/
      languages.json        # ["en","zh-TW"]
      nest/en.json, nest/zh-TW.json
      flat/en.json, flat/zh-TW.json
      sheet.json            # string[][]，第一列為 ["key", ...languages]
    002-missing-translation/   # 缺譯 → sheet 空字串、pull 時略過
    003-numeric-key-rejected/  # "a.0.b" → 兩邊都必須報錯（現行 TS 行為：throw 'Key can not be Number'）
    004-empty-string-dropped/  # 現行 flatToNest 會跳過 falsy 值，需在夾具中定死
```

兩邊各寫一個 table-driven test 讀此目錄：Rust 用 `insta`/`serde_json`，TS 用 `vitest`。CI 於 root 同時跑 `cargo test` 與 `pnpm test`，任何一方改行為必須同時更新夾具，自然迫使另一方跟上。

---

## 6. 建議的 repo 佈局

```
google-sheet-languages-model/
├── Cargo.toml                 # workspace
├── pnpm-workspace.yaml        # packages/*
├── package.json               # private，root scripts（changesets、turbo 可選）
├── crates/
│   ├── gslm-core/             # flat⇄nest、sheet⇄model、錯誤型別（純邏輯，無 I/O）
│   ├── gslm-sheets/           # Google Sheets HTTP + service-account JWT（reqwest/ureq）
│   └── gslm-cli/              # clap，產出 `gslm` 二進位
├── packages/
│   ├── sdk/                   # @gn00678465/gslm（Node SDK，ESM-only）
│   │   ├── src/
│   │   │   ├── index.ts       # export { GslmClient, LanguagesModel, errors, types }
│   │   │   ├── client.ts      # GslmClient({ sheetId, auth, fetch? })
│   │   │   ├── model.ts       # LanguagesModel（flat/nest）
│   │   │   ├── sheet-codec.ts # sheetValue ⇄ model（純函式）
│   │   │   ├── fs.ts          # loadFromFolder / saveToFolder（Node-only 子路徑）
│   │   │   └── errors.ts      # GslmError > SheetsApiError(status, body) / InvalidKeyError …
│   │   ├── tsdown.config.ts   # format: 'esm', dts, exports: true, publint: true, attw: true
│   │   └── package.json       # type: module, engines ^20.19.0 || >=22.12.0, sideEffects: false
│   ├── cli/                   # gslm（npm launcher；可選，等同 @biomejs/biome 的 bin/biome）
│   │   ├── bin/gslm.js
│   │   └── package.json       # optionalDependencies: @gn00678465/gslm-cli-<platform>
│   └── cli-*/                 # 由 CI 生成的平台套件（參考 Biome packages/@biomejs/cli-*）
├── fixtures/conformance/      # §5.2 黃金夾具（Rust 與 TS 共用）
├── .changeset/
└── .github/workflows/
    ├── ci.yml                 # cargo test + pnpm test + publint/attw
    └── release.yml            # changesets 發 npm；Rust 以 cargo-dist 或 matrix 產二進位
```

SDK `package.json` 建議：

```jsonc
{
  "name": "@gn00678465/gslm",
  "type": "module",
  "sideEffects": false,
  "engines": { "node": "^20.19.0 || >=22.12.0" },
  "exports": {
    ".":        { "types": "./dist/index.d.mts", "default": "./dist/index.mjs" },
    "./node":   { "types": "./dist/fs.d.mts",    "default": "./dist/fs.mjs" },
    "./package.json": "./package.json"
  },
  "files": ["dist"],
  "peerDependencies": { "google-auth-library": ">=10" },
  "peerDependenciesMeta": { "google-auth-library": { "optional": true } }
}
```

- `./node` 子路徑隔離 `node:fs` 依賴，讓核心入口可在 Edge/瀏覽器使用（比照 `@biomejs/js-api` 的 `./nodejs` 與 Anthropic 的 `dangerouslyAllowBrowser` 思路）。
- 錯誤類別比照 Anthropic：`SheetsApiError` 依 status 再細分（`AuthenticationError` 401、`PermissionDeniedError` 403、`NotFoundError` 404、`RateLimitError` 429）。
- CLI 的 npm launcher 是否要做，取決於使用者是否習慣 `npx gslm`；若只走 `cargo install` / GitHub Releases，可省掉 `packages/cli*` 與整個平台矩陣。

---

## 7. Open risks

1. **兩份實作分叉**：黃金夾具只能覆蓋「可列舉」的行為；邊界情形（key 含 `.` 但非巢狀、空字串、`null`、非字串值、重複 key）必須在夾具裡明文定義，否則 Rust/TS 會各自選擇。建議先寫一份 `SPEC.md` 定義轉換規則再動工。
2. **`google-auth-library@11` 要求 Node ≥22**，與 ESM-only 的 Node 20.19 底線衝突；要嘛鎖 `^10`，要嘛自行實作 service-account JWT（RS256）換 token，但後者要承擔金鑰處理的正確性責任。
3. **`lodash-es/set` 的 prototype pollution 防護**：現行 `flatToNest` 用 `set()`，key 為 `__proto__.x` 會被 lodash 擋下；Rust 版用 `serde_json::Map` 無此問題，但 TS 若改自寫 set 必須補檢查，並放進夾具。
4. **現行行為中可能不想保留的「特性」**：`flatToNest` 跳過 falsy 值、`sheetValueToLanguageModel` 跳過空 cell、只以 `languages[0]` 的 key 集合決定列順序。改版時要決定是修正還是相容，並同步到 Rust。
5. **`require(esm)` 在 Node 20.19–22.12 之間的灰色地帶**：Node 20.18 以下與 22.11 以下的 CJS 使用者會直接失敗；`engines` 要寫清楚，並在 README 註明。若仍需 dual，tsdown `format: ['esm','cjs']` + `attw` 可驗證，但維護成本回升。
6. **若未來真的要 napi**：napi-rs v3 的官方範本含 13 target CI，且 `oxc-parser` 的 loader 顯示版本不一致會直接 throw；pnpm/yarn 的 `optionalDependencies` 被 `--no-optional` 關掉時會裝不到平台套件（esbuild 文件特別提到需要 install script 補救）。這些成本在決定前要有心理準備。
7. **Biome 的 JS API 仍是 alpha**：不要把「成熟專案也這麼做」解讀為「wasm 綁定很成熟」——Biome 自己的 JS API 至今仍標示 not ready for production。
