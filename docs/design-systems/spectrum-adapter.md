---
title: "Spectrum Adapter 對照"
status: stable
updated: 2026-09-22
type: Reference
description: "Spectrum 2 對照 WDA Adapter 邊界七個 facet 的實例文件。"
tags: []
---

# Spectrum Adapter 對照

本文件是 Spectrum 的實例對照，記錄 Spectrum 2 對到「Adapter 邊界」七個 facet 的每個具體做法。

七個 facet 的定義與原則是 `docs/design-system.md` 的「Adapter 邊界」一節。本文件不重抄，只寫 Spectrum 對到每個 facet 的做法。Spectrum 2 的實作已於 2026-09-07 落地。本文件是已落地對照的紀錄，不是紙上推演。

## 七個 facet 的對照

### 1. Token mapping

專案的 canonical token 保持語意與專案自有，例如 `color.background.primary`、`color.text.body`、`space.medium`。專案的 token 契約不直接嵌入 vendor token 名稱。

canonical 語意 token 對到 Spectrum 2 的設計 token。Spectrum 2 token 在 `<sp-theme system="spectrum-two">` 的範疇內解析，解析結果由該範疇載入的 theme 與 scale module 決定。

`wda build` 產生一層 vendor token 綁定，寫在 `dist/styles/spectrum-tokens.css`。這個檔案把 Spectrum 2 token 綁到專案的語意 custom property，例如 `--wda-color-background-primary`。本文件不定死 Spectrum 2 的屬性名稱。build 時從該範疇載入的 theme module 解析。

### 2. Styles

專案直接用 Spectrum CSS 時，原生標記套用對應的 Spectrum CSS 類別名稱，例如 `.spectrum-Button`、`.spectrum-Textfield`、`.spectrum-Card`。

衍生的 token 綁定（`dist/styles/spectrum-tokens.css`）落在 Spectrum 的 CSS custom property 層，與全域 theme 樣式一起生效。

### 3. Fonts

字型採用 Adobe Clean font stack（`adobe-clean, sans-serif`），加系統字型退回。

web font 資產在 `wda build` 時從 npm 套件內容解析。

### 4. Icons

圖示走 Spectrum Workflow Icons（`@spectrum-css/icon`），以 inline SVG symbol 接入頁面。

icon SVG sprite 在 `wda build` 時從 npm 套件內容解析。

### 5. Theme

主題用 `<sp-theme>` 容器建立範疇。Spectrum 2 需要 `system="spectrum-two"` opt-in：

- `<sp-theme system="spectrum-two" color="light" scale="medium">`
- `<sp-theme system="spectrum-two" color="dark" scale="medium">`

漏帶 `system="spectrum-two"` 會無聲 render 出 Spectrum 1 預設，所以對到本 adapter 時這個屬性必帶。

`data-theme` 與 `color-scheme: light dark` 直接對到 `<sp-theme system="spectrum-two">` 的 `color` 與 `scale`。這個綁定保證 light/dark 切換與高對比無障礙。

### 6. Component dependencies

原生 HTML 對到 Spectrum Web Components：

- `<button>` / CTA → `<sp-button variant="accent">`
- `<input type="text">` → `<sp-textfield label="...">`
- `<select>` → `<sp-picker label="...">`
- `<article>` / card container → `<sp-card heading="...">`
- `<dialog>` → `<sp-dialog>`（在 `<sp-dialog-wrapper>` 內）

custom tag 要在瀏覽器 DOM render 前註冊進 custom element registry（`window.customElements.define('sp-button', SpectrumButton)`）。兩條 Design System 路徑皆預設採用 Lit 作為 Custom Element 撰寫標準。Spectrum 2 下同時具備六個 vendor Web Component 套件。若專案不需要 Lit，可執行 `wda deps remove lit` 移除。需要時透過 `wda deps add lit` 復原。

每個元件對到一個套件相依：`@spectrum-web-components/button`、`@spectrum-web-components/textfield`、`@spectrum-web-components/picker`、`@spectrum-web-components/theme`。`@spectrum-web-components/button` 依 `base`、`icon`、`theme`。adapter manifest 追蹤元件相依樹，確保 build 時解析到全部需要的 custom element 與資產資源。

直接相依集合的每個套件都釘在 `1.0.0` 以上。Spectrum 2 支援從這個下限開始；低於這個下限的套件沒有 `system="spectrum-two"` opt-in。

`<wda-include>` 這類 build 原語在 render 前就靜態展開。Spectrum 元件用 Shadow DOM slot（`slot="label"`、`slot="icon"`）封住內部版面。因此，build 工具與元件內部的 runtime render 解耦。

### 7. Dependency integration

`@spectrum-web-components/*` 套件在同一 release 以同一版本齊步釋出。這是實測觀察，不是 vendor 的契約保證；本 adapter 的單一版本釘選以這個前提為根據。

直接相依集合是消費端（包括 Spectrum 2 starter script）唯一能 import 的 `@spectrum-web-components/*` vendor 集合，共六個套件：

- `@spectrum-web-components/theme`（theme provider、scale modules、color modules）
- `@spectrum-web-components/button`
- `@spectrum-web-components/textfield`
- `@spectrum-web-components/picker`
- `@spectrum-web-components/card`
- `@spectrum-web-components/dialog`

集合外不可 import；只靠傳遞而得的套件，也不在允許範圍。除了這六個 vendor 套件之外，專案亦獨立解析並加入與 WDA Minimal 共用的 `lit` 作為 Custom Element 撰寫標準，共七個直接相依。

相依宣告在專案的 `deno.jsonc` 的 `imports` 欄位中，並鎖定於 `deno.lock`。六個 vendor 套件同時在 `wda.json` 的 vendor 相依 manifest 追蹤（`lit` 不進入 vendor 套件清單），每個 entry 釘到齊步前提下解析出的單一版本，且守住 `1.0.0` 下限：

```json
{
  "designSystem": {
    "name": "Spectrum 2",
    "version": "1.0.0",
    "packages": {
      "@spectrum-web-components/theme": "^1.0.0",
      "@spectrum-web-components/button": "^1.0.0",
      "@spectrum-web-components/textfield": "^1.0.0",
      "@spectrum-web-components/picker": "^1.0.0",
      "@spectrum-web-components/card": "^1.0.0",
      "@spectrum-web-components/dialog": "^1.0.0"
    }
  }
}
```

`wda build` 產生 ES module import map，把 vendor specifier（`import { Button } from '@spectrum-web-components/button'`）解析到本機 node_modules 的 vendor 分發（`/vendor/@spectrum-web-components/button/index.js`）。

theme provider 與 Spectrum 2 module 從 theme 套件的 Spectrum 2 subpath import，不走獨立的 CSS-variables 套件：

- `@spectrum-web-components/theme/sp-theme.js`（`<sp-theme>` provider）
- `@spectrum-web-components/theme/spectrum-two/scale-medium.js`、`scale-large.js`（scale modules）
- `@spectrum-web-components/theme/spectrum-two/theme-light.js`、`theme-dark.js`（color modules）

icon SVG sprite（`@spectrum-css/icon`）與 web font 資產（`adobe-clean`）在 `wda build` 時從 npm 套件內容解析。

外部 Spectrum npm 套件線上解析不到，或七個相依套件任一安裝失敗時，`wda init` 採不可分割（all-or-nothing）機制清除所有已寫入的 Spectrum 專屬檔案（如 `scripts/main.ts`）與相依設定（`deno.jsonc` 與 `deno.lock`），退回建立 release 固定的 `WDA Minimal` 專案。若在離線環境下建置且缺乏快取的 `node_modules`，建置將依鎖定狀態處理。

## 對照摘要表

| Facet | Spectrum 對照 |
| --- | --- |
| Token mapping | `<sp-theme system="spectrum-two">` 範疇內解析 Spectrum 2 token，`wda build` 綁到 `dist/styles/spectrum-tokens.css` |
| Styles | 原生標記套對應的 `.spectrum-*` 類別名稱，衍生 token 綁定落在 Spectrum CSS custom property 層 |
| Fonts | Adobe Clean font stack（`adobe-clean`）加系統退回，web font 資產 build 時從 npm 解析 |
| Icons | Spectrum Workflow Icons（`@spectrum-css/icon`），inline SVG symbol |
| Theme | `<sp-theme system="spectrum-two">`，`data-theme` 與 `color-scheme` 對到 `color` 與 `scale` |
| Component dependencies | 原生標記對到 `@spectrum-web-components/*` custom element，需註冊，六個 vendor 套件加共用 Lit，套件守住 `1.0.0` 下限 |
| Dependency integration | `deno.jsonc`（六個 vendor 套件加獨立 Lit，共七個相依）與 `wda.json` 齊步釘選，ES module import map，npm 資產解析，相依失敗不可分割退回 `WDA Minimal` |
