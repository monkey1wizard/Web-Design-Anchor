---
title: "WDA Minimal"
status: stable
updated: 2026-11-12
type: Reference
description: "WDA 內建的最小 Design System，說明定義、token 出處、starter 結構與 design.md 判準來源。"
tags: []
source: docs/design-systems/wda-minimal.md
lang: zh-TW
source_sha256: ea3928d64958d74f40926d431e835bd14c44e322991f778f5c8af37d3701ea5c
translated_at: 2026-11-12
---

# WDA Minimal

## 定義

WDA Minimal 是 WDA 內建的最小 Design System，也是 WDA 驗證真實專案基準的參照。它使用專案自有的語意 Design Token、原生語意 HTML 與 Modern CSS，不依賴 vendor 元件庫。它採用與 Spectrum 2 共用的 Lit，作為 Custom Element 的撰寫標準。

WDA Minimal 以小型、語意化、原生 Web 與專案 token 為核心，可用於正式專案，也可建立 WDA Minimal Starter。WDA Core、專案結構、token 處理、build 結果與文件，都不依賴外部 Design System 的 vendor 元件庫或專屬執行環境。

初始化時，WDA Minimal 專案沒有網站套件相依。其 `package.json` 的 `dependencies` 為空物件，也沒有 `packageManager` 欄位。在第一次外部安裝前，不會有套件管理器的 lockfile。WDA Minimal 不含 vendor 元件庫，也不產生 starter script（`scripts/main.ts`）。

WDA Minimal 不取代完整的大型商業 Design System，也不為了模仿其他系統而加入抽象層。

## 在 WDA 的定位

線上找不到已設定的 Design System，或 Spectrum 2 相依安裝失敗時，WDA 會退回固定的 WDA Minimal 內建集合。同一個 release 使用相同的集合。WDA Minimal 也是參照 Design System，與預設的 latest stable Spectrum 並行，用來測試不同的相依模型與元件模型。Spectrum 2 包含六個 vendor 元件套件與獨立的 Lit；WDA Minimal 初始化時沒有網站套件相依，也不含 vendor 元件庫。

WDA Minimal 是架構與離線基準，不是 Designer 首次啟動時的視覺預設。預設 Design System 仍是 latest stable Spectrum。線上解析成功時，專案會記錄確切版本。預設套件解析或安裝失敗時，WDA 會清除本次執行已寫入的 Spectrum 專屬檔案與相依設定，再建立 WDA Minimal 專案。

若執行 `wda init` 時離線，或沒有可用的受支援套件管理器，初始化會建立完整的 WDA Minimal 專案，且沒有網站套件相依。本機套件快取不會讓初始化改用 Spectrum 2。`anchor.json` 會在 `initialization.fallback` 下記錄原本預定的 Design System（Spectrum 2）與退回原因。stdout 會回報實際結果與套件管理器給出的原因，並指向 WDA Skill，說明之後可以再加入預設套件。Skill 只會在使用者同意後加入，不會重新執行 `wda init`。這讓專案保有離線可用性，也不會削弱 Standard Web 部署產出。

WDA 以 WDA Minimal 和 Spectrum 的比較測試相依模型與元件模型，並確認 Core 不受特定 Design System 綁定。這項比較不代表自動或無損遷移的承諾。

## Token 出處

token 值是從 `@adobe/spectrum-tokens` 15.2.0 凍結的快照，不會追蹤上游更新。快照依據 npm registry 的 `dist-tags.latest`，版本為 15.2.0。

WDA 複製的是 token 值，不是名稱。名稱仍由專案定義為語意名稱，例如 `color.brand.primary` 和 `spacing.md`。

字型 family token 使用 Noto 開頭的 font stack。Noto 採用 SIL OFL 1.1 授權。

儲存庫根目錄的 `NOTICE` 記錄版權歸屬，並依 Apache-2.0 第 4(d) 節列出上游套件名稱、確切版本、授權與修改內容。

快照不會自動更新。重新整理快照時，必須讀取新版本並更新 `NOTICE`。

## Token

`tokens/tokens.json` 是專案權威 token 的 DTCG 來源。`wda init` 會建立 Minimal Starter 所需的語意 token。這份檔案代表 Design System 的實際契約，不是佔位內容。

專案自行擁有語意 token 名稱與命名空間。專案 token 之間可以使用 DTCG alias 或 reference。若專案改用其他 Design System，adapter 可以將 vendor 語意對應到專案 token 值。

build 會將 `tokens/tokens.json` 轉成 CSS Custom Properties，並寫入 `dist/styles/tokens.css`。source 不得包含已生成的 `tokens.css`，衍生的 token CSS 只會出現在 `dist/`。

WDA 不要求 token 一定分成 Primitive Token 與 Semantic Token 兩層。專案有 scale、重用或階層需求時，再加入 primitive 或 reference 層。

## Starter 結構

WDA Minimal Starter 的來源是 `src/builtins/wda_minimal/` 內嵌資產，並以 `include_str!` 編入 binary：

- `tokens/tokens.json`：DTCG token。
- `pages/index.html`：單頁 Starter。
- `docs/design.md`：參數化的視覺審查判準來源。

測試 fixture 位於 `tests/fixtures/wda_minimal_starter/`。上述三個檔案與內嵌資產逐位元組相同。Fixture 另含 `README.md`、`anchor.json`、`package.json`、`docs/architecture.md` 和 `docs/glossary.md`。

Fixture 中的 `anchor.json` 與 `package.json` 是離線 Minimal 專案的 golden manifest。`package.json` 必須與生成結果逐位元組相同。`anchor.json` 則必須與生成結果相同，但不含 `projectVersion`、`wdaVersion`、Design System 的 `version` 與退回的 `reason`，因為這些值取決於執行當下與本機環境。

## 參數化的 design.md 判準來源

`src/builtins/wda_minimal/docs/design.md` 是參數化的視覺審查判準來源。檔案以 `{{#if_design_system "WDA Minimal"}}` 和 `{{#if_design_system "Spectrum 2"}}` 標記各 Design System 的區塊。render 時只保留符合條件的區塊。

`## Attribution` 和 `## 10. Craft Floor` 不屬於單一 Design System，位於條件區塊之外，因此每次 render 都會保留。

第 10 節「Craft Floor」是最後一節，含有 11 條視覺品質審查判準。WDA Minimal 與 Spectrum 2 的第 10 節內容相同。Designer 可以修改或移除任何判準，並在該節記錄變更。這些判準不是 `wda check` 的強制檢查。

WDA 只選取並改寫 impeccable 的部分文字，沒有引入其 Skill、指令或相依項目。這份 Apache-2.0 文字快照不會追蹤上游更新。`NOTICE` 記錄來源與確切 commit。

判準有兩個替換標記：`{{DESIGN_SYSTEM_NAME}}` 會換成 Design System 名稱，`{{ACCESSIBILITY_BASELINE}}` 會換成無障礙基準。render 後不應留下未展開的 `{{` 或 `}}` 標記。

`src/builtins/wda_minimal.rs` 的 `render_criteria` 負責 render。`src/init/documents.rs` 會把結果寫入 Generated Project 的 `docs/design.md`。

## Markup 與元件邊界

版面、文字、圖片、按鈕、表單與靜態內容優先使用原生語意 HTML 和 CSS。只有在專案需要重用行為、產品語意或封裝時，才使用 Web Components。WDA 的兩條 Design System 路徑都採用 Lit。Lit 是撰寫 Custom Element 的標準。

標準 LitElement 類別宣告和 `customElements.define` 註冊都符合這項標準。這項標準不使用 decorator 編譯旗標。WDA Minimal 不含 vendor 元件庫，也不產生 starter script（`scripts/main.ts`）。

Lit 是不綁定特定前端框架的 Web 平台輕量封裝工具。專案要加入或移除 Lit 時，使用外部套件管理器，例如 `npm install lit` 或 `bun add lit`。WDA 沒有 `wda deps` 指令。Minimal 專案之後需要 Lit 時，WDA Skill 可以在使用者同意後加入所需套件。只加入 Lit 時，Design System 與退回記錄保持不變。Spectrum 整合須遵循 [Design System 與 Tokens](../../design-system.md) 的驗證、更新與還原程序，並在核准範圍內使用外部管理器。

WDA Minimal 不需要 vendor 元件庫或其專屬執行環境。專案改用其他 Design System 時，可以使用該 vendor 公開的元素名稱或 API。WDA 不會為了隱藏 vendor 名稱而建立通用 wrapper。

`<wda-include src="..."></wda-include>` 是 WDA 唯一的自訂 build 時組合原語。它符合 Custom Element 命名規則，會在 build 時展開，不是 runtime Web Component，也不會留在 `dist/`。

WDA Minimal 不定義 template language。它沒有 `<slot>`、條件、迴圈、變數、運算式或資料繫結。

## 使用邊界

本文件是 WDA Minimal 在 Core 中的完整定義。Generated Project 的 `docs/design.md` 只記錄該專案實際採用的 token 與專案專屬審查判準，不會複製這份 WDA 層級的定義。

WDA Minimal 是可重用的基準與參照，不保證「零 HTML 修改」、「100% 重用」或「自動遷移」。它的遷移目標是降低成本，同時保留專案的語意結構與選擇權。
