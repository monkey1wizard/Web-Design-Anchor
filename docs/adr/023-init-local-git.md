---
title: "wda init 建立本地 Git Repository，Core 硬綁 git"
status: stable
updated: 2026-09-09
sourceRefs:
  - ".dev/plans/feat-init-git-local.prompt.md"
  - "docs/architecture.md §2.3、§2.12"
  - "ROADMAP.md §3.2 G9、R3"
description: "Why does wda init create a local Git repository and require git?"
tags: []
type: ADR
---

## Historical decision (frozen)

* **狀態**：Accepted
* **背景**：`wda init` 原本只接受空目錄，且從不要求 `git` 存在；Designer 沒有還原基準點，一旦變更看錯方向就無法回到「上一個已接受狀態」。
* **決策**：`wda init` 成功後，專案目錄是一個本地 Git repository，第一個 commit 涵蓋整個目錄（含 Designer 原本帶進來的檔案），且從不設定 remote。「空目錄」前提換成更窄也更嚴格的 collision rule：只有本次 run 會建立的名稱（write-plan 檔案、其所需目錄、`.git/`，Spectrum 2 再加 `package.json` 與 `deno.lock`）不得已存在；目錄裡的其他內容一律不讀不動。`git` 因此從「Core 從不硬綁」的舊立場，變成與 `deno` 並列的第二個 PATH 硬性前置條件：`git` 不在 PATH 上時，`wda init` 回報 Error（`wda.init.tool-unavailable`）並且不寫入任何東西。
* **影響**：Designer 取得一個不需要自己執行 `git init` 的還原基準點；`wda check`、`wda build`、`wda deps` 不受影響，仍不要求既有 repository。

## Current status

Accepted. `docs/architecture.md` §2.12 記載 Core 硬綁 git，§2.3 的 `wda init` 職責行加入 repository 建立步驟；`ROADMAP.md` §3.2 的 G9 排除項收窄為「`wda init` 為唯一例外」，R3 修訂為 `deno` 與 `git` 兩個必須在 PATH 上的外部工具。

**本決策取代兩處先前立場，理由與日期如下：**

1. **ROADMAP.md §3.2 R3，Project Owner 於 2026-09-02 的裁定**——`deno` 是唯一必須在 PATH 上的外部工具。該裁定於 2026-09-08 由 Project Owner 修訂：`git` 成為第二個必須在 PATH 上的工具。理由：`wda init` 的新測試需要對真實 `git` 執行 shell out（`git init`、`git add -A`、`git commit`），沒有其他本地工具形態夠可靠地提供還原基準點；到目前為止本地測試環境對這個宣稱也不成立。
2. **`docs/architecture.md` 舊 §2.12 wording**——「Core is never hard-bound to git」。該立場於 2026-09-08 由 Architect 裁定反轉為「Core is hard-bound to git」。理由：可逆迭代（§2.8 的第 3 條「未經確認看過的變更不算已接受」）需要一個實際的還原基準點，而本地 Git repository 是唯一夠可靠的實作，因此把「從不硬綁」的排除改為「硬綁」是唯一能兌現可逆保證的做法。

兩處立場的修訂已同步反映於 `docs/architecture.md`（commit `f324523`）與 `ROADMAP.md`（commit `79f5571`）。本 ADR 是這兩處修訂共同指向的決策紀錄，避免 repository 中同時存在兩個互相矛盾的說法。
