---
title: "採用零冗餘的精簡架構文件標準（Lean Architecture Document Specification）"
status: deprecated
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-021"
description: "What standard keeps the architecture document lean and non-redundant?"
tags: []
type: ADR
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：專案遵循 Documentation-Driven Development 原則，`README.md` 已擔任專案唯一入口並涵蓋專案目的與背景。若 `architecture.md` 包含過多前言與重複性介紹，會造成資訊多處維護的負擔。  
* **決策**：`docs/architecture.md` 移除系統概觀與關係人等冗餘章節，直接聚焦於純技術規範：  
  * 1\. Technical Baselines（雙 Baseline 與 Framework-neutral 輸出原則）  
  * 2\. Directory Boundaries（目錄邊界與責任約束矩陣）  
  * 3\. Build & Pipeline（雙原生工具鏈轉換流程）  
  * 4\. Component & Interaction Model（HTML Partials 與 Web Components 邊界）  
  * 5\. Design System Adapter Boundary（DTCG Token 與語意 CSS 適配）  
  * 6\. Framework Escalation Path（工程師框架升級與遷移邊界）  
  * 7\. ADR Index（關鍵決策索引）  
* **影響**：徹底消除與 `README.md` 的資訊重複，降低專案維護成本，並大幅提升 AI Agent 與工程師查閱系統規範的精確度。

---

# **5\. docs/architecture.md 標準範本 (Reference Template)**

```
# Architecture

> 描述「網站現在怎麼組成」、技術基準與模組責任邊界。

---

## 1. Technical Baselines

| 基準 | 技術範疇 | 規範與原則 |
| :--- | :--- | :--- |
| **Source Baseline** | HTML5, Modern CSS, TypeScript, HTML Partials (<include>), Web Components | 語意結構優先、邏輯與 DOM 分離、響應式優先（Container Queries, Grid, Flexbox）。 |
| **Deployment Baseline** | 純 HTML, CSS, JavaScript, assets | 零外部 Runtime、無框架依賴、原生瀏覽器直接執行。 |

### Framework-neutral 輸出原則
1. **HTML 負責結構**：靜態內容與 semantic markup 留在 HTML，禁止用 TS 大量生成 DOM。
2. **CSS 負責呈現**：Layout、響應式、狀態與視覺效果留在 CSS，使用語意化 class。
3. **TypeScript 負責行為**：僅處理 interaction、狀態與 Browser APIs。

---

## 2. Directory Boundaries

project/
├── pages/        # 頁面入口（支援 Clean URLs，例如 pages/about.html 輸出為 dist/about/index.html）
├── components/   # 靜態 HTML Partials（透過 <include src="..."> 與 <slot> 展開）
├── styles/       # 樣式（main.css 與由 tokens.json 生成的 tokens.css）
├── scripts/      # TypeScript 互動邏輯（main.ts 與 vendor/ 快取的標準 ESM 模組）
├── tokens/       # W3C DTCG 標準格式 tokens.json
└── assets/       # 靜態圖片、字型與圖標

### 責任約束矩陣
- **`pages/`**：只定義單頁結構與 SEO Meta，不直接寫複雜 CSS/JS。
- **`components/`**：只放可複用片段，不引入特定框架專屬標籤。
- **`styles/`**：只使用語意 Class（如 `.btn--primary`）與 CSS Custom Properties，禁止深層脆弱選擇器。
- **`scripts/`**：獨立模組或 Island-style 掛載，不建立全域 Runtime，禁止直接在 TS 內寫死 HTML。

---

## 3. Build & Pipeline

建置由 Rust 原生工具鏈協調，無 Node.js 執行期負擔：

tokens/tokens.json   ──> LightningCSS ──> dist/styles/tokens.css
pages/ + components/ ──> Partials展開  ──> dist/**/*.html (Clean URLs)
scripts/main.ts      ──> esbuild (bin) ──> dist/scripts/main.js
assets/              ──> 資產同步     ──> dist/assets/

---

## 4. Component & Interaction Model

- **原生 HTML 優先**：一般文字、排版、卡片、按鈕一律使用純 HTML + 語意 CSS。
- **Build-time Partials**：共用導覽列、頁尾採用 <include src="components/nav.html"></include> 靜態展開。
- **Web Components**：僅在需要封裝複雜互動或引入官方 Lit 元件時使用 Custom Elements。
- **互動腳本掛載**：採用標準 Web Events（click, input, CustomEvent），不建立 SPA Router。

---

## 5. Design System Adapter Boundary

- **Token 映射**：tokens/tokens.json 採 W3C DTCG 標準，編譯為 CSS Variables。
- **語意樣式層**：HTML 標籤一律使用中立語意 Class，更換 Design System 時 0 行 HTML 修改。
- **Vendor 依賴**：官方 Lit / WC 模組以標準 ESM 存放於 scripts/vendor/ 並透過 importmap 映射。

---

## 6. Framework Escalation Path

當業務複雜度超出純靜態網頁範圍時，工程師導入框架（Astro / React / Vue）的資產保留邊界：

- **100% 保留重用**：Semantic HTML、CSS Tokens / Styles、Web Components、靜態資產與 /docs。
- **工程師替換**：將 <include> 轉換為框架 Template/JSX，將 TS 邏輯對接框架 State 管理。

---

## 7. Architecture Decision Records (ADR) Index

關鍵架構決策脈絡請查閱對應 ADR：
- [ADR-0001: 核心工具選用 Rust Native CLI](adr/0001-rust-core-cli.md)
- [ADR-0006: 選用免 Node.js 原生工具鏈](adr/0006-standalone-native-toolchain.md)
- [ADR-0007: 採用 Build-time HTML Partials 機制](adr/0007-build-time-html-partials.md)
- [ADR-0008: 分層式 Design System 適配模型](adr/0008-layered-design-system-adapter.md)
- [ADR-0011: 雙原生建置引擎（LightningCSS + esbuild Standalone）](adr/0011-dual-native-engine.md)
- [ADR-0014: 採用 W3C DTCG Token 標準格式](adr/0014-dtcg-token-format.md)
```

---

## Current status

Superseded by the current Core documentation contract. The historical template remains frozen here; the active Generated Project architecture template is maintained separately.
