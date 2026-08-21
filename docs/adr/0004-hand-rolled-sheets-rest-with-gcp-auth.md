---
status: accepted
date: 2026-08-21
---

# Google Sheets 以手寫 REST 呼叫 + `gcp_auth` 實作，不用生成式 SDK

`gslm-core` 以 `reqwest`（rustls + `ring`）直接呼叫 Sheets v4 的 `values.get` 與 `values.update` 兩個 endpoint，驗證交給 `gcp_auth`（Service Account JSON／檔案／`GOOGLE_APPLICATION_CREDENTIALS`）。不採用 `google-sheets4`（google-apis-rs）或官方 `google-cloud-auth`。

理由：只用到兩個 endpoint，生成式 SDK 會拖進整個 yup-oauth2/hyper 堆疊；官方 `google-cloud-rust` 沒有 Sheets client，且其 Service Account 路徑只發 self-signed JWT，Google 文件未保證 Sheets API 接受。`gcp_auth` 走標準 JWT-bearer token 交換，與 Node 版 `google-auth-library` 行為一致。證據見 `docs/research/rust-cli-dependencies.md` §2。

## Consequences

- TLS 後端統一 rustls + `ring`，`reqwest` 需關閉預設 feature 以免與 `aws-lc-rs` provider 衝突；musl target 需確認根憑證來源。
- `gcp_auth` 為 0.x 單一維護者專案，若停更，替代路徑是 `jsonwebtoken` 自行簽 RS256 + 打 token endpoint（約 50 行）。
