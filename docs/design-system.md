---
title: "Design System 與 Tokens"
status: stable
updated: 2026-09-30
type: Architecture
description: "Design System 預設與退回、token 來源與命名、adapter 邊界七個 facet、身分分支介面與中立性定義。"
tags: []
---

# Design System 與 Tokens

本文件為 Design System 範疇的契約規格：通用規則定義於本文件中，具體實例則載於各獨立文件。內文定義預設值與退回機制、Token 規範、Adapter 邊界的七個面向（facet）、身分分支的允許位置，以及中立性原則。各別 Design System 如何對應至這七個 facet，屬於實例文件之範疇。`WDA Minimal` 詳見 `docs/design-systems/wda-minimal.md`，Spectrum 詳見 `docs/design-systems/spectrum-adapter.md`。本文件不重複贅述該兩份文件的內容。

## 預設與退回

### 預設是 latest stable Spectrum

`wda init` 於線上查詢 npm registry，取得 `@spectrum-web-components/bundle` 的 `dist-tags.latest`。版本必須是 1.0.0 以上的正式 semver。查詢成功時將確切版本寫入 `wda.json`。starter script 實際 import 的六個直接相依套件固定於同一版本，外加獨立解析的 `lit`（共七個相依套件宣告於 `deno.jsonc` 並鎖定於 `deno.lock`）。

### 退回 WDA Minimal 的條件

若發生以下任一情況，`wda init` 將退回使用 `WDA Minimal`：

- `deno` 未加入 PATH 環境變數
- 查詢指令結束狀態碼非零
- 查詢逾時
- 查詢輸出無法解析
- 輸出缺少必要欄位
- 所取得之版本非正式版本（pre-release）
- 所取得之版本低於 1.0.0
- 相依套件安裝失敗

`wda init` 針對各原因提供相應的提示訊息，於 stdout 清楚說明實際採用的 Design System 與退回原因。退回時採不可分割機制，一併清除已寫入的 Spectrum 2 專屬檔案（如 `scripts/main.ts`）與相依設定（`deno.jsonc` 與 `deno.lock`）。清除後再重新建立 `WDA Minimal` 專案。若 `WDA Minimal` 在離線或 registry 不可用時建立，其 Lit 安裝採 best-effort 原則：初始化仍會成功產出專案，並於 stdout 提示 Lit 未安裝及提供 `wda deps add lit` 復原指令。

### 告知義務

由於 `wda init` 採非互動式執行，Designer 無法在執行過程中察覺退回事件。因此，主動告知為 Core 的基本責任，必須明確記錄於 `init` 的輸出訊息中，不得依賴 Skill 於事後補救。

## Token

### canonical source 是 tokens/tokens.json

Token 的唯一權威來源（canonical source）為 `tokens/tokens.json`，採用 DTCG 格式。建置時將其轉換為 CSS 自訂屬性（CSS Custom Properties），並寫入 `dist/styles/tokens.css`。

### 衍生 CSS 只在 dist/

原始程式碼目錄（source）端不得包含 `styles/tokens.css`。該檔案屬於衍生產出物，僅能存在於 `dist/`。若 `wda build` 偵測到 source 端存在此檔案，便立即回報 Error 並拒絕建置。

### 命名

Token 採語意化命名，命名空間（namespace）由專案獨立擁有。允許使用 DTCG 別名（alias）。但專案的權威 token 絕不直接綁定特定第三方廠商（vendor）的 token 名稱。

### 不強制 primitive layer

不強制區分為基底層（primitive）與語意層（semantic）。唯有專案確實需要擴充規模、元件重用或階層架構時方行劃分。

## Adapter 邊界

Adapter 定義「Design System 如何接入 WDA 專案」的介面規範。它屬於文件化規格，並非 Rust trait，亦非外掛註冊表（plugin registry）。整套介面由七個面向（facet）組成。任一 Design System 若欲接入 WDA，皆須針對此七個 facet 逐一提供規格對照。以下列出七個 facet 的定義：

設計原則優先採用原生 HTML 與 CSS。第三方廠商（vendor）公開的元素名稱與 API 皆屬合法用法。WDA 絕不為了隱藏 vendor 名稱而建立通用的封裝層（wrapper）。專案自訂的 wrapper 僅在賦予產品語意、可重用行為或元件封裝時方屬合理。

### 1. Token mapping

此規範定義專案權威 DTCG 語意 token（`tokens/tokens.json`）如何對應至 Design System 的 token 定義。

- `WDA Minimal`：直接採用專案自有的 DTCG 語意 token，中間不增設 vendor 抽象層。Token 別名（alias）僅於 `tokens.json` 內部解析。專案權威 token 皆採語意命名，例如 `color.background.surface`、`spacing.md`。
- Spectrum：專案權威 DTCG 語意 token 對應至 Spectrum 設計 token，例如對應至 `--spectrum-gray-100` 或 `--spectrum-background-base-color`。

### 2. Styles

規範 CSS 的發布與組織規則：包含基準重設（baseline reset）、版面配置系統與全域樣式的編譯方式。

- `WDA Minimal`：建置時將 `tokens/tokens.json` 轉譯為 `dist/styles/tokens.css`。專案樣式表採用原生 CSS 基準重設與 Modern CSS 版面規則，不使用前置處理器（preprocessor），亦不使用前端框架。
- Spectrum：引入 Spectrum CSS 樣式表與套件，例如 `@spectrum-css/page`、`@spectrum-css/tokens`。衍生的 token 變數直接對應 Spectrum 的 CSS 自訂屬性層與全域佈景主題（theme）樣式。

### 3. Fonts

規範字型設定規則：包含字型家族（font family）、字型堆疊（font stack）、行高、字級比例與備援退回策略。

- `WDA Minimal`：採用 Noto-first 字型堆疊（`'Noto Sans', system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif`）。若裝置已安裝 Noto Sans 才會顯示，否則退回系統字型。不載入 web font，因此不會產生網路請求。
- Spectrum：採用 Adobe Clean 字型堆疊（`adobe-clean, sans-serif`）搭配系統字型退回機制。可藉由 `@font-face` 宣告，或藉由 npm 與 web font 發布方式載入字型資源。

### 4. Icons

規範圖示資源的處理規則：包含資源打包方式、SVG 內嵌（inline）或精靈圖（sprite）處理機制，以及命名規範。

- `WDA Minimal`：採用原生 inline SVG 或本機簡易 SVG 資源，不需引用圖示 Web Component、圖示字型（icon font）或外部圖示庫。
- Spectrum：採用 Spectrum Workflow Icons（`@spectrum-css/icon` 或 `@adobe/spectrum-css-workflow-icons`），以 inline SVG symbol 或 Spectrum 的自訂圖示元素接入。

### 5. Theme

規範色彩配置（color scheme）切換規則：包含淺色、深色、高對比模式，以及 theme 屬性與容器範疇（container scope）。

- `WDA Minimal`：由標準 CSS `color-scheme` 屬性與 `data-theme` 屬性控制，屬性值為 `data-theme="light"` 或 `data-theme="dark"`。
- Spectrum：採用 Spectrum 的 theme 容器與屬性，例如 `data-theme`、`color-scheme`，以及 Spectrum 2 的 `sp-theme` 屬性：`system="spectrum-two"`、`color="light"`／`color="dark"`、`scale="medium"`。

### 6. Component dependencies

規範元件模型規則：包含可使用的 UI 元件、自訂元素（custom element），以及原生 HTML 標記（markup）之要求。

- `WDA Minimal`：優先採用原生語意化 HTML 元素（`<header>`、`<main>`、`<nav>`、`<article>`、`<button>`）。無第三方 vendor 元件庫，亦不產生 starter script；兩條路徑皆預設以 Lit 作為 Custom Element 撰寫標準（採 best-effort 安裝）。專案若不需要 Lit，可透過 `wda deps remove lit` 移除，需要時透過 `wda deps add lit` 復原。
- Spectrum：採用 Adobe Spectrum Web Components（`@spectrum-web-components/*`）或 Spectrum CSS 標記，例如 `<sp-button>`、`<sp-textfield>`、`<sp-theme>`。需進行 Web Component 註冊，包含六個 vendor 元件套件相依，並同樣以獨立解析的 Lit 作為 custom element 基礎。

### 7. Dependency integration

規範外部相依套件如何整合至專案生命週期：包含套件管理、建置時的資源解析、vendor import map 與離線退回機制。

- `WDA Minimal`：無 vendor 元件庫套件相依。預設採 best-effort 安裝 `lit`（宣告於 `deno.jsonc` `imports` 並鎖定於 `deno.lock`）；在離線或 registry 不可用時仍可成功建立專案（無相依檔案與 starter script），退回差異可見且可透過 `wda deps add lit` 復原，不削弱 Standard Web 部署產出。資源樣板隨 release 版本固定並內嵌於 WDA binary（`src/builtins/wda_minimal/`），作為離線時的內建退回集合。
- Spectrum：採用 Deno 原生相依宣告（`deno.jsonc` 的 `imports` 與 `deno.lock`），包含六個固定於同一版本的 vendor 元件套件以及獨立解析的 `lit`（共七個直接相依套件）、`wda.json` 中的 vendor 相依清單，以及建置時的 ES module import map。相依解析或安裝失敗時採不可分割（all-or-nothing）機制退回 `WDA Minimal`。

| Facet | `WDA Minimal` | Spectrum |
| --- | --- | --- |
| Token mapping | 直接採用專案 DTCG 語意 token | 專案 DTCG token 對應至 Spectrum token 別名 |
| Styles | 衍生 `dist/styles/tokens.css` 搭配原生 Modern CSS 重設 | Spectrum CSS（`@spectrum-css/*`）樣式套件 |
| Fonts | Noto-first 字型堆疊（`'Noto Sans', system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif`），退回系統字型，不載入 web font | Adobe Clean 字型堆疊搭配系統字型退回 |
| Icons | 原生 inline SVG | Spectrum Workflow Icons（`@spectrum-css/icon`） |
| Theme | `data-theme` 與 `color-scheme` CSS | Spectrum theme 屬性與樣式 |
| Component dependencies | 原生語意化 HTML5 為主，預設共用 Lit，無 vendor 元件庫 | Spectrum Web Components（六個 vendor 套件）加共用 Lit |
| Dependency integration | best-effort Lit（`deno.jsonc`），無 vendor 套件，可離線退回 | 六個 vendor 套件加獨立 Lit（共七個相依於 `deno.jsonc`） |

## 身分分支的允許位置

Core 程式碼中，僅有兩個銜接介面允許進行「這是哪一套 Design System」的判斷或指定系統名稱：

| 銜接介面 | 所在檔案路徑 |
| :--- | :--- |
| 解析銜接層 | `src/init/resolve.rs` |
| 內建資產層 | `src/init/mod.rs`、`src/init/documents.rs`、`src/builtins/wda_minimal.rs`、`src/builtins/spectrum_two.rs` |

上述介面以外的 Core 程式碼一律不得依 Design System 產生邏輯分支。`tests/neutrality.rs` 採用真實的 Rust 語法解析器驗證此規則，先剔除 `cfg(test)` 模組與註解後再行判定。路徑若有異動，必須同步更新該測試案例。

## 中立性的定義

WDA 要求 Core 絕不綁定單一 Design System。實作方式為同時支援兩套具備實質差異的系統。「具備實質差異」的定義如下：兩套系統的相依模型相異（Spectrum 包含六個 vendor 元件套件加獨立 Lit；WDA Minimal 無 vendor 套件，僅預設 best-effort Lit）。元件模型亦相異（Spectrum 使用 vendor custom elements；WDA Minimal 優先使用原生 HTML5，無 vendor 元件庫與 starter script）。兩套系統各自皆能完整走完 `init`、`check`、`build` 三個階段。兩條路徑皆預設以 Lit 作為 Custom Element 撰寫標準，以兼顧元件封裝需求與 Standard Web 中立性。

- `WDA Minimal`：無 vendor 元件庫相依，best-effort 預裝獨立 Lit，純原生 HTML 優先。離線退回時無相依檔案，狀態可見且可復原。
- Spectrum 2：六個 vendor 元件套件加獨立 Lit，vendor custom element。

中立性並非指「兩套系統外觀相同」，亦非「一套系統可自動轉譯為另一套」。專案文件嚴禁宣稱可自動轉換、零成本遷移、無損遷移或零 HTML 變更。

### 已知的代價

`WDA Minimal` 的 token 數值取自 `@adobe/spectrum-tokens` 15.2.0 之單次凍結快照，不主動追蹤上游版本變更。字型改採 Noto 系列。第三方授權與版權歸屬宣告記錄於專案根目錄的 `NOTICE`。因此中立性僅落實了一半：相依模型保持獨立，但設計數值並未獨立。描述中立性時僅能表述為「無 vendor 元件庫相依」。不得宣稱「設計數值獨立於 vendor」。

### 覆蓋範圍的不對稱

`WDA Minimal` 流程在預設的 `cargo test` 中皆可離線執行。Spectrum 2 流程則需仰賴真實的 `deno` 與可連線之 npm registry，標記為 `#[ignore]`，僅於 CI 的非阻斷性探針工作（probe job）中執行。在描述測試覆蓋率時，Spectrum 2 的流程不列入阻擋型檢查閘門。唯一的補償驗證機制為 `src/init/mod.rs` 中的 `spectrum_two_dependency_specs`，該項目由未標記 `#[ignore]` 的測試以嚴格相等斷言把關。
