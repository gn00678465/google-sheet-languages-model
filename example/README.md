# gslm CLI 範例

此目錄本身就是一個 gslm 專案：`gslm.toml` 放在專案根目錄，指令從此目錄執行
即可自動找到它，不需要 `--config`。`path` 與 `credentials.file` 都相對於設定
檔所在目錄解析。

將 Google service-account 憑證放為未提交的 `credentials.json`，並把
`YOUR_GOOGLE_SHEET_ID` 改成實際 ID。

```bash
# 從 Sheet 下載，建立或更新 example/i18n/*.json
pnpm example:cli:pull

# 將本地 Catalog 完整寫回 Sheet
pnpm example:cli:push
```

上面兩個 script 等同於在此目錄內執行：

```bash
cd example
node ../packages/gslm/bin/gslm.js pull
node ../packages/gslm/bin/gslm.js push
```

也可以不修改檔案而預覽操作：

```bash
cd example
node ../packages/gslm/bin/gslm.js pull --dry-run
node ../packages/gslm/bin/gslm.js push --dry-run
```

若偏好從環境變數提供憑證，請把 `[credentials]` 改為：

```toml
[credentials]
env = "GSLM_CREDENTIALS_JSON"
```

然後將完整 JSON 放在未提交的 `.env`（可參考 `.env.example`）。
