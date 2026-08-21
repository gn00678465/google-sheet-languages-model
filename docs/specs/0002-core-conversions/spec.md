---
status: done
date: 2026-08-21
adrs: [0001, 0002]
depends_on: [0001]
---

# Spec 0002：核心轉換——Catalog、Model 與 Tab 之間的雙向轉換

## Problem Statement

舊版 TypeScript 的轉換邏輯（`nest ⇄ flat`、`sheet 二維陣列 ⇄ model`）散在兩個 class 裡，而且有多個**從未被寫成規格的隱性行為**：空字串與缺譯一律丟掉、列順序只看第一個 locale、其他 locale 多出的 key 被靜默丟棄、欄位對應靠位置而非標題、陣列攤成 `days.0` 後又被自己的反向轉換拒絕。這些行為沒有測試保護，搬進 Rust 時若不先定案，Rust 與舊版會各自選一種，使用者升級後 Sheet 或翻譯檔會悄悄變樣。

Spec 0001 只搬了 `flatten`。`gslm-core` 目前還沒有 Catalog、Locale、Model 這些型別，也沒有任何 Tab 方向的轉換。

## Solution

在 `gslm-core` 建立領域型別（Locale、Catalog、Model、Format、Key separator，詞彙依 `CONTEXT.md`），實作四個純函式轉換：Catalog 的 nest/flat 雙向、Model 與 Tab 表格（`Vec<Vec<String>>`）的雙向。每個舊版隱性行為都在本 spec 明文定案、寫成測試。napi 層以薄包裝暴露同樣的函式，讓 Node SDK 使用者在 HTTP 與檔案 I/O 到位前就能用這些轉換。

所有轉換只處理記憶體資料：不碰檔案、不打網路。

## User Stories

### Catalog（單一 locale）

1. As a 使用者, I want 把 nest format 的 catalog 轉成 flat, so that 可以推到 Sheet 的 key 欄。
2. As a 使用者, I want 把 flat format 的 catalog 轉成 nest, so that pull 下來的資料能寫成程式慣用的巢狀 JSON。
3. As a 使用者, I want 兩個方向都保留 key 的原始順序, so that 翻譯檔與 Sheet 的 diff 穩定可讀。
4. As a 使用者, I want 轉換時 translation 的空字串被原樣保留, so that 「故意留空」與「尚未翻譯」在本地檔中是可區分的（見 CONTEXT.md：Translation）。
5. As a 使用者, I want 自訂 key separator, so that 既有專案用 `/` 或 `:` 分層時不必改 key。
6. As a 使用者, I want 讀取 catalog 時不需事先宣告它是 nest 還是 flat, so that 混用（部分巢狀、部分帶點號）的檔案也能正確攤平。
7. As a 使用者, I want catalog 的葉節點若不是字串（數字、布林、null、陣列）就得到明確錯誤並指出 key, so that 不會有非字串值被偷偷轉成文字寫進 Sheet。
8. As a 使用者, I want unflatten 遇到 `a` 與 `a.b` 同時存在時得到明確錯誤, so that 不會靜默覆蓋其中一個。
9. As a 使用者, I want key 任一段是純數字時得到錯誤, so that 不會在 nest 輸出裡產生陣列索引般的歧義結構。
10. As a 使用者, I want 錯誤訊息中包含完整 key 路徑, so that 在數千條翻譯中能直接定位。

### Model（多 locale）

11. As a 使用者, I want 以有序的 locale 清單建立 Model, so that 第一個 locale 成為 Source locale 並決定 Sheet 的列順序。
12. As a 使用者, I want Model 內每個 locale 各持有一份 Catalog, so that 缺譯（某 locale 沒有該 key）能被表示而不是補空字串。
13. As a 使用者, I want 查詢 Model 中「只存在於非 Source locale」的 key, so that 能在 push 前知道哪些翻譯會被排在 Source 之後。
14. As a 使用者, I want 以一致的規則把 Model 轉成任一 format 的每 locale 輸出, so that pull 端寫檔時不必各自處理 nest/flat。

### Model → Tab（push 方向）

15. As a 使用者, I want 轉出的表格第一列是 `key` 加上各 locale（依 Model 順序）, so that Sheet 的欄位與設定一致且譯者看得懂。
16. As a 使用者, I want 列順序依 Source locale 的 key 順序, so that Sheet 的排列與原始翻譯檔一致。
17. As a 使用者, I want 只存在於非 Source locale 的 key 被**附加在 Source 的 key 之後**（依 locale 順序再依 key 順序），而不是被丟棄, so that 既有翻譯不會因 Source 缺 key 而消失。
18. As a 使用者, I want 缺譯的儲存格輸出為空字串, so that 譯者在 Sheet 上一眼看出待翻譯的格子。
19. As a 使用者, I want catalog 中的空字串 translation 同樣輸出為空儲存格, so that Sheet 上不會出現無法分辨的 `""`（接受此方向的資訊損失，見 Further Notes）。
20. As a 使用者, I want 所有儲存格都是字串, so that 寫入 Sheets API 時不會觸發型別自動轉換。

### Tab → Model（pull 方向）

21. As a 使用者, I want 第一列被視為標題列，第一欄為 key 欄（不論標題文字），其餘欄依**標題文字**對應到 locale, so that 譯者在 Sheet 上調換欄位順序不會把翻譯配錯語言。
22. As a 使用者, I want 請求的 locale 若在標題列找不到就得到錯誤並列出 Sheet 實際有的欄位, so that 設定錯字能立刻被發現。
23. As a 使用者, I want Sheet 上多出的欄（未在 locale 清單中）被忽略, so that 譯者可以加備註欄。
24. As a 使用者, I want 空儲存格視為缺譯（該 locale 的 catalog 不含此 key）, so that 不會把未翻譯的內容寫成空字串進翻譯檔。
25. As a 使用者, I want key 欄為空的列被跳過, so that 譯者用空白列分組不會造成錯誤。
26. As a 使用者, I want 重複的 key 得到錯誤並指出列號, so that 不會讓後者靜默覆蓋前者。
27. As a 使用者, I want 列比標題短（Sheets API 會省略尾端空格）時視為缺譯, so that 常見的 API 回傳形態不會被當成格式錯誤。
28. As a 使用者, I want 空表格（沒有任何列）得到明確錯誤, so that 打錯 tab 名稱時不會得到一個空的 Model 然後清空本地翻譯檔。
29. As a 使用者, I want 儲存格內容前後的空白被原樣保留, so that 刻意的前置空白（例如排版用）不會被改動。

### Node SDK（napi 邊界）

30. As a Node SDK 使用者, I want `flatten` / `unflatten` 接受並回傳純 JS 物件, so that 不必理解 Rust 型別。
31. As a Node SDK 使用者, I want `sheetToModel(rows, locales, options?)` 與 `modelToSheet(model, options?)` 兩個函式, so that 能自行串接任何 Sheets client 或測試資料。
32. As a Node SDK 使用者, I want Model 在 JS 端是 `{ locales: string[], catalogs: Record<locale, Record<key, string>> }` 這種可 JSON 序列化的純資料, so that 能直接存檔、比對、或在測試中手寫。
33. As a Node SDK 使用者, I want 所有錯誤都是 JS `Error` 且訊息與 Rust 端一致, so that 兩邊的文件和錯誤處理可以共用。
34. As a Node SDK 使用者, I want TypeScript 型別由 napi 自動產生並包含 options 的形狀, so that IDE 能提示 `separator`、`format` 等參數。
35. As a Node SDK 使用者, I want 物件 key 順序跨越 JS ⇄ Rust 邊界後仍然保留, so that 0001 驗證過的順序保證延伸到所有新函式。

### 維護者

36. As a 維護者, I want 每個舊版隱性行為的定案都有對應測試, so that 後續重構不會無意間改回去。
37. As a 維護者, I want 錯誤型別是一個列舉（enum）而非字串, so that CLI 與 SDK 能依種類決定訊息與 exit code。
38. As a 維護者, I want `gslm-core` 仍然不依賴 napi 與任何 I/O, so that ADR-0002 延後的獨立 CLI 之路保持暢通。

## Implementation Decisions

### 型別（`gslm-core`）

- **Locale**：字串 newtype，不做格式驗證（舊版 `validateLanguageCodes` 從未被使用；格式限制留給 config 層）。
- **Catalog**：內部一律以 flat 儲存——有序映射 `key → translation`，translation 為字串。建構來源可以是 nest、flat 或混合的 JSON 物件（攤平規則同 0001 的 `flatten`），但**葉節點必須是字串**，否則回傳帶 key 路徑的錯誤。`flatten` 本身維持泛型（0001 行為不變），字串檢查在 Catalog 建構時做。
- **Model**：有序的 locale 清單 + 每 locale 一個 Catalog。`locales[0]` 是 Source locale。提供查詢「非 Source 獨有 key」的方法。
- **Format**：`Nest | Flat` 列舉，只影響輸出；輸入不需指定。
- **Key separator**：所有轉換函式接受 separator 參數，預設 `.`；空字串為錯誤。
- **Tab 表格**：`Vec<Vec<String>>`，第一列為標題。這是與 Sheets API `values.get` / `values.update` 交換的形態，HTTP 層（後續 spec）只負責搬運，不做轉換。
- **錯誤**：單一 `ConversionError` 列舉，0001 的 `FlattenError` 併入此列舉（`flatten` 的行為與訊息不變，只有型別名稱改變；此時尚無外部使用者），涵蓋：根節點非物件、葉節點非字串（含 key）、數字 key 段（含 key）、陣列（含 key）、空 separator、unflatten 衝突（含 key）、表格為空、標題缺少 locale（含請求的 locale 與實際欄位清單）、重複 key（含 key 與列號）。實作 `Display` 與 `std::error::Error`。

### 轉換規則（定案）

| 情境 | 舊版行為 | 定案 |
|---|---|---|
| catalog 中的空字串 | `flatToNest` 丟掉 | **保留**在 Catalog 與 nest/flat 輸出 |
| push：缺譯 | 空儲存格 | 空儲存格 |
| push：空字串 translation | 空儲存格 | 空儲存格（有損，見 Further Notes） |
| push：列順序 | Source locale 的 key 順序 | 同左 |
| push：非 Source 獨有 key | 靜默丟棄 | **附加**於 Source 的 key 之後，依 locale 順序、再依 key 順序；Source 儲存格為空 |
| pull：欄位對應 | 依位置（假設與 locale 清單同序） | **依標題文字**；第一欄恆為 key 欄 |
| pull：請求的 locale 不在標題 | 整欄變 undefined，靜默 | 錯誤 |
| pull：多餘欄 | 忽略 | 忽略 |
| pull：空儲存格／短列 | 跳過（缺譯） | 缺譯 |
| pull：key 欄為空的列 | 寫入 key `undefined` | **跳過** |
| pull：重複 key | 後者覆蓋 | 錯誤（含列號） |
| pull：空表格 | 空 Model | 錯誤 |
| 葉節點非字串 | 數字等直接寫入 | 錯誤 |
| 陣列 | 攤成 `days.0` | 錯誤（0001 已定） |
| 數字 key 段 | unflatten 時 throw | 錯誤（兩方向） |
| unflatten：`a` 與 `a.b` 並存 | lodash `set` 靜默改寫 | 錯誤 |
| 儲存格前後空白 | 原樣 | 原樣 |
| 混合輸入攤平後 key 重複（`{"a":{"b":..},"a.b":..}`） | 後者覆蓋 | 錯誤（含 key） |
| flat 輸入中 `a` 與 `a.b` 並存 | — | Catalog 可建構、flat 輸出正常；**nest 輸出**時報 KeyConflict |
| pull：標題比對 | 位置 | **精確比對**（不 trim、區分大小寫）；重複標題取第一個；第一欄不參與比對 |
| pull：只有標題列 | 空 Model | 合法的空 Model（CLI 層須在覆寫非空本地檔前警告，見 Further Notes） |
| pull：key 含數字段 | 接受 | Tab → Model **接受**（key 在此層為不透明字串）；nest 輸出時才報錯，錯誤含 key 可在 Sheet 搜尋 |
| push：key 含數字段（`errors.404`） | 接受 | Catalog 建構時**報錯**——相對舊版的行為變更，見 Further Notes |

### napi 層

- 暴露 `flatten`（0001 既有）、`unflatten(flat, separator?)`、`sheetToModel(rows, locales, { separator? })`、`modelToSheet(model, { separator? })`，以及 `catalogToNest` / `catalogToFlat` 若從 Model 取單一 locale 輸出需要。
- Model 在 JS 端是純資料物件 `{ locales: string[], catalogs: Record<string, Record<string, string>> }`；napi 以 `#[napi(object)]` 結構體對應，不暴露 class。
- 錯誤以 `InvalidArg` 狀態的 JS `Error` 拋出，訊息取自 Rust 的 `Display`。
- 繼續使用 `serde-json-ordered` 保持順序。

## Testing Decisions

- 好的測試描述「給這個輸入，得到這個輸出或這個錯誤」，不檢查內部結構。每一條「定案」表格的列至少對應一個測試。
- **Rust 單元測試**（`gslm-core`）：Catalog 建構（nest、flat、混合、各種錯誤）；unflatten（順序、衝突、separator）；Model → Tab（標題、列順序、附加 key、空儲存格）；Tab → Model（標題對應、欄位調換、多餘欄、短列、空 key 列、重複 key、空表格、空白保留）；以及 **往返測試**：`Catalog → nest → Catalog` 必須恆等（含空字串）；`Model → Tab → Model` 在不含空字串的資料上，Source locale 的 Catalog 必須恆等，非 Source locale 的 Catalog 以集合比較（其順序經往返後會變成 Source 的列順序，這是 Tab 單一列順序的必然結果）。延續 0001 的 `#[cfg(test)] mod tests` 風格。
- **JS 整合測試**（`packages/gslm/__tests__`，`node:test`）：每個新函式一組基本案例 + 一個錯誤案例 + key 順序檢查；重點是跨邊界的型別與錯誤轉換正確，不重複 Rust 端的邊界矩陣。延續 0001 的 `flatten.test.cjs`。
- 先例：舊版 `src/__test__/LanguagesModel.test.ts`、`GoogleSheetLanguagesModel.test.ts` 與其 `i18n/` 夾具可作為案例來源（但行為依本 spec 的定案，不依舊版輸出）。

## Out of Scope

- 檔案讀寫（`{locale}.json` 的載入與儲存、路徑樣板）——留給 config / CLI spec。
- Google Sheets HTTP 與驗證（ADR-0004）。
- config 解析、`gslm migrate`（ADR-0003）。
- clap CLI 與 `pull` / `push` 指令。
- 多 Target。
- 舊版 TypeScript 實作的修改或刪除。
- 發佈與 `verify-install`（依 0001 的決定延後到首個 beta tag）。

## Further Notes

- **空字串的單向損失**：Sheet 的空儲存格無法與 `""` 區分，因此 catalog 中的 `""` 經 push → pull 後會變成缺譯。這是 Sheet 作為媒介的固有限制；本 spec 選擇在本地端保留 `""`（不丟資料），在 Sheet 端接受損失。若日後需要無損，可考慮以特殊標記表示，但不在本 spec 內。
- **非 Source 獨有 key 的附加**是相對舊版的行為變更，目的是不丟資料；CLI spec 應在 push 時把這些 key 列為警告，讓使用者決定是否補到 Source。
- **依標題對應欄位**也是行為變更：舊版假設欄位順序與 locale 清單一致。若使用者的 Sheet 標題列與 locale 代碼不一致（例如標題寫 `English`），pull 會直接報錯，比舊版靜默配錯更安全；錯誤訊息會列出實際欄位以便修正。
- `flatten` 對葉節點的泛型行為（0001）不變，避免破壞已公開的 API；字串約束只在 Catalog 層。
- **數字 key 段在 push 端報錯**是相對舊版的行為變更：舊版只在 `flatToNest`（pull）檢查，push `errors.404` 可以成功。新版在 Catalog 建構時即拒絕，避免推上去的 key 拉不回來。遷移說明需提醒使用者改用非純數字的段（如 `errors.e404`）。
- **只有標題列的 Tab** 是合法的空 Model（新建 tab 的正常狀態）。但 pull 寫檔時若本地檔非空而 Model 為空，極可能是 tab 名稱錯誤；這個防護屬於 CLI / 檔案 I/O 層，後續 spec 必須加入。

## Comments

### 2026-08-21 實作備註

- `gslm-core` 新增 `unflatten`、`Catalog`（`from_value` / `to_value(Format)`）、`Model`（`new` / `set_catalog` / `orphan_keys` / `ordered_keys` / `to_table` / `from_table`）與統一的 `ConversionError`；0001 的 `FlattenError` 併入 `ConversionError`，`flatten` 行為與錯誤訊息不變。
- `Locale` 以 `String` 型別別名實作而非 newtype：本層不驗證格式，newtype 只會增加 napi 邊界的轉換成本。
- napi 層暴露 `unflatten`、`sheetToModel(rows, locales)`、`modelToSheet(model)`、`orphanKeys(model)`；**未加** spec 提到的 `options.separator`——Model ⇄ Tab 轉換中 key 是不透明字串，separator 沒有作用，保留只會誤導。`catalogToNest` / `catalogToFlat` 也未加：`unflatten` 已涵蓋。
- TS 介面以 `Model` 命名（`#[napi(object, js_name = "Model")]`），形狀為 `{ locales: string[], catalogs: Record<string, Record<string, string>> }`，透過 napi 的 `object_indexmap` feature 保留 locale 與 key 順序。
- 測試：Rust 46 個（含往返測試），JS 18 個（`node:test`）。定案表每一列都有對應 Rust 測試。
