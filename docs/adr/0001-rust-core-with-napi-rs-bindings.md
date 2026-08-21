---
status: accepted
date: 2026-08-21
---

# Rust core 搭配 napi-rs 產生 Node SDK

gslm 重寫時，所有領域邏輯（flat⇄nest、sheet⇄model、Google Sheets HTTP、Service Account 驗證、config 解析）只存在於 Rust crate `gslm-core`；Node SDK 是以 napi-rs 包裝該 crate 的原生模組，不另寫 TypeScript 實作。

## Considered Options

- **純 TypeScript SDK + Rust CLI + 共用 conformance fixtures**（研究報告 `docs/research/node-sdk-architecture.md` 的建議）：邏輯約 100 行，napi 的平台矩陣看似不成比例。被否決的原因是它要同時維護 rs、ts、json 夾具三份東西，任何行為變更都要改三處；維護者明確表示寧可不重構也不接受這種分叉風險。
- **wasm（wasm-bindgen / wasi）**：免平台套件，但 Sheets HTTP 需橋接 host fetch，且 Biome 的 wasm JS API 至今仍標示 alpha。
- **SDK spawn CLI 子程序（esbuild 模式）**：需自訂 stdin/stdout 協定，平台套件仍要發佈，對純資料轉換的工具是過度設計。

## Consequences

- 每個支援平台需發佈一個 `optionalDependencies` 子套件（初期 7 個 target），CI 照 `napi-rs/node-rs` 的 build → test → publish 三段式（見 `docs/research/node-rs-reference.md`）。
- SDK 僅能在支援 Node-API 的 runtime 執行（Node、Bun、Deno），不能在瀏覽器／Edge 使用。
- HTTP 與驗證必須在 Rust 端完成，否則 napi 失去意義；技術選型見 ADR-0004。
