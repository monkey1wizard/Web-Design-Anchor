---
title: "Generated Project"
status: stable
updated: 2026-10-15
type: Architecture
description: "wda init 所產出專案的契約規格：source 基準、語言分工、專案目錄樹、`wda.json` 欄位、`dist/` 對應，以及專案的文件集合。"
tags: []
---

# Generated Project

`wda init` 產生的專案稱為 Generated Project，屬於 Standard Web 專案（定義見 `docs/glossary.md`）。這份規格說明專案的技術基準、source 職責、目錄結構、`wda.json` 欄位、source 與 `dist/` 的對應規則、支援範圍，以及文件集合。指令行為與驗證規則請見 `docs/commands.md`。

## Source 基準與 Deployment 基準

| 基準 | 內容說明 |
| :--- | :--- |
| Source | HTML5 與語意化 HTML、Modern CSS、TypeScript、原生瀏覽器能力。當專案確實需要元件封裝時採用 Web Components |
| Deployment | HTML、CSS、JavaScript、assets |

前端框架（Framework）不是預設選項。應用程式的實際複雜度決定是否引入框架，單有路由（routing）需求不足以構成理由。WDA 承諾降低日後遷移成本，但不承諾自動、無損或零成本遷移。

## HTML、CSS 與 TypeScript 的職責劃分

- HTML 負責結構與靜態內容。
- CSS 負責視覺呈現、版面配置、響應式設計，以及僅憑 CSS 即可處理的狀態。
- TypeScript 負責互動行為、必要的狀態（state）管理，以及呼叫瀏覽器 API。

TypeScript 必須具備實質的靜態型別檢查（static type-check）。打包工具（bundler）的轉譯（transpile）程序不得視為型別檢查。

## 專案目錄樹

### wda init 建立的目錄樹

`wda init` 僅建立以下路徑：

```text
專案根/
├── .git/                 本機 Git 儲存庫（Repository）
├── .gitignore            內容固定為 /dist/
├── README.md
├── wda.json
├── docs/
│   ├── architecture.md
│   ├── design.md
│   └── glossary.md
├── pages/
│   └── index.html
└── tokens/
    └── tokens.json
```

上述目錄結構是離線初始化或相依解析失敗時使用的 `WDA Minimal` 基本結構。線上初始化成功時，建立內容依 Design System 與相依安裝結果而異，詳見 `docs/commands.md`。

`deno.jsonc` 是 Generated Project 唯一的相依宣告檔，使用 `imports` 欄位列出套件，並由 `deno.lock` 管理外部相依。專案不支援 `deno.json`；`wda build` 使用的嚴格 `deno.json` 僅放在專案外的暫存目錄。

### 視需求建立的目錄路徑

以下路徑只在專案有需要時建立。`wda init` 不建立佔位目錄（placeholder 目錄）。

```text
components/            可重用的靜態 HTML 片段，或具備實質重用性的產品語意元件
styles/                單一樣式表不足以支撐需求時的共用 source 樣式表
scripts/               行為邏輯需要 source 組織劃分時的 TypeScript 互動模組
assets/                專案自備的圖片、字型與圖示等資產
pages/<name>.html      額外的 MPA 頁面進入點
docs/development.md    四個 WDA 指令與必要文件未涵蓋的專案專屬工作流程
```

## `wda.json`

每個 Generated Project 的根目錄都固定包含 `wda.json`。此檔案是專案契約的核心，定義專案識別、技術基準與 Design System。`wda check` 和 `wda build` 都會優先讀取此檔。

### 嚴格 Schema 的理由

`wda.json` 只允許明確定義的欄位，讓人類、AI 與 Core 都能理解檔案內容。若加入自訂內容，其他讀取者便無法判定其意義與用途。

- 不提供 `metadata`，不設自訂擴充區，亦不允許任何任意的頂層（top-level）欄位。
- 出現未知欄位一律視為 Error，而非 Warning。
- 檔案內容本身不包含 `$schema` 欄位。
- 不使用 `projectFormatVersion`、`deploymentDirectory`、`projectDescription` 此三個過時或非契約名稱。

### 欄位

| 欄位 | 內容說明 | 寫入時機與權限 |
| :--- | :--- | :--- |
| `projectName` | 預設為 `Web Design Anchor Project`。用於組成網頁標題 | 由 `init` 寫入預設值，後續由專案自行修改 |
| `projectVersion` | `init` 執行當下的主機本機時間，包含秒與 UTC 偏移量（offset），不含毫秒。後續不自動遞增 | 僅由 `init` 寫入一次 |
| `wdaVersion` | 最近一次成功執行 `init` 或 `build` 時使用的 WDA 版本 | 由 `init` 寫入，並於每次 `build` 成功時更新 |
| `pageArchitecture` | 可用值為 `mpa`（多頁式架構）與 `spa`（單頁式架構）；預設為 `mpa` | 由 `init` 寫入預設值，後續由專案自行修改 |
| `browserBaseline` | 唯一允許值為 `baseline-widely-available`（Baseline Widely Available），也是預設值 | 僅由 `init` 寫入 |
| `accessibilityBaseline` | 唯一允許值為 `wcag-2.2-aa`（WCAG 2.2 AA），也是預設值 | 僅由 `init` 寫入 |
| `designSystem` | 預設為 latest stable Spectrum。線上解析成功時，`version` 為確切的 semver 版本號碼；退回 `WDA Minimal` 時，`version` 則為該 WDA release 內建的版本 | 僅由 `init` 寫入 |

各欄位是否允許由專案自行修改，取決於其穩定度分層，詳見 `docs/architecture.md` 的「穩定度分層」一節。

### Schema 的存放形態

權威 Schema（canonical schema）是版本控管中的 `schemas/wda.schema.json`。binary 直接內嵌同一份內容，不會在儲存庫提交自動產生的複本。binary 也會驗證 JSON Schema 無法表達的跨欄位規則，不重複定義 Schema 已宣告的欄位。系統支援舊格式時，才新增舊版 Schema。

### wdaVersion 的更新規則

`wdaVersion` 記錄「最後一次成功處理此專案的 WDA 版本」，不是專案格式版本。不可用此欄位判定格式相容性。格式相容規則見 `docs/architecture.md` 的「版本相容性」一節。

| 指令執行結果 | `wdaVersion` 處理方式 |
| :--- | :--- |
| `wda init` 成功 | 寫入 |
| `wda build` 成功 | 更新 |
| `wda build` 失敗 | 維持原狀 |
| `wda check` | 維持原狀 |
| `wda deps` | 維持原狀 |

## `dist/`

### dist/ 目錄樹

`dist/` 是唯一的部署產物目錄。以下範例展示建置後的目錄結構：

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

### source 至 dist/ 的對應規則

source 到 `dist/` 的路徑對應固定，不提供設定選項：

| Source 路徑 | `dist/` 對應路徑 | 規範說明 |
| :--- | :--- | :--- |
| `pages/<rel>.html` | `dist/<rel>.html` | 保留副檔名，不建立巢狀 `index.html` 目錄 |
| `styles/<rel>.css` | `dist/styles/<rel>.css` | 直接對應輸出 |
| `scripts/<rel>.ts` | `dist/scripts/<rel>.js` | 先執行型別檢查（type-check），再轉譯 |
| `assets/<rel>` | `dist/assets/<rel>` | 只複製頁面或樣式表實際參照的檔案 |
| `tokens/tokens.json` | `dist/styles/tokens.css` | 於建置（build）期間轉譯產出 |

### dist/ 的不變條件

- `dist/` 為固定的輸出目錄名稱，不提供自訂設定。
- 每次建置都從 source 全新重建，不保留前次產物。
- `dist/` 必須能在沒有 WDA、source 原始程式碼或開發工具的環境中獨立部署並運作。
- 衍生產物只能放在 `dist/`，不可寫回 source。例如 `tokens/tokens.json` 轉譯出的 CSS 只能放在 `dist/styles/tokens.css`，source 不得包含 `styles/tokens.css`。
- Source 端與 `dist/` 為兩棵彼此獨立的目錄樹。同一檔案識別名稱在同一目錄樹中僅能存在於單一位置。

### 不會輸出至 dist/ 的項目

建置流程不會將 `components/`、`wda.json`、`README.md`、`docs/`、`.gitignore`、點開頭的隱藏檔案（dotfile）或 `.env*` 環境設定檔輸出至 `dist/`。`dist/` 不屬於 source，建置流程也不會掃描該目錄。

## 非承諾事項

- **`file://` 協定**：WDA 承諾 `dist/` 部署在標準靜態 HTTP(S) 主機時可正常運作，但不保證透過 `file://` 在本機直接開啟時能正常執行。如果 `wda build` 偵測到 ES module、`fetch` 或其他已知無法透過 `file://` 運作的特徵，就會提醒使用者改用 HTTP host。沒有收到提醒也不代表 WDA 保證能運作。
- **簡潔網址（Clean URL）**：WDA 不承諾移除副檔名，也不會建立巢狀 `index.html` 來模擬前端路由。URL 重寫（rewrite）與備援路由（fallback routing）由部署主機處理。
- 框架遷移限制：相關承諾見「Source 基準與 Deployment 基準」一節。

## 專案的文件集合

本節說明 Generated Project 的文件分工。`wda check` 如何檢查文件，請見 `docs/commands.md` 的「規則模組：文件」一節。

### README.md 為唯一進入點

`README.md` 是 Generated Project 的唯一閱讀入口。其他規格與文件依職責放在 `docs/`。

### 四份必要文件

專案有四份必要文件：`README.md`、`docs/architecture.md`、`docs/design.md`、`docs/glossary.md`。

- `README.md` 是專案入口，說明專案目的與背景。
- `docs/architecture.md` 說明網站目前的組成、技術基準，以及各 source 路徑的職責。
- `docs/design.md` 說明專案採用的 Design System 設定。
- `docs/glossary.md` 說明專案術語與命名慣例。

`docs/naming.md` 不屬於支援的舊格式，驗證器不提供相容處理。遷移時，專案必須手動將檔案改名為 `docs/glossary.md`。

### development.md 為條件式文件

`docs/development.md` 不是必要文件，`wda init` 不會建立此檔。只有當專案有 WDA 四個指令與四份必要文件都未涵蓋的特殊工作流程時，才需要建立此檔，例如專案專用的開發指令、工具前置條件、自動化流程或人工驗收步驟。`wda.json` 是瀏覽器與無障礙基準的權威來源。專案可在 `docs/architecture.md` 或 `docs/design.md` 摘要說明這兩項基準，但不必因此建立此檔。

### architecture.md 範本的兩大章節與零冗餘原則

`docs/architecture.md` 必須依據 `docs/templates/generated-project/architecture.md` 起稿。範本有兩個主要章節：`Created by wda init` 列出 `wda init` 實際建立的目錄與檔案，`Allowed when needed` 列出選用的擴充路徑與建立時機。範本也說明 `wda init` 不會建立佔位目錄（placeholder 目錄）。Architecture 文件不重述 `README.md` 的內容，專案目的與背景只記在 `README.md`。
