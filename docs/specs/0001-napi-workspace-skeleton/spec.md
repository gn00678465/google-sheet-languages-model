---
status: done
date: 2026-08-21
adrs: [0001, 0002, 0005]
---

# Spec 0001：napi-rs workspace 骨架與發佈管線驗證

## Problem Statement

維護者決定以 Rust core + napi-rs 重寫 gslm（ADR-0001），CLI 以 JS bin 包 napi binding（ADR-0002），套件繼續發佈到 GitHub Packages（ADR-0005）。這條路線最大的不確定性不在領域邏輯——那只有約 100 行——而在**發佈管線**：7 個平台的原生模組能否在 CI 全部建出、測過，並且從 GitHub Packages 安裝時 `optionalDependencies` 是否只拉到對應平台的子套件且能載入。若這一點不成立，ADR-0005 要重評，後面搬進去的所有邏輯都得跟著搬家。

目前 repo 是單一 npm 套件（tsdown 建置的 TypeScript），沒有 Cargo workspace、沒有 pnpm workspace、沒有任何 Rust 程式碼。

## Solution

建立 monorepo 骨架：`crates/gslm-core`（Rust lib，本票只放一個 `flatten`）、`packages/gslm`（napi-rs crate 兼 npm 主套件，暴露 `flatten` 並附一支只會印版本的 `bin/gslm.js`），以及照 `napi-rs/node-rs` 三段式（build → test → publish）設計的 GitHub Actions。發佈到 GitHub Packages 後，多加一個 `verify-install` job 在乾淨容器從 GitHub Packages 安裝並實際呼叫 `flatten`。

本票完成的判準是：**一個 canary 版本從 GitHub Packages 裝得起來、`.node` 載得起來、`flatten` 算得對、且 `node_modules` 裡只有一個平台子套件**。既有 TypeScript 實作原封不動留在原位，待後續票逐步取代。

## User Stories

1. As a 維護者, I want 一個 Cargo workspace 與 pnpm workspace 共存的 repo 佈局, so that Rust crate 與 npm 套件能在同一個 repo 內以各自的工具鏈建置。
2. As a 維護者, I want 領域邏輯放在不依賴 napi 的 `gslm-core` crate, so that 日後新增獨立 CLI 二進位（ADR-0002 的延後項）時不需改動 core。
3. As a 維護者, I want `packages/gslm` 的 Rust 端只做 `#[napi]` 薄包裝, so that napi 邊界的程式碼量維持最小，邏輯變更不需碰綁定層。
4. As a 維護者, I want `flatten` 在 JS ⇄ Rust 之間保留物件 key 的順序, so that 日後 pull 產生的翻譯檔 diff 穩定。
5. As a 維護者, I want napi CLI 自動產生 `index.js` 載入器與 `index.d.ts`, so that 型別與平台偵測（含 musl）不需手寫。
6. As a 維護者, I want CI 在 7 個 target（darwin x64/arm64、windows x64、linux gnu x64/arm64、linux musl x64/arm64）建出 `.node`, so that 主要桌面與伺服器平台都有原生模組。
7. As a 維護者, I want 每個 target 建出的 `.node` 在對應平台（原生 runner 或 docker 容器）實際載入並跑測試, so that 不會發佈一個建得出來但載不起來的二進位。
8. As a 維護者, I want PR 上的 CI 跑完整 build + test 但只做 publish dry-run, so that 合併前就知道發佈步驟會不會壞，又不會汙染 registry。
9. As a 維護者, I want 推 tag 時自動以 `napi create-npm-dirs` + `napi artifacts` 組裝平台子套件並發佈到 GitHub Packages, so that 發佈是一鍵且可重現的。
10. As a 維護者, I want 發佈後有一個 job 在乾淨容器用 `.npmrc` 從 GitHub Packages 安裝剛發佈的版本, so that ADR-0005 的風險在第一次發佈就得到驗證，而不是等使用者回報。
11. As a 維護者, I want 該驗證 job 斷言 `node_modules` 內只存在一個 `@gn00678465/gslm-*` 平台子套件, so that 確認 GitHub Packages 尊重 `os` / `cpu` / `libc` 欄位，沒有把全部平台都裝下來。
12. As a 維護者, I want 該驗證 job 呼叫 `flatten` 並比對結果, so that 證明的是端到端可用，不只是檔案存在。
13. As a 維護者, I want 平台子套件的 `optionalDependencies` 版本與主套件版本嚴格一致, so that 不會出現主套件新版配到舊版 binding 的載入錯誤。
14. As a 維護者, I want `bin/gslm.js` 在本票只印出版本號, so that `npx gslm --version` 能證明 bin → napi → Rust 的呼叫鏈通了，而不需先實作任何指令。
15. As a 維護者, I want 根目錄的 lint / format 涵蓋 Rust（`cargo fmt`、`cargo clippy`）與 JS/TS, so that 兩種語言的程式碼風格從第一天就受 CI 把關。
16. As a 維護者, I want `rust-toolchain.toml` 鎖定 Rust 版本, so that 本地與 CI 的編譯結果一致。
17. As a 維護者, I want 既有的 TypeScript 實作與範例在本票後仍能 `pnpm build` 與執行, so that 在邏輯搬完之前，現行使用者的流程不中斷。
18. As a 維護者, I want 工作在獨立分支上進行而非直接在 `main`, so that 重寫期間 `main` 仍可發佈修補版。
19. As a 維護者, I want Cargo release profile 開啟 LTO 與 symbol strip, so that `.node` 體積最小。
20. As a 使用者（Node 專案）, I want 安裝主套件時只下載自己平台的二進位, so that 安裝時間與磁碟用量不會因為支援 7 個平台而膨脹。
21. As a 使用者, I want 在不支援的平台安裝時得到明確的錯誤訊息而非 `MODULE_NOT_FOUND`, so that 能快速判斷是平台問題而非安裝壞掉。
22. As a 使用者, I want TypeScript 型別隨主套件附帶, so that 不需另裝 `@types` 套件。
23. As a 使用者, I want 主套件宣告支援的 Node 最低版本, so that 套件管理器能在不相容時提早警告。
23a. As a 使用者, I want 以 `npm i -g` / `pnpm add -g` 全域安裝後直接在終端執行 `gslm`, so that 重構前的全域安裝使用方式維持不變。
24. As a 後續票的實作者, I want 一份 README 說明本地如何建置、測試、手動發佈 canary, so that 不必重新摸索 napi CLI 指令。
25. As a 後續票的實作者, I want `gslm-core` 的 `flatten` 有 Rust 單元測試示範測試寫法, so that 後續搬邏輯時有樣板可循。

## Implementation Decisions

### 佈局

- 根目錄 `Cargo.toml` 為 workspace，members 為 `crates/gslm-core` 與 `packages/gslm`；所有第三方依賴版本集中在 `[workspace.dependencies]`；`[profile.release]` 設 `lto = true`、`codegen-units = 1`、`strip = "symbols"`（照 node-rs）。
- 根目錄 `package.json` 改為 `private: true` 的 pnpm workspace root；`pnpm-workspace.yaml` 包含 `packages/*`。既有 TypeScript 原始碼、`tsdown.config.ts`、`example/` 與其 scripts 原地保留，不搬、不改，僅因 root 變 private 而暫時不可從 root 發佈——這是預期行為，直到舊實作被取代。
- `crates/gslm-core`：`[lib]`，無 napi 依賴。本票只暴露 `flatten(value, separator) -> Result`：將巢狀物件攤平為以 separator 連接的單層 key；key 任一段為純數字時回傳錯誤（與 CONTEXT.md 的 Key 定義一致）。使用 `serde_json` 並開啟 `preserve_order`。
- `packages/gslm`：同目錄放 `Cargo.toml`（`crate-type = ["cdylib"]`，依賴 `gslm-core`、`napi`、`napi-derive`）、`build.rs`、`package.json`、`src/lib.rs`。`napi` 開啟 `napi3`、`serde-json-ordered`、`object_indexmap` features 以保 key 順序。`#[napi] fn flatten(value: Object/Value, separator?: String)`，錯誤轉為 JS `Error`。另暴露 `#[napi] fn version() -> String` 回傳 core crate 版本。
- `packages/gslm/bin/gslm.js`：本票只處理 `--version` / `-v`，印出 `version()`；其他參數印出「尚未實作」並以非零碼退出。不引入 clap，clap 在後續 CLI 票加入。
- 主套件 `package.json`：`name` 沿用既有 `@gn00678465/google-sheet-languages-model` 或改為 `@gn00678465/gslm` —— 見 Further Notes，實作前須由維護者確認。`napi.binaryName = "gslm"`，`napi.targets` 為上述 7 個 target。`engines.node` 為 `>=20`。`publishConfig.registry` 指向 `https://npm.pkg.github.com`。`files` 只含 `index.js`、`index.d.ts`、`bin/`。
- `rust-toolchain.toml` 鎖 stable 最新次版本；edition 2024。

### CI（`.github/workflows/ci.yml`）

- 觸發：PR 與 push 到重寫分支跑 `lint` + `build` + `test-*` + `publish`（dry-run）；push tag `v*` 另跑真正發佈與 `verify-install`。
- `build` 矩陣 7 個 target：macOS 兩個原生；windows x64 原生；linux gnu x64/arm64 用 `--use-napi-cross`；linux musl x64/arm64 用 `-x`（zig）。產物以 `bindings-<target>` 上傳。
- `test-*`：macOS／windows 直接跑；linux gnu x64 原生；linux musl x64 用 `node:<ver>-alpine` 容器；linux arm64 gnu/musl 用 `ubuntu-24.04-arm` runner + 對應容器。測試以 vitest 執行 `packages/gslm/__tests__`，Node 版本矩陣 `20`、`22`、`24`。
- `publish`（tag 觸發）：下載全部 artifacts → `napi create-npm-dirs` → `napi artifacts` → 對主套件與各 `npm/<target>/` 子套件執行 `npm publish`，認證用 `GITHUB_TOKEN`（`packages: write`）。PR 上同樣步驟但 `--dry-run`。
- `verify-install`（`needs: publish`）：矩陣至少涵蓋 linux gnu x64、linux musl x64（alpine 容器）、macOS arm64、windows x64。在空目錄寫入 `.npmrc`（`@gn00678465:registry=https://npm.pkg.github.com`、token 為 `GITHUB_TOKEN`），`npm install <主套件>@<剛發佈版本>`，然後執行一支腳本：(1) 列出 `node_modules/@gn00678465/` 並斷言平台子套件數量恰為 1 且名稱符合當前平台；(2) `require` 主套件呼叫 `flatten({a:{b:"x"}, c:"y"})`，斷言結果為 `{"a.b":"x", c:"y"}` 且 `Object.keys` 順序為 `["a.b","c"]`；(3) 執行 `npx gslm --version` 斷言輸出含版本號；(4) 另以 `npm i -g <主套件>@<版本>` 全域安裝，確認 `gslm --version` 在 PATH 上可直接執行且輸出相同版本號。任一斷言失敗即 job 失敗。
- canary 版本號格式 `0.0.0-canary.<short-sha>`，以 `napi version` 或腳本統一寫入主套件與子套件，確保嚴格一致。

### 分支

- 所有工作在 `rewrite/napi` 分支；本票不合併回 `main`。

## Testing Decisions

- 好的測試只觀察外部行為：對 JS 呼叫者而言是 `flatten` 的回傳值、key 順序與錯誤訊息；對安裝者而言是「裝得起來、載得起來、算得對」。不測 napi 內部、不測 `index.js` 載入器的分支（那是 napi CLI 產生的）。
- **Rust 單元測試**（`gslm-core`）：`flatten` 的基本攤平、多層巢狀、空物件、separator 自訂、純數字段落報錯、key 順序保留。以 `cargo test` 執行。
- **JS 整合測試**（`packages/gslm/__tests__`，vitest）：從套件根目錄 import，驗證與 Rust 測試相同的案例加上 JS 特有的：非物件輸入的錯誤、錯誤是 `Error` 實例、`version()` 回傳非空字串。每個 CI 平台都跑同一份。
- **安裝驗證**（`verify-install` job 內的腳本）：如上節所述，是本票的驗收測試。
- 先例：既有 `src/__test__/LanguagesModel.test.ts` 的 vitest 風格（`describe`/`it`/`expect`）沿用；Rust 端無先例，採標準 `#[cfg(test)] mod tests`。

## Out of Scope

- 任何領域邏輯搬遷：unflatten、sheet⇄model、Sheets HTTP、驗證、config 解析、pull/push 指令。
- clap 與 CLI 參數解析（`bin/gslm.js` 只有 `--version`）。
- `wasm32-wasip1-threads`、Android、FreeBSD、armv7、i686 target。
- 刪除或搬移既有 TypeScript 實作。
- changesets / 正式版本號策略（本票只發 canary）。
- npm provenance（GitHub Packages 不支援）。
- 文件網站、JSON Schema、`gslm migrate`。

## Further Notes

- **套件名稱待確認**：既有名稱 `@gn00678465/google-sheet-languages-model` 與平台子套件命名（napi 預設 `<name>-<platform>`）會非常長；改名為 `@gn00678465/gslm` 較乾淨但等於新套件，舊名需另行 deprecate。實作者動工前請維護者決定，本 spec 不預設。
- **GitHub Packages 的 canary 堆積**：GitHub Packages 可透過 API 刪除版本，但不建議在 CI 自動刪；canary 版本累積是可接受的代價，必要時手動清理。
- **若 `verify-install` 失敗**且原因是 GitHub Packages 不處理 `optionalDependencies` 的 `os`/`cpu`/`libc` 篩選，這是 ADR-0005 的重評觸發條件，應回報維護者而非在本票內繞過（例如改用 postinstall 下載）。
- 參考實作：`docs/research/node-rs-reference.md`（node-rs 的 workspace、package.json、CI 原檔連結）與 `docs/research/node-sdk-architecture.md` §2（Rolldown / oxc 的 optionalDependencies 與載入器行為）。

## Comments

### 2026-08-21 實作備註

- 套件名沿用 `@gn00678465/google-sheet-languages-model`（維護者決定）；平台子套件為 `@gn00678465/google-sheet-languages-model-<platform>`。根目錄 `package.json` 改名為 `google-sheet-languages-model-workspace` 並設 `private: true` 以避免 workspace 內同名。
- 發佈 tag 改為 `napi-v<version>` 而非 spec 所寫的 `v*`：既有 `publish.yml` 仍綁 `v*.*.*`，兩者並存期間需區隔。
- JS 測試改用 Node 內建 `node:test`（`.cjs`）而非 vitest：vitest/esbuild 帶平台二進位，無法在 alpine／arm64 容器裡沿用 host 的 `node_modules`；`node:test` 讓容器測試零依賴。根目錄舊實作仍用 vitest。
- `bin/gslm.js --version` 印 npm 套件版本（非 Rust core 版本），但仍會先載入 binding 以證明呼叫鏈；`version()` API 回報 core 版本。
- `napi prepublish --dry-run` 不會寫入 `optionalDependencies`，因此 PR 上的 `npm publish --dry-run` 輸出不含平台子套件清單；真正發佈時才會有。

### 2026-08-21 code review 後的修正

- **版本來源**：CI 不再於發佈時改寫版本；`packages/gslm/package.json` 的 `version` 為唯一真相，tag `napi-v<version>` 必須相符。`bump.config.ts` 改為只 bump 該檔並打 `napi-v` tag，舊 `publish.yml` 刪除。`version()` 透過 `build.rs` 讀 package.json，與 `gslm --version` 一致；Cargo crate 版本維持 0.0.0（不發佈到 crates.io）。
- **loader 版本**：napi 產生的 `index.js` 內含套件版本檢查，故 publish job 改用 linux-gnu build job 上傳的 `index.js`/`index.d.ts`，不用 repo 內可能過期的副本。
- 移除 `prepublishOnly`（會在 `npm publish` 時二次執行 `napi prepublish`，對 GitHub Packages 的 409 訊息不相容而失敗），CI 明確呼叫 `napi prepublish` 後 `npm publish --ignore-scripts`。
- `node --test` 的 glob 在 Node 20 不支援，改為明列測試檔。
- `dtolnay/rust-toolchain@stable` 會覆寫 `rust-toolchain.toml`，改釘 `@1.97`。
- **陣列**：`flatten` 明確拒絕陣列（`ArrayNotSupported`，帶出 key 路徑）。舊 TS 會攤成 `days.0`，但其 `flatToNest` 又拒絕數字 key，陣列從未能完整往返；CONTEXT.md 的 Key 定義亦不允許數字段。
- `verify-install.cjs` 改為檢查 `require.cache` 中實際載入的 `.node` 是否位於唯一安裝的平台子套件內，不再自行推導 libc。
- `bin/gslm.js` 改用 `process.exitCode`，避免 macOS pipe 下 stdout 未 flush 即退出。
- 兩個 verify-install job 共用 `scripts/verify-install.sh`，並補上 linux arm64 gnu/musl。

### 2026-08-21 CI 驗證結果與延後項

- 分支 `rewrite/napi` 的 CI run [#32477775078](https://github.com/gn00678465/google-sheet-languages-model/actions/runs/32477775078) 全綠：lint、7 個 target build、21 個 test job（7 target × Node 20/22/24，含 alpine 與 arm64 容器）、publish dry-run（7 個 `.node` 正確組裝進 `npm/<platform>/`，主套件 tarball 6.1 kB）。
- 途中修正兩個 CI 問題：step 名稱含 `: ` 導致 YAML 解析失敗；`rust-toolchain.toml` 的 `channel = "1.97"` 與 action 安裝的 `1.97.1` 被 rustup 視為不同 toolchain，交叉編譯 target 缺標準函式庫，改釘精確版本。
- **維護者決定**：不發 canary。`verify-install`（ADR-0005 的 GitHub Packages `optionalDependencies` 驗證）延後到重構完成後，以第一個 `napi-v<version>-beta.1` tag 一次驗證。在此之前 ADR-0005 的風險視為未驗證。
