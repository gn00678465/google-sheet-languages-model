# @gn00678465/google-sheet-languages-model (napi-rs 版)

Rust core（`crates/gslm-core`）+ napi-rs 綁定。本套件同時提供 Node SDK 與 `gslm` CLI（ADR-0001、ADR-0002）。

> 目前為 spec 0001 的骨架：只暴露 `flatten()` 與 `version()`，`gslm` 只支援 `--version`。

## 本地開發

```bash
# 需求：Rust（見 rust-toolchain.toml）、Node >= 20、pnpm
pnpm install

# Rust 單元測試 / lint
cargo test --workspace
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings

# 建 .node（debug）並跑 JS 測試
pnpm -C packages/gslm build:debug
pnpm -C packages/gslm test          # node:test，不需 vitest
node packages/gslm/bin/gslm.js --version
```

`index.js` / `index.d.ts` 由 `napi build` 自動產生並提交到 repo；`*.node` 與 `npm/` 不提交。

## 發佈（canary）

推送 tag `napi-v<version>`（例如 `napi-v0.0.0-canary.1`）會觸發 `.github/workflows/napi.yml`：

1. 7 個 target 建 `.node`，並在各平台（含 alpine / arm64 容器）跑測試
2. `napi create-npm-dirs` + `napi artifacts` 組裝平台子套件，`napi prepublish` 發佈子套件並把 `optionalDependencies` 寫進主套件，再 `npm publish` 主套件到 GitHub Packages
3. `verify-install` 在乾淨環境（linux gnu / alpine / macOS / Windows）從 GitHub Packages 安裝，執行 `scripts/verify-install.cjs`，並測 `npm i -g` 後的 `gslm --version`

PR 與 `rewrite/**` 分支的 push 跑同樣流程但 publish 為 `--dry-run`。

手動驗證已發佈版本：

```bash
mkdir /tmp/consumer && cd /tmp/consumer && npm init -y
npm install @gn00678465/google-sheet-languages-model@<version>
node /path/to/repo/scripts/verify-install.cjs <version>
```

## 支援平台

darwin x64/arm64、windows x64、linux gnu x64/arm64、linux musl x64/arm64。
