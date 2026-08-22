# gslm CLI 範例

此目錄使用新式、不可執行的 `gslm.toml` 設定檔。將 Google service-account
憑證放為未提交的 `credentials.json`，並把 `YOUR_GOOGLE_SHEET_ID` 改成實際 ID。

```bash
# 從 Sheet 下載，建立或更新 example/i18n/*.json
pnpm example:cli:pull

# 將本地 Catalog 完整寫回 Sheet
pnpm example:cli:push
```

也可以不修改檔案而預覽操作：

```bash
node ../packages/gslm/bin/gslm.js --config gslm.toml pull --dry-run
node ../packages/gslm/bin/gslm.js --config gslm.toml push --dry-run
```

若偏好從環境變數提供憑證，請把 `[credentials]` 改為：

```toml
[credentials]
env = "GSLM_CREDENTIALS_JSON"
```

然後將完整 JSON 放在未提交的 `.env`（可參考 `.env.example`）。
