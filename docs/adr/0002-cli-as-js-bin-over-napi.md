---
status: accepted
date: 2026-08-21
---

# CLI 入口是 JS bin，呼叫 napi binding；獨立二進位延後

`gslm` 指令由 npm 主套件的 `bin/gslm.js` 提供，它只把 `process.argv` 交給 Rust 端的 clap（透過 napi 暴露的函式），參數解析與所有邏輯仍在 Rust。不另外建立 `crates/gslm-cli` 獨立執行檔。

理由：使用者幾乎全是 Node／前端專案，一組平台套件就能同時供應 SDK 與 CLI，版本天然一致，CI 只需 napi 一套流程。代價是 CLI 必須有 Node 才能執行，以及多一層 Node 啟動時間。

## Consequences

- 若未來出現無 Node 環境（非 JS 專案、精簡容器）的需求，再新增 `crates/gslm-cli`（clap 二進位）發佈到 GitHub Releases，**不發 npm**；`gslm-core` 的介面設計應讓這一步只是加一個 crate，不需改動 core。
- `--help` 輸出、顏色偵測（isatty）、exit code、signal 處理都經過 Node 一層，需在 napi 邊界驗證。
