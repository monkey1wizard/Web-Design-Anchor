---
title: "Generated Project 生命週期與可逆性"
status: stable
updated: 2026-10-15
type: ADR
description: "本地 Git repository 基準點、.gitignore 建立時機與條件式 development.md 文件。"
tags: []
absorbs: ["023", "029", "030"]
---

# ADR-05 Generated Project 生命週期與可逆性

## 背景

Designer 在視覺迭代時需要還原基準點。若沒有版本控制，Designer 就無法回到上一個已接受的狀態。產出專案的 build 產物預設不應納入版本控制。專案也需要明確規範 `.gitignore` 的建立與管理方式。多數專案只需執行四個指令即可開發，也不需要預設建立空白的開發流程說明文件。

## 選項

**由 Skill 執行 Git 操作、每個指令都管理 `.gitignore`，並一律建立 development.md。** 這會讓 Skill 承擔版本控制工作。
Core 指令也會在每次執行時檢查 `.gitignore`，並為每個新專案建立五份文件。
Skill 承擔確定性操作時，也可能改動使用者自訂的 repository 設定。
這個做法還會產生許多空白文件。

**由 AI host 的 checkpoint 取代 Git、合併既有 `.gitignore`，並由 baseline 觸發建立 development.md。** 這依賴各 AI 平台的私有快照功能。
這個做法也需要解析及合併既有的忽略清單。
browser 或 a11y baseline 會觸發建立第五份文件。
不同 host 的機制並不一致，也可能不存在。
合併規則複雜，容易破壞使用者既有設定。
baseline 設定其實只需摘要記錄在 architecture.md 或 design.md。

**由 `wda init` 建立本機 Git repository，`.gitignore` 只建立一次，development.md 則依條件建立。** `init` 成功時會在專案根目錄建立本機 Git 儲存庫。它也會建立第一筆 commit。
它不會設定 remote，因此 `git` 是 PATH 前置工具。
`init` 會建立內容固定為 `/dist/` 的 `.gitignore`。
後續指令不會讀寫該檔案。
`init` 不會建立 development.md。
只有當專案有四個指令以外的專屬工作流程時，專案才自行建立 development.md。

## 決策

`wda init` 成功後，專案目錄會成為本機 Git repository。第一筆 commit 涵蓋整個目錄，訊息固定為 `wda init`，且不設定 remote。初始化不再要求目錄完全空白，而是檢查指定碰撞集合：write-plan 目標、必要目錄、`.git/` 與目前的相依性設定都不得已存在。其他檔案不會讀取或修改。`git` 與 `deno` 都是 PATH 前置工具。若缺少 `deno`，`wda init` 會改用 WDA Minimal。若缺少 `git`，指令會在寫入前中止。

`wda init` 會在專案根目錄建立 `.gitignore`，內容固定為 `/dist/`。這份檔案只提供便利，不是系統依賴。其他指令不會重寫或管理它。`wda check` 會針對缺少的 `deno` 或 `git` 回報 Warning。`wda build` 與 `wda deps` 遇到缺少的工具時，會以具名診斷失敗。

Generated Project 的 development.md 是條件式文件。`wda init` 不會主動建立它。只有當四個 WDA 指令與四份必要文件未涵蓋專案特定工作流程時，專案才自行建立並記錄該檔案。

## 後果

- Designer 可使用明確的本機還原基準點。Skill 不需要執行版本控制指令，也不得執行這些指令。
- 測試環境會呼叫真實的 `git`。若環境缺少使用者身分設定，`init` 會以臨時身分建立 commit，並發出 Warning。
- Core 不會管理 repository，因此尊重使用者後續對 `.gitignore` 的修改。
- 必要專案文件維持四份。`wda check` 只驗證這四份必要文件。若專案自行建立 development.md，`wda check` 會依選用文件規則驗證。

## 狀態沿革

- 2026-09-02：Project Owner 裁定 `deno` 是唯一 PATH 前置工具。
- 2026-09-08：Project Owner 修訂為兩個，`git` 加入。同日 Architect 裁定「Core 從不硬綁 git」反轉為「Core 硬綁 git」，理由是可逆迭代需要一個實際的還原基準點，本地 Git repository 是唯一夠可靠的實作。
- 2026-09-09：實作落地。
- 2026-09-18：改寫成固定五節的格式。決策不變。
- 2026-08-24：裁定，當時 `init` 只接受空目錄。
- 2026-09-08：碰撞規則取代空目錄前提，`.gitignore` 納入碰撞集合。
- 2026-09-18：補寫成 ADR。
- 2026-08-24：裁定。
- 2026-09-18：補寫成 ADR。
- 2026-10-14：補上缺少 `deno` 或 `git` 時各指令目前採用的處理方式。
- 2026-09-21：本記錄由既有原子決策收斂而成，來源見 front matter absorbs。
