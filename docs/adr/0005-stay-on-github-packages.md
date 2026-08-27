---
status: accepted
date: 2026-08-21
---

# npm 套件繼續發佈到 GitHub Packages

主套件與各平台 `optionalDependencies` 子套件都留在 `https://npm.pkg.github.com`，不遷往 npmjs。這是維護者在知悉風險後的選擇：研究建議遷到 npmjs，因為 napi-rs 的平台子套件解析與 npm provenance 只在 npmjs 上有已驗證的前例（`napi-rs/node-rs`）。

## Consequences

- 重寫的**第一張票**必須包含「從 GitHub Packages 實際安裝主套件，確認 `optionalDependencies` 只拉到對應平台的 `.node` 並能載入」的驗證；若失敗，本 ADR 需重新評估。
- 使用者安裝仍需 `.npmrc` 設定 GitHub token（GitHub Packages 既有限制，與重寫無關）。
- 不使用 npm provenance。
