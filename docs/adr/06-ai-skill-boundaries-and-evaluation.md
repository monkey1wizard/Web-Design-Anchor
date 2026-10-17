---
title: "AI Skill 邊界與評測"
status: stable
updated: 2026-10-15
type: ADR
description: "AI Skill 作為主要操作介面、隨 release 發布之產物定位、與缺乏 harness 時的 L3 逐條文字比對評測法。"
tags: []
absorbs: ["003", "022", "026"]
---

# ADR-06 AI Skill 邊界與評測

## 背景

Designer 以自然語言操作 WDA，但對話記憶無法跨 session 或工具保存。Skill 用來引導 AI 操作 CLI。專案必須說明 Skill 是文件還是發布產物，並定義版本原則與規格邊界。原本的 Skill 行為驗收（L3 評測）要求使用 transcript harness 重播真實觸發，並量化 recall 與 precision。若沒有 harness 工具，Skill 仍需要可執行的替代驗證方式。

## 選項

**開發獨立 GUI 應用程式，把 Skill 放在文件目錄維護，並在沒有 harness 時暫停驗收或填入估算值。** GUI 會增加各平台的打包與維護成本，也不符合 AI 的操作習慣。把 Skill 視為一般文件會混淆規範與執行程序的界線。沒有 harness 時，這種作法會卡住交付，或以不實數字掩蓋量化缺口。

**只依賴對話記憶、為 Skill 設定獨立版本與相容矩陣，並把一般文字比對宣稱為完整評測。** 對話記憶不能作為持久且可靠的依據。獨立版本矩陣只會增加維護負擔。文字比對無法檢查 undertriggering，若把它當成行為評測，會掩蓋未觸發的風險。

**AI 透過 Skill 操作 CLI。專案檔案是唯一真實來源。** Skill 隨 release 發布，並與 release 使用相同版本。沒有 harness 時，改用逐條文字比對，並明確標示 not-measured。Designer 使用自然語言。AI 依照 Skill 程序呼叫 CLI，並將所有決策寫回專案文件。Skill 作為發布產物存放在 skills/wda/。Skill 版本與 release 一致，也不保存 Core 規則。若沒有可執行的 harness，L3 評測就使用確定性 fixture 逐條比對文字。沒有可執行 harness 時，recall 與 precision 都如實記為 not-measured，不得填入估算數據。

## 決策

Designer 以自然語言與 AI 對話。AI 載入 Skill 後操作 wda CLI，並將所有規格與決策寫回專案文件。專案檔案是唯一真實來源。AI 負責讀寫專案檔案，不得要求 Designer 先手動閱讀檔案。

Skill 是隨 release 發布的產物，不是文件。檔案位於 skills/wda/，與 Core 原始碼及文件層並列。Skill 提供操作程序與路由指標，不保存 Core 內部規則，也不形成第二套產生器。Skill 沒有獨立版本號，版本完全對齊所屬 WDA release。發布套件佈局遵循 Agent Plugins 1.0.0 標準。

若沒有可執行的 transcript harness，Skill L3 行為評測暫時改用逐條文字比對。沒有即時判定工具時，recall 與 precision 一律記為 `not-measured`。評測不得用估算值、前輪數據或通過率代替 recall 或 precision。評測結果只適用於實際測試過的 AI host（Claude Code）。

## 後果

- 專案檔案能支援交接與跨 session 延續，降低遺失決策的風險。
- release package 包含 skills/wda/ 與 plugin.json，標準 client 可直接安裝。Core 不必額外提供 Skill 指令。
- 若 Skill 版本比 binary 新，系統必須停止並提示升級，不得靜默降級。Skill 內容受白名單限制，以防規則外洩。
- 逐條文字比對無法檢查漏觸發（undertriggering），這是已知缺口。具備可執行 harness 後，必須恢復真實觸發重播評測。

## 狀態沿革

- 2026-08-24：從企劃文件遷入。
- 2026-09-18：改寫成完整紀錄。決策不變。
- 2026-08-24：決策定案，當時路徑寫成單數 `skill/wda/`。
- 2026-08-26：Project Owner 更正為複數 `skills/wda/`，對齊 Agent Plugins 1.0.0 的探索規則，並加入根目錄的 `plugin.json`。
- 2026-09-08：Designer 視覺迭代工作流解凍，第七份 reference `skills/wda/references/iteration.md` 加入。它在每一次視覺變更執行之前載入，不只第一次。
- 2026-09-18：改寫成固定五節的格式。決策不變。
- 2026-09-05：Project Owner 裁定目標 host 為 Claude Code。
- 2026-09-09：Project Owner 接受本方法為已知限制，未因此擋下交付。
- 2026-09-18：改寫成固定五節的格式。維持 `draft`，因為這是能力缺席下的退而求其次。可執行 harness 一旦存在，本紀錄應被真實觸發重播的做法取代。
- 2026-09-21：本記錄由既有原子決策收斂而成，來源見 front matter absorbs。
