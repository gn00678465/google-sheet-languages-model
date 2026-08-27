# Contributing

## 開發與驗證

```bash
pnpm install
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm -C packages/gslm build:debug
pnpm -C packages/gslm test
pnpm -C packages/gslm typecheck
```

請將核心行為放在對應 Rust crate；`packages/gslm` 僅負責 napi 資料轉換和
JavaScript 包裝。CLI 行為以 `gslm_cli::run(argv, options)` 作為整合測試邊界，
bin 行為則以 `packages/gslm/bin/gslm.js` 的子程序測試驗證。

## 提交訊息

遵循 [Conventional Commits](https://www.conventionalcommits.org/)：

```text
feat(cli): 新增 Catalog 同步指令
fix(config): 修正環境變數覆寫
```

若有 breaking change，請使用 `feat!:` 或在提交訊息正文加入 `BREAKING CHANGE:`。
