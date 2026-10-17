---
title: "產品範圍與輸出邊界"
status: stable
updated: 2026-10-15
type: ADR
description: "Core 的執行形態、標準 Web 輸出邊界、封閉指令集與部署環境假設。"
tags: []
absorbs: ["001", "002", "016", "017", "028"]
---

# ADR-01 產品範圍與輸出邊界

## 背景

WDA 讓 Designer 能和 AI 一起長期建立與維護網站。
Designer 的環境通常沒有開發工具鏈，也不必承擔維護工具鏈的工作。
網站必須容易長期維護，不能因前端框架換代而被迫重寫。
AI 能直接讀寫檔案時，過多 CLI 指令會增加維護成本。
確定性程式必須確保關鍵操作正確。
網站輸出必須說明支援的部署環境與 URL 行為，避免承諾 Core 無法控制的結果。

## 選項

**以 Node.js、Python 或 Go 實作 CLI，或採用前端框架與靜態網站產生器。** 這些選擇有豐富的生態系與現成解法。
使用者必須處理 Node.js 版本與套件衝突。
專案也會受框架升級節奏與複雜度限制，失去不依賴 runtime 的優勢。

**開放指令集、保證 Clean URL 重寫，並保證 file:// 協定可用。** 這能配合特定工作流程或本地預覽。代價是 Core 的責任會持續擴大，必須為不同 host 產生專用 rewrite 設定，或為了相容 file:// 而限制 Standard Web 的 ES module 與 fetch 能力。

**以 Rust 撰寫 CLI、輸出純標準 Web、限制為四個指令，並只保證 static HTTP host。** Core 是單一 standalone native binary。
輸出遵循 HTML5、Modern CSS、TypeScript 與瀏覽器原生能力。
指令固定為 init、deps、check、build 四個。
輸出只保證能部署到一般 static HTTP(S) host，不保證支援 file:// 或 Clean URL。

## 決策

Core 以 Rust 編譯成 standalone native binary。使用者只需將 `wda` 放在 PATH 上，無須安裝任何語言 runtime。

專案與產出只使用標準 Web。Source 使用 HTML5、Modern CSS、TypeScript 與瀏覽器原生能力。部署產物包含 HTML、CSS、JavaScript 與 assets。專案不綁定前端框架，網站執行時也不依賴 WDA。

指令固定為 init、deps、check、build 四個。不得為 Designer 工作流程、Skill 安裝或預覽新增第五個指令。只有代表不同責任且需要確保正確執行的操作，才符合新增指令的條件。需要判斷或決策的操作由 AI 負責。

WDA 不保證 Clean URL，也不承諾移除副檔名或產生巢狀目錄來模擬路由。URL rewrite 與 fallback routing 由部署環境負責。

dist/ 只保證能部署到一般 static HTTP(S) host，不保證可透過 file:// 直接開啟。若 wda build 偵測到已知的不相容特徵，就會提出提醒，但不提供 fallback。

## 後果

- 使用者無需安裝 Node.js 或其他語言 runtime，就能執行 Core。
- 網站不依賴任何框架或 WDA。工程師接手時會面對標準 Web。
- 相依性變更只有一個入口，build 會使用可信任的鎖定狀態。預覽與 Skill 安裝由 host 負責，因此不增加 CLI 指令。
- source 到 dist/ 的輸出可逐一預期。需要 Clean URL 的專案必須由部署環境設定。
- Standard Web 能力不受 file:// 限制。若 host 不支援預覽，必須清楚說明，而且不得在無人檢視時宣告變更完成。

## 狀態沿革

- 2026-08-24：從企劃文件遷入，原文為四行條列。
- 2026-09-18：改寫成完整紀錄。決策不變。
- 2026-08-24：從企劃文件遷入。
- 2026-09-18：改寫成完整紀錄。決策不變。
- 2026-08-24：從企劃文件遷入，當時是三個指令。
- 2026-08-24：加入 `deps`，改為四個。
- 2026-09-18：改寫成新決策的完整紀錄。
- 2026-08-24：從企劃文件遷入，當時的決策是輸出 nested `index.html`。
- 2026-08-24：改為不保證。
- 2026-09-18：改寫成新決策的完整紀錄。
- 2026-08-24：裁定。
- 2026-09-18：補寫成 ADR。
- 2026-09-21：本記錄由既有原子決策收斂而成，來源見 front matter absorbs。
