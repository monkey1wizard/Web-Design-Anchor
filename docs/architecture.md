---
title: "WDA Core 架構與現行契約"
status: stable
updated: 2026-08-24
sourceRefs:
  - "ROADMAP.md §2.1–§2.12"
  - "ROADMAP.md §3.2 B2"
  - "ROADMAP.md §3.2 B3"
  - "ROADMAP.md §3.2 G1"
  - "ROADMAP.md §3.2 G3"
  - "ROADMAP.md §3.2 G7"
  - "ROADMAP.md §3.2 G8"
  - "ROADMAP.md §3.2 G9"
type: Architecture
description: "本文件說明 WDA Core 的架構與現行契約。"
tags: []
---

# WDA Core 架構

本文件是 WDA Core 的架構與現行契約，承接 `ROADMAP.md` §2.1–§2.12。它描述 Core 工具、標準 Web 專案、文件、Design System、Skill 與預覽責任之間的邊界。產品程式碼尚未建立時，本文件描述的是待實作的契約，不是已驗證的執行結果。

## 2.1 核心原則

### Standards-first

WDA 優先採用 Web Platform 標準；沒有直接標準時，採用簡單、明確且廣泛使用的當代做法。只有真的存在缺口才新增 WDA-specific 機制，且該機制必須維持最小。

### Simple / Explicit / Single Responsibility / Easy to Understand

Simple、Explicit、Single Responsibility、Easy to Understand 是 Core 的設計原則。這不等於「指令越少越好」；若一個新指令代表真正不同的責任，就可以新增。

### 人機雙可讀

重要 project 資訊必須讓 Designer、AI、Engineer 都能理解。精確結構化狀態可以由 manifest 或 lockfile 作為 canonical source，但重要狀態仍必須在文件中說明。

### WDA 是工具，不是治理

WDA 不替 project owner 決定治理流程。WDA 可以支援 ADR，但不要求所有決策都必須以 ADR 記錄。

### 不依賴單一 AI 產品

WDA 必須 AI-friendly，但不得依賴任何特定 AI 產品、host 或其生態。

## 2.2 分層

WDA 的穩定度分層如下：

| 層 | 意義 | 例 |
| :--- | :--- | :--- |
| Invariant | 長期保持的 contract | deployment 是標準 HTML/CSS/JS/assets；WDA 不是網站 runtime dependency；`dist/` 是唯一部署產物；會讓 build correctness 無效的 Error 不允許通過 |
| Default | 除非明確改掉的專案選擇 | MPA、Baseline Widely Available、WCAG 2.2 AA、latest stable Spectrum |
| Tool Form | Core 的執行形態 | standalone CLI executable |
| Tooling Choice | 實現 Tool Form 的技術，可更換 | Rust（CLI binary）、esbuild（TS/JS 轉換）、Deno + `deno.lock`（dependency resolution） |
| Implementation Detail | 可替換且不影響外部 contract | Rust 內部模組組織、atomic init 的 rollback 實作方式 |

更換 Tooling Choice 不得改變 Project Contract、CLI 責任或 Standard Web output contract。

## 2.3 CLI 責任

Core 的四個指令與責任如下：

```text
wda init    建立新的 WDA Project 與本地 Git repository
wda deps    管理 dependency graph / state
wda check   執行 validation，不產生部署產物
wda build   執行必要 validation，並產生部署產物
```

**四指令封閉。** 不得為了 Designer 工作流、Skill 安裝或 preview 需求新增第五個 mandatory 指令。

日常 source 與文件編輯屬於 Designer + AI/Skill 直接修改專案檔案。Core **不是** general-purpose editor。

## 2.4 Source 與 Deployment

### Baseline 與責任分工

| 項目 | 內容 |
| :--- | :--- |
| Source Baseline | HTML5 / semantic HTML、Modern CSS、TypeScript、原生瀏覽器能力；真正需要 component 時使用 Web Components |
| Deployment Baseline | HTML、CSS、JavaScript、assets |
| 責任分工 | HTML 負責結構與靜態內容；CSS 負責呈現、版面、響應式與可由 CSS 處理的狀態；TypeScript 負責互動、必要 state 與 browser APIs |
| `file://` 邊界 | Core 保證產物可部署於正常的 static HTTP(S) host，但不保證以 `file://` 直接開啟可用。`wda build` 僅在偵測到 ES module、`fetch` 或其他已知不相容特徵時回報 HTTP requirement；未偵測到已知 requirement 不構成 `file://` 保證。（裁定 G8） |
| Type check | 必須有真正的 static type-check。bundler transpile 不等於 type-check |
| Page architecture | 預設 `mpa`，合法值 `mpa` / `spa`。MPA 是 silent default，不要求 Designer 先回答架構問題 |
| Clean URL | 全面不提供保證。不承諾 extensionless URL，也不產生 nested `index.html` 模擬路由。URL rewrite / fallback routing 屬部署環境 |
| Page title | 每頁必須有 `<title>`。首頁 `{projectName}`；其他頁 `{pageTitle} \| {projectName}`。不產生 `<meta name="description">` |
| Framework | 不是 V1 default。是否升級由真正的應用複雜度決定，routing 本身不是理由。migration 目標只承諾降低成本，不承諾自動、無損或零成本 |

### `dist/` invariant

`dist/` 是唯一部署產物，並且：

- 固定 canonical 目錄，不可設定；
- 每次 build 從 source 乾淨重建，避免 stale output；
- 可在無 WDA、無 source repo、無開發工具鏈的環境下 deploy 並執行；
- build 不會把 secret file 放進 `dist/`；
- 預設進 `.gitignore`；
- `wda init` 在 root atomically 建立 `.gitignore`，寫入 canonical `/dist/`；後續任何 WDA command 都不重寫或管理它；
- `wda init` 遵守碰撞規則（collision rule），接受已含檔案的目錄，但只要預計建立的任何名稱已被佔用即拒絕執行，不覆寫亦不 merge 既有檔案；`wda init` 成功時建立本地 Git repository 與包含全目錄的初次 commit；
- `.gitignore` 是一次性的 convenience artifact，不是 dependency；`wda check`、`wda build` 與 `wda deps` 在沒有 Git repository、也沒有 `.gitignore` 時都必須可執行。（裁定 G9）
- 衍生產物（如 token CSS）只存在 `dist/`，不反寫 source。

Source 與 `dist/` 是兩棵獨立的樹，各自遵守「一個名稱在同一棵樹內只有一個 canonical location」。

## 2.5 Design System 與 Tokens

| 項目 | 內容 |
| :--- | :--- |
| 預設 Design System | latest stable Spectrum。線上解析成功時必須寫入 exact version |
| Offline fallback | 使用該 WDA release 內建的 fallback set，對同一 release 固定；內建 fallback 是 `WDA Minimal` |
| `WDA Minimal` | WDA 自有的最小內建 Design System；完整定義只有一個 Core canonical source：[`docs/design-systems/wda-minimal.md`](design-systems/wda-minimal.md)。本文件只引用該定義，不複製其內容。 |
| Design System ownership | `docs/design-systems/wda-minimal.md` 是 `WDA Minimal` 的唯一 Core 定義。專案文件只保存該專案實例化後的 choices、effective tokens 與 project-specific review guidance，不複製 WDA-wide 定義。（裁定 B2） |
| Token canonical source | `tokens/tokens.json`，DTCG 格式；build 後轉成 `dist/styles/tokens.css` 的 CSS Custom Properties |
| Source 端 tokens.css | 不得存在；generated token CSS 只在 `dist/` |
| Token 命名 | 以 semantic 為主，namespace 是 project-owned。允許 DTCG alias，但 canonical project token 不直接綁 vendor token identifier |
| Primitive layer | 不強制兩層。只有專案真的需要 scale / reuse / hierarchy 時才加入 |
| Adapter 邊界 | 關注 token mapping、styles、fonts、icons、theme、component dependencies、dependency integration。native HTML/CSS first；vendor public element name / API 是合法用法；不為隱藏 vendor 名稱而建立 universal wrapper。七個 facet 的文件化 public interface 定義於 [`docs/design-systems/adapter-boundary.md`](design-systems/adapter-boundary.md)，並針對 `WDA Minimal` 與 Spectrum 兩案例演練；Spectrum 的紙上映射見 [`docs/design-systems/spectrum-paper-adapter.md`](design-systems/spectrum-paper-adapter.md)。這是文件化 interface，不是 P3 的 Rust trait 或 plugin registry |
| Neutrality proof | V1 至少支援兩套差異夠大的 Design System，用以驗證 Core 沒有綁死單一系統；這不是證明 A 可自動轉 B |

## 2.6 Components 與 Build-time Composition

唯一的 WDA-specific build-time primitive 是：

```text
<wda-include src="..."></wda-include>
```

它符合 Custom Element 命名慣例，在 build time 展開，**不是 runtime Web Component**，而且 `dist/` 不得殘留該標籤。

**V1 不建立 template language**：沒有 `<slot>` composition、`if`、`for`、variables、expressions、data binding。

一般 layout、文字、圖片、按鈕、表單、靜態內容優先使用 native semantic HTML + CSS。只有真的需要 reusable behavior、reusable product semantics 或封裝時，才建立 component abstraction。

## 2.7 Dependencies

| 項目 | 內容 |
| :--- | :--- |
| 宣告 | `package.json` 作為 JS/npm canonical declaration |
| 解析 / lock | Deno + `deno.lock`（V1 的 Tooling Choice） |
| 使用者負擔 | 不要求使用者管理 Node.js 前置環境 |
| `wda deps` | 唯一能改變 dependency graph 的責任邊界：add / remove / update |
| `wda build` | 可取得已 locked 的 dependency，但不得選新版本、改 graph 或 silent upgrade |
| Runtime | WDA 永遠不是網站的 runtime dependency。production output 預設 self-contained；remote runtime dependency 只在使用者明確要求或第三方服務本質限制時作為例外 |
| Secrets | build 不會把 secret file 放進 `dist/` |
| 空狀態 | 沒有實際 dependency 時不建立空的 dependency state |

## 2.8 診斷與嚴重度

**Severity 依影響判斷，不依問題類別硬切。**

| 級別 | 意義 |
| :--- | :--- |
| Error | 問題會影響 build / project correctness，嚴重到 build 不允許通過。只要能 deterministic 判定產物會不正確，就可以是 Error |
| Warning | 不阻擋 build，但代表 correctness / quality concern，必須夠醒目、persistent，讓 Designer 與 AI 不能輕易忽略 |

不得永久寫成「所有 a11y 都是 Warning」或「所有 script 問題都是 Error」。

V1 source-check allowlist 固定為：non-empty `html[lang]`、每個 `img` 都有 `alt` attribute（不判斷文字品質），以及 document IDs 唯一。已證明的 violation 依 impact 分級；meaningful alt、cascade／imagery 後的 contrast、focus visibility／order、keyboard behavior、zoom／reflow 與 motion 等 rendered/contextual 判斷保留給人工 review。（裁定 G7）

自動檢查全過不得宣稱符合 WCAG。這項能力邊界應寫在文件或成功時的 scope summary，不得每次產生無法解除的 generic `manual-review-required` Warning。只有存在具體且可採取行動的 condition 時才發出 Warning。

V1 的 docs consistency 只限文件結構與 metadata 驗證：`wda check` 驗證四份 required documents，解析由 G4 規範的 WDA-managed `docs/**/*.md` metadata；缺少 required document 是 Error；required metadata 缺漏或格式錯誤依文件類別分級，在 required document 上是 Error，在 opt-in 的 `docs/**/*.md` 上是 Warning；無法解析的 `sourceRefs` 是 persistent Warning。Conditional 文件只在存在時驗證；不得以 prose 語意與程式碼、`wda.json`、tokens 或 dependency state 比對來判定 consistency。（裁定 B3）

`wda check` 與 `wda build` 必須使用同一套 validation engine。build 時 Error 阻擋、Warning 不阻擋。不能靠「使用者記得先跑 check」來維持正確性。

### 診斷輸出契約

V1 不提供 `--json`，也不承諾任何 public structured diagnostic contract。Core 內部使用單一 typed `Diagnostic` value model，但該型別是 implementation detail，不是外部 schema。（裁定 G1）

所有 diagnostics 採 deterministic、single-line、讓 Designer 與 AI 都能直接閱讀的文字。對外穩定的語意只有 severity 的意義、diagnostic code、適用時經正規化的 project-relative path 與行列位置、stderr/stdout 分流，以及 exit status；message 與 next action 必須存在且可理解，但逐字內容可演進。Project-controlled newline 或 control characters 不得注入第二筆偽造 diagnostic，也不得洩漏 host absolute path。

同一 binary 對相同輸入必須產生相同 diagnostic 集合與順序；內部實作須定義 total-order tie-breaker。標點、delimiter、欄寬與 exact envelope 不屬 compatibility contract，consumer 不得靠切割文字欄位來解析。Diagnostics 寫入 stderr，成功結果與非 diagnostic summary 寫入 stdout。

Exit status 固定為：沒有 Error 時 `0`（允許 Warning）；validation report 含任何 Error 時 `1`；CLI usage error、未完成指令或 `ToolFault` 時 `2`。傳入未支援的 `--json` 必須在 stderr 明確拒絕、stdout 不輸出任何 JSON，並以 `2` 結束，不得暴露 accidental JSON surface。

只有具名 machine consumer 與可重現 failing fixture 證明上述文字介面不足時，才可另案提出帶版本的結構化輸出，並重新經過 ADR 與 architect review。Fixture 必須證明缺少必要資訊，不得只以 consumer 不願處理文字作為理由。第一個可考慮的範圍只限 `check`／`build` diagnostics，不從全指令 contract 開始。

## 2.9 Project Contract（`wda.json`）

每個 WDA Project root 固定有 `wda.json`，採 strict schema：

- 沒有 `metadata`、沒有自訂擴充區、沒有任意 top-level 欄位；
- **unknown field = Error**，不是 Warning；
- instance 本身不放 `$schema`；
- 不使用 `projectFormatVersion`、`deploymentDirectory`、`projectDescription`。

已定案欄位如下：

| 欄位 | 規則 |
| :--- | :--- |
| `projectName` | 預設 `Web Design Anchor Project`，參與 page title 生成 |
| `projectVersion` | `init` 建立，格式為 host 本地時間 + 秒 + UTC offset，無毫秒；init 後不自動遞增 |
| `wdaVersion` | 記錄最近一次成功 init 或成功 build 所使用的 WDA 版本 |
| `pageArchitecture` | `mpa` 或 `spa` |
| `browserBaseline` | `baseline-widely-available` |
| `accessibilityBaseline` | `wcag-2.2-aa` |
| `designSystem` | `{ name, version }` |

Canonical schema 是 checked-in 檔案 `schemas/wda.schema.json`；standalone binary compile/embed 同一份檔案，不 commit 任何 generated duplicate。Binary 內部的 semantic checks 可補充 cross-field 規則，但不得重新定義 schema 已表達的欄位；只有實際支援 legacy format 時才新增 legacy schema。（裁定 G3）

`wdaVersion` 的更新規則是：成功 build 更新；失敗 build 不更新；`check` 與 `deps` 不更新。

## 2.10 版本相容性

優先維持對既有 Project Format 的向後相容。新版 WDA 應盡可能繼續支援舊格式；破壞性的格式變更必須明確、有版本標示，且不得在 `check` 或 `build` 期間隱式套用。

`wdaVersion` 不得被當成偽裝的 format version。新版 WDA 可以成功 build 一個舊但相容的專案並更新 `wdaVersion`，但專案結構並未因此自動 migration。因此新版實作必須保留它承諾支援的 legacy validator。

驗證語意四級如下：

```text
Current format            → OK
Supported legacy format   → Warning
Deprecated legacy format  → 更明確的 Warning
Unsupported format        → Error
```

<a id="skill-boundary"></a>

## 2.11 Skill 層

AI Skill 是 AI 的使用手冊與工作流程層，告訴 AI 如何辨識、安裝與使用 `wda` CLI、如何讀取專案、何時呼叫哪個指令、如何修改專案、如何維護文件。

**Skill 不是規格來源。** 它不保存核心規則、不形成第二套 generator。規則存在於 project documentation 與 Core Tooling。Core 與 Project Contract 不得建立在「只有 Skill 能操作」的假設上。

### 切分

**切分軸是 trigger 可分辨性**，不是責任、不是內容量、不是 CLI 指令對應。

| 層 | 誰在選擇載入什麼 | 選擇的時點 | 切分軸 |
| :--- | :--- | :--- | :--- |
| CLI 指令 | 呼叫方 | 呼叫方已經知道要做什麼之後 | 責任 + side-effect 邊界 |
| Skill | AI 自己 | AI 還不知道要做什麼之前（語意觸發） | trigger 可分辨性 |

CLI 挑錯指令會立刻報錯；Skill 挑錯不會報錯，只會靜默走錯流程。因此「責任不同就可以新增指令」的原則不適用於 Skill 切分。兩個 Skill 只有在「AI 載入任何一個之前就能無歧義判斷該載哪一個」時才成立。

**V1 只有一個 Skill，名稱 `wda`。** 三種替代切法皆已否決：依 CLI 指令切，因為單次請求可橫跨多個指令，且 `deps` 對 Designer 幾乎不觸發；依 Designer 工作階段切，因為階段間 trigger 重疊，且各份都要複述同一組不變量，該複製本身就是第二套 generator 的起點；依角色切，因為做 Engineer Skill 等於承認 Engineer 也需要 Skill 才能操作。

### 結構

```text
skills/wda/
├── SKILL.md
└── references/
    ├── install.md          只在 wda 不在 PATH 時載入
    ├── intake.md           只在建立新專案或重大風格轉向前載入
    ├── assets.md           只在 Designer 提供素材時載入
    ├── reversibility.md    只在第一次視覺變更前、或要求還原時載入
    ├── iteration.md        只在任何視覺變更請求即將被執行之前，每一次都載入
    ├── reporting.md        只在觸發回報義務時載入
    └── troubleshooting.md  只在 check / build 回報 Error 時載入
```

`scripts/` 與 `assets/` **不建立**。

佈局的標準來源是 Agent Plugins 1.0.0 開放標準（https://agent-plugins.org/specification）：`skills/<name>/SKILL.md` 是該標準與 Claude Code 共同的探索規則。release package 的 archive 根部放一個最小 `plugin.json`（僅 `$schema` 與 `name` 兩欄），整個 package 因此成為合規套件，任何合規 client 都能直接安裝。這就是「安裝是 host 的事」的具體機制，Core 不需要任何安裝指令。

`SKILL.md` 內容白名單只准這六類，其餘一律是違規訊號：角色與邊界宣告、版本比對步驟、決策路由表、停止條件、回報與宣告義務、指向權威來源的指標。**不放規則內容本身。**

行數兩層：硬上限 **500 行 / 5000 tokens**；審查觸發線 **300 行**（不 fail，觸發重複性審查）。四個指令加七份 reference 的路由不需要 300 行，越線即「有規則被複製進來」的訊號。

**切分口訣：程序進 Skill，判準進專案文件。** 品牌與風格 intake 是程序（問什麼、什麼順序），進 Skill；視覺審查準則是判準（什麼算好），歸專案的設計文件。

### 歸屬

Skill 不是 documentation，而是隨 binary 發佈的產物。三件事分開：

| 東西 | 歸屬 |
| :--- | :--- |
| Skill 的實體內容 | `skills/wda/`，與 `src/`、`docs/` 並列，不在 `docs/` 底下 |
| 關於 Skill 的規範 | Core 的本文件加上一條 ADR |
| 安裝到 AI host 的副本 | 部署副本，等同 binary 裝進 PATH，無獨立歸屬 |

Core **產出並發佈** Skill（隨 release package），但把 Skill 安裝進 AI host 是 host 的事，**不得**為此新增 `wda skill` 指令。

### 版本

Skill 無獨立版本號，版本等於所屬 WDA release 版本，承載於 frontmatter 的 `metadata.version`。不建立 Skill 與 CLI 的相容矩陣。

真正的風險是安裝副本的 skew：Skill 副本在 AI host、binary 在 PATH，兩者可各自更新。三分支處置如下：

| 情況 | 行為 |
| :--- | :--- |
| 相等 | OK，繼續 |
| Skill 較舊 | Warning，可繼續（CLI 向後相容，Skill 只是少用到新能力） |
| Skill 較新 | Error，停止並要求更新；不得靜默降級為盡力而為的 role-play |

比對步驟必須寫成「讀取本檔 frontmatter 的 `metadata.version`，與 `wda --version` 比對」；AI 讀的是文字，不依賴 host 解析 frontmatter 欄位語意。這讓版本比對對 host 差異免疫。

Skill **不得**自行解讀 `wda.json` 的 `wdaVersion` 做專案格式相容判定。那是 Core 的責任，Skill 只轉述 `wda check` 的輸出。否則會長出第二套 validator。

### 禁止可執行腳本

**`scripts/` 為空，目錄不建立。**

理由是它與「零 runtime 前提」直接衝突。選 Rust 實作 CLI 的理由之一就是使用者不需要為了執行 WDA 管理 Node.js runtime；而 Skill 腳本支援的語言取決於 AI host 實作，任何 Python 或 Node 腳本都會把這個刻意消除的前提從側門裝回來——使用者以為只裝了 `wda`，實際還需要直譯器。

爭點不是 host 能不能執行 shell，而是「native binary，零 runtime」對上「需要直譯器的腳本」。

**紅線判準**：需要保證正確執行的操作屬 `wda` binary，評估是否值得成為新指令；需要判斷／翻譯／決策的操作屬 AI，寫在 `SKILL.md`。中間沒有第三類。

支持腳本的最強論點是確定性，但它導出的結論是「屬 Core」而非「寫腳本」。用 Skill 腳本承接確定性需求，等於從側門長出一個沒有指令名稱、沒有 side-effect 契約、沒有版本相容保證的第五個 Core 能力。

例外程序：新增任何腳本須同時滿足零語言 runtime 前提、且證明不屬於 Core 責任，並附一條 ADR 與 architect review。

### 跨 host 可攜性

frontmatter 只使用 `name`、`description`、`compatibility`、`metadata`、`license`。**明確不使用 `allowed-tools`**；它是實驗性欄位且各 AI host 支援度不一，依賴它就是依賴特定實作。

**Skill 對 host 的唯一能力假設是：能執行一個 native binary，並捕捉 stdout、stderr 與 exit status。** 這與可靠執行 `wda check`／`wda build` 是同一項 process contract，不新增特定 host API。Warning-only 的 exit `0` 路徑仍必須讀取 stderr，不得只看成功狀態或 stdout。

### 不得依賴外部 skill

| 行為 | 判定 |
| :--- | :--- |
| 缺少某外部 skill 時流程無法完成 | 禁止 |
| 具名推薦某個外部 skill | 禁止。skill 名稱是移動目標；寫進去會變成過期指涉，且把 WDA 綁在單一廠商的生態上 |
| host 剛好有別的 skill 一起作用 | 允許。WDA Skill 既不知道也不在意 |

### 驗收四層

| 層 | 內容 | 性質 |
| :--- | :--- | :--- |
| L1 規格合規 | 通過 Agent Skills 規格驗證工具 | CI blocking |
| L2 邊界 guardrail | `scripts/` 與 `assets/` 不存在；`SKILL.md` 與 `references/**` 中語言標記為 `html` / `json` / `css` / `toml` 且超過 5 行的 code block 為零（只允許 `text` 流程圖與 `bash` 指令呼叫）；含「必須／不得／只能」的規範句同段落必有權威指標；Skill 字串與 Core 的 schema 或診斷訊息無重複；行數未超上限 | CI blocking，僅防回歸 |
| L3 行為評測 | 測試 prompt 集 → 執行 → 質性與量化評估 → 迭代；必含 undertriggering 檢查 | 主驗收 |
| L4 Deletion Test | 刪掉整個 Skill 後，Core 與專案文件仍足以讓人正確操作 WDA | 人工，驗收級 |

**L3 決定 Skill 是否可交付；L1 / L2 只決定它是否可進 CI。** L2 全綠但 L3 不過，等於不可交付。

L2 的 `assets/` 為 WDA 專屬的收緊，不是通則；專案檔案的生成是 `wda init` 的獨佔責任，Skill 若攜帶可複製成專案檔案的模板，就與 init 形成兩個生成來源。

L3 必含 undertriggering 檢查，因為 AI 傾向不觸發本該觸發的 skill。對 WDA 這是產品級風險：Skill 沒觸發，AI 就會用通用 web 知識直接改檔案，繞過 `wda check` 與 `wda build`，產出不合約的專案，而且完全靜默。因此 `description` 的撰寫與評測是獨立任務，不是附帶工作。

## 2.12 Preview

**Core 永不擁有 preview。** 所有權鏈如下：

```text
V1   → host 承擔（AI host 的 WebView / Artifact，或用瀏覽器開 dist/）
V2   → plugin 與設計工具整合承擔
Core → 永不承擔，CLI 不處理
```

因為並非每個 AI host 都有顯示網頁的能力，必須有三條地板：

1. **最低保證**：`wda build` 成功後必須報出 `dist/` 的絕對路徑。任何能執行 `wda` 的 host 都具備接收這個輸出的能力；
2. **明示降級**：host 無內嵌預覽能力時必須明講，不得在未被看見的情況下宣告視覺變更完成；
3. **禁止靜默**：未經使用者確認看過的變更，不得計為「已接受狀態」。

Preview 不保證以 `file://` 直接開啟可正常執行；Core 不提供 file:// fallback，實際可用性由 host 與部署環境決定。（裁定 G8）

第 3 條同時定義可逆迭代的基準：沒看過就不算接受，因此「回到上一個已接受狀態」不會回到一個 Designer 從未看過的版本。

可逆機制以 Git 為基礎，Core 硬綁 git（見 ADR-023），由 `wda init` 建立本地 Git repository 作為還原基準點。AI host checkpoint、file snapshot、accepted-state marker 或手動還原等其他機制可作為輔助。
