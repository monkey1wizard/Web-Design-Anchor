---
title: "Generated Project 生命週期與可逆性"
status: stable
updated: 2026-09-21
type: ADR
description: "本地 Git repository 基準點、.gitignore 建立時機與條件式 development.md 文件。"
tags: []
absorbs: ["023", "029", "030"]
---

# ADR-05 Generated Project 生命週期與可逆性

## 背景

Designer 在視覺迭代中需要還原基準點。若無版本控制，Designer 無法回到上一個已接受的狀態。產出專案的 build 產物預設不應進版本控制。專案需釐清 .gitignore 的建立與管理權責。產出專案的文件集合中，多數專案僅需執行四個指令即可開發。多數專案不需要預設建立空殼的開發流程說明文件。

## 選項

**由 Skill 代為執行 Git 操作、每個指令管理 .gitignore、一律建立 development.md。** 由 Skill 承擔版本控制職責。由 Core 指令在每次執行時確保 .gitignore 內容。由 Core 為每個新專案建立固定的五份文件。代價是 Skill 越權承擔確定性操作。Core 侵入使用者自訂的 repository 設定。這個選項也會產生大量無用的空殼文件。

**由 AI host 的 checkpoint 替代 Git、合併既有 .gitignore、由 baseline 觸發建立 development.md。** 這個選項依賴各 AI 平台的私有快照功能。這個選項嘗試解析與合併既有的忽略清單。這個選項由 browser 或 a11y baseline 觸發建立第五份文件。代價是不同 host 機制不一且可能缺席。合併規則複雜且易破壞使用者既有設定。baseline 設定實質上只需在 architecture.md 或 design.md 摘要即可。

**wda init 建立本地 Git repository、.gitignore 建立一次後不再管理、development.md 採條件式建立。** init 成功時在專案根目錄建立本地 Git 儲存庫並完成初次 commit。init 不設 remote，使 git 成為 PATH 前置工具。init 建立固定內容為 `/dist/` 的 .gitignore，後續指令不讀不寫。init 不建立 development.md。僅在專案具備四指令以外之專屬工作流時，專案自行建立 development.md。

## 決策

wda init 成功後，專案目錄為本地 Git repository。第一個 commit 涵蓋整個目錄（訊息固定為 `wda init`），不設定 remote。空目錄前提改為嚴格的碰撞集合規則：write-plan 目標、所需目錄、`.git/` 以及當前相依性設定不得已存在；其餘檔案不讀不動。`git` 與 `deno` 並列為 PATH 前置工具，缺任一工具時明確失敗。

.gitignore 由 wda init 在專案根目錄建立一次，內容固定為 `/dist/`，作為便利產物而非系統依賴。之後任何指令都不重寫、不管理它。即使沒有 Git repository 或 .gitignore，wda check、wda build、wda deps 亦可正常執行。

Generated Project 的 development.md 是條件式文件，wda init 永遠不主動建立它。只有當專案擁有四個 WDA 指令與四份必要文件未涵蓋之專案特定工作流時，才由專案自行建立並記錄。

## 後果

- Designer 擁有確定性的本機還原基準點，Skill 無需也絕不執行版本控制指令。
- 測試環境會呼叫真實 git；若環境缺少使用者身分設定，init 以臨時身分完成 commit 並發出 Warning。
- Core 不成為 repository 治理者，尊重使用者對 .gitignore 的後續修改。
- 必要專案文件維持四份，wda check 僅驗證四份必要文件；若專案自行建立 development.md 則按選用文件規則驗證。

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
- 2026-09-21：本記錄由既有原子決策收斂而成，來源見 front matter absorbs。
