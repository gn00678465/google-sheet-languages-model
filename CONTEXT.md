# gslm

在 Google Sheet 與本地翻譯檔之間雙向同步 i18n 內容的工具。Sheet 是譯者的工作介面，本地檔是程式讀取的格式，gslm 負責兩者間的轉換與搬運。

## Language

### 內容

**Locale**:
一個語言／地區代碼，例如 `en`、`zh-TW`。同時是 Sheet 的欄標題與翻譯檔的檔名來源。
_Avoid_: language, lang

**Source locale**:
Target 的 locales 清單中的第一個 locale。它的 key 集合與順序決定 Sheet 的列順序；其他 locale 多出的 key 不會出現在 Sheet 中。
_Avoid_: main language, default language, primary locale

**Key**:
翻譯條目的唯一識別字串，以分隔符號（預設 `.`）表示階層，例如 `user.name`。Key 的任一段不得是純數字。
_Avoid_: path, id, message id

**Translation**:
某個 key 在某個 locale 下的文字值。缺少 translation 與 translation 為空字串是不同的狀態。
_Avoid_: value, string, message

**Catalog**:
單一 locale 的全部 translation，對應本地的一個 JSON 檔。
_Avoid_: dictionary, resource, bundle

**Format**:
Catalog 在檔案中的結構：`nest`（巢狀物件）或 `flat`（以分隔符號連接的單層 key）。
_Avoid_: type, structure, shape

**Key separator**:
在 flat format 與 Sheet 的 key 欄中用來連接階層的字元，預設 `.`。
_Avoid_: delimiter, namespace separator

### 同步

**Sheet**:
一份 Google 試算表，以 spreadsheet ID 識別。
_Avoid_: spreadsheet, document

**Tab**:
Sheet 內的單一工作表，以名稱識別。一個 tab 的第一列是 `key` 加上各 locale，其後每列一個 key。
_Avoid_: sheet title, worksheet, page

**Target**:
一組同步設定：一個 tab、一組 locales、本地 catalog 的路徑樣板與 format。一份設定檔可以有多個 target。
_Avoid_: project, group, entry, job

**Pull**:
從 tab 讀取內容並寫成本地 catalog 的動作。
_Avoid_: download, fetch, sync down

**Push**:
從本地 catalog 讀取內容並寫回 tab 的動作。
_Avoid_: upload, publish, sync up

**Credentials**:
用來存取 Sheet 的 Google Service Account 金鑰。只能以檔案路徑、環境變數或 Application Default Credentials 提供，不會出現在設定檔內容中。
_Avoid_: key file, token, auth object
