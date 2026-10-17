---
title: "Design System、tokens 與中立性"
status: stable
updated: 2026-10-15
type: ADR
description: "Design System adapter 介面、DTCG token 轉換、預設 Spectrum 與 WDA Minimal 退回、共用 Lit authoring、中立性判準與單一定義。"
tags: []
absorbs: ["008", "014", "019", "024", "025", "031"]
---

# ADR-04 Design System、tokens 與中立性

## 背景

Core 不得綁定單一 Design System。為了驗證中立性，Core 必須同時支援兩套架構差異足夠大的系統，也就是 Spectrum 2 與 WDA Minimal。專案需要定義如何處理 vendor 官方 Web Components、design tokens 的格式與存放位置、共用 Custom Elements 的撰寫標準，以及離線時的降級與還原方式。

## 選項

**建立通用 wrapper、以 CSS 撰寫 token，或任意混用元件庫。** 若要求兩套系統都使用 vendor 函式庫或完全相同的相依模型，也屬於此選項。通用 wrapper 可能演變成沉重的第二套元件庫，也無法履行不改 HTML 的遷移承諾。若兩套系統都依賴 vendor 函式庫，就無法可靠地離線退回。若任意手寫元件，則缺乏一致標準，難以維護。

**以文件定義 adapter，採用 DTCG token，並限制中立性接縫。** build 時將 token 轉成 CSS。預設使用 Spectrum 2，離線時則退回 WDA Minimal。兩套系統共用 Lit 作為 Custom Elements 的預設 authoring 標準。Adapter 定義七個 facet。Token 來源是 DTCG JSON，產物只放在 dist/。預設系統是 latest stable Spectrum，離線退回方案是 WDA Minimal。兩套系統都以 Lit 作為 Custom Elements 的預設 authoring library。WDA Minimal 會盡力安裝 Lit，以確保離線時仍可使用。
中立性判準只涵蓋解析與資產接縫。

## 決策

Adapter 是文件定義的介面規範，涵蓋七個 facet。優先使用原生 HTML 與 Modern CSS，不為了隱藏 vendor 名稱而建立通用 wrapper。

Token 的唯一來源是 tokens/tokens.json，格式採用 DTCG。build 會將 token 轉成 CSS Custom Properties，輸出到 dist/styles/tokens.css。原始碼中不得存放衍生檔案。

Core 支援兩套 Design System。預設採用 latest stable Spectrum，並釘選 npm 套件與獨立 Lit。離線退回選項是 WDA Minimal，其凍結的 Spectrum token 快照數值只在 Core 定義一次。

兩套系統共用 Lit 作為 Custom Elements 的預設 authoring library。版面與靜態內容仍優先使用原生語意化 HTML。需要封裝行為或產品語意時，才使用 Lit 撰寫，並採用標準 class 與 customElements.define，不使用 decorator。

初始化 WDA Minimal 時會盡力安裝 Lit。若使用者離線或沒有網路，初始化仍會成功，並以純語意化 HTML 建立專案。摘要會說明 Lit 未安裝，並提供補救指令 `wda deps add lit`。

中立性的判準是兩套系統各自使用不同的相依與元件模型，並完成 init、check、build。Design System 身分分支只允許出現在解析接縫（src/init/resolve.rs）與資產接縫（src/init/、src/builtins/）。其他程式碼都必須維持中立。

## 後果

- 新增或更換 Design System 時，只要擴充接縫與文件，就無須修改 Core 主流程。
- 設計數值會以結構化格式保存，讓 AI 與設計工具都能精確讀取，衍生 CSS 也不會污染原始碼。
- 兩套系統使用相同的 Custom Elements 撰寫方式。WDA Minimal 在沒有網路時仍能成功初始化。
- 自動化測試會把關中立性，避免程式碼或文件不實宣稱自動轉換或無損遷移。

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
