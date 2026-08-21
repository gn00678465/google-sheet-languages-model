---
status: done
date: 2026-08-21
adrs: [0001, 0002, 0004]
depends_on: [0002]
---

# Spec 0003：Google Sheets client——讀寫 Tab 與 Service Account 驗證

## Problem Statement

`gslm-core` 已能在 Catalog、Model 與 Tab 表格之間轉換，但還沒有任何方式把表格真正從 Google Sheet 讀出來或寫回去。舊版靠 `googleapis` 套件；依 ADR-0004，Rust 端要以手寫 REST（`reqwest`）加 `gcp_auth` 實作，而且這是整個重寫中**第一段進入 Rust 的 I/O 與 async**——napi 的 `async fn` + tokio runtime、TLS 後端選擇、交叉編譯時的根憑證，都在這裡首次被驗證。

舊版在這一層也有幾個沒寫成規格的行為：`values.update` 從 `A1` 覆寫但**不清除**舊有的多餘列（Sheet 上刪掉的 key 會殘留）；`valueInputOption: USER_ENTERED` 會讓 Sheets 把 `=SUM(...)` 當公式、把 `1,000` 當數字；tab 名稱未加引號直接塞進 A1 notation，含空白或特殊字元時會失敗；錯誤只有 Google 回傳的原始訊息，沒有針對「忘記把 Sheet 分享給 service account」這類最常見情況的提示。

## Solution

新增 `gslm-sheets` crate（`gslm-core` 維持無 I/O），提供一個 `SheetsClient`：以 Service Account 檔案、Service Account JSON 字串、Application Default Credentials 或**靜態 access token** 建立，提供 `read_tab(sheet_id, tab) → Table` 與 `write_tab(sheet_id, tab, table)` 兩個 async 操作。寫入時先清除整個 tab 再寫入，`valueInputOption` 改用 `RAW`，tab 名稱一律以 A1 notation 規則加引號並 URL 編碼。錯誤分類為可操作的種類（權限不足附上分享提示、找不到 Sheet、找不到 tab、憑證問題、速率限制、網路）。

napi 層暴露 `SheetsClient` class，方法回傳 Promise；憑證與 base URL 皆可由 JS 指定，讓 SDK 使用者不必碰 Rust 也能在測試中對本機 fixture server 跑完整流程。

## User Stories

### 建立 client 與憑證

1. As a 使用者, I want 以 Service Account JSON 檔案路徑建立 client, so that 沿用既有的 `credentials.json` 流程。
2. As a 使用者, I want 以 Service Account JSON 字串建立 client, so that 能從環境變數（例如 CI secret）直接提供憑證而不落地成檔案。
3. As a 使用者, I want 不提供任何憑證時走 Google Application Default Credentials（`GOOGLE_APPLICATION_CREDENTIALS` → gcloud ADC 檔 → metadata server）, so that 在 GCP 環境與本機 `gcloud auth application-default login` 後零設定可用。
4. As a 使用者, I want 以一個已取得的 access token 建立 client, so that 能用 `gcloud auth print-access-token` 或其他 OAuth 流程的結果，而不必有 service account。
5. As a 使用者, I want 憑證載入失敗（檔案不存在、JSON 格式錯、非 service_account 類型）時在**建立 client 時**就得到明確錯誤, so that 不會等到第一次 API 呼叫才發現。
6. As a 使用者, I want client 只要求 `https://www.googleapis.com/auth/spreadsheets` scope, so that service account 的權限最小化。
7. As a 使用者, I want token 在 client 生命週期內被快取與自動更新, so that 一次 push/pull 的多個請求不會重複換 token。

### 讀取 Tab

8. As a 使用者, I want `read_tab(sheet_id, tab)` 回傳與 `Model::from_table` 相容的 `Table`, so that pull 流程就是 `read_tab` → `from_table` 兩步。
9. As a 使用者, I want tab 名稱含空白、單引號、非 ASCII 字元時仍能正確讀取, so that 譯者可以用 `翻譯 (v2)` 這類名稱。
10. As a 使用者, I want 讀取時取得譯者在 Sheet 上看到的文字（formatted value）, so that 顯示為 `1,000` 的儲存格不會變成 `1000`。
11. As a 使用者, I want Sheets API 省略的尾端空儲存格與空列被保留為 `Table` 的短列／空列, so that `from_table` 的「短列即缺譯」規則直接適用。
12. As a 使用者, I want 整個 tab 為空時得到空 `Table`（而非錯誤）, so that 「空表格」的判斷統一由 `from_table` 負責。

### 寫入 Tab

13. As a 使用者, I want `write_tab(sheet_id, tab, table)` 先清除 tab 的全部內容再寫入, so that 本地刪除的 key 不會殘留在 Sheet 底部。
14. As a 使用者, I want 寫入使用 `RAW` 輸入模式, so that 以 `=`、`+` 開頭或含逗號的翻譯不會被 Sheets 解讀成公式或數字。
15. As a 使用者, I want 寫入從 `A1` 開始、以列為主維度, so that 表格與 `Model::to_table` 的形狀一一對應。
16. As a 使用者, I want 寫入空表格（只有標題列）也成功, so that 清空一個 tab 是合法操作。
17. As a 使用者, I want 清除成功但寫入失敗時得到一個指明「tab 已被清空」的錯誤, so that 我知道需要重新 push 而不是以為 Sheet 沒變。

### 錯誤

18. As a 使用者, I want 403 時的錯誤訊息提示「請將 Sheet 分享給 service account 的 email」並附上該 email（若憑證中可得）, so that 最常見的設定錯誤能一步解決。
19. As a 使用者, I want 404 時的錯誤指出是 Sheet ID 找不到, so that 能和 tab 名稱錯誤區分。
20. As a 使用者, I want tab 名稱不存在時（Google 回 400 "Unable to parse range"）得到「找不到 tab」的錯誤並附上名稱, so that 不必解讀 Google 的原始訊息。
21. As a 使用者, I want 429 與 5xx 被歸類為「暫時性」錯誤, so that CLI 層能決定是否重試。
22. As a 使用者, I want 網路層錯誤（DNS、連線、TLS）與 API 錯誤分開, so that 離線與權限問題的訊息不同。
23. As a 使用者, I want 錯誤保留 Google 回傳的 HTTP 狀態碼與訊息, so that 遇到未分類的情況仍有原始資訊可查。
24. As a 維護者, I want 錯誤型別是列舉並實作 `std::error::Error`, so that CLI 與 napi 層能依種類處理。

### Node SDK

25. As a Node SDK 使用者, I want `new SheetsClient({ credentials })` 或等價的工廠函式, so that 用 JS 物件描述憑證來源。
26. As a Node SDK 使用者, I want `readTab(sheetId, tab)` 與 `writeTab(sheetId, tab, rows)` 回傳 Promise, so that 能 `await` 並自然融入既有 async 程式。
27. As a Node SDK 使用者, I want 憑證物件支援 `{ file }`、`{ json }`、`{ accessToken }` 與「不提供＝ADC」四種形式, so that 與 Rust 端一致。
28. As a Node SDK 使用者, I want 可選的 `baseUrl` 覆寫 Sheets API 端點, so that 能在測試中指向本機 fixture server。
29. As a Node SDK 使用者, I want 拋出的 `Error` 帶有 `code` 屬性（字串列舉，對應 Rust 的錯誤種類）, so that 程式能判斷而不用比對訊息文字。
30. As a Node SDK 使用者, I want TypeScript 型別涵蓋憑證物件與錯誤 code, so that IDE 能提示。
31. As a Node SDK 使用者, I want client 建立與每次呼叫都不阻塞 Node 事件迴圈, so that 在伺服器或建置工具中使用時不會卡住其他工作。

### 維護者

32. As a 維護者, I want `gslm-sheets` 的測試不依賴 Google 服務與網路, so that CI 在任何平台都能跑、也不需要 secret。
33. As a 維護者, I want 一組以環境變數啟用的 live 測試（預設 ignore）, so that 發版前能手動對真實 Sheet 驗證一次。
34. As a 維護者, I want TLS 統一使用 rustls + `ring`，不引入 OpenSSL 或 `aws-lc-rs`, so that 7 個 target 的交叉編譯設定不變（ADR-0004）。
35. As a 維護者, I want 根憑證來源在 musl target 上也能運作, so that alpine 容器內的測試與使用者環境不會因缺 CA 而失敗。
36. As a 維護者, I want CI 的容器測試實際執行一次「對本機 HTTPS 或 HTTP fixture server 的讀寫」, so that tokio runtime 在 napi 內、在每個平台上都被驗證過。

## Implementation Decisions

### 模組

- 新 crate **`gslm-sheets`**：依賴 `gslm-core`（只用 `Table`）、`reqwest`（`default-features = false`，`json` + `rustls-no-provider`）、`rustls`（`ring` provider）、`gcp_auth`、`tokio`、`serde`/`serde_json`、`thiserror`。在 client 建立時以 `install_default()` 安裝 `ring` provider（失敗即表示已安裝，忽略）。
- `gslm-core` 不變，維持無 I/O。
- napi crate 新增對 `gslm-sheets` 的依賴，開啟 napi 的 `tokio_rt` feature 以支援 `async fn`。

### 憑證

- `Credentials` 列舉：`ServiceAccountFile(path)`、`ServiceAccountJson(string)`、`ApplicationDefault`、`AccessToken(string)`。
- 內部以一個 **token provider 抽象**（trait）統一：`gcp_auth` 實作前三種（`CustomServiceAccount::from_file` / `from_json`、`gcp_auth::provider()`），靜態 token 實作第四種。這個抽象就是測試注入點之一。
- 建立 client 時即載入憑證並驗證格式（service account JSON 必須是 `type: "service_account"` 且含 `client_email` 與 `private_key`）；**不**在建立時換 token（ADC 的 metadata server 情境下應延後到第一次請求）。
- 從 service account JSON 取出 `client_email` 保存在 client 內，供 403 錯誤訊息使用。
- scope 固定為 `https://www.googleapis.com/auth/spreadsheets`。

### HTTP

- base URL 預設 `https://sheets.googleapis.com`，可由建構參數覆寫（測試注入點之二）。
- **讀取**：`GET /v4/spreadsheets/{sheetId}/values/{range}?majorDimension=ROWS&valueRenderOption=FORMATTED_VALUE`，回傳 `values`（缺少時為空陣列）。
- **寫入**：先 `POST /v4/spreadsheets/{sheetId}/values/{range}:clear`，再 `PUT /v4/spreadsheets/{sheetId}/values/{range}?valueInputOption=RAW`，body 為 `{ "range": ..., "majorDimension": "ROWS", "values": [...] }`。兩步非原子；第二步失敗回傳專屬錯誤種類。
- **range**：tab 名稱依 A1 notation 規則一律以單引號包住、內部單引號寫成兩個單引號；清除與讀取用整個 tab（`'name'`），寫入用 `'name'!A1`。組好的 range 再做 URL path 編碼。
- 所有請求帶 `Authorization: Bearer <token>`；token 由 provider 取得並快取，401 時清除快取重取一次後才報錯。
- 不做自動重試（429/5xx 只分類為暫時性）；重試策略留給 CLI 層。
- 逾時：連線 10 秒、整體請求 60 秒。
- User-Agent 含套件名與版本。

### 錯誤

- `SheetsError` 列舉：`Credentials(訊息)`、`Auth(訊息)`（換 token 失敗）、`PermissionDenied { sheet_id, service_account_email: Option }`、`SheetNotFound { sheet_id }`、`TabNotFound { sheet_id, tab }`、`RateLimited`、`ServerError { status }`、`Http { status, message }`（其他 4xx）、`Network(訊息)`、`InvalidResponse(訊息)`、`WriteAfterClearFailed { source: Box<SheetsError> }`。
- 分類規則：403 → `PermissionDenied`；404 → `SheetNotFound`；400 且 Google 訊息含 "Unable to parse range" → `TabNotFound`；429 → `RateLimited`；5xx → `ServerError`；其他 4xx → `Http`。
- `Display` 訊息以使用者可採取行動為目標，例如 `PermissionDenied` 的訊息包含「share the sheet with <email> (Editor)」。

### napi 層

- `#[napi] class SheetsClient`，以 `#[napi(factory)] async fn create(options)` 建立（因為憑證載入可能讀檔，且 ADC 可能需要 async）；方法 `readTab` / `writeTab` 為 `async fn`，回傳 Promise。
- `options`：`{ credentials?: { file?: string; json?: string; accessToken?: string }, baseUrl?: string }`；`credentials` 省略或為空物件 = ADC；同時給多個欄位為錯誤。
- 錯誤以 JS `Error` 拋出，`message` 為 Rust `Display`，並附 `code` 屬性（如 `PERMISSION_DENIED`、`SHEET_NOT_FOUND`、`TAB_NOT_FOUND`、`RATE_LIMITED`、`CREDENTIALS`、`NETWORK`…）。napi 的 `Error` 可帶自訂屬性或以 `Status` + reason 映射；實作時擇一但 `code` 必須是字串列舉且在 `.d.ts` 中宣告為 union type。
- `Table` 在 JS 端就是 `string[][]`，與 0002 的 `sheetToModel` / `modelToSheet` 直接銜接。

## Testing Decisions

- 好的測試描述「給這個 HTTP 回應，client 回傳這個 Table 或這個錯誤種類」以及「呼叫這個方法，發出這些請求（路徑、query、body、header）」。不測 `gcp_auth` 內部、不測 TLS。
- **Rust 整合測試**（`gslm-sheets`，`wiremock` 起本機 HTTP server，靜態 token provider）：
  - 讀取：正常表格、空 `values`、短列保留、tab 名稱含空白／單引號／中文時的 range 編碼、`FORMATTED_VALUE` query。
  - 寫入：先 clear 後 update 的順序與 body、`RAW`、空表格、clear 成功 update 失敗 → `WriteAfterClearFailed`。
  - 錯誤分類：403 / 404 / 400 parse-range / 429 / 503 / 其他 400 / 連線拒絕，各對應一個變體；403 訊息含 service account email。
  - 401 一次：第一次 401、重取 token 後成功。
  - 憑證：不存在的檔案、非 service_account JSON、同時不可用的 ADC（以環境變數清空模擬）各自在建立時報錯。
- **Live 測試**（`#[ignore]`）：`GSLM_TEST_SHEET_ID`、`GSLM_TEST_TAB`、`GOOGLE_APPLICATION_CREDENTIALS` 皆設定時，寫入一張小表再讀回比對。CI 不跑。
- **JS 整合測試**（`node:test`）：以 `node:http` 起 fixture server，`accessToken` 憑證 + `baseUrl`，驗證 `readTab` 回傳 `string[][]`、`writeTab` 發出的請求、錯誤的 `code` 屬性、以及 `sheetToModel(await client.readTab(...))` 的串接。這組測試在 CI 的每個平台（含容器）執行，即是 story 36 的驗證。
- 先例：0002 的 `node:test` 與 Rust `#[cfg(test)]` 風格；wiremock 為新引入的 dev-dependency。

## Out of Scope

- 自動重試與退避。
- 批次／部分更新（只更新差異列）——永遠整表覆寫。
- 建立 Sheet 或 tab、列出 tab、調整欄寬等 Sheets 管理操作。
- OAuth 使用者授權流程（瀏覽器登入）。
- config 檔、CLI 指令、檔案 I/O、`gslm migrate`。
- Node 端以 `google-auth-library` 取 token 再傳入（使用者可自行以 `accessToken` 形式做到）。
- 發佈與 `verify-install`（延後到首個 beta tag）。

## Further Notes

- **`RAW` 取代 `USER_ENTERED`** 是行為變更：舊版下以 `=` 開頭的翻譯會變成公式（通常是錯誤），含逗號的數字會被轉型；新版一律存字面值。遷移說明需提及：若有人刻意利用 USER_ENTERED 在 Sheet 內放公式，新版不再支援。
- **清除後寫入**也是行為變更：舊版會殘留多餘列。這是修正而非相容性問題，但應在 changelog 標明。
- **靜態 access token** 是新增的憑證形式，主要為了可測試性，但對沒有 service account 的使用者也是實用的逃生口；token 過期後 client 不會自動更新，錯誤會以 `Auth` 種類呈現。
- **musl 根憑證**：`reqwest` 與 `gcp_auth` 皆在執行期讀系統 CA。alpine 容器需 `ca-certificates`；若 CI 容器測試因此失敗，改用 `webpki-roots` feature（兩個 crate 都支援）並記錄在 ADR-0004 的後果中。
- **napi async**：`#[napi] async fn` 需要 `tokio_rt` feature 與 napi 管理的 runtime；`gcp_auth` 與 `reqwest` 的 client 必須在該 runtime 內建立（或至少在其內使用）。若 `SheetsClient::create` 在 Rust 端同步載入憑證但 ADC 需要 async，以 factory async fn 解決。
- **sheet ID 格式驗證**（舊版 `validateSheetId` 的 20+ 字元規則）不在此層：交給 Google 回 404。

## Comments

### 2026-08-21 實作備註

- 新 crate `gslm-sheets`：`SheetsClient::builder(creds).base_url(..).token_provider(..).build()`；`read_tab` / `write_tab`；`SheetsError` 含 `code()` 與 `is_transient()`。Rust 測試：10 unit + 13 integration（wiremock）+ 1 `#[ignore]` live。
- **napi 的 `async fn` 不支援自訂 error status 型別**（`Error<S>` 只能用在同步函式），因此 `error.code` 無法從 Rust 直接設定。解法：Rust 端訊息帶 `[CODE] ` 前綴；napi 產生的檔改名 `binding.js` / `binding.d.ts`，手寫 `index.js`（約 40 行）包裝 `SheetsClient` 把前綴轉成 `code` 並去除，`index.d.ts` 手寫 re-export 並加上 `SheetsErrorCode` union 與 `SheetsError` 介面。其他函式原樣 re-export。
- `gcp_auth` 會在建構時即解析 RSA 私鑰並要求 `token_uri` 欄位，因此測試夾具是一把本機產生的拋棄式 RSA key（`tests/fixtures/service-account.json`，附 README 說明）。
- `gcp_auth` 沒有暴露「清除快取」的 API，`TokenProvider::invalidate` 對它是 no-op；401 重試一次仍會走 gcp_auth 自身的過期判斷。靜態 token 與測試用 provider 則可真正輪替。
- `FORMATTED_VALUE` 回傳的非字串儲存格（數字、布林）以 JSON 字面值字串化；`null` 為空字串。
- JS 測試 5 個（`node:http` fixture server + `accessToken`），在 CI 每個平台執行，驗證 napi 內的 tokio runtime。CI 的 `js-loader` artifact 改上傳 `binding.js` / `binding.d.ts`。
- musl 根憑證問題在本票未觸發（測試走 HTTP，不經 TLS），首個 beta 的 live 驗證才會碰到；若失敗依 spec 改 `webpki-roots`。
