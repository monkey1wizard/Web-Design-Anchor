---
title: "Design System 與 Tokens"
status: stable
updated: 2026-10-15
type: Architecture
description: "Design System 預設與退回、token 契約、adapter 邊界、中立性原則與實例文件索引。"
tags: []
---

# Design System 與 Tokens

本文件說明 WDA 的 Design System 契約，包括初始化預設與退回、token 規則、adapter 的七個 facet、身分分支邊界及中立性。各系統的實作細節見 [WDA Minimal](design-systems/wda-minimal.md) 與 [Spectrum](design-systems/spectrum-adapter.md)。

## 預設與退回

### 預設為 latest stable Spectrum

`wda init` 線上查詢 npm registry 的 `@spectrum-web-components/bundle` `dist-tags.latest`。版本須為 1.0.0 以上的正式 semver。
查詢成功後，WDA 將確切版本寫入 `wda.json`。接著安裝 `src/init/mod.rs::spectrum_two_dependency_specs` 列出的六個 Spectrum Web Components 套件及 `lit`。

### 退回 WDA Minimal 的條件

遇到下列任一情況，`wda init` 會改用 `WDA Minimal`：

- `deno` 不在 PATH。
- 查詢失敗、逾時、輸出無法解析或缺少必要欄位。
- 查得的版本不是正式版本，或低於 1.0.0。
- 相依套件安裝失敗。

退回時，WDA 會在 stdout 說明採用的系統與原因。接著清除已建立的 Spectrum 專屬檔案及相依設定，再建立 `WDA Minimal` 專案。
`WDA Minimal` 的 Lit 安裝採 best-effort。離線或 registry 不可用時，初始化仍會建立專案，並提示 Lit 未安裝及可用的 `wda deps add lit` 指令。

### 告知義務

`wda init` 為非互動式命令。若發生退回，輸出訊息須說明採用的 Design System 與退回原因。

## Token

### 權威來源

`tokens/tokens.json` 是採 DTCG 格式的唯一權威 token 來源。建置會將其轉為 CSS 自訂屬性，輸出至 `dist/styles/tokens.css`。

### 衍生 CSS

原始程式碼目錄不得包含 `styles/tokens.css`。此檔案是衍生產物，只能存在於 `dist/`。若 `wda build` 在原始程式碼目錄找到該檔案，便會回報錯誤並拒絕建置。

### 命名

Token 使用專案自有的語意命名空間，可以在 `tokens.json` 內使用別名。權威 token 名稱不得直接綁定第三方廠商的 token 名稱。

### Primitive layer

WDA 不要求 token 必須分成 primitive 與 semantic 層。專案可按規模、元件重用或階層需求決定是否分層。

## Adapter 邊界

Adapter 是文件規格，用來說明 Design System 如何接入 WDA。它不是 Rust trait，也不是外掛註冊表。下列七個 facet 定義共同的對照方式。各系統實例見 [WDA Minimal](design-systems/wda-minimal.md) 與 [Spectrum](design-systems/spectrum-adapter.md)。

### 1. Token mapping

說明專案 DTCG 語意 token 如何對應至系統 token。

### 2. Styles

說明基準樣式、版面配置與全域樣式如何組織及建置。

### 3. Fonts

說明字型家族、字型堆疊、行高、字級比例與備援方式。

### 4. Icons

說明圖示資源、打包方式、呈現方式與命名。

### 5. Theme

說明色彩模式與主題範疇如何設定及切換。

### 6. Component dependencies

說明系統使用的元件、自訂元素及原生 HTML 標記。

### 7. Dependency integration

說明外部套件如何納入專案生命週期，以及相依失敗時如何處理。

實作優先使用原生 HTML 與 CSS。使用廠商公開的元素名稱與 API 是允許的。只有在提供產品語意、可重用行為或元件封裝時，專案自訂 wrapper 才有必要。

## 身分分支邊界

Core 只有兩處可判斷或指定 Design System：

| 銜接介面 | 檔案 |
| --- | --- |
| 解析銜接層 | `src/init/resolve.rs` |
| 內建資產層 | `src/init/mod.rs`、`src/init/documents.rs`、`src/builtins/wda_minimal.rs`、`src/builtins/spectrum_two.rs` |

其他 Core 程式碼不得依 Design System 分支。`tests/neutrality.rs` 以文字掃描檢查允許路徑外的身分判斷與硬編碼名稱。掃描會略過註解，並以大括號配對找出及略過 `cfg(test)` 模組。若路徑改變，須同步更新測試。

## 中立性

WDA 不綁定單一 Design System，並以 Spectrum 及 `WDA Minimal` 兩種實作驗證這項原則。兩者使用不同的相依與元件模型，且各自支援 `init`、`check` 與 `build`。目前兩條路徑都預設提供 Lit，Spectrum 使用 vendor custom elements，`WDA Minimal` 使用原生 HTML。

`WDA Minimal` 的內嵌資產位於 `src/builtins/wda_minimal/`。該目錄包含設計說明、HTML 起始頁與 DTCG token 檔。這些檔案未設定 `data-theme` 或 `color-scheme`。

系統實例與 facet 對照請見 [WDA Minimal](design-systems/wda-minimal.md) 和 [Spectrum](design-systems/spectrum-adapter.md)。

中立性不代表兩套系統外觀相同，或可自動轉換。文件不得宣稱系統間可以自動轉換、零成本遷移、無損遷移或完全不需修改 HTML。

### 已知取捨

`WDA Minimal` 的 token 數值是從 `@adobe/spectrum-tokens` 15.2.0 擷取的一次性快照，不會隨上游更新。字型採用 Noto 系列。授權與版權資訊見專案根目錄的 `NOTICE`。因此，描述此系統時可說它沒有 vendor 元件庫相依，不應聲稱其設計數值獨立於 vendor。

### 測試範圍

`WDA Minimal` 路徑可在預設 `cargo test` 中離線執行。Spectrum 2 流程依賴可用的 `deno` 與 npm registry，因此標記為 `#[ignore]`，只在 CI 非阻斷性探針工作執行。阻擋型測試則以 `src/init/mod.rs` 的 `spectrum_two_dependency_specs` 嚴格比對直接相依清單。
