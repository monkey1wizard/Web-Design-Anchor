---
title: "Generated Project"
status: stable
updated: 2026-11-12
type: Architecture
description: "wda init 所產出專案的契約規格：source 基準、語言分工、專案目錄樹、`anchor.json` 與 `package.json` 契約檔、`dist/` 對應，以及專案的文件集合。"
tags: []
source: docs/generated-project.md
lang: zh-TW
source_sha256: e2e90a5e954d9b1e2c05db3764124ab0254b55f8291815a509f6e903c5159e29
translated_at: 2026-11-12
---

# Generated Project

`wda check` 和 `wda build` 以包含該標籤的檔案為基準解析 `<wda-include src="...">`。Include 必須留在專案根目錄內，且不得形成循環。兩個指令都會回報找不到的目標、超出根目錄的路徑，以及循環的完整引用鏈，並在展開後檢查 ID。診斷位置指向原始檔，行號和 UTF-8 位元組欄號均從 1 起算。CRLF 算一個換行。`wda check` 不會寫入 `dist/`。

`wda init` 產出的專案稱為 Generated Project。它是一個 Standard Web Project（定義見 `docs/glossary.md`）。本規格描述專案的技術基準、各類 source 的角色、目錄結構、`anchor.json` 與 `package.json` 契約檔、外部 lockfile 的責任、source 與 `dist/` 之間的對應規則、支援範圍與文件集合。命令行為與驗證規則見 `docs/commands.md`。

## Source 基準與部署基準

| 基準 | 內容 |
| :--- | :--- |
| Source | HTML5 與語意化 HTML、Modern CSS、TypeScript，以及原生瀏覽器能力。只有在專案確實需要元件封裝時才使用 Web Components |
| 部署 | HTML、CSS、JavaScript 與 assets |

前端框架不是預設選項。是否引入框架，取決於應用程式的實際複雜度。單獨的路由需求不足以成為引入框架的理由。WDA 承諾降低日後遷移的成本，但這不是自動遷移、無損遷移或零成本遷移的承諾。

## HTML、CSS 與 TypeScript 的角色

- HTML 負責結構與靜態內容。
- CSS 負責視覺呈現、版面、響應式設計，以及 CSS 本身能處理的狀態。
- TypeScript 負責互動行為、必要時的狀態管理，以及呼叫瀏覽器 API。

TypeScript 必須經過真正的靜態型別檢查。打包工具的 transpile 步驟不能算作型別檢查。

## 專案目錄樹

### `wda init` 建立的目錄樹

`wda init` 只建立下列路徑：

```text
project root/
├── .git/                 Git 可用時的本機 Git 儲存庫
├── .gitignore            內容固定為 /dist/
├── README.md
├── anchor.json           WDA 專案契約
├── package.json          網站依賴宣告
├── docs/
│   ├── architecture.md
│   ├── design.md
│   └── glossary.md
├── pages/
│   └── index.html
└── tokens/
    └── tokens.json
```

此目錄結構是 `WDA Minimal` 的基礎結構，用於離線初始化，以及依賴解析失敗時。線上初始化成功時，建立的內容取決於 Design System 與依賴安裝結果。細節見 `docs/commands.md`。

Generated Project 只在 `package.json` 中宣告網站依賴。WDA 不會建立 `deno.jsonc`、`deno.json` 或 `deno.lock`。既有的舊檔案不能替代任一必要的 manifest。

### 需要時才建立的目錄路徑

下列路徑只在專案需要時建立。`wda init` 不建立佔位目錄。

```text
components/            可重用的靜態 HTML 片段，或具有真實重用需求的產品語意元件
styles/                共用的 source 樣式表，單一樣式表不足時使用
scripts/               TypeScript 互動模組，行為邏輯需要 source 組織時使用
assets/                專案自有的 assets，例如圖片、字型與圖示
pages/<name>.html      額外的 MPA 頁面入口
docs/development.md    四個 WDA 命令與必要文件未涵蓋的專案專屬工作流程
```

## 專案契約檔

每個 Generated Project 在根目錄都有兩個契約檔，兩者都是必要的。

| 檔案 | 角色 |
| :--- | :--- |
| `anchor.json` | WDA 專案契約。定義專案身分、技術基準、使用中的 Design System，以及初始化退回記錄。依 `schemas/anchor.schema.json` 驗證 |
| `package.json` | 唯一的網站依賴宣告。不承載 WDA metadata |

缺少任一檔案時，`wda check` 與 `wda build` 會針對該檔案回報具名 Error。缺少的檔案絕不會從其他檔案補齊，也不會把舊的 `wda.json` 當成替代。缺少任一檔案的專案無法建置。WDA 不會自動轉換舊專案。建置成功時會更新 `anchor.json` 的 `wdaVersion` 並寫入 `dist/`。驗證與失敗的建置不會改變 `wdaVersion`。要使用目前格式，請以 `wda init` 建立新專案，或手動改寫兩個檔案。

## `anchor.json`

`anchor.json` 是專案契約的核心。它定義專案身分、技術基準與 Design System。`wda check` 與 `wda build` 都會先讀取此檔案。

### 為什麼 Schema 採嚴格模式

`anchor.json` 只允許明確定義的欄位，讓人、AI 與 Core 都能理解此檔案。若加入自訂內容，其他讀取者就無法判斷其意義或用途。

- 沒有 `metadata` 欄位、沒有自訂擴充區域，也沒有任意的頂層欄位。
- 未知欄位一律是 Error，不是 Warning。
- 檔案內容不包含 `$schema` 欄位。
- `projectFormatVersion`、`deploymentDirectory`、`projectDescription` 與 `wda.json` 是過時或非契約名稱，不再使用。

### 欄位

| 欄位 | 內容 | 何時寫入與誰可以修改 |
| :--- | :--- | :--- |
| `projectName` | 預設為 `Web Design Anchor Project`。用於組成頁面標題 | `init` 寫入預設值，之後由專案修改 |
| `projectVersion` | `init` 執行當下主機的本機時間，包含秒與 UTC 偏移，不含毫秒。之後不會自動遞增 | 只由 `init` 寫入一次 |
| `wdaVersion` | 最近一次成功 `init` 或 `build` 所用的 WDA 版本 | `init` 寫入，每次成功的 `build` 都會更新 |
| `pageArchitecture` | 允許值為 `mpa`（多頁架構）與 `spa`（單頁架構）。預設為 `mpa` | `init` 寫入預設值，之後由專案修改 |
| `browserBaseline` | 唯一允許值為 `baseline-widely-available`（Baseline Widely Available），也是預設值 | 只由 `init` 寫入 |
| `accessibilityBaseline` | 唯一允許值為 `wcag-2.2-aa`（WCAG 2.2 AA），也是預設值 | 只由 `init` 寫入 |
| `designSystem` | 含必要欄位 `name` 與 `version` 的物件。預設為最新穩定版 Spectrum。線上解析成功時，`version` 為確切的 semver 版本號。退回 `WDA Minimal` 時，`version` 為該 WDA 發行版內建的版本 | 由 `init` 寫入。Spectrum 整合並驗證成功後，Skill 更新為實際使用的系統。只加入 Lit 時保持不變 |
| `initialization.fallback` | 選填。只有當 `wda init` 建立 `WDA Minimal` 而非原定 Design System 時才出現。包含 `requestedDesignSystem`（init 原先嘗試的 Design System）與 `reason`（說明退回原因的非空文字）。未發生退回時不存在 | 由 `init` 寫入。Spectrum 整合並驗證成功後，Skill 移除 `initialization` 包裝。只加入 Lit、拒絕整合或整合失敗時保持不變 |

專案能否修改某個欄位，取決於該欄位的穩定性層級。詳見 `docs/architecture.md` 的「Stability Layers」。

### Schema 的儲存方式

標準 schema 是 `schemas/anchor.schema.json`，納入版本控制。執行檔直接內嵌相同內容，儲存庫不提交自動產生的副本。執行檔也會驗證 JSON Schema 無法表達的跨欄位規則，且不重新定義 schema 已宣告的欄位。產品中不存在早期 `wda.json` 格式的 schema，驗證器也從不讀取該格式。

### wdaVersion 如何更新

`wdaVersion` 記錄最近一次成功處理此專案的 WDA 版本。它不是專案格式版本，不可用來判斷格式相容性。格式相容性規則見 `docs/architecture.md` 的「Version Compatibility」。

| 命令結果 | `wdaVersion` 的處理 |
| :--- | :--- |
| `wda init` 成功 | 寫入 |
| `wda build` 成功 | 在 `anchor.json` 中更新 |
| `wda build` 失敗 | 不變 |
| `wda check` | 不變 |

## `package.json` 與外部 lockfile

`package.json` 是唯一的網站依賴宣告。外部套件管理器 npm 或 Bun 負責其變更。WDA 讀取它，且不在 `anchor.json` 中重複任何依賴。

| 欄位 | 內容 | 由誰寫入 |
| :--- | :--- | :--- |
| `dependencies` | 選填物件。每個值都是字串版本。沒有網站依賴的專案可以省略它或保留空物件 | 外部管理器在安裝後寫入。只有線上解析成功時，`init` 才寫入 Spectrum 與 Lit 項目 |
| `packageManager` | 選填。管理器名稱、`@` 與確切版本，例如 `npm@10.9.2`。Spectrum init 成功時記錄實際管理器與確切版本，即使只有一個可用。初始 Minimal 不含此欄位 | `init` 在 Spectrum 安裝成功時寫入 |
| `name`、`private`、`type` | 由 `init` 寫入，作為專案套件身分。WDA 不檢查其值 | `init` 寫入，專案可以修改 |

`package.json` 的根必須是物件。若形狀不同、依賴值不是字串，或 `packageManager` 值不符合上述規則，WDA 會回報具名 Error。

Manager Lockfile 是外部套件管理器寫入的 lockfile，例如 npm 的 `package-lock.json` 或 Bun 的 `bun.lock`。它由管理器負責。WDA 絕不寫入、改寫或遷移 Manager Lockfile。`wda check` 與 `wda build` 只讀取已安裝的內容，`wda tools fetch` 也不改變 `anchor.json` 或 `package.json`。

## `dist/`

### dist/ 目錄樹

`dist/` 是唯一的部署產物目錄。下列範例顯示建置後的目錄結構：

```text
dist/
├── index.html
├── about.html
├── styles/
│   ├── main.css
│   └── tokens.css
├── scripts/
│   └── main.js
└── assets/
    └── images/
        └── hero.png
```

### 從 source 到 dist/ 的對應規則

從 source 到 `dist/` 的路徑對應是固定的，沒有任何設定選項：

| Source 路徑 | `dist/` 路徑 | 規則 |
| :--- | :--- | :--- |
| `pages/<rel>.html` | `dist/<rel>.html` | 保留副檔名。不建立巢狀 `index.html` 目錄 |
| `styles/<rel>.css` | `dist/styles/<rel>.css` | 直接對應到輸出 |
| `scripts/<rel>.ts` | `dist/scripts/<rel>.js` | 先做型別檢查，再 transpile |
| `assets/<rel>` | `dist/assets/<rel>` | 只複製頁面或樣式表實際引用的檔案 |
| `tokens/tokens.json` | `dist/styles/tokens.css` | 建置期間 transpile 並產生 |

### dist/ 的不變條件

- `dist/` 是固定的輸出目錄名稱，沒有自訂設定。
- 每次建置都從 source 重新建置，不保留上一次建置的輸出。
- `dist/` 必須能在沒有 WDA、沒有 source 程式碼、也沒有開發工具的環境中，獨立部署並執行。
- 衍生產物只能存在於 `dist/`，不得寫回 source。例如，由 `tokens/tokens.json` transpile 出的 CSS 只存在於 `dist/styles/tokens.css`，source 中不得出現 `styles/tokens.css`。
- source 端與 `dist/` 是兩棵彼此獨立的目錄樹。在同一棵目錄樹中，一個檔案識別碼只能存在於一個位置。

### 不寫入 dist/ 的項目

建置不會把 `components/`、`anchor.json`、`package.json`、Manager Lockfile、`README.md`、`docs/`、`.gitignore`、隱藏 dotfiles 或 `.env*` 環境檔寫入 `dist/`。`dist/` 不屬於 source，建置也不掃描該目錄。

## 不作承諾的事項

- **`file://` 協定**：WDA 承諾 `dist/` 部署在標準靜態 HTTP(S) 主機上時可以運作。WDA 不保證直接以 `file://` 在本機開啟時可以運作。若 `wda build` 偵測到 ES module、`fetch` 或其他已知無法透過 `file://` 運作的功能，會提醒使用者改用 HTTP 主機。沒有收到提醒，不代表 WDA 保證它可以運作。
- **Clean URL**：WDA 不承諾移除檔案副檔名，也不建立巢狀 `index.html` 來模擬前端路由。URL 改寫與備援路由（fallback routing）由部署主機處理。
- 框架遷移限制：相關承諾見「Source 基準與部署基準」。

## 專案文件集合

本節描述 Generated Project 的文件如何分工。`wda check` 如何檢查文件，見 `docs/commands.md` 的「Rule Module: Documents」。

### README.md 是唯一入口

`README.md` 是 Generated Project 唯一的閱讀入口。其他規格與文件依職責放在 `docs/` 中。

### 四份必要文件

專案有四份必要文件：`README.md`、`docs/architecture.md`、`docs/design.md` 與 `docs/glossary.md`。

- `README.md` 是專案入口，說明專案的目的與背景。
- `docs/architecture.md` 說明網站的目前組成、技術基準，以及各 source 路徑的角色。
- `docs/design.md` 說明專案採用的 Design System 設定。
- `docs/glossary.md` 說明專案術語與命名慣例。

`docs/naming.md` 不是支援的舊格式，驗證器也不提供相容處理。遷移時，專案必須手動將該檔更名為 `docs/glossary.md`。

### development.md 是條件式文件

`docs/development.md` 不是必要文件，`wda init` 也不建立它。只有當專案有四個 WDA 命令與四份必要文件都未涵蓋的特殊工作流程時，才建立此檔。例如專案專屬的開發命令、工具前置條件、自動化流程或人工驗收步驟。`anchor.json` 是瀏覽器基準與無障礙基準的權威來源。專案可以在 `docs/architecture.md` 或 `docs/design.md` 中摘要這兩項基準，因此不需要為此建立此檔。

### architecture.md 範本的兩個區段與零重複原則

`docs/architecture.md` 必須依 `docs/templates/generated-project/architecture.md` 撰寫。範本有兩個主要區段。`Created by wda init` 列出 `wda init` 實際建立的目錄與檔案。`Allowed when needed` 列出選用的延伸路徑，以及何時建立它們。範本也說明 `wda init` 不建立佔位目錄。架構文件不重述 `README.md` 的內容。專案目的與背景只記錄在 `README.md`。
