---
title: "架構"
status: stable
updated: 2026-10-01
type: Architecture
description: "WDA 專案儲存庫（Repository）的組成架構、目錄配置與責任邊界，以及本專案的文件規範。"
tags: []
---

# 架構

本文件說明 `web-design-anchor` 這個專案儲存庫（Repository，以下簡稱 Repo）的架構。本文件涵蓋其組成元件、各檔案的配置位置，以及各部分之間的責任劃分。細節規則記錄在各自的文件中。指令行為詳見 `docs/commands.md`。`wda init` 產出的專案詳見 `docs/generated-project.md`。Design System 詳見 `docs/design-system.md`。Skill 詳見 `docs/skill-spec.md`。若本文件與上述文件有所衝突，邊界劃分以本文件為準。細節規則以各專屬文件為準。

## 核心原則

### 標準優先

WDA 優先採用 Web 平台標準。沒有直接對應的標準時，採用簡單、明確、廣泛使用的當代做法。唯有確實存在缺口時，才新增 WDA 自訂機制，且必須維持最小幅度。

### 簡單、明確、單一責任

Core 的每個指令、每個檔案、每條規則皆僅專注於單一責任，且須直觀易懂。這並不等於「指令越少越好」；若新指令代表真正相異的責任，即可新增。

### 人和 AI 都讀得懂

重要的專案資訊必須讓 Designer、AI、Engineer 都看得懂。精確的結構化狀態可由 `wda.json`、`tokens.json` 或 lockfile 保存，但關鍵狀態仍須於文件中說明。

### WDA 是工具，不是治理

WDA 不替專案擁有者決定治理流程。WDA 可以配合 ADR，但不要求所有決策都寫成 ADR。

### 不依賴單一 AI 產品

WDA 必須對 AI 友善，但不得依賴任何特定的 AI 產品、host 或其生態系。

## 系統組成

WDA 由三個實體組成，三者並列，互不隸屬。

| 實體元件 | 所在位置 / 路徑 | 性質與說明 |
| :--- | :--- | :--- |
| 規範 | `docs/` | 唯一的規範來源。若與 `docs/` 衝突，以 `docs/` 為準（`README.md` 僅為導覽） |
| Core | `src/`、`tests/`、`schemas/` | 可執行的工具本體。單一 Cargo package，包含 `wda_core` lib target 與 `wda` bin target |
| Skill | `skills/wda/` | 隨 release 一同釋出的產出成果，並非文件 |

Skill 不置於 `docs/` 目錄下，因為它與 binary 同屬交付成果（deliverable）。規範不置於 `src/` 目錄下，因為契約存續期長於任何版本的實作程式碼。

## 穩定度分層

WDA 的每條規則皆歸屬於五個層級之一。層級決定三件事：該規則能否變更、由誰變更，以及變更後由誰承擔代價。層級越上位，穩定度越高。

```text
Invariant              固定不變
Default                專案可自訂，寫在 wda.json
Tool Form              WDA 重大改版才會變更
Tooling Choice         WDA 改版時可替換，專案不受影響
Implementation Detail  隨時可替換，外部不可見
```

### Invariant

不可變更的承諾。變更此層級等同修改產品核心定義，任何版本皆不安排變更。

- 部署產物是標準的 HTML、CSS、JavaScript 與 assets
- WDA 不是網站的 runtime dependency
- `dist/` 是唯一的部署產物
- 凡會導致建置結果不正確的 Error 一律予以阻擋。Error 的具體定義詳見 `docs/commands.md` 的「診斷輸出」一節

### Default

專案可自訂調整的預設值。專案若需修改，直接在 `wda.json` 明確指定新值。Designer 未提供明確選擇時，一律採用預設值，不主動追問。具體的預設值清單詳見 `docs/generated-project.md` 的 `wda.json` 欄位表。

### Tool Form

Core 的執行形態。變更此層級屬於 WDA 的架構重大改版，例如改為常駐服務或編輯器外掛模組（editor plugin）。

- standalone CLI executable
- 使用者環境只需在 PATH 中包含單一 native binary

### Tooling Choice

實現 Tool Form 所採用的技術。WDA 改版時可替換其中任何項目；替換之後，`wda.json` 的契約規格、四個指令的責任邊界，以及 build 的產出結果三者皆不得變更，專案亦無須配合修改。

- 以 Rust 撰寫 CLI binary
- 由 esbuild 負責 TypeScript 至 JavaScript 的轉譯
- 搭配 Deno 與 `deno.lock` 處理相依性解析（dependency resolution）與宣告管理。現行專案以 `deno.jsonc` 宣告 `imports`，並保留 legacy `package.json` 相容。

### Implementation Detail

Core 內部的程式組織。可隨時替換，外部契約對此層完全透明（外部無法探知）。

- Rust 模組的切割與劃分方式
- atomic init 的 rollback 實作機制

## 範圍

WDA 僅負責建置（build）。它產出可獨立部署的靜態檔案，不包含代管（hosting）或部署（deployment）服務。實際上線由 Designer 或 AI host 處理，可部署至任何靜態託管空間，例如 GitHub Pages、Cloudflare Pages、Netlify 或 S3。

不提供 hosting 的主要考量為維護成本。若內建 hosting 則必須綁定雲端服務，且需處理帳號、金鑰、計費及各家服務商的 API 差異。這些功能皆與產出標準 Web 無涉，反而會成為長期的維護負擔。

以下兩項永久排除：

- Clean URL、URL rewrite 與 fallback routing：此類機制屬於部署環境範疇，不納入 WDA 的契約承諾
- preview server：Core 永不內建 preview server 功能

## 專案目錄結構

```text
web-design-anchor/
├── Cargo.toml              單一 package：wda_core lib 加 wda bin
├── plugin.json             Agent Plugins 1.0.0 的最小宣告，只有 $schema 與 name
├── NOTICE                  第三方素材的歸屬，含 WDA Minimal token 快照的出處
├── SECURITY.md             安全模型，僅記錄已落實實作的部分
├── README.md               導覽，不是規格
├── src/
│   ├── lib.rs              唯一對外的整合介面：validate_project(root) 與各指令入口
│   ├── main.rs             只做 stdout、stderr 分流與 exit code 對應，不含任何規則
│   ├── cli.rs              參數解析
│   ├── codes.rs            診斷碼的唯一宣告處
│   ├── diagnostics.rs      Diagnostic 的值模型、排序、單行輸出
│   ├── validation/         四大規則模組，pub(crate)，binary 不可直接呼叫
│   │   ├── project_contract.rs   wda.json 對 schemas/wda.schema.json
│   │   ├── documents.rs          必要文件與 front matter
│   │   ├── html.rs               HTML source-check
│   │   ├── tokens.rs             DTCG tokens
│   │   └── build_output.rs       dist/ 產物檢查
│   ├── init/               wda init：manifest、documents、resolve（解析銜接層）
│   ├── deps/               wda deps：manifest、resolve
│   ├── build/              wda build：layout、include、tokens、scripts、report
│   └── builtins/           內嵌資產（內建資產層）
│       ├── wda_minimal/    WDA Minimal 的 tokens、starter、design.md 來源
│       └── spectrum_two/   Spectrum 2 的 starter script
├── schemas/
│   └── wda.schema.json     wda.json 的 canonical schema，binary 內嵌同一份
├── tests/                  整合測試；fixtures/ 放實體化的專案與 Skill 案例
├── skills/
│   └── wda/                Skill 實體：SKILL.md 加 references/
├── docs/
│   ├── architecture.md          本文件：Repo 的組成、邊界與文件規則
│   ├── generated-project.md     wda init 產出的專案
│   ├── commands.md              四個指令與整個診斷契約
│   ├── design-system.md         Design System 的契約
│   ├── skill-spec.md            Skill 層的契約
│   ├── glossary.md              術語定義
│   ├── adr/                     決策紀錄
│   ├── design-systems/          WDA Minimal 與 Spectrum 的實例定義
│   └── templates/generated-project/   wda init 寫進專案的文件範本
└── packaging/              release 與安裝腳本
```

四大規則模組僅能經由 `src/lib.rs` 單一介面入口對外公開，`src/main.rs` 不包含任何驗證規則。此組織方式雖屬 Implementation Detail，但落實了一項關鍵約定：`wda check` 與 `wda build` 必須共用同一套驗證引擎。單一介面入口使這項約定在軟體架構上無法被繞過。

## 版本相容性

### wdaVersion 不是 format version

WDA 優先維持對既有專案格式的向下相容。中斷性（破壞性）的格式變更必須明確定義並標註版本，且不得於 `check` 或 `build` 執行期間暗中套用。新版 WDA 雖可成功建置相容的舊格式專案並更新 `wdaVersion`，但專案結構並不會自動遷移。因此新版實作必須持續保留承諾支援的舊格式驗證器（validator）與相依處理分支。

### 格式向下相容與禁止暗中遷移

相依宣告格式的演進遵循向下相容與不可侵入原則：
- 現行專案格式以 `deno.jsonc` 宣告 `imports`（搭配 `deno.lock`），專案端使用 `deno.json` 屬於不支援的格式，會由 Core 阻擋並回報診斷錯誤。
- 既有以 `package.json` 宣告相依性的專案為承諾支援的舊格式。Core 保留 legacy 專屬的相依管理與建置分支，確保舊專案能持續正常執行 `wda deps` 與 `wda build`。
- 禁止唯讀暗中遷移：`wda check` 與 `wda build` 等唯讀指令絕不在執行期自動將 `package.json` 專案改寫或遷移為 `deno.jsonc`。專案格式的升級必須是明確、可控且由專屬流程驅動的行為。

### 四級驗證語意

```text
現行格式          → OK（如 deno.jsonc 宣告 imports）
支援的舊格式      → Warning（如既有 package.json 專案）
已棄用的舊格式    → 更明確的 Warning
不支援的格式      → Error（如專案端使用 deno.json 或雙重設定）
```

## 相依模型

WDA 的相依性架構圍繞五項核心區分與嚴格的信任邊界建立，確保外部套件宣告明確、建置環境乾淨可重現，且不污染專案檔案。

### 現行專案相依宣告（deno.jsonc）

現行 Generated Project 統一採用 Deno 原生的 `deno.jsonc` 宣告外部相依性，並鎖定於專案根目錄的 `deno.lock`。
- 相依套件宣告於 `deno.jsonc` 的 `imports` 欄位中，使用穩定的 `npm:<package>@<version>` 規格字串。
- `deno.jsonc` 允許 JSONC 語法中的單行註解、區塊註解與尾端逗號，方便 Designer 與開發者記錄註記。
- 現行專案不採用 `package.json` 作為宣告檔，亦不接受專案端使用 `deno.json` 作為設定檔。若專案端存在 `deno.json` 或同時出現 `deno.json` 與 `deno.jsonc`，WDA 在執行相依性操作前會直接拒絕並回報診斷錯誤，要求更名或排除衝突，防止雙重設定產生歧異。

### 內部建置工作區（Build Workspace）與嚴格 deno.json

`wda build` 執行時會在專案目錄外的作業系統暫存目錄建立隔離的內部建置工作區（`BuildWorkspace`）。
- 內部工作區由 WDA 寫入專屬且嚴格的 JSON 格式 `deno.json`，承載 WDA 要求的編譯與模組設定（包括 `nodeModulesDir: "manual"` 與標準 `lib` 清單）。
- 對於現行格式專案，WDA 僅透過 `jsonc-parser` 解析專案端 `deno.jsonc` 的 `imports` 映射。WDA 將該映射合併至內部工作區的 `deno.json` 中。專案端其他無關欄位（例如 `tasks`、`compilerOptions`、`name` 等）一律不複製亦不帶入建置工作區。
- 只有當選定格式的相依宣告非空（現行 `imports` 非空，或 legacy `dependencies` 非空）時，才會在工作區內執行 `deno install --frozen`。若相依宣告為空，則跳過安裝步驟且不要求 `deno.lock` 存在。
- 專案根目錄的 `deno.lock` 在整個建置過程中維持唯讀，絕不進行任何位元組的寫入或改寫。

### 既有專案向下相容（Legacy package.json）

WDA 持續完整支援既有以 `package.json` 宣告相依性的專案：
- `wda deps` 在執行前先判斷專案格式；若專案根目錄存在 `package.json`，則走 legacy 分支，維持呼叫 Deno 時帶入 `--package-json` 的既有行為。
- `wda build` 遇到 legacy 專案時，將 `package.json` 原樣複製至內部工作區，確保既有專案仍能正確建置。
- 既有專案無須被迫更動檔案結構即可持續正常運作。

### Deno 擁有寫入權（Deno-Owned Writes）

`deno.jsonc` 的檔案寫入與結構變更全權由 Deno 本身管理：
- 當執行 `wda deps add`、`remove` 或 `update` 時，WDA 呼叫 Deno 原生指令對暫存檔進行修改。該指令明確指定 `--config deno.jsonc`。Deno 原生機制會完整保留註解、格式、尾端逗號與無關欄位。
- WDA 內部不實作自訂的 JSONC 寫入器或格式化引擎，僅使用 `jsonc-parser` 讀取並驗證 `imports` 資料模型。
- WDA 僅負責處理空狀態的檔案清理。當移除相依後，若根層級已無其他語意欄位，WDA 才會刪除 `deno.jsonc`。單純註解不視為保留檔案的依據。當鎖定檔僅剩版本標頭（`{"version": ...}`）時，WDA 會負責刪除空白的 `deno.lock`。若仍有其他自訂根欄位，檔案內容與註解則依 Deno 行為完整保留。

### 禁止唯讀暗中遷移（No Silent Read-only Migration）

WDA 堅持工具操作的明確性與可預測性：
- `wda check` 與 `wda build` 屬於唯讀檢驗與產出指令，絕不在執行期間暗中將舊格式 `package.json` 遷移或重寫為 `deno.jsonc`。
- 相依格式的升級或轉換屬於架構層級變更，必須由使用者在明確知情且具備獨立遷移流程的情境下啟動，不可作為其他指令的隱式副產物。

### 信任與執行環境邊界

相依解析與安裝維持明確且受限的信任邊界：
- 信任對象僅限於 Deno 子行程、npm 官方 registry，以及透過 `env_clear()` 後繼承自父行程的完整環境變數（用於傳遞必要的 registry 認證或網路代理設定）。
- 執行 Deno 相關相依指令時，一律不傳入 `--allow-scripts` 參數，嚴格禁用任何第三方 npm 套件的生命週期腳本（lifecycle scripts）。
- WDA 不引入常駐守護程式（daemon）、不建立新的行程家族、不新增專屬環境變數頻道，亦不在執行期產生遠端網路連線相依性。
- Deno 快取寫入全數落在本機預設路徑或繼承的 `DENO_DIR`，完全位於 WDA 專案目錄之外，不納入專案層級的 atomic rollback 範疇。

## Design System 邊界

預設 Design System 是 latest stable Spectrum。線上解析成功時寫入確切版本。解析失敗時退回 WDA 內建的 `WDA Minimal`，並由 `wda init` 說明退回原因。Token 的 canonical source 是 `tokens/tokens.json`。衍生的 CSS 只在 `dist/`。Design System 的身分判斷與名稱只允許出現在解析銜接層 `src/init/resolve.rs` 與內建資產層 `src/init/`、`src/builtins/`。其餘 Core 程式碼皆不得依系統身分產生邏輯分支。Adapter 是定義於文件中的介面規範（interface）。Adapter 並非 Rust trait，亦非外掛註冊表（plugin registry）。完整契約規範詳見 `docs/design-system.md`。

## Skill 邊界

Skill 為 AI 的操作手冊與工作流程引導層，並非規格來源。它不重複保存核心規則，亦不作為第二套產生器。Core 與 `wda.json` 的契約設計不得預設僅有 Skill 會進行操作。專案僅具備單一 Skill，其名稱為 `wda`，實體目錄位於 `skills/wda/`，隨版本發布（release）一同釋出，其版本號與所屬 release 相同。Skill 對 host 的唯一能力假設為：環境具備執行原生二進位檔案（native binary）的能力，並能取得 stdout、stderr 與 exit status。完整契約規範詳見 `docs/skill-spec.md`。

## 網頁預覽

### Core 不提供預覽功能

預覽功能由 AI host 負責。AI host 可藉由本身的 WebView 或 Artifact 呈現 `dist/`，亦可藉由外部瀏覽器開啟 `dist/`。CLI 本身不處理網頁預覽。

Core 絕不內建 preview server。若選擇內建，Core 將增加常駐行程（daemon）、通訊埠（port）以及維護 HTTP 伺服器的負擔。這亦與 Tool Form 定位衝突。Core 為執行完畢即結束的 standalone CLI。日後若需解決跨 AI host 的預覽需求，將藉由外掛（plugin）或設計工具整合處理，而非修改 CLI。

### 三條底線

並非所有 AI host 都具備網頁顯示能力。以下三項基本原則（底線）可確保 Designer 不會在未檢視變更前，誤以為視覺變更已經完成：

1. 最低保證：`wda build` 成功後必須印出 `dist/` 的絕對路徑，確保任何能執行 `wda` 的 AI host 皆可接收此輸出。
2. 明示降級：若 AI host 未具備內嵌預覽能力，必須明確提示降級，不得在無人檢視的情況下宣告視覺變更完成。
3. 禁止靜默：未經 Designer 確認檢視的變更，不得視為已接受的狀態。

### 可逆基準

第三項底線原則同時確立了可逆迭代的基準。未經檢視即不構成接受，因此在回溯至前一個已接受狀態時，絕不會回復到 Designer 未曾檢視的版本。

還原基準點由 `wda init` 建立的本機 Git 儲存庫（Repository）提供保證。AI host 的 checkpoint、檔案快照或手動還原機制僅作為輔助，無法取代 Git 儲存庫的基準點。

## 開發與驗證

### 前置工具

本專案的測試與建置需要環境具備以下工具，且皆已加入 PATH 環境變數：

- `deno` 與 `git`：測試時會實際呼叫這兩項工具。若缺少工具，將回報具名診斷並宣告失敗，不降級亦不略過。

### 驗證指令

```sh
cargo test
```

`cargo test` 是本專案唯一的權威驗證指令。
標記 `#[ignore]` 的測試屬於探針測試，預設不會執行：該類測試會使用本機的 `deno` 連線至 npm registry，僅於 CI 的非阻斷性探針工作（probe job）中執行。

## 本專案儲存庫的文件規範

本節的寫作規範適用於必要集合。本節的其他規範僅適用於本專案 Repo 的 `docs/` 目錄。`wda init` 產出專案的文件規則由 `wda check` 負責驗證，具體說明詳見 `docs/commands.md` 的「規則模組：文件」一節。

### 寫作規範

本專案的寫作優先順序是準確、清楚、淺白。準確與其他要求衝突時，保留準確性。清楚與淺白衝突時，保留清楚性。

必要集合包含下列路徑樣式：

- zh-TW：`docs/*.md`、`docs/adr/*.md`、`docs/design-systems/*.md`、`skills/wda/SKILL.md`、`skills/wda/references/*.md`、`README.md`、`SECURITY.md`、`AGENTS.md`、`.dev/project.md`
- en-US：`docs/templates/generated-project/*.md`、`CHANGELOG.md`、`src/builtins/wda_minimal/docs/design.md`

檢查器位於 `tools/writing/`。首次安裝時，從儲存庫根目錄執行下列命令：

```sh
deno install --config tools/writing/deno.json
```

檢查必要集合時，從儲存庫根目錄執行下列命令：

```sh
deno run --cached-only --frozen --allow-read --allow-env --allow-sys --config tools/writing/deno.json tools/writing/check.ts --required
```

`prh` 與 `no-unmatched-pair` 發現是硬性錯誤。`sentence-length` 與 `write-good` 發現是建議。修正 zh-TW 文件時，`zhtw-mcp` 只提供建議，不屬於權威檢查關卡。專案指定術語與 `zhtw-mcp` 建議衝突時，以專案指定術語為準。

每次修正都必須保留所有事實、數字、條件、範圍限定與真實的不確定性。每個修正過的文件都必須檢查修改前後的不變條件。改變意思的修改必須撤回。若必要修正會改變語意，必須停止並回報，不得自行修改。

ADR 的「狀態沿革」小節及其內容不得重寫。檢查器仍會檢查該小節，但其中的發現只列為建議。

### front matter 必填鍵

`docs/` 底下每一份 Markdown 都要有 YAML front matter，六個鍵（key）皆為必填：

| 鍵 | 內容 |
| :--- | :--- |
| `title` | 不得為空 |
| `status` | `draft`、`stable`、`deprecated` 三選一 |
| `updated` | `YYYY-MM-DD`。內容有語意變更時更新 |
| `type` | 本專案自訂的字彙：`Architecture`、`Reference`、`Guide`、`ADR`、`Template`、`Checklist` |
| `description` | 簡明單句描述 |
| `tags` | 陣列，可為空陣列 |

`type`、`description`、`tags` 三個鍵依 [open-knowledge-format(OKF)](https://github.com/GoogleCloudPlatform/open-knowledge-format/blob/main/SPEC.md) 的 SPEC.md 定義。OKF 把 `type` 定為必填的自由字串，所以這六個值是本專案的慣例，不是 OKF 定義的字彙。只有 `status` 的三個值由 OKF 定義。

### status 字彙

| 值 | 意義 |
| :--- | :--- |
| `draft` | 仍在編修中，內容不可作為正式依據 |
| `stable` | 現行規則 |
| `deprecated` | 已被取代，保留僅供歷史追溯 |

### sourceRefs 何時必填

文件若衍生自另一份 canonical 來源，則 `sourceRefs` 為必填欄位，其中各項目皆為相對於專案根目錄的檔案路徑或章節。`README.md` 與根目錄的 `prefile_*` 不適用此項規則。ADR 依規範一律不填寫 `sourceRefs`，具體理由詳見下一節。

### ADR 是一手來源

一份 ADR 記錄一個持久的架構關注點（one durable architectural concern per ADR），其本身即為該關注點的第一手權威來源。因此 ADR 不填寫 `sourceRefs`，內文亦不參照其他文件：不參照其他 ADR、不參照架構文件或其章節，亦不參照 `README.md`。此項限制的考量在於存續週期：被參照的文件未來可能經歷改寫、搬移或刪除，外部連結與參照終將失效，而決策紀錄必須能供長期查閱。若遇到需要外部規則方能完整說明的場景，ADR 應自行摘要陳述該規則的要旨，而非逐字抄錄條文，更不應留下脆弱的外部參照。

ADR 可直接參照程式碼與實體檔案（例如 `src/` 模組、`tests/` fixture、專案根目錄的 `NOTICE`），因為此類檔案屬於客觀實體事實，而非抽象文件。

### 准入條件

新 ADR 的准入條件必須同時滿足兩項（two-part evidence gate）：

1. 被否決的替代方案先前曾被實作、排程或曾為預設值。
2. 支援決策的度量必須指明具體數字、失敗訊息或被推翻的前提假設。

現行的操作規則保留在其所屬的契約文件中，不另立 ADR。

### 編號與檔名

顯示編號格式為 `ADR-NN`；檔案名稱格式為 `docs/adr/NN-<kebab-case-slug>.md`。兩者皆採用相同的二位數字（不足二位補零）。

### 固定五節

每份 ADR 依序包含固定五個小節。若無可供比較的替代選項，即不構成決策，無須撰寫 ADR。

| 節 | 內容 |
| :--- | :--- |
| 背景 | 說明問題背景與當時面臨的限制 |
| 選項 | 至少列出兩個選項，各佔一段，詳述各自的代價與取捨 |
| 決策 | 明確記載所選項目，獨立成段 |
| 後果 | 說明獲得的效益、放棄的取捨，以及對系統其他部分的影響 |
| 狀態沿革 | 記載日期與單句說明，記錄被取代或修訂的歷程 |

YAML front matter 的 `status` 欄位即代表現行狀態。被取代的 ADR 應將狀態修改為 `deprecated`，並於「狀態沿革」中明確記載被何者取代。

一次性歷史沿革繼承規則：每個收斂後的 ADR 依序逐字繼承其所吸收之原始 ADR 的所有狀態沿革項目。每個收斂後的 ADR 於末尾追加一筆 2026-09-21 的收斂記錄。該記錄載明來源見 front matter `absorbs`。內文不寫字面 `ADR-<數字>`。此為單一檔案層級 append-only 規則的一次性例外。Git 歷史紀錄仍為精確的檔案層級封存。

### 主題收斂與對照表

本儲存庫將既有的 26 份原子決策收斂為 6 份主題式 ADR（Thematic ADR），每份記錄一個持久的架構關注點。對照關係如下：

| 新編號 | 主題 | 吸收之舊決策（absorbs） | 檔案名稱 |
| :--- | :--- | :--- | :--- |
| ADR-01 | 產品範圍與輸出邊界 | 001, 002, 016, 017, 028 | `docs/adr/01-product-scope-and-output-boundaries.md` |
| ADR-02 | Authoring、相依與工具鏈 | 006, 011, 013 | `docs/adr/02-authoring-dependencies-and-toolchain.md` |
| ADR-03 | Build、驗證與診斷 | 007, 010, 012, 015, 018, 027 | `docs/adr/03-build-validation-and-diagnostics.md` |
| ADR-04 | Design System、tokens 與中立性 | 008, 014, 019, 024, 025, 031 | `docs/adr/04-design-systems-tokens-and-neutrality.md` |
| ADR-05 | Generated Project 生命週期與可逆性 | 023, 029, 030 | `docs/adr/05-generated-project-lifecycle-and-reversibility.md` |
| ADR-06 | AI Skill 邊界與評測 | 003, 022, 026 | `docs/adr/06-ai-skill-boundaries-and-evaluation.md` |

### 行尾與編碼

Git 儲存庫內一律採用 LF 行尾與無 BOM 的 UTF-8 編碼。若本機環境設定 `core.autocrlf=true`，於 checkout 時所見的 CRLF 並非提交內容。編輯時請沿用檔案既有的行尾格式，切勿僅為轉換行尾而重寫整個檔案。

## 已廢止的 ADR 編號

以下編號不再使用，亦不重新分配：

| 編號 | 原因 |
| :--- | :--- |
| ADR-004 | 內容為僅負責 build、不處理 hosting，屬於架構範疇，已併入本文件「範圍」一節 |
| ADR-005 | 內容為 `wda init` 產出專案的文件結構規範，屬於架構範疇，已併入 `docs/generated-project.md` |
| ADR-009 | 內容為 preview 由 AI host 負責及三項底線原則，屬於架構範疇，已併入本文件「網頁預覽」一節 |
| ADR-020 | 內容為參考用 landing page 的規格，不屬於架構決策，已移至 `.dev/plans/reference-landing-page.md` |
| ADR-021 | 內容為 `wda init` 產出專案架構文件的範本標準，屬於架構範疇，已併入 `docs/generated-project.md` 與 `docs/templates/generated-project/` |
