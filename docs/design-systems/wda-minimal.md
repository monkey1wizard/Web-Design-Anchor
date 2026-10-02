---
title: "WDA Minimal"
status: stable
updated: 2026-09-30
type: Reference
description: "WDA 內建的最小 Design System：定義、token 出處、starter 結構與參數化的 design.md 判準來源。"
tags: []
---

# WDA Minimal

## 定義

`WDA Minimal` 是 WDA 自有的內建 Design System。它也是 WDA 能當真實專案基準的最小參照 Design System。它以專案自有的語意 Design Token、原生語意 HTML 與 Modern CSS 為主要實作。它不需要 vendor 元件庫。它預設採用與 Spectrum 2 共用的 Lit 作為 Custom Element 撰寫標準。

WDA Minimal 小而語意、原生 Web 優先、以專案 token 驅動。它容易理解，適合正式專案，也足以建立 WDA Minimal Starter。它也是驗證用的原生／最小參照。WDA Core、專案結構、token 處理、build 結果與文件都不依附外部 Design System 的 vendor 元件庫或 vendor 專屬的元件執行環境。線上成功初始化時，WDA Minimal 採 best-effort 方式預裝 `lit` 並產生 `deno.jsonc` 與 `deno.lock`。線上成功初始化時，這條路徑不再是零相依（zero-dependency）路徑。線上成功初始化時，WDA Minimal 不包含任何 vendor 元件庫，也不產生 starter script（`scripts/main.ts`）。

WDA Minimal 不是要取代一套完整的大型商業 Design System。它不得只是為了模仿而引入抽象層。

## 在 WDA 的定位

線上解析不到已設定的 Design System 或 Spectrum 2 相依安裝失敗時，WDA Minimal 是固定的內建退回集合。退回集合對同一個 release 固定。它也是參照 Design System 之一，與預設的 latest stable Spectrum 並行。這兩者用來測試不同的相依模型與元件模型。Spectrum 2 包含六個 vendor 元件套件加獨立 Lit。WDA Minimal 僅有 best-effort Lit，無 vendor 元件庫。

WDA Minimal 是架構與離線基準，不是 Designer 第一次啟動時的視覺預設。預設 Design System 仍是 latest stable Spectrum。線上解析成功時，專案記錄它的確切版本。若 Spectrum 2 相依安裝失敗，WDA 依不可分割機制清理已寫入的 Spectrum 專屬檔案與相依設定。若 Spectrum 2 相依安裝失敗，WDA 接著退回建立 WDA Minimal 專案。

當在離線或 npm registry 無法連線的環境下執行 `wda init`，WDA Minimal 的 Lit 安裝採 best-effort 原則。當在離線或 npm registry 無法連線的環境下執行 `wda init`，初始化仍會成功建立專案，且不產生 `deno.jsonc` 與 `deno.lock`。當在離線或 npm registry 無法連線的環境下執行 `wda init`，離線退回差異在 stdout 中清楚可見。當在離線或 npm registry 無法連線的環境下執行 `wda init`，初始化也提供 `wda deps add lit` 復原指令，以確保離線可用性而不削弱 Standard Web 部署產出。

拿 WDA Minimal 與 Spectrum 比較，是為了測試不同的相依模型與元件模型，並驗證 Core 不綁死任何 Design System。這項比較不是自動或無損遷移的承諾。

## Token 出處

token 值是 `@adobe/spectrum-tokens` 15.2.0 的一次凍結快照，不追隨上游更新。快照取自 npm registry 的 `dist-tags.latest`，是 15.2.0 這個確切版本。

複製的是值，不是名稱。token 名稱仍是專案自有的語意名稱，例如 `color.brand.primary`、`spacing.md`。只有數值取自快照。

字型 family token 改為以 Noto 為首的 font stack。Noto 的授權是 SIL OFL 1.1。

版權歸屬記錄在儲存庫根的 `NOTICE`。`NOTICE` 依 Apache-2.0 第 4(d) 節載明上游套件名稱、確切版本、授權與所做的修改。

快照是凍結的，不追隨上游。重新整理是刻意的動作，會讀取新版本並更新 `NOTICE`。

## Token

專案的 canonical token 來源是 `tokens/tokens.json`，格式是 DTCG。`wda init` 會建立 Minimal Starter 需要的語意 token。這個檔案是真實的 Design System 契約狀態，不是佔位符。

專案使用的 token 名稱是語意的。命名空間由專案自己擁有。DTCG alias 或 reference 在專案 token 之間是允許的。專案改用其他 Design System 時，adapter 可以把 vendor 語意對到專案 token 值。

build 把 `tokens/tokens.json` 轉成 CSS Custom Properties，寫進 `dist/styles/tokens.css`。source 端不得有已生成的 `tokens.css`；衍生的 token CSS 只存在 `dist/`。

WDA 不強制兩層的 Primitive Token → Semantic Token 階層。只有專案真的需要 scale、重用或階層時，才加 primitive 或 reference 層。

## Starter 結構

Starter 的來源是內嵌資產，放在 `src/builtins/wda_minimal/`，以 `include_str!` 內嵌進 binary：

- `tokens/tokens.json`：DTCG token
- `pages/index.html`：一頁式 Starter 頁面
- `docs/design.md`：參數化的判準來源

測試用的 fixture 在 `tests/fixtures/wda_minimal_starter/`，內容與內嵌資產位元組相同。

## 參數化的 design.md 判準來源

`src/builtins/wda_minimal/docs/design.md` 是參數化的視覺審查判準來源。判準用 `{{#if_design_system "WDA Minimal"}}` 與 `{{#if_design_system "Spectrum 2"}}` 包住各 Design System 的區塊。render 時只保留對應的區塊。

不屬於任何單一 Design System 的區段（`## Attribution` 與 `## 10. Craft Floor`）位於條件區塊之外，且每次 render 都會保留。

第 10 節「Craft Floor」是最後一節，包含 11 條視覺品質審查判準。第 10 節中，WDA Minimal 與 Spectrum 2 的輸出內容相同。Designer 可以修改或移除任一判準，並在該節記錄變更。這些判準不是 `wda check` 的強制檢查。WDA 只選取並改寫 impeccable 的部分文字，沒有引入其 Skill、指令或相依項目。這份 Apache-2.0 文字快照不追隨上游更新。來源與確切 commit 記錄在儲存庫根的 `NOTICE`。

替換標記有兩個。`{{DESIGN_SYSTEM_NAME}}` 換成 Design System 名稱。`{{ACCESSIBILITY_BASELINE}}` 換成無障礙基準。render 之後不該殘留未展開的 `{{` 或 `}}` 標記。

render 由 `src/builtins/wda_minimal.rs` 的 `render_criteria` 執行，`src/init/documents.rs` 把結果寫成 Generated Project 的 `docs/design.md`。

## Markup 與元件邊界

版面、文字、圖片、按鈕、表單與靜態內容，優先用原生語意 HTML 與 CSS。專案真的需要可重用的行為、可重用的產品語意或封裝時，才用 Web Components。WDA 兩條 Design System 路徑皆預設採用 Lit 作為 Custom Element 的撰寫標準。標準 LitElement 類別宣告與 `customElements.define` 註冊都屬於這項標準。這項標準不使用 decorator 編譯旗標。WDA Minimal 不包含任何 vendor 元件庫，也不產生 starter script（`scripts/main.ts`）。

Lit 是 Web 平台的輕量封裝工具，不綁定特定前端框架。若專案不需要 Lit，可執行 `wda deps remove lit` 移除；若離線建立後續需要或移除後欲恢復，可執行 `wda deps add lit` 復原。

WDA Minimal 不需要 vendor 元件庫或 vendor 專屬的執行環境。專案改用其他 Design System 時，可以使用該 vendor 的公開元素名稱或 API。WDA 不為了藏 vendor 名稱而建立通用的 wrapper。

唯一的 WDA 自訂 build 時組合原語是 `<wda-include src="..."></wda-include>`。它遵循 Custom Element 命名規則。它在 build 時展開，不是 runtime 的 Web Component，也不得留在 `dist/`。WDA Minimal 不定義 template language。它沒有 `<slot>`、條件、迴圈、變數、運算式或資料繫結。

## 使用邊界

本檔案是 WDA Minimal 在 Core 的唯一完整定義。Generated Project 的 `docs/design.md` 只記錄該專案實例化後的選擇。這些選擇包括實際生效的 token 與專案專屬的審查判準。它不複製這份 WDA 層級的定義。

WDA Minimal 是可重用的基準與參照。它不是「零 HTML 修改」「100% 重用」或「自動遷移」的保證。它的遷移目標是降低遷移成本，同時保留專案的語意結構，也保留專案對選擇的所有權。
