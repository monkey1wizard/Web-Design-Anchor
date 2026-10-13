---
title: "Design System、tokens 與中立性"
status: stable
updated: 2026-09-21
type: ADR
description: "Design System adapter 介面、DTCG token 轉換、預設 Spectrum 與 WDA Minimal 退回、共用 Lit authoring、中立性判準與單一定義。"
tags: []
absorbs: ["008", "014", "019", "024", "025", "031"]
---

# ADR-04 Design System、tokens 與中立性

## 背景

WDA 要求 Core 不綁死單一 Design System。Core 必須同時支援兩套架構差異足夠大的 Design System（Spectrum 2 與 WDA Minimal）以證實中立性。專案需要處理 vendor 官方 Web Components、design tokens 格式與存放位置、共用 Custom Elements 的撰寫標準，以及離線無網路時的優雅降級與還原途徑。

## 選項

**為 vendor 元件建立通用抽象 wrapper、將 token 直接寫成 CSS、兩套系統皆採用 vendor 函式庫或強制完全一致的相依模型、允許不同元件庫隨意混用。** 代價是 wrapper 演變成沉重的第二套元件庫。這樣無法履行零 HTML 變更遷移承諾。這樣無法提供可靠的離線退回。手寫元件缺乏統一標準，導致維護困難。

**Design System adapter 為文件化介面、Token 採 DTCG 格式並在 build 時轉為 CSS、預設 Spectrum 2 並以 WDA Minimal 為離線退回、兩路徑共享 Lit 作為 Custom Elements 預設 authoring 標準、嚴格約束中立性接縫。** Adapter 定義七個 facet。Token 來源為 DTCG JSON，產物僅在 dist/。預設為 latest stable Spectrum，退回選項為 WDA Minimal。兩套系統均以 Lit 為預設 Custom Elements authoring library。WDA Minimal 採 best-effort 安裝 Lit 以確保離線可用。中立性判準限制於解析與資產接縫。

## 決策

Adapter 是定義於文件中的介面規範，涵蓋七個 facet。原生 HTML 與 Modern CSS 優先，不為隱藏 vendor 名稱建立通用 wrapper。

Token 唯一來源為 tokens/tokens.json（DTCG 格式），於 build 時轉為 CSS Custom Properties 輸出至 dist/styles/tokens.css，原始碼端不得存在衍生檔案。

支援兩套 Design System。預設為 latest stable Spectrum（釘版 npm 套件加獨立 Lit）。退回選項為 WDA Minimal（凍結之 Spectrum token 快照數值，單一定義僅存於 Core）。

兩條路徑共用 Lit 作為 Custom Elements 預設 authoring library。原生語意化 HTML 仍是版面與靜態內容首選。當需要封裝行為或產品語意時以 Lit 撰寫（使用標準 class 加 customElements.define，不使用 decorator）。

WDA Minimal 在初始化時採 best-effort 安裝 Lit：若離線或無網路，初始化仍宣告成功，專案以純語意化 HTML 建立，並在摘要中明示未安裝 Lit 及提供 `wda deps add lit` 補救指令。

中立性操作型定義：兩套系統各自以不同的相依與元件模型走完 init、check、build。Design System 身分分支僅限於解析接縫（src/init/resolve.rs）與資產接縫（src/init/、src/builtins/），其餘程式碼一律中立。

## 後果

- 新增或更換 Design System 無需修改 Core 主流程，只要擴充接縫與文件。
- 設計數值結構化，AI 與設計工具均可精確讀取，衍生 CSS 不污染原始碼。
- 兩條路徑具備一致的 Custom Elements 撰寫體驗，同時維持 WDA Minimal 在無網路環境下的成功保證。
- 中立性具備自動化測試閘門，杜絕程式碼與文件對自動轉換或無損遷移的不實宣稱。

## 狀態沿革

- 2026-08-24：從企劃文件遷入，當時寫的是「分層適配」。
- 2026-08-27：七個 facet 的定義落地。
- 2026-09-18：改寫成完整紀錄。
- 2026-08-24：從企劃文件遷入，當時寫的是產生到 `styles/tokens.css`。
- 2026-08-24：改為只產生到 `dist/`。
- 2026-09-18：改寫成完整紀錄。
- 2026-08-24：從企劃文件遷入，當時第二套叫 Modern Neutral Clean。
- 2026-08-24：正式定名 `WDA Minimal`。
- 2026-08-27：`WDA Minimal` 的實作落地。
- 2026-09-07：Spectrum 的實作落地。
- 2026-09-18：改寫成新決策的完整紀錄。
- 2026-09-05：Project Owner 裁定。
- 2026-09-18：改寫成固定五節的格式。決策仍在行使中。
- 2026-09-05：Project Owner 裁定真實 registry 探針移出阻擋型閘門。
- 2026-09-07：第二套 Design System 的實作落地時定案。
- 2026-09-18：改寫成固定五節的格式。約束仍在行使中。
- 2026-08-24：裁定。
- 2026-08-27：`WDA Minimal` 的定義文件與 `design.md` 範本落地。
- 2026-09-18：補寫成 ADR。
- 2026-09-21：本記錄由既有原子決策收斂而成，來源見 front matter absorbs。
