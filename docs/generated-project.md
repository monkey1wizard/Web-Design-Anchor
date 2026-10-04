---
title: "Generated Project"
status: stable
updated: 2026-09-22
type: Architecture
description: "wda init 所產出專案的契約規格：source 基準、語言分工、專案目錄樹、`wda.json` 欄位、`dist/` 對應，以及專案的文件集合。"
tags: []
---

# Generated Project

`wda init` 產出的專案稱為 Generated Project，屬於 Standard Web 專案（定義詳見 `docs/glossary.md`）。本文件為此專案的契約規格。本文載明其技術基準、原始程式碼（source）的職責劃分、專案目錄樹與 `dist/` 結構配置、`wda.json` 各欄位定義、source 至 `dist/` 的對應規則、非承諾事項，以及專案的文件集合。執行契約的指令行為與驗證規則詳見 `docs/commands.md`。

## Source 基準與 Deployment 基準

| 基準 | 內容說明 |
| :--- | :--- |
| Source | HTML5 與語意化 HTML、Modern CSS、TypeScript、原生瀏覽器能力。當專案確實需要元件封裝時採用 Web Components |
| Deployment | HTML、CSS、JavaScript、assets |

前端框架（Framework）並非預設選項。是否引入框架，應由應用程式的實際複雜度決定，「需要路由（routing）」本身並非充分理由。WDA 僅承諾降低日後遷移的成本，不承諾自動、無損或零成本的遷移。

## HTML、CSS 與 TypeScript 的職責劃分

- HTML 負責結構與靜態內容。
- CSS 負責視覺呈現、版面配置、響應式設計，以及僅憑 CSS 即可處理的狀態。
- TypeScript 負責互動行為、必要之狀態（state）管理與呼叫瀏覽器 API。

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
│   └── naming.md
├── pages/
│   └── index.html
└── tokens/
    └── tokens.json
```

上述為離線環境或相依解析失敗時退回之 `WDA Minimal` 基本目錄結構路徑。在成功執行線上初始化時：

- `WDA Minimal` 採 best-effort 方式安裝 `lit` 作為 Custom Element 撰寫標準，額外寫入專案相依宣告檔 `deno.jsonc` 與鎖定檔 `deno.lock`，但不建立 `scripts/main.ts`。若於離線環境建立或相依安裝失敗，初始化仍順利完成，但不建立 `deno.jsonc` 與 `deno.lock`，維持上述無外部相依之目錄結構。
- `Spectrum 2` 線上解析成功並完成相依安裝時，額外寫入 `scripts/main.ts`、專案相依宣告檔 `deno.jsonc` 與鎖定檔 `deno.lock`（鎖定六個 Spectrum 2 vendor 套件與獨立解析之 `lit`，共七個相依）。若相依安裝失敗，則採不可分割（all-or-nothing）機制清理已寫入之 Spectrum 專屬檔案與相依設定，退回建立上述 `WDA Minimal` 專案。

現行 Generated Project 僅以 `deno.jsonc` 作為唯一的專案相依宣告檔案（於 `imports` 欄位宣告相依套件），並與 `deno.lock` 共同管理外部相依性。專案端不採用亦不支援 `deno.json` 作為設定檔；`wda build` 內部建置工作區所使用的嚴格 `deno.json` 設定檔僅存在於專案外的暫存目錄中，與專案端檔案完全隔離。

### 視需求建立之目錄路徑

以下路徑僅於專案確實需要時方行建立。`wda init` 不建立任何佔位目錄（placeholder 目錄）。

```text
components/            可重用的靜態 HTML 片段，或具備實質重用性的產品語意元件
styles/                單一樣式表不足以支撐需求時的共用 source 樣式表
scripts/               行為邏輯需要 source 組織劃分時的 TypeScript 互動模組
assets/                專案自備之圖片、字型與圖示等資產
pages/<name>.html      額外的 MPA 頁面進入點
docs/development.md    四個 WDA 指令與必要文件未涵蓋之專案專屬工作流程
```

## `wda.json`

每個 Generated Project 的根目錄皆固定包含 `wda.json`。該檔案為專案的契約核心。它定義專案識別、採用的技術基準以及所使用的 Design System。`wda check` 與 `wda build` 皆優先讀取此檔。

### 嚴格 Schema 的理由

`wda.json` 僅允許已明確定義的欄位。理由在於此檔案必須同時供人類、AI 與 Core 讀取並理解，若任一方於其中置入自訂內容，其餘各方將無法判定該內容之語意與用途。

- 不提供 `metadata`，不設自訂擴充區，亦不允許任何任意的頂層（top-level）欄位。
- 出現未知欄位一律視為 Error，而非 Warning。
- 檔案內容本身不包含 `$schema` 欄位。
- 不使用 `projectFormatVersion`、`deploymentDirectory`、`projectDescription` 此三個過時或非契約名稱。

### 欄位

| 欄位 | 內容說明 | 寫入時機與權限 |
| :--- | :--- | :--- |
| `projectName` | 預設為 `Web Design Anchor Project`。用於組成網頁標題 | 由 `init` 寫入預設值，後續由專案自行修改 |
| `projectVersion` | `init` 執行當下的主機本機時間，包含秒與 UTC 偏移量（offset），不含毫秒。後續不自動遞增 | 僅由 `init` 寫入一次 |
| `wdaVersion` | 最近一次成功執行 `init` 或成功執行 `build` 所使用之 WDA 版本 | 由 `init` 寫入，並於每次 `build` 成功時更新 |
| `pageArchitecture` | 預設為 `mpa`，即多頁式架構（MPA） | 由 `init` 寫入預設值，後續由專案自行修改 |
| `browserBaseline` | 預設為 `baseline-widely-available`，即 Baseline Widely Available | 僅由 `init` 寫入 |
| `accessibilityBaseline` | 預設為 `wcag-2.2-aa`，即 WCAG 2.2 AA | 僅由 `init` 寫入 |
| `designSystem` | 預設為 latest stable Spectrum。線上解析成功時，`version` 為確切的 semver 版本號碼；退回 `WDA Minimal` 時，`version` 則為該 WDA release 內建的版本 | 僅由 `init` 寫入 |

各欄位是否允許由專案自行修改，取決於其穩定度分層，詳見 `docs/architecture.md` 的「穩定度分層」一節。

### Schema 的存放形態

權威 Schema（canonical schema）為納入版本控管的 `schemas/wda.schema.json`。binary 內部直接內嵌同一份檔案內容，不於儲存庫中提交任何自動產出的複本。binary 內部另包含語意驗證機制，但僅補充 JSON Schema 無法表達之跨欄位關聯規則，絕不重複定義 Schema 中已宣告之欄位。唯有在系統實際支援舊格式之情境下，方可新增舊版格式之 Schema。

### wdaVersion 的更新規則

`wdaVersion` 僅記錄「最後一次成功處理此專案的 WDA 版本」。該欄位並非專案格式的版本號碼。嚴禁使用該欄位判定專案格式相容性。格式相容性之處理規則詳見 `docs/architecture.md` 的「版本相容性」一節。

| 指令執行結果 | `wdaVersion` 處理方式 |
| :--- | :--- |
| `wda init` 成功 | 寫入 |
| `wda build` 成功 | 更新 |
| `wda build` 失敗 | 維持原狀 |
| `wda check` | 維持原狀 |
| `wda deps` | 維持原狀 |

## `dist/`

### dist/ 目錄樹

`dist/` 為唯一的部署產物目錄。以下為建置產出之 `dist/` 目錄樹範例：

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

source 至 `dist/` 的路徑對應為固定規則，不提供設定選項：

| Source 路徑 | `dist/` 對應路徑 | 規範說明 |
| :--- | :--- | :--- |
| `pages/<rel>.html` | `dist/<rel>.html` | 保留副檔名，不建立巢狀 `index.html` 目錄 |
| `styles/<rel>.css` | `dist/styles/<rel>.css` | 直接對應輸出 |
| `scripts/<rel>.ts` | `dist/scripts/<rel>.js` | 先執行型別檢查（type-check），再行轉譯 |
| `assets/<rel>` | `dist/assets/<rel>` | 僅複製頁面或樣式表所實際參照之檔案 |
| `tokens/tokens.json` | `dist/styles/tokens.css` | 於建置（build）期間轉譯產出 |

### dist/ 的不變條件

- `dist/` 為固定的輸出目錄名稱，不提供自訂設定。
- 每次建置皆自 source 端進行全新乾淨重建，絕不殘留前次建置之產物。
- `dist/` 必須能在無 WDA、無 source 原始程式碼且無開發工具之環境下獨立部署並正常運作。
- 衍生產出物僅能存在於 `dist/`，絕不寫回 source 端。例如 Token 衍生之 CSS 即為一例：該樣式由 `tokens/tokens.json` 轉譯產出，僅能存在於 `dist/styles/tokens.css`，source 端嚴禁包含 `styles/tokens.css`。
- Source 端與 `dist/` 為兩棵彼此獨立的目錄樹。同一檔案識別名稱在同一目錄樹中僅能存在於單一位置。

### 嚴禁進入 dist/ 之項目

`components/`、`wda.json`、`README.md`、`docs/`、`.gitignore`、任何點開頭的隱藏檔案（dotfile）以及任何 `.env*` 環境設定檔，皆嚴禁輸出至 `dist/`。`dist/` 目錄本身不屬於 source 原始程式碼範疇，建置流程亦不對其進行掃描。

## 非承諾事項

- **`file://` 協定**：僅承諾 `dist/` 部署於標準靜態 HTTP(S) 託管主機時可正常運作，不保證經由 `file://` 協定於本機直接開啟時能正常執行。當 `wda build` 偵測到 ES module、`fetch` 或其他已知無法於 `file://` 運作之特徵時，將主動發出提醒，告知需使用 HTTP host。未出現提醒亦不代表提供運作保證。
- **簡潔網址（Clean URL）**：不承諾移除副檔名之簡潔網址，亦不藉由建立巢狀 `index.html` 模擬前端路由。URL 重寫（rewrite）與備援路由（fallback routing）屬於部署主機環境的責任範疇。
- 框架遷移限制：相關承諾範圍詳見「Source 基準與 Deployment 基準」一節之說明。

## 專案的文件集合

Generated Project 的文件集合規範如下。本節僅界定各份文件之職責邊界；`wda check` 的檢驗方式詳見 `docs/commands.md` 的「規則模組：文件」一節。

### README.md 為唯一進入點

Generated Project 以 `README.md` 為唯一閱讀進入點，其餘規格與文件則依據職責劃分歸放於 `docs/` 目錄。

### 四份必要文件

專案必要文件共有四份：`README.md`、`docs/architecture.md`、`docs/design.md`、`docs/naming.md`。

- `README.md` 為專案進入點，記錄專案目的與背景資訊。
- `docs/architecture.md` 記錄網站目前的組成結構、技術基準以及各 source 路徑之職責。
- `docs/design.md` 記錄專案實體化所選用之 Design System 配置。
- `docs/naming.md` 記錄專案命名慣例與規範。

### development.md 為條件式文件

`docs/development.md` 並非必要文件，`wda init` 絕不主動建立此檔。唯有當專案確實存在 WDA 四個指令與四份必要文件皆未涵蓋之特殊工作流程時，方行建立；例如專案專屬之開發指令、工具前置相依條件、自動化流程或人工驗收步驟。瀏覽器基準與無障礙基準以 `wda.json` 為權威來源，摘要記載於 `docs/architecture.md` 或 `docs/design.md`，此兩項基準本身不會觸發建立此文件。

### architecture.md 範本之兩大章節與零冗餘原則

`docs/architecture.md` 必須依據範本起稿，範本路徑為 `docs/templates/generated-project/architecture.md`。範本由兩個主要章節構成：`Created by wda init` 僅列出 `wda init` 實際建立之目錄與檔案路徑；`Allowed when needed` 則列出選用之擴充路徑及建立時機。範本並明確載明 `wda init` 絕不建立任何佔位目錄（placeholder 目錄）。本文件嚴禁重複贅述 `README.md` 之內容：專案目的與背景僅能記載於 `README.md`。
