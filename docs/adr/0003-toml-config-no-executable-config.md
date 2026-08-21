---
status: accepted
date: 2026-08-21
---

# 設定檔以 TOML 為主，不再支援可執行的 JS/TS 設定檔與內嵌憑證

新版 config 為 `gslm.toml`（JSON/JSONC 為次要格式，共用同一份由 `schemars` 產生的 JSON Schema）。不支援 YAML、不支援 `.js/.ts/.mjs/.cjs`，設定檔內不得內嵌 Service Account 物件，憑證只能以檔案路徑、環境變數名稱或 Google ADC 提供。

理由：CLI 核心改為 Rust（ADR-0001），無法執行 JS/TS；Rust 生態的 YAML serde 實作皆已封存或自標 deprecated，TOML 與 JSON 是唯二一線維護的選項；`[[targets]]` 陣列在 TOML 下可讀性優於 JSON。內嵌憑證是既有設計的洩漏隱患，趁 major 版一併移除。詳細比較見 `docs/research/config-format-redesign.md`。

## Consequences

- 這是 breaking change，需提供 `gslm migrate` 將舊 `gslm.config.js` 轉為 `gslm.toml`；因為只有 Node 能執行舊設定檔，migrate 邏輯放在 JS 端（SDK 套件內），不進 Rust core。
- 值的優先序固定為 CLI flag > `GSLM_*` 環境變數 > 設定檔 > 內建預設。
- 設定檔含 `version` 欄位，供日後 schema 演進。
