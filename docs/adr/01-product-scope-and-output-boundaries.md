---
title: "產品範圍與輸出邊界"
status: stable
updated: 2026-09-21
type: ADR
description: "Core 的執行形態、標準 Web 輸出邊界、封閉指令集與部署環境假設。"
tags: []
absorbs: ["001", "002", "016", "017", "028"]
---

# ADR-01 產品範圍與輸出邊界

## 背景

WDA 是給 Designer 搭配 AI 長期建立與維護網站的工具。
Designer 的環境通常沒有開發工具鏈，也不應承擔環境維護負擔。
產出的網站必須長久可維護，不因前端框架生命週期換代而被迫重寫。
在 AI 能直接讀寫檔案的環境裡，CLI 指令過多會增加維護成本。
關鍵操作必須由確定性程式保證正確。
網站輸出需明確定義部署環境與 URL 行為，避免做出 Core 無法控制的承諾。

## 選項

**以 Node.js、Python 或 Go 實作 CLI，或採用前端框架與靜態網站產生器。** 生態豐富、現成解法多。
代價是使用者必須面對 Node.js 版本與套件衝突。
專案被迫綁定框架的升級節奏與複雜度，失去零 runtime 相依性優勢。

**開放式指令集、提供 Clean URL 重寫保證，並保證 file:// 協定可用。** 對特定工作流或本地預覽友善。代價是 Core 責任無限膨脹，為不同 host 產生特定 rewrite 設定，或因相容 file:// 而限制 Standard Web 的 ES module 與 fetch 能力。

**Rust native CLI、純標準 Web 輸出、封閉四指令集、僅保證 static HTTP host。** Core 是單一 standalone native binary。
產出遵循 HTML5、Modern CSS、TypeScript 與原生瀏覽器能力。
指令集封閉為 init、deps、check、build 四個。
輸出只保證一般 static HTTP(S) host 部署，不保證 file:// 與 Clean URL。

## 決策

Core 用 Rust 寫成 standalone native binary，使用者只需一個 `wda` 在 PATH 上，不依賴任何語言 runtime。

專案與產出只用標準 Web。Source 基準是 HTML5、Modern CSS、TypeScript、原生瀏覽器能力；Deployment 基準是 HTML、CSS、JavaScript、assets。不綁定任何前端框架，WDA 本身亦不是網站的 runtime dependency。

指令集封閉為 init、deps、check、build 四個，不得為 Designer 工作流、Skill 安裝或預覽新增第五個指令。新增指令的判準為：代表真正不同的責任且需要保證正確執行；需要判斷或決策的操作屬於 AI。

不提供 Clean URL 保證。不承諾去除副檔名，不產生巢狀目錄模擬路由，URL rewrite 與 fallback routing 屬於部署環境。

dist/ 只保證一般 static HTTP(S) host 部署可用，不保證 file:// 直接開啟可用。wda build 偵測到已知不相容特徵時提出提醒，不提供 fallback。

## 後果

- 使用者無需安裝 Node.js 或其他語言 runtime 即可執行 Core。
- 產出的網站沒有任何框架依賴，亦不依賴 WDA；工程師接手面對標準 Web。
- 相依性變更有唯一入口，build 可信任鎖定狀態；預覽與 Skill 安裝屬於 host，不會有額外指令。
- source 到 dist/ 一對一可預期；想要 Clean URL 的專案由部署環境設定。
- Standard Web 能力不受 file:// 限制；不具備預覽能力的 host 必須明示，不得在無人檢視下宣告變更完成。

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
