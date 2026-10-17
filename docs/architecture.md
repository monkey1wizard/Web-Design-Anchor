---
title: "架構"
status: stable
updated: 2026-10-15
type: Architecture
description: "說明 WDA 儲存庫的組成、架構規則與文件規範。"
tags: []
---

# 架構

本文件說明 WDA 儲存庫的組成、責任邊界與架構規則。指令行為詳見 `docs/commands.md`。`wda init` 產出的專案詳見 `docs/generated-project.md`。Design System 與 Skill 的規範分別詳見 `docs/design-system.md` 和 `docs/skill-spec.md`。若規範內容與本文件衝突，以各專屬文件為準。

## 核心原則

### 標準優先

WDA 優先採用 Web 平台標準。沒有合適標準時，採用簡單、明確且廣泛使用的做法。只有標準無法滿足需求時，才新增最少量的 WDA 自訂機制。

### 簡單、明確、單一責任

每個指令、檔案與規則都只負責一件事，並且容易理解。若新指令負責不同工作，就可以新增。

### 人和 AI 都讀得懂

重要資訊必須讓 Designer、AI 與 Engineer 都能理解。結構化狀態可以保存在 `wda.json`、`tokens.json` 或 lockfile，但文件也要說明關鍵狀態。

### WDA 是工具，不是治理

WDA 不替專案擁有者決定治理流程。WDA 可以配合 ADR，但不要求所有決策都寫成 ADR。

### 不依賴單一 AI 產品

WDA 必須方便 AI 使用，但不得依賴特定 AI 產品、host 或其生態系。

## 系統組成

WDA 由規範、Core 與 Skill 三部分組成。三者互相配合，但沒有隸屬關係。

| 實體元件 | 所在位置 / 路徑 | 性質與說明 |
| :--- | :--- | :--- |
| 規範 | `docs/` | 唯一的規範來源。`README.md` 僅供導覽。 |
| Core | `src/`、`tests/`、`schemas/` | 可執行的工具本體。單一 Cargo package，包含 `wda_core` library 與 `wda` binary。 |
| Skill | `skills/wda/` | 隨版本釋出的操作手冊，不是規範來源。 |

規範放在 `docs/`，因為規範的存續時間比任何一版程式碼長。Skill 與 binary 都是交付成果，因此 Skill 不放在 `docs/`。

## 穩定度分層

WDA 的規則分成五個層級。層級決定規則的變更方式、決策者與變更成本。越上位的層級越穩定。

```text
Invariant              固定不變
Default                專案可自訂，寫在 wda.json
Tool Form              WDA 重大改版才會變更
Tooling Choice         WDA 改版時可替換，專案不受影響
Implementation Detail  隨時可替換，外部不可見
```

### Invariant

Invariant 是不可變更的承諾。變更這些承諾就等於改變產品核心定義。

- 部署產物是標準的 HTML、CSS、JavaScript 與 assets。
- WDA 不是網站的 runtime dependency。
- `dist/` 是唯一的部署產物。
- 會讓建置結果錯誤的 Error 一律阻擋建置。Error 的定義詳見 `docs/commands.md` 的「診斷輸出」。

### Default

專案可以在 `wda.json` 明確指定不同值。Designer 沒有明確選擇時，WDA 使用預設值，不主動追問。預設值詳見 `docs/generated-project.md` 的 `wda.json` 欄位表。

### Tool Form

Tool Form 說明 Core 的執行方式。變更執行方式屬於架構重大改版。

- WDA 是獨立執行的 CLI。
- 使用者只需要將一個原生 binary 放進 PATH。

### Tooling Choice

Tooling Choice 說明 WDA 如何實作 Tool Form。WDA 改版時可以替換下列工具，但不得改變 `wda.json` 契約、四個指令的責任或建置結果，也不得要求專案配合修改。

- CLI binary 使用 Rust 撰寫。
- esbuild 將 TypeScript 轉成 JavaScript。
- Deno 與 `deno.lock` 負責相依性解析與鎖定。現行專案以 `deno.jsonc` 宣告 `imports`，並保留舊版 `package.json` 相容性。

### Implementation Detail

Implementation Detail 是 Core 內部的程式組織，可以替換而不改變外部契約。

- Rust 模組的切分方式。
- atomic init 的 rollback 實作。

## 範圍

WDA 只負責建置，並產出可獨立部署的靜態檔案。WDA 不提供代管或部署服務。Designer 或 AI host 可將產物部署到 GitHub Pages、Cloudflare Pages、Netlify 或 S3 等靜態託管服務。

內建代管會讓 WDA 綁定雲端服務，並增加帳號、金鑰、計費與服務商 API 的維護工作。這些工作不影響標準 Web 產物，因此不屬於 WDA 的範圍。

WDA 永久排除下列功能：

- Clean URL、URL rewrite 與 fallback routing。這些功能由部署環境負責。
- preview server。Core 永不內建 preview server。

## 專案目錄結構

下列 `src/` 樹狀結構依目前 `git ls-files src` 整理。其他項目列出儲存庫現有的頂層目錄與檔案。

```text
web-design-anchor/
├── .github/                 CI、發布流程與 GitHub 指示
├── AGENTS.md                專案代理指示與權威檢查命令
├── CHANGELOG.md             版本變更紀錄
├── examples/                範例專案
├── tools/writing/           文件寫作品質檢查器
├── Cargo.toml               單一 package：wda_core library 與 wda binary
├── Cargo.lock               Rust 相依鎖定檔
├── schemas/                 wda.json 的 schema
├── tests/                   整合測試與測試案例
├── docs/                    專案規範與參考文件
├── skills/wda/              隨版本釋出的 Skill
├── packaging/               發布與安裝檔案
├── src/
│   ├── build/               wda build：include、layout、tokens、scripts、report
│   ├── builtins/            內嵌的 WDA Minimal 與 Spectrum 2 資產
│   │   ├── spectrum_two.rs
│   │   ├── spectrum_two/pages/index.html
│   │   ├── spectrum_two/scripts/main.ts
│   │   ├── spectrum_two/tokens/tokens.json
│   │   ├── wda_minimal.rs
│   │   ├── wda_minimal/docs/design.md
│   │   ├── wda_minimal/pages/index.html
│   │   └── wda_minimal/tokens/tokens.json
│   ├── deps/                 wda deps：manifest 與解析
│   ├── init/                 wda init：manifest、文件與解析銜接層
│   ├── validation/           專案規則驗證
│   │   ├── build_output.rs   dist/ 產物檢查
│   │   ├── documents.rs      必要文件與 front matter 檢查
│   │   ├── html.rs            HTML source 檢查
│   │   ├── project_contract.rs wda.json 與 schema 檢查
│   │   ├── tokens.rs          DTCG tokens 檢查
│   │   └── toolchain.rs       工具鏈檢查
│   ├── builtins/mod.rs
│   ├── cli.rs                 參數解析
│   ├── codes.rs               診斷碼
│   ├── diagnostics.rs         診斷資料模型、排序與單行輸出
│   ├── lib.rs                 對外整合介面
│   └── main.rs                stdout、stderr 與結束狀態
├── plugin.json                Agent Plugins 宣告
├── README.md                  專案導覽
├── SECURITY.md                安全模型
├── NOTICE                     第三方素材歸屬
└── LICENSE                    授權條款
```

`src/builtins/spectrum_two/` 保存 Spectrum 2 的 starter page、tokens 與 `scripts/main.ts`。`src/init/mod.rs` 會使用這些資產產生初始化專案。四個驗證模組透過 `src/lib.rs` 提供的共用介面使用，`src/main.rs` 不包含驗證規則。這讓 `wda check` 與 `wda build` 共用驗證引擎。

## 版本相容性

### wdaVersion 不是 format version

WDA 優先維持既有專案格式的向下相容。格式若有破壞性變更，必須明確定義版本，也不得在 `check` 或 `build` 時暗中套用。新版 WDA 可以建置相容的舊格式專案並更新 `wdaVersion`，但不會自動遷移專案結構。新版實作必須保留承諾支援的舊格式驗證器與相依處理分支。

### 格式向下相容與禁止暗中遷移

相依宣告格式必須向下相容，且不得暗中改寫專案檔案。

- 現行格式使用 `deno.jsonc` 宣告 `imports`，並搭配 `deno.lock`。專案使用 `deno.json` 屬於不支援格式，Core 會回報錯誤。
- 舊專案可以使用 `package.json` 宣告相依性。Core 保留舊格式的相依管理與建置流程，讓這些專案可以繼續執行 `wda deps` 與 `wda build`。
- `wda check` 與 `wda build` 是唯讀指令，不會把 `package.json` 專案改寫或遷移成 `deno.jsonc`。

### 四級驗證語意

```text
現行格式          → OK，例如 deno.jsonc 宣告 imports
支援的舊格式      → Warning，例如既有 package.json 專案
已棄用的舊格式    → 更明確的 Warning
不支援的格式      → Error，例如專案使用 deno.json 或雙重設定
```

## 相依模型

Core 依下列架構規則處理相依性：

- 專案格式與相依宣告必須明確，並保留 `package.json` 舊格式的向下相容性。
- 建置工作區位於專案目錄之外。Core 只把支援的相依資訊帶入工作區。
- 專案根目錄的 `deno.lock` 在建置期間維持唯讀。
- `deno.jsonc` 的修改由 Deno 負責，Core 不自行寫入 JSONC。
- 唯讀指令不得遷移或重寫相依宣告。
- 相依操作不得執行第三方 npm 套件的 lifecycle scripts。

各指令的相依行為詳見 `docs/commands.md`。

## Design System 邊界

預設 Design System 是 latest stable Spectrum。線上解析成功時，WDA 寫入確切版本。解析失敗時，`wda init` 使用內建的 `WDA Minimal`，並說明原因。Token 的 canonical source 是 `tokens/tokens.json`，衍生 CSS 只放在 `dist/`。

Design System 的名稱與身分只可出現在解析銜接層 `src/init/` 和內建資產層 `src/builtins/`。其他 Core 程式碼不得依 Design System 身分分支。`tests/neutrality.rs` 的 `IDENTITY_SEAM_FILES` 是唯一允許清單。完整規範詳見 `docs/design-system.md`。

## Skill 邊界

Skill 是 AI 的操作手冊，不是規範來源，也不是第二套產生器。Core 與 `wda.json` 契約不得假設只有 Skill 會操作 WDA。WDA 只提供一個 Skill，名稱為 `wda`，路徑為 `skills/wda/`，版本與所屬 release 相同。Skill 假設 host 能執行原生 binary，並讀取 stdout、stderr 與 exit status。完整規範詳見 `docs/skill-spec.md`。

## 網頁預覽

### Core 不提供預覽功能

預覽由 AI host 負責。AI host 可以使用 WebView、Artifact 或外部瀏覽器呈現 `dist/`。CLI 不負責網頁預覽，也不會內建 preview server。

preview server 會增加常駐行程、通訊埠與 HTTP 伺服器的維護工作，也不符合獨立 CLI 的定位。若未來需要跨 AI host 的預覽功能，應由外掛或設計工具整合提供。

### 三條底線

並非每個 AI host 都能顯示網頁。下列原則可避免 Designer 未檢視變更就接受視覺結果：

1. `wda build` 成功後必須輸出 `dist/` 的絕對路徑，讓 AI host 可以取得產物。
2. AI host 沒有內嵌預覽功能時，必須說明預覽能力受限，不得宣告視覺變更已完成。
3. Designer 尚未檢視的變更，不得視為已接受。

### 可逆基準

未經檢視的變更不算已接受。因此，回復到前一個已接受狀態時，不會回到 Designer 未檢視的版本。

`wda init` 建立的本機 Git 儲存庫提供還原基準點。AI host 的 checkpoint、檔案快照或手動還原只能作為輔助。

## 開發與驗證

`AGENTS.md` 的 authoritative-check fence 指定兩個權威命令：

```sh
cargo test
deno run --cached-only --frozen --allow-read --allow-env --allow-sys --config tools/writing/deno.json tools/writing/check.ts --required
```

缺少 `deno` 或 `git` 時，部分測試會以 `wda.toolchain.deno-missing` 或 `wda.toolchain.git-missing` 診斷失敗。`tests/build.rs` 與 `tests/deps.rs` 中以 `deno_available()` 檢查的測試會在找不到 Deno 時提早返回。帶有 `#[ignore]` 的測試預設不執行。

必要文件的寫作檢查器位於 `tools/writing/`。首次使用前，依 `AGENTS.md` 的指示安裝檢查器相依套件。

## 本專案儲存庫的文件規範

本節的寫作規範適用於必要文件集合。本節其他規範只適用於本儲存庫的 `docs/`。`wda init` 產出專案的文件規則由 `wda check` 驗證，詳見 `docs/commands.md` 的「規則模組：文件」。

### 寫作規範

文件以準確、清楚、淺白為優先順序。要求互相衝突時，先保留準確性，再保留清楚性。

必要文件集合包含下列路徑：

- zh-TW：`docs/*.md`、`docs/adr/*.md`、`docs/design-systems/*.md`。
- zh-TW：`skills/wda/SKILL.md` 與 `skills/wda/references/*.md`。
- zh-TW：`README.md`、`SECURITY.md`、`AGENTS.md` 與 `.dev/project.md`。
- en-US：`docs/templates/generated-project/*.md`、`CHANGELOG.md`。
- en-US：`src/builtins/wda_minimal/docs/design.md`。

檢查器位於 `tools/writing/`。首次安裝時，從儲存庫根目錄執行下列命令：

```sh
deno install --config tools/writing/deno.json
```

檢查必要文件時，從儲存庫根目錄執行下列命令：

```sh
deno run --cached-only --frozen --allow-read --allow-env --allow-sys --config tools/writing/deno.json tools/writing/check.ts --required
```

`prh` 與 `no-unmatched-pair` 的發現屬於硬性錯誤。`sentence-length` 與 `write-good` 的發現屬於建議。修正 zh-TW 文件時，`zhtw-mcp` 只提供建議，不是權威檢查關卡。專案指定術語與 `zhtw-mcp` 建議衝突時，以專案指定術語為準。

每次修正都必須保留事實、數字、條件、範圍與不確定性。修正文件時，必須檢查修改前後的不變條件。若必要修改會改變意思，必須停止並回報。

ADR 的「狀態沿革」小節與內容不得重寫。檢查器仍會檢查該小節，但相關發現只列為建議。

### front matter 必填鍵

`docs/` 下的每份 Markdown 文件都必須包含下列六個鍵：

| 鍵 | 內容 |
| :--- | :--- |
| `title` | 不得為空。 |
| `status` | 使用 `draft`、`stable` 或 `deprecated`。 |
| `updated` | 使用 `YYYY-MM-DD` 格式；內容有語意變更時更新。 |
| `type` | 使用本專案的字彙：`Architecture`、`Reference`、`Guide`、`ADR`、`Template` 或 `Checklist`。 |
| `description` | 使用簡短的單句說明。 |
| `tags` | 使用陣列，可以是空陣列。 |

`type`、`description` 與 `tags` 依 open-knowledge-format（OKF）的 SPEC.md 定義。OKF 規定 `type` 是必填的自由字串，因此上述六個值是本專案慣例，不是 OKF 字彙。只有 `status` 的三種值由 OKF 定義。

### status 字彙

| 值 | 意義 |
| :--- | :--- |
| `draft` | 文件仍在編修，不可作為正式依據。 |
| `stable` | 文件記錄現行規則。 |
| `deprecated` | 文件內容已被取代，保留供歷史查閱。 |

### sourceRefs 何時必填

文件衍生自另一份 canonical 來源時，必須填寫 `sourceRefs`。各項目使用相對於儲存庫根目錄的路徑或章節。`README.md` 與根目錄的 `prefile_*` 不適用這項規則。ADR 一律不填寫 `sourceRefs`，理由見下一節。

### ADR 是一手來源

每份 ADR 記錄一個持久的架構關注點，並且是該關注點的第一手來源。因此 ADR 不填寫 `sourceRefs`，內文也不參照其他 ADR、架構文件或 `README.md`。被參照的文件可能改寫、搬移或刪除，讓參照失效。需要外部規則才能理解時，ADR 應摘要說明規則，不要逐字抄錄或留下容易失效的參照。

ADR 可以直接參照程式碼與實體檔案，例如 `src/` 模組、`tests/` fixture 或儲存庫根目錄的 `NOTICE`。這些路徑指向客觀存在的檔案，不是抽象文件。

### 准入條件

新 ADR 必須同時符合下列條件：

1. 被否決的替代方案曾經實作、排程或作為預設值。
2. 支持決策的度量必須指出具體數字、失敗訊息或遭推翻的前提。

現行操作規則留在所屬的契約文件，不另立 ADR。

### 編號與檔名

顯示編號使用 `ADR-NN` 格式。檔名使用 `docs/adr/NN-<kebab-case-slug>.md` 格式。兩者都使用兩位數字，不足兩位時在前面補零。

### 固定五節

每份 ADR 依序包含下列五節。沒有可比較的替代選項時，不構成決策，不需要撰寫 ADR。

| 節 | 內容 |
| :--- | :--- |
| 背景 | 說明問題背景與當時的限制。 |
| 選項 | 至少列出兩個選項，分別說明代價與取捨。 |
| 決策 | 明確記載選擇。 |
| 後果 | 說明效益、放棄的取捨，以及對系統其他部分的影響。 |
| 狀態沿革 | 記載日期與單句說明，追蹤取代或修訂歷程。 |

YAML front matter 的 `status` 欄位記錄現行狀態。ADR 被取代時，應將狀態設為 `deprecated`，並在「狀態沿革」中記載取代來源。

每份收斂後的 ADR 都依序逐字保留所吸收 ADR 的所有狀態沿革項目，並在末尾新增一筆 2026-09-21 的收斂記錄。收斂記錄的來源列於 front matter 的 `absorbs`。內文不寫出 `ADR-<數字>`。這是單一檔案層級 append-only 規則的一次性例外。Git 歷史仍保存精確的檔案封存。

### 主題收斂與對照表

本儲存庫將 26 份原子決策收斂為 6 份主題式 ADR，每份記錄一個持久的架構關注點。對照關係如下：

| 新編號 | 主題 | 吸收之舊決策（absorbs） | 檔案名稱 |
| :--- | :--- | :--- | :--- |
| ADR-01 | 產品範圍與輸出邊界 | 001, 002, 016, 017, 028 | `docs/adr/01-product-scope-and-output-boundaries.md` |
| ADR-02 | Authoring、相依與工具鏈 | 006, 011, 013 | `docs/adr/02-authoring-dependencies-and-toolchain.md` |
| ADR-03 | Build、驗證與診斷 | 007, 010, 012, 015, 018, 027 | `docs/adr/03-build-validation-and-diagnostics.md` |
| ADR-04 | Design System、tokens 與中立性 | 008, 014, 019, 024, 025, 031 | `docs/adr/04-design-systems-tokens-and-neutrality.md` |
| ADR-05 | Generated Project 生命週期與可逆性 | 023, 029, 030 | `docs/adr/05-generated-project-lifecycle-and-reversibility.md` |
| ADR-06 | AI Skill 邊界與評測 | 003, 022, 026 | `docs/adr/06-ai-skill-boundaries-and-evaluation.md` |

### 行尾與編碼

Git 儲存庫使用 LF 行尾與無 BOM 的 UTF-8 編碼。若本機設定 `core.autocrlf=true`，checkout 後看到的 CRLF 不代表提交內容使用 CRLF。編輯時沿用檔案既有行尾，不要只為轉換行尾而重寫整份檔案。

## 已廢止的 ADR 編號

下列編號已停用，不會重新分配：

| 編號 | 原因 |
| :--- | :--- |
| ADR-004 | 不提供 hosting 的決策已併入本文件「範圍」。 |
| ADR-005 | `wda init` 產出專案的文件結構規範已併入 `docs/generated-project.md`。 |
| ADR-009 | 預覽由 AI host 負責及三項底線原則已併入本文件「網頁預覽」。 |
| ADR-020 | 已於 2026-10-13 刪除，無現行落點 |
| ADR-021 | `wda init` 產出專案架構文件的範本規範已併入 `docs/generated-project.md` 與 `docs/templates/generated-project/`。 |
