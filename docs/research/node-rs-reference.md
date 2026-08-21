# 參考 `napi-rs/node-rs`：monorepo 佈局與 CI 對應到 gslm

> 調查日期：2026-08-21。來源皆為 <https://github.com/napi-rs/node-rs> `main` 分支的實際檔案。
> 前提：已決定採 Rust core + napi-rs（見記憶／ADR），本文只回答「node-rs 怎麼組織，我們照搬哪些、改哪些」。

## 1. node-rs 的結構

### 1.1 根目錄

| 檔案 | 作用 | 來源 |
|---|---|---|
| `Cargo.toml` | `[workspace] members = ["./crates/alloc", "./packages/argon2", …]`，`resolver = "2"`；所有第三方依賴集中在 `[workspace.dependencies]`；`[profile.release]` 設 `codegen-units = 1`、`lto = true`、`strip = 'symbols'` | [Cargo.toml](https://github.com/napi-rs/node-rs/blob/main/Cargo.toml) |
| `package.json` | `private: true`，`workspaces: ["packages/*"]`；scripts 用 `yarn workspaces foreach -A --no-private run build/artifacts`；devDeps：`@napi-rs/cli ^3.8`、`@napi-rs/wasm-runtime`、`emnapi`、`ava`、`lerna`、`oxlint`/`oxfmt` | [package.json](https://github.com/napi-rs/node-rs/blob/main/package.json) |
| `crates/` | 只有 `alloc`（共用的 mimalloc global allocator），**不是**業務邏輯 | [crates/](https://github.com/napi-rs/node-rs/tree/main/crates) |
| `packages/<name>/` | **每個 npm 套件同時也是一個 Rust crate**（`Cargo.toml` + `package.json` 同目錄） | 下節 |
| `rust-toolchain.toml`、`.cargo/`、`.taplo.toml`、`renovate.json`、`lerna.json` | 工具鏈鎖定、cargo config、TOML 格式化、依賴升級、發佈 | 根目錄 |

### 1.2 單一 package（以 `packages/jsonwebtoken` 為例）

```
packages/jsonwebtoken/
├── Cargo.toml          # [lib] crate-type = ["cdylib"]；napi/napi-derive/napi-build 走 workspace
├── build.rs            # fn main() { napi_build::setup(); }
├── src/lib.rs + *.rs   # #[napi] 函式；async 用 napi Task / AsyncTask
├── package.json        # "napi": { binaryName, targets[] }；scripts: build = napi build --platform --release
├── index.js            # napi CLI 自動產生的 loader（musl 偵測、逐平台 require、wasi fallback）
├── index.d.ts          # napi CLI 自動產生
├── browser.js / *.wasi*.js / wasi-worker*.mjs   # wasm32-wasip1-threads 的瀏覽器 / WASI 載入器
├── __tests__/          # ava .spec.ts
├── benchmark/
└── npm/<target>/       # napi create-npm-dirs 產生的平台子套件（gitignore）
```

`package.json` 重點欄位（[原檔](https://github.com/napi-rs/node-rs/blob/main/packages/jsonwebtoken/package.json)）：

```jsonc
{
  "main": "index.js", "types": "index.d.ts", "browser": "browser.js",
  "files": ["index.d.ts", "index.js"],
  "publishConfig": { "access": "public", "registry": "https://registry.npmjs.org/" },
  "scripts": {
    "artifacts": "napi artifacts -d ../../artifacts",
    "build": "napi build --platform --release",
    "prepublishOnly": "napi prepublish",
    "version": "napi version"
  },
  "napi": { "binaryName": "jsonwebtoken", "targets": [ /* 14 個 */ ] }
}
```

`Cargo.toml`（[原檔](https://github.com/napi-rs/node-rs/blob/main/packages/jsonwebtoken/Cargo.toml)）：

```toml
[lib]
crate-type = ["cdylib"]
[dependencies]
napi = { workspace = true, default-features = false, features = ["napi3", "serde-json-ordered", "object_indexmap"] }
napi-derive = { workspace = true }
[build-dependencies]
napi-build = { workspace = true }
```

注意 `serde-json-ordered` + `object_indexmap`：讓 JS 物件 ⇄ `serde_json::Value` 保持 key 順序——正是我們 i18n 檔 diff 需要的。

### 1.3 CI（`.github/workflows/ci.yaml`，610 行）

Job 拓撲（[原檔](https://github.com/napi-rs/node-rs/blob/main/.github/workflows/ci.yaml)）：

```
lint ─┐
bench ┤
build (13 target 矩陣) ──► test-* (每個平台各一個 job，下載 .node 後跑 ava) ──► publish
build-freebsd ─┘
```

- **build 矩陣**：macOS（x64/arm64 原生）、Windows（x64/i686/arm64 原生）、Linux gnu（`--use-napi-cross` + `TARGET_CC=clang`）、Linux musl（`-x` 用 zig，`mlugg/setup-zig`）、Android、armv7、`wasm32-wasip1-threads`（下載 wasi-sdk）。每個 target 上傳 `packages/*/*.node` 為 artifact。
- **test 矩陣**：macOS/Windows 直接跑；Linux 各變體用 `tj-actions/docker-run` 起 `node:<ver>-slim` / `-alpine` 容器，arm64 用 `ubuntu-24.04-arm` runner，armv7 用 QEMU；WASI 用 `NAPI_RS_FORCE_WASI=true`。
- **publish**：`napi create-npm-dirs` → `yarn artifacts`（把 artifact 搬到各 `npm/<target>/`）→ `lerna publish from-package`，只在 commit message 以 `chore(release): publish` 開頭時執行；`npm config set provenance true`，用 `id-token: write` 做 npm provenance。

## 2. 對應到 gslm：照搬 vs 調整

| 面向 | node-rs 做法 | gslm 建議 | 理由 |
|---|---|---|---|
| package = crate 同目錄 | 是 | **是**，`packages/gslm/` 放 napi crate | 少一層間接，napi CLI 預設就找同目錄的 `Cargo.toml` |
| 業務邏輯位置 | 直接寫在 `packages/*/src`（每個套件只包一個上游 crate） | **抽到 `crates/gslm-core`**，`packages/gslm` 只做 `#[napi]` 薄包裝 | 我們還要給 `crates/gslm-cli`（clap 二進位）共用；node-rs 沒有 CLI 需求所以不用抽 |
| targets | 14 個（含 Android、FreeBSD、i686、armv7、wasm） | 先 **7 個**：darwin x64/arm64、win x64、linux gnu x64/arm64、linux musl x64/arm64；`wasm32-wasip1-threads` 暫緩 | Sheets HTTP 走 reqwest/tokio，wasm 端 HTTP 需橋接 host fetch，額外工程；Android/FreeBSD/armv7 沒有使用情境 |
| 套件管理 | yarn 4 + lerna | **pnpm workspace + changesets**（或 `napi version` + 單套件） | 現有專案已用 pnpm；只有一個 npm 主套件時 lerna 過重 |
| 測試 | ava + `@oxc-node/core/register` | vitest（現有） | 無需改 |
| Lint/format | oxlint + oxfmt + cargo fmt/clippy | 同上，可直接照抄 `lint-staged` 段落 | 輕量、Rust 寫的，與專案調性一致 |
| 發佈 | npmjs + provenance | **npmjs**（從 GitHub Packages 遷出） | `optionalDependencies` 平台套件在 GitHub Packages 上未驗證；provenance 只有 npmjs 支援 |
| allocator | `crates/alloc` 共用 mimalloc | 不需要 | CLI 工具非高頻呼叫 |
| `[profile.release]` | lto / codegen-units=1 / strip | 照抄 | 減小 `.node` 與二進位體積 |
| CI 觸發發佈 | commit message 前綴 | 改用 changesets 的 release PR 或 tag | 與現有 bumpp 流程接近 |

### 2.1 建議佈局

```
google-sheet-languages-model/
├── Cargo.toml                  # workspace: crates/*, packages/gslm
├── package.json                # private, pnpm workspaces
├── pnpm-workspace.yaml         # packages/*
├── rust-toolchain.toml
├── crates/
│   ├── gslm-core/              # lib：flat⇄nest、sheet⇄model、config 解析、Sheets HTTP、auth
│   └── gslm-cli/               # bin：clap → `gslm`
├── packages/
│   ├── gslm/                   # napi crate + npm 主套件（@gn00678465/gslm）
│   │   ├── Cargo.toml          # cdylib，依賴 gslm-core
│   │   ├── build.rs
│   │   ├── src/lib.rs          # #[napi] 包裝
│   │   ├── package.json        # "napi": { binaryName: "gslm", targets: [7 個] }
│   │   ├── index.js / index.d.ts   # 自動產生
│   │   ├── npm/<target>/       # 自動產生（gitignore）
│   │   └── __tests__/
│   └── gslm-cli-<target>/      # (若 CLI 走 npm 發佈) 平台二進位套件，CI 產生
└── .github/workflows/ci.yml    # 照 node-rs 的 build→test→publish 三段
```

### 2.2 napi 端的 async

node-rs 的 `jsonwebtoken` 用 `napi::Task` + `AsyncTask`（CPU-bound，在 libuv threadpool 跑）。我們是 I/O-bound（HTTP），應改用 napi-rs v3 的 `#[napi] async fn` 搭配 tokio（napi 的 `tokio_rt` feature），回傳 `Promise`。兩種模式官方文件：<https://napi.rs/docs/concepts/async-task> 與 <https://napi.rs/docs/concepts/async-fn>。

## 3. 第一張票應該做什麼

照 node-rs 骨架建 workspace，`gslm-core` 只放一個 `flatten()`，`packages/gslm` 暴露它，CI 跑通 7 個 target 的 build → test → `napi create-npm-dirs` → dry-run publish。這一步驗證的是整條發佈管線，而不是邏輯；通過後再把 Sheets/auth/config 搬進 `gslm-core`。
