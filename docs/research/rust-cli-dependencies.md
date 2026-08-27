# `gslm` Rust 重寫:依賴替代方案研究

> 調查日期:2026-08-21。所有版本號、釋出日期、issue 數皆取自 crates.io API / GitHub API 當日快照。
> 本文只處理「Node 依賴 → Rust crate」的對應;`.js/.ts` config 檔載入方式另行研究(見 §7 限制)。

## 0. 現況摘要(Node 版實際用到的功能)

| 檔案 | 用到什麼 |
|---|---|
| `src/core/GoogleSheetLanguagesModel.ts` | `spreadsheets.values.get({spreadsheetId, range: sheetTitle})`、`spreadsheets.values.update({range: "<title>!A1", valueInputOption: 'USER_ENTERED', requestBody: {values}})`;表格第一列為 `['key', ...languages]` |
| `src/utils/auth.ts` | `google.auth.GoogleAuth({credentials | <none> , scopes:['https://www.googleapis.com/auth/spreadsheets']})`;優先序:credentials 物件 > 檔案路徑 > `GOOGLE_APPLICATION_CREDENTIALS` |
| `src/commands/pull.ts` / `push.ts` | yargs:`--config/-C`、`--sheet-id/-s`、`--sheet-title/-t`、`--credentials/-c`、`--directory/-d`、`--languages/-l`(array)、`--type`(choices nest/flat)、`.example([...])`、`--help/--version` |
| `src/core/LanguagesModel.ts` | `lodash-es` 只用 `set(obj, 'a.b.c', v)` 做 flat → nest;JSON 以 2 空白縮排寫檔 |
| `src/utils/config.ts` | `import(pathToFileURL(...))` 動態載入 `.js/.mjs/.cjs/.ts` |

## 1. TL;DR:建議技術棧

| Node 依賴 | Rust crate(版本) | 為什麼 |
|---|---|---|
| `googleapis`(Sheets v4 呼叫) | **手寫 REST**:`reqwest` 0.13.4(`default-features = false, features = ["json", "rustls-no-provider"]`)| 只用到 2 個 endpoint(`values.get` / `values.update`),生成式 SDK 帶來的型別與依賴遠超需求;REST 路徑穩定且有官方文件 |
| `googleapis`(Service Account 驗證 / ADC) | **`gcp_auth` 0.12.7**(預設 `ring` 後端)| 最小、維護中(2026-06 釋出、2026-08 仍有 commit)、原生支援 `GOOGLE_APPLICATION_CREDENTIALS` / 檔案 / JSON 字串、走標準 JWT-bearer token 交換(與 Node `google-auth-library` 行為一致),會快取 token |
| `yargs` | **`clap` 4.6.6**(`features = ["derive"]`)+ `clap_complete` 4.6.9 | derive API 一次涵蓋 subcommand、short/long alias、`value_enum`、`Vec` + `value_delimiter`、`after_help`、`--version`;`clap_complete` 額外送 shell 補全 |
| `lodash-es` (`set`) | **手寫 ~30 行**(`serde_json::Value` 遞迴)| `flatten-json-object` 0.6.1 最後釋出 2022-04、只做 flatten 沒有 unflatten;需求太小不值得多一個依賴 |
| `fs` + `JSON.stringify(…, null, 2)` | **`serde_json` 1.0.151**(`features = ["preserve_order"]`)| `to_string_pretty` 預設就是 2 空白;`preserve_order` 讓 `Map` 改用 `IndexMap`,保住 key 順序以利 diff |
| 錯誤處理 | `anyhow` 1.0.104 | CLI 不需要自訂錯誤型別階層;`miette` 功能更華麗但上次釋出 2025-04 |
| 輸出 / 色彩 | `owo-colors` 4.3.0(+ 可選 `indicatif` 0.18.6) | 取代現有 `logger.ts` 的 info/success/error 著色;純 Rust、無 `atty` 類過時依賴 |
| async runtime | `tokio` 1.53.1(`features = ["rt", "macros"]`,單執行緒即可)| `reqwest` / `gcp_auth` 皆需 tokio |
| 發布到 npm | 主套件 + per-platform `optionalDependencies`(Biome / esbuild 模式)或 `cargo-dist` 0.32.0 的 npm installer | 使用者仍可 `npx gslm`;另外附 GitHub Releases 與 `cargo-binstall` 支援 |
| 交叉編譯 | `cargo-zigbuild` 0.23.0(Linux gnu/musl × x64/arm64)、原生 runner 處理 macOS / Windows | `cross` 最後正式版 0.2.5 是 2023-02,雖 main 分支仍活躍但需要 Docker |

**TLS 後端統一用 rustls**(`reqwest` 0.13 預設已是 rustls),並選擇 **`ring`** 當 crypto provider 而非預設 `aws-lc-rs`,理由見 §2.6(交叉編譯時不需 C toolchain 差異)。

---

## 2. Google Sheets API + Service Account 驗證

### 2.1 候選比較總表

| 方案 | 最新版 / 釋出日 | 維護狀態 | Runtime | TLS | SA JSON 物件 | SA 檔案路徑 | `GOOGLE_APPLICATION_CREDENTIALS` | 有 Sheets client? |
|---|---|---|---|---|---|---|---|---|
| `google-sheets4` (google-apis-rs) | 7.0.0+20251215 / 2026-01-01 | 半自動生成;repo 2026-07 仍有 push、36 open issues | tokio + hyper 1 | `hyper-rustls` 0.27(`rustls-native-certs`),`ring` 或 `aws-lc-rs` feature | 透過 `yup-oauth2` | 透過 `yup-oauth2` | 透過 `yup-oauth2::ApplicationDefaultCredentialsAuthenticator` | **有**(生成) |
| `yup-oauth2` | 12.1.2 / 2026-01-07 | repo 2026-02 最後 push、33 open issues | tokio + hyper 1 | `hyper-rustls` 或 `hyper-tls` feature | `parse_service_account_key` | `read_service_account_key` | `ApplicationDefaultCredentialsAuthenticator::from_environment()` 只讀該 env var | 無(純 auth) |
| `google-cloud-auth` (官方 google-cloud-rust) | 1.15.0 / 2026-07-30 | 官方、極活躍(2026-08-20 仍在 push),236 open issues;MSRV 1.88(crate)/ README 寫 1.90 | tokio + reqwest 0.13 | reqwest rustls;預設 `aws-lc-rs` | `credentials::service_account::Builder::new(serde_json::Value)` | 需自行讀檔轉 `Value` | `credentials::Builder::default().build()` 走完整 ADC(env var → well-known file → metadata server)| **無**(只生成 GCP 服務,`src/generated/` 沒有 sheets) |
| `gcp_auth` | 0.12.7 / 2026-06-22 | repo 2026-08-17 最後 push、13 open issues;MSRV 1.85 | tokio + hyper 1 | `hyper-rustls` 0.27(`rustls-native-certs`),`ring`(預設)或 `aws-lc-rs` | `CustomServiceAccount::from_json(&str)` | `CustomServiceAccount::from_file(path)` | `CustomServiceAccount::from_env()` 或 `gcp_auth::provider()` | 無(純 auth) |
| 手寫(`reqwest` + `jsonwebtoken`) | reqwest 0.13.4 / 2026-05-25;jsonwebtoken 11.0.0 / 2026-07-24 | 兩者皆主流、活躍 | tokio | reqwest rustls | 自己 parse | 自己讀檔 | 自己讀 env | 自己打 REST |

來源:
- crates.io API:<https://crates.io/api/v1/crates/google-sheets4>、<https://crates.io/api/v1/crates/yup-oauth2>、<https://crates.io/api/v1/crates/google-cloud-auth>、<https://crates.io/api/v1/crates/gcp_auth>、<https://crates.io/api/v1/crates/reqwest>、<https://crates.io/api/v1/crates/jsonwebtoken>
- GitHub API:<https://api.github.com/repos/Byron/google-apis-rs>、<https://api.github.com/repos/dermesser/yup-oauth2>、<https://api.github.com/repos/googleapis/google-cloud-rust>、<https://api.github.com/repos/djc/gcp_auth>
- google-cloud-rust 生成目錄(無 sheets):<https://github.com/googleapis/google-cloud-rust/tree/main/src/generated>
- google-cloud-rust README(MSRV 1.90、語意化版本政策):<https://github.com/googleapis/google-cloud-rust/blob/main/README.md>

### 2.2 Sheets REST endpoint(手寫方案的依據)

官方 REST 參考:
- `GET https://sheets.googleapis.com/v4/spreadsheets/{spreadsheetId}/values/{range}` — <https://developers.google.com/workspace/sheets/api/reference/rest/v4/spreadsheets.values/get>
- `PUT https://sheets.googleapis.com/v4/spreadsheets/{spreadsheetId}/values/{range}?valueInputOption=USER_ENTERED`,body 為 `ValueRange { values: [[...]] }` — <https://developers.google.com/workspace/sheets/api/reference/rest/v4/spreadsheets.values/update>

兩者與 Node 版 `googleSheet.spreadsheets.values.get / update` 是一對一對應;`range` 為 A1 notation,需 URL-encode(工作表名稱含空白或中文時尤其重要)。

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct ValueRange { #[serde(default)] values: Vec<Vec<String>> }

let url = format!("https://sheets.googleapis.com/v4/spreadsheets/{id}/values/{}",
                  urlencoding::encode(&format!("{title}!A1")));
client.put(url)
    .query(&[("valueInputOption", "USER_ENTERED")])
    .bearer_auth(token.as_str())
    .json(&ValueRange { values })
    .send().await?.error_for_status()?;
```

### 2.3 Service Account 驗證流程(官方規格)

Google 官方文件 <https://developers.google.com/identity/protocols/oauth2/service-account>:
- JWT claims:`iss`(SA email)、`scope`(空白分隔)、`aud` = `https://oauth2.googleapis.com/token`、`iat`、`exp`(最長 1 小時);簽章演算法**只支援 RS256**。
- 以 `grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer&assertion=<jwt>` POST 到 `https://oauth2.googleapis.com/token` 換 access token。
- 另有「Addendum: Service account authorization without OAuth」:**部分** Google API 可直接拿自簽 JWT 當 bearer token,文件只保證「在 googleapis GitHub repo 有 service definition 的 API」,範例為 Firestore。Sheets 不在該 repo,**不能假設** Sheets 接受自簽 JWT。AIP-4111 亦說明此機制:<https://google.aip.dev/auth/4111>。

各 crate 實作方式:
- `yup-oauth2`:標準 token 交換,`aud` 取自 key 檔的 `token_uri`,`GRANT_TYPE = "urn:ietf:params:oauth:grant-type:jwt-bearer"` — <https://github.com/dermesser/yup-oauth2/blob/master/src/service_account.rs>
- `gcp_auth`:同樣使用 `GRANT_TYPE = "urn:ietf:params:oauth:grant-type:jwt-bearer"` 做交換 — <https://github.com/djc/gcp_auth/blob/main/src/custom_service_account.rs>
- `google-cloud-auth`:`ServiceAccountTokenProvider::token()` 直接 `ServiceAccountTokenGenerator.generate()` 產生自簽 JWT 當 `Bearer`,**原始碼中沒有對 `oauth2.googleapis.com/token` 的交換**(grep 無結果)— <https://github.com/googleapis/google-cloud-rust/blob/main/src/auth/src/credentials/service_account.rs>。這對 GCP API 沒問題,但對 Sheets(Workspace API)是未驗證的風險。

### 2.4 逐一評析

**`google-sheets4`(google-apis-rs)**
- 優點:完整型別(`ValueRange`、`SpreadsheetValueUpdateCall`…),`hub.spreadsheets().values_get(id, range).doit()`,`values_update(req, id, range).value_input_option("USER_ENTERED")`;README 範例見 <https://github.com/Byron/google-apis-rs/blob/main/gen/sheets4/README.md>。
- 缺點:必須手動組裝 `hyper_util::client::legacy::Client` + `hyper_rustls::HttpsConnectorBuilder`(README 範例約 20 行樣板);依賴鏈綁死 `yup-oauth2 ^12`、`hyper 1`、`serde_with`、`chrono`、`mime`、`utoipa`(可選)。crate 本身是 mako 模板生成(README 開頭標註 DO NOT EDIT),bug 修正需等整個 repo 重新生成。
- 適合:之後若會大量擴展 Sheets 功能(batchUpdate、格式、建立工作表)。

**`yup-oauth2`**
- 只負責 OAuth;`ServiceAccountAuthenticator::builder(key).build()` 後 `auth.token(&scopes)`。ADC 支援僅 `GOOGLE_APPLICATION_CREDENTIALS` + GCE metadata 兩種(`ApplicationDefaultCredentialsTypes::{InstanceMetadata, ServiceAccount}`),沒有 gcloud well-known 檔 — <https://github.com/dermesser/yup-oauth2/blob/master/src/authenticator.rs>。
- 預設 feature 拉進 `hyper-util` 的 server 功能(`server-auto`, `server-graceful`)與 `time` 的 `local-offset`,對 CLI 是多餘重量(crates.io dependencies 列表)。
- 沒有獨立使用的理由;只有搭配 `google-sheets4` 時才會用到。

**`google-cloud-auth`(官方)**
- 是 google-cloud-rust 的 auth 元件,API 乾淨:`credentials::Builder::default().with_scopes([...]).build()`(ADC)或 `credentials::service_account::Builder::new(json_value).with_access_specifier(AccessSpecifier::from_scopes([...])).build()`;`build_access_token_credentials()` 可拿裸 token 給「SDK 尚未支援的 API」用 — 原始碼 <https://github.com/googleapis/google-cloud-rust/blob/main/src/auth/src/credentials.rs>。
- 限制:(a) **沒有 Sheets client**;(b) SA 憑證走自簽 JWT,Sheets 支援未驗證(§2.3);(c) 預設 feature `default-rustls-provider` = `reqwest/default-tls` + `rustls/aws_lc_rs`,且 `default-idtoken-backend` 也拉 `aws-lc-rs`(需 C compiler);(d) MSRV 1.88+、README 宣告 1.90,且「依賴變更不視為 breaking」。
- 結論:官方長期最有前途,但現階段對本專案不是最省事的選擇。可列為未來若 Google 把 Workspace API 納入 google-cloud-rust 時的遷移目標。

**`gcp_auth`**
- 文件明列四種來源順序:`GOOGLE_APPLICATION_CREDENTIALS` → `~/.config/gcloud/application_default_credentials.json` → metadata server → `gcloud` CLI — <https://github.com/djc/gcp_auth/blob/main/src/lib.rs>。
- 對本專案的三種輸入都有直接 API:`CustomServiceAccount::from_json(&str)`(對應 Node 的 credentials 物件)、`from_file(path)`、`from_env()`;以及 `with_subject()`。`provider.token(&["https://www.googleapis.com/auth/spreadsheets"]).await?` 會快取 token。
- crate 小(26 KB)、依賴精簡(hyper 1 / hyper-rustls / ring / tokio),MSRV 1.85。維護者 djc 是 rustls 核心維護者之一;13 open issues。
- 缺點:0.x 版號;star 數低(76)但下載 1,200 萬。

**手寫(`reqwest` + `jsonwebtoken`)**
- `jsonwebtoken` 11 要求自選 crypto 後端 `aws_lc_rs` 或 `rust_crypto`(README:<https://github.com/Keats/jsonwebtoken/blob/master/README.md>);`rust_crypto` 是純 Rust(`rsa` crate),交叉編譯零負擔。
- 約 60 行即可完成:讀 JSON → `EncodingKey::from_rsa_pem(private_key)` → `encode(Header::new(RS256), claims, key)` → POST token endpoint → 快取 `expires_in`。
- 缺點:要自己處理 ADC 搜尋、token 過期、錯誤碼;等於重寫 `gcp_auth` 的一小部分。

### 2.5 建議

1. **HTTP 呼叫:手寫 REST(`reqwest`)**。只有 2 個 endpoint,生成式 SDK 的樣板與依賴不划算;手寫還能精準對應 Node 版行為(例如 `values || []` 的空表處理)。
2. **驗證:`gcp_auth`**。它完整覆蓋 Node 版 `createAuth()` 的三種優先序,走與 `google-auth-library` 相同的 JWT-bearer 交換流程,避免 `google-cloud-auth` 自簽 JWT 對 Sheets 的不確定性。
3. 若未來要擴充大量 Sheets 操作,再評估切換到 `google-sheets4`(此時 `gcp_auth` 的 token 仍可透過 `google_apis_common::GetToken` trait 接入,不必改用 yup-oauth2)。

### 2.6 TLS / crypto provider 與二進位大小

- `reqwest` 0.13 起**預設 rustls**,且 crypto provider 預設 **aws-lc-rs**;`rustls-no-provider` feature 可自行安裝 provider — CHANGELOG:<https://github.com/seanmonstar/reqwest/blob/master/CHANGELOG.md>(v0.13.0 條目)。`native-tls` 在 Linux 需要 OpenSSL 或 `native-tls-vendored` 編譯一份 OpenSSL — README:<https://github.com/seanmonstar/reqwest/blob/master/README.md>。
- `aws-lc-rs` 非 FIPS 建置「不需 CMake / bindgen / Go,但需要 C/C++ compiler」;Windows x86-64 需 NASM 或其 prebuilt NASM 物件 — <https://github.com/aws/aws-lc-rs/blob/main/aws-lc-rs/README.md>。
- `ring` 同樣含 C/asm,但 `cargo-zigbuild`/`cross` 對 ring 的支援歷史最久、問題最少;`gcp_auth` 預設即 `ring`。
- **建議組合**(全部用同一個 rustls provider,避免 rustls 的「multiple providers」執行期 panic):
  ```toml
  reqwest  = { version = "0.13", default-features = false, features = ["json", "rustls-no-provider", "http2"] }
  gcp_auth = "0.12"          # 預設 ring
  rustls   = { version = "0.23", default-features = false, features = ["ring", "std", "tls12"] }
  ```
  並在 `main()` 開頭 `rustls::crypto::ring::default_provider().install_default().ok();`。
- 根憑證:`reqwest` 的 `rustls-platform-verifier` 走 OS 信任庫;`gcp_auth` 走 `rustls-native-certs`。兩者在 musl 靜態二進位中都是執行期讀系統 CA,alpine 容器需安裝 `ca-certificates`;若要零依賴可改 `webpki-roots` feature(`gcp_auth` 有提供)。
- 二進位大小:tokio + hyper + rustls + ring 的 CLI 在 `opt-level="z"`, `lto="fat"`, `codegen-units=1`, `panic="abort"`, `strip=true` 下一般落在數 MB 級(未在本專案實測,實作後應量測);`google-sheets4` 會再多 serde_with/chrono 等但差距不大;`google-cloud-auth` + aws-lc-rs 通常最大。Cargo profile 設定參考:<https://doc.rust-lang.org/cargo/reference/profiles.html>。

---

## 3. CLI 解析:`clap`

`clap` 4.6.6(2026-08-06),`clap_complete` 4.6.9;MSRV 1.85 — <https://crates.io/api/v1/crates/clap>。

需求對照(皆為 builder 方法,derive 屬性同名;來源 `clap_builder/src/builder/arg.rs` 與 `command.rs`:<https://github.com/clap-rs/clap/tree/master/clap_builder/src/builder>):

| yargs 用法 | clap |
|---|---|
| `command: 'pull' / 'push'` | `#[command(subcommand)] cmd: Commands` + `enum Commands { Pull(PullArgs), Push(PushArgs) }` |
| `alias: 's'` | `#[arg(short = 's', long = "sheet-id")]`;額外別名 `alias` / `visible_alias` / `short_alias` / `visible_short_alias` |
| `choices: ['nest','flat']` | `#[arg(value_enum)] r#type: ContentType` + `#[derive(ValueEnum)]`;會自動列在 help 的 `[possible values: nest, flat]` |
| `type: 'array'`(`-l en zh ja`) | `#[arg(short, long, num_args = 1.., value_delimiter = ',')] languages: Vec<String>`;同時支援 `-l en zh ja`、`-l en,zh,ja`、重複 `-l` |
| `.example([...])` | `#[command(after_help = "Examples:\n  gslm pull --config ...")]`(或 `after_long_help` 只在 `--help` 顯示) |
| `--version` | `#[command(version)]` 讀取 `CARGO_PKG_VERSION` |
| `.strict()` | clap 預設即拒絕未知參數(並附「did you mean」建議,`suggestions` feature 預設開啟) |
| env var fallback | `features = ["env"]` + `#[arg(env = "GOOGLE_APPLICATION_CREDENTIALS")]`(可選) |

derive 總覽:<https://docs.rs/clap/latest/clap/_derive/index.html>。

```rust
#[derive(Parser)]
#[command(name = "gslm", version, about, after_help = EXAMPLES)]
struct Cli { #[command(subcommand)] cmd: Commands }

#[derive(Subcommand)]
enum Commands { Pull(SyncArgs), Push(SyncArgs) }

#[derive(Args)]
struct SyncArgs {
    #[arg(short = 'C', long)] config: Option<PathBuf>,
    #[arg(short = 's', long)] sheet_id: Option<String>,
    #[arg(short = 't', long)] sheet_title: Option<String>,
    #[arg(short = 'c', long)] credentials: Option<PathBuf>,
    #[arg(short = 'd', long)] directory: Option<PathBuf>,
    #[arg(short = 'l', long, num_args = 1.., value_delimiter = ',')] languages: Vec<String>,
    #[arg(long, value_enum)] r#type: Option<ContentType>,   // pull only
}
```

注意:clap 中 `Option<T>` 的 `None` 就等於 yargs 的「未明確給值」,可直接實作 `config.ts` 的 `extractExplicitArgs` 語意(CLI 覆蓋 config)。

**Bonus:`clap_complete`** — 可加 `gslm completions <shell>` 子命令輸出 bash/zsh/fish/powershell/elvish 補全腳本;README:<https://github.com/clap-rs/clap/blob/master/clap_complete/README.md>。Node 版沒有這個功能。

---

## 4. JSON 與 nest ⇄ flat 轉換:`serde_json`

- `serde_json` 1.0.151(2026-07-20),MSRV 1.71 — <https://crates.io/api/v1/crates/serde_json>。
- `to_string_pretty` 使用 `PrettyFormatter::new()`,其 indent 為 `b"  "`(2 空白)— <https://github.com/serde-rs/json/blob/master/src/ser.rs>(`PrettyFormatter::with_indent(b"  ")`)。與 `JSON.stringify(obj, null, 2)` 輸出一致,但 **serde_json 不會在檔尾加換行**,寫檔時需自行 `push('\n')` 以免 diff 噪音。
- `preserve_order` feature:「By default the map is backed by `BTreeMap`. Enable the `preserve_order` feature of serde_json to use `IndexMap` instead.」— <https://github.com/serde-rs/json/blob/master/src/map.rs>;Cargo.toml 定義 `preserve_order = ["indexmap", "std"]` — <https://github.com/serde-rs/json/blob/master/Cargo.toml>。**必開**:否則 pull 下來的 `en.json` key 會被字母排序,和 Sheet 列順序 / 原檔順序不同,造成 i18n 檔大量 diff。
- 解析 `credentials.json` 與 `ValueRange` 直接用 `#[derive(Deserialize)]`。

**flatten / unflatten crate?**
- `flatten-json-object` 0.6.1:最後釋出 2022-04-16、9 stars、只提供 `Flattener::flatten`(無 unflatten),且會處理陣列、空物件等本專案不需要的情況 — <https://crates.io/api/v1/crates/flatten-json-object>、<https://github.com/vtselfa/flatten-json-object/blob/master/src/lib.rs>。
- `json_flatten`:crates.io 查無此 crate。
- **建議手寫**。需求僅「字串葉節點 + `.` 分隔」雙向轉換,對應 Node 版 `LanguagesModel.ts` 的 `set()`:

```rust
use serde_json::{Map, Value};

fn flatten(v: &Value, prefix: &str, out: &mut Map<String, Value>) {
    match v {
        Value::Object(m) => for (k, v) in m {
            let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
            flatten(v, &key, out);
        },
        other => { out.insert(prefix.to_owned(), other.clone()); }
    }
}

fn unflatten(flat: &Map<String, Value>) -> Value {
    let mut root = Map::new();
    for (path, v) in flat {
        let mut cur = &mut root;
        let mut parts = path.split('.').peekable();
        while let Some(p) = parts.next() {
            if parts.peek().is_none() { cur.insert(p.to_owned(), v.clone()); break; }
            cur = cur.entry(p).or_insert_with(|| Value::Object(Map::new()))
                     .as_object_mut().expect("key collision: leaf vs branch");
        }
    }
    Value::Object(root)
}
```
(`expect` 處應改為回傳錯誤:同一個前綴既是字串又是物件時,lodash `set` 會直接覆寫,Rust 版建議明確報錯。)

push 時「自動偵測 nest/flat」可沿用 Node 邏輯:任一 value 為 `Value::Object` 即為 nest。

---

## 5. 其他 CLI 工程:錯誤、輸出、檔案

| 用途 | 建議 | 版本 / 日期 | 備註 |
|---|---|---|---|
| 錯誤處理 | `anyhow` | 1.0.104 / 2026-07-18 | `main() -> anyhow::Result<()>`;`.with_context(|| format!("Credentials file not found: {path}"))` 對應 Node 版各種錯誤訊息。退出碼 1 由 `?` 傳到 main 自動處理 |
| (替代)| `miette` | 7.6.0 / 2025-04-27 | 漂亮的診斷輸出、支援 source span,但本 CLI 沒有需要標記位置的輸入;上次釋出已逾一年 |
| 終端著色 | `owo-colors` | 4.3.0 / 2026-04-14 | 零依賴,`"ok".green()`;搭配 `supports-color` 或 `std::io::IsTerminal` 判斷是否為 TTY,對應 `logger.ts` 的 info/success/warn/error |
| 進度 / spinner | `indicatif` | 0.18.6 / 2026-07-01 | 「Fetching data from Google Sheet...」可用 `ProgressBar::new_spinner()`;可選 |
| `console` | 0.16.4 / 2026-07-01 | `indicatif` 的底層,若不用 indicatif 則不需要 |
| 結構化日誌 | `tracing` 0.1.44 | 2025-12-18 | 對兩個子命令的 CLI 屬過度設計;若要 `--verbose` 可用 `env_logger`/`tracing-subscriber` 擇一 |
| 檔案 I/O | `std::fs` | — | 只需 `create_dir_all`、`read_to_string`、`write`;檔名固定 `<lang>.json`,不需要 `walkdir` |
| 路徑 | `std::path` + `std::env::current_dir` | — | 對應 `path.resolve` |

來源:crates.io API(各 crate 同上查詢方式)。

---

## 6. 發布到 npm 使用者

### 6.1 Biome / esbuild 模式(主套件 + per-platform optionalDependencies)

Biome `packages/@biomejs/biome/package.json`(v2.5.9)— <https://github.com/biomejs/biome/blob/main/packages/%40biomejs/biome/package.json>:
```json
"bin": { "biome": "bin/biome" },
"optionalDependencies": {
  "@biomejs/cli-win32-x64": "2.5.9", "@biomejs/cli-win32-arm64": "2.5.9",
  "@biomejs/cli-darwin-x64": "2.5.9", "@biomejs/cli-darwin-arm64": "2.5.9",
  "@biomejs/cli-linux-x64": "2.5.9", "@biomejs/cli-linux-arm64": "2.5.9",
  "@biomejs/cli-linux-x64-musl": "2.5.9", "@biomejs/cli-linux-arm64-musl": "2.5.9"
}
```
平台套件 `@biomejs/cli-linux-x64/package.json` 以 `"os": ["linux"], "cpu": ["x64"], "libc": ["glibc"]` 讓 npm/pnpm 只安裝符合的那一個 — <https://github.com/biomejs/biome/blob/main/packages/%40biomejs/cli-linux-x64/package.json>。

JS shim `bin/biome`(<https://github.com/biomejs/biome/blob/main/packages/%40biomejs/biome/bin/biome>)做三件事:
1. `ldd --version` 輸出含 `musl` 即選 `linux-musl` 套件;
2. `require.resolve("@biomejs/cli-<platform>-<arch>/biome")` 找到二進位;
3. `spawnSync(binPath, process.argv.slice(2), { stdio: "inherit" })` 並把 `result.status` 設為 `process.exitCode`;找不到時印出訊息並 exit 1。
另支援 `BIOME_BINARY` env 覆寫路徑(方便本地開發)。

esbuild `npm/esbuild/package.json` 同樣用 `optionalDependencies`(26 個平台),但多了 `"postinstall": "node install.js"` — <https://github.com/evanw/esbuild/blob/main/npm/esbuild/package.json>;`lib/npm/node-install.ts` 在 optionalDependencies 被 `--no-optional` 略過時,會退而在暫存目錄 `npm install` 對應平台套件、或直接從 npm registry 下載 tarball,並用 `validateBinaryVersion` 驗證版本 — <https://github.com/evanw/esbuild/blob/main/lib/npm/node-install.ts>。**對本專案建議採 Biome 的無 postinstall 模式**(更簡單、企業環境也不會因 postinstall 被擋)。

**本專案落地方式**:
- 主套件維持現名 `@gn00678465/google-sheet-languages-model`,`bin.gslm` 指向 JS shim;新增 `@gn00678465/gslm-{linux-x64,linux-x64-musl,linux-arm64,linux-arm64-musl,darwin-x64,darwin-arm64,win32-x64}` 七個平台套件,版本號必須與主套件完全相同(Biome 用精確版本而非 `^`)。
- 現有 `publishConfig.registry` 為 GitHub Packages,平台套件也要發到同一 registry;GitHub Packages 的 scoped 套件名稱需與 repo owner 一致(已符合)。
- 由於 Node 版還提供 library API(`src/index.ts`),若要保留可考慮主套件同時包含 JS 版 library + Rust CLI;或把 library 拆成獨立套件。此決策超出本文範圍。

### 6.2 `cargo-dist`

0.32.0(2026-05-22)— <https://crates.io/api/v1/crates/cargo-dist>。內建 npm installer:`installers = ["npm"]`、`npm-scope = "@scope"`、`publish-jobs = ["npm"]`,會生成一個 npm 套件,安裝時**從 GitHub Release 下載** archive 而非 per-platform optionalDependencies — <https://github.com/axodotdev/cargo-dist/blob/main/book/src/installers/npm.md>。0.31.0 起附 `npm-shrinkwrap.json` 鎖定下載用的 JS 依賴。支援 targets 列表(darwin x64/arm64、windows x64、linux gnu/musl × x64/arm64)— <https://github.com/axodotdev/cargo-dist/blob/main/book/src/reference/config.md>。
- 優點:一個 `dist init` 同時得到 GitHub Release、shell/powershell installer、npm、Homebrew。
- 缺點:npm 套件在 `postinstall` 下載二進位(離線 / 防火牆環境會失敗);332 open issues;無法自訂成 GitHub Packages registry 的 optionalDependencies 模式。
- 建議:用 cargo-dist 產 GitHub Release + checksums;npm 部分自己照 Biome 模式在 CI 發布(兩者不衝突)。

### 6.3 `cargo-binstall` / GitHub Releases

`cargo-binstall` 1.21.1(2026-07-25):「fetching the crate information from crates.io and searching the linked repository for matching releases and artifacts」;預設 URL 樣板 `{repo}/releases/download/{version}/{name}-{version}-{target}.{archive-format}`,可用 `[package.metadata.binstall]` 覆寫 — <https://github.com/cargo-bins/cargo-binstall/blob/main/README.md>。只要 GitHub Release 資產命名符合(cargo-dist 預設命名即相容),Rust 使用者可 `cargo binstall gslm`。前提是 crate 發到 crates.io(名稱 `gslm` 需先確認可用)。

---

## 7. 交叉編譯

目標矩陣:`x86_64-unknown-linux-gnu`、`x86_64-unknown-linux-musl`、`aarch64-unknown-linux-gnu`、`aarch64-unknown-linux-musl`、`x86_64-apple-darwin`、`aarch64-apple-darwin`、`x86_64-pc-windows-msvc`。

| 工具 | 版本 / 狀態 | 適用 |
|---|---|---|
| `cargo-zigbuild` | 0.23.0 / 2026-06-16;repo 2026-08 活躍、17 open issues | 以 `zig cc` 當 linker;`rustup target add` 後 `cargo zigbuild --target aarch64-unknown-linux-gnu`;`--target x86_64-unknown-linux-gnu.2.17` 可指定最低 glibc(預設隨 zig 版本,0.12–0.14 為 2.28)— <https://github.com/rust-cross/cargo-zigbuild/blob/main/README.md>。注意 README 警告:`+crt-static` 靜態 glibc 不支援(musl 不受影響) |
| `cross` | crates.io 最後正式版 0.2.5 / **2023-02-04**;但 main 分支 2026-08-19 仍有 commit,README 建議 `cargo install cross --git` | 需 Docker/Podman;每個 target 一個容器映像,對 ring 等 C 依賴最穩,但 CI 時間較長 — <https://github.com/cross-rs/cross/blob/main/README.md> |
| 原生 runner | GitHub 2025-08 起公開 repo 免費提供 `ubuntu-24.04-arm` / `ubuntu-22.04-arm` / `windows-11-arm` — <https://github.blog/changelog/2025-08-07-arm64-hosted-runners-for-public-repositories-are-now-generally-available/> | linux-arm64 可直接原生編譯,不必交叉 |

cargo-dist 0.26+ 會自動對 `aarch64-unknown-linux-gnu` 等 target 套用 `cargo-zigbuild`(Windows 則 `cargo-xwin`)— <https://github.com/axodotdev/cargo-dist/blob/main/book/src/ci/customizing.md>。

建議 GitHub Actions matrix(公開 repo):

| target | runner | 工具 |
|---|---|---|
| x86_64-unknown-linux-gnu | ubuntu-22.04 | `cargo zigbuild --target x86_64-unknown-linux-gnu.2.17`(相容舊 glibc) |
| x86_64-unknown-linux-musl | ubuntu-22.04 | `cargo zigbuild`(或原生 + `musl-tools`) |
| aarch64-unknown-linux-gnu | ubuntu-22.04-arm(原生)或 ubuntu-22.04 + zigbuild | — |
| aarch64-unknown-linux-musl | 同上 | — |
| x86_64-apple-darwin / aarch64-apple-darwin | macos-14(arm64 原生;x64 用 `--target x86_64-apple-darwin`,Apple toolchain 原生支援) | `cargo build` |
| x86_64-pc-windows-msvc | windows-latest | `cargo build` |

C 依賴注意事項(影響 ring / aws-lc-rs):
- `ring`:zigbuild 與 cross 皆長期支援;這是本文建議 `ring` provider 的主因。
- `aws-lc-rs`:需 C/C++ compiler(zig cc 可勝任),Windows x64 需 NASM 或啟用 prebuilt NASM — <https://github.com/aws/aws-lc-rs/blob/main/aws-lc-rs/README.md>。若選 `google-cloud-auth` 或保留 reqwest 預設 feature 就會遇到。
- 完全避免 C 的路線:`jsonwebtoken` + `rust_crypto` feature(純 Rust RSA)+ `rustls` 的 ring… 仍有 ring;目前 rustls 官方 provider 只有 ring 與 aws-lc-rs 兩種,純 Rust provider(如 `rustls-rustcrypto`)尚未成熟,不建議。

---

## 8. Node → Rust 無法一對一對應的部分(限制)

- **`.js/.ts/.mjs/.cjs` config 檔**:Rust 二進位無法執行 JS。選項:(a) 改用 `gslm.config.{json,toml,yaml}`(`serde` + `toml`/`serde_yaml`);(b) 若偵測到 `.js/.ts` 設定檔,由 JS shim(npm 發布時本來就有)先用 Node `import()` 載入、序列化成 JSON 後透過 stdin 或暫存檔交給 Rust 二進位 —— 但這樣 `cargo binstall` / Homebrew 使用者就沒有此功能;(c) 內嵌 JS 引擎(`boa`/`deno_core`)—— 體積與複雜度不成比例。**留待 config 設計的研究決定**,本文僅標記限制。
- **`credentials` 為物件**:Node config 可直接放 JSON 物件;Rust 版若 config 改為 JSON/TOML,可同樣內嵌物件(`gcp_auth::CustomServiceAccount::from_json`)。
- **library API(`src/index.ts`)**:Rust 重寫只涵蓋 CLI;JS library 若要保留需另行決定(見 §6.1)。

---

## 9. Open risks

1. **Sheets 是否接受自簽 JWT** — 只影響 `google-cloud-auth` 路線;本文建議 `gcp_auth` 即可迴避。若日後想改用官方 crate,必須先用真實 Sheet 做 spike 驗證(<https://developers.google.com/identity/protocols/oauth2/service-account> 僅保證 googleapis repo 內的 API)。
2. **`gcp_auth` 為 0.x、單一主要維護者** — 雖然 0.12.x 系列 API 穩定、維護者是 rustls 核心成員,仍需在 `Cargo.lock` 鎖版並關注 breaking 變更。備援方案是 §2.4 的手寫 JWT(~60 行)。
3. **rustls crypto provider 衝突** — `reqwest`(預設 aws-lc-rs)與 `gcp_auth`(預設 ring)若 feature 沒對齊,rustls 0.23 在兩個 provider 同時啟用且未 `install_default()` 時會 panic。Cargo.toml 必須如 §2.6 明確關閉 reqwest 預設 feature。
4. **根憑證在 musl/Alpine** — `rustls-platform-verifier` / `rustls-native-certs` 依賴系統 CA 檔;在 scratch / alpine 容器中需 `ca-certificates`,否則考慮 `webpki-roots`(會把 Mozilla CA 內嵌進二進位,需隨版本更新)。
5. **`value_delimiter` 與語言代碼** — 若日後語言代碼含 `,`(不太可能)會被切開;目前以 `-l en zh ja`(`num_args = 1..`)為主即可。另外 clap 的 `num_args = 1..` 選項後若緊接位置參數會被吞掉,本 CLI 無位置參數,不受影響。
6. **key 順序語意** — 開啟 `preserve_order` 後,pull 的輸出順序 = Sheet 列順序;push 的欄位順序 = 第一個語言檔的 key 順序(與 Node 版 `for...in` 行為相同,Node 物件對「整數型 key」會先排序,Rust `IndexMap` 不會;若有 key 如 `"1"`, `"2"` 會出現順序差異,屬改善而非退化,但應在 CHANGELOG 註明)。
7. **serde_json 檔尾無換行** — 需手動補 `\n`,否則與 Node 版產出(`JSON.stringify` 同樣無換行)一致但與多數 editor/prettier 習慣不符;建議決定一種並寫進測試。
8. **GitHub Packages 對 npm `optionalDependencies` 的支援** — Biome/esbuild 都發在 npmjs.com;GitHub Packages 需要 `.npmrc` 的 scoped registry 與 token,平台套件同 scope 應可行,但未在本研究中實測。
9. **`cross` 的 crates.io 版本老舊** — 若選 `cross`,應依 README 用 `--git` 安裝,而非 `cargo install cross`(0.2.5 無法處理新版 rustup/targets 的若干問題)。
10. **MSRV 漂移** — `google-cloud-auth` README 明言會定期提升 MSRV 且不視為 breaking;`reqwest`/`clap`/`gcp_auth` 目前皆為 1.85。建議在 `Cargo.toml` 設 `rust-version` 並在 CI 用 `cargo +1.85 check` 守住。
