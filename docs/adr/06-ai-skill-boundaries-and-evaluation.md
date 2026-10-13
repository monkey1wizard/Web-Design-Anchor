---
title: "AI Skill 邊界與評測"
status: stable
updated: 2026-09-21
type: ADR
description: "AI Skill 作為主要操作介面、隨 release 發佈之產物定位、與缺乏 harness 時的 L3 逐條文字比對評測法。"
tags: []
absorbs: ["003", "022", "026"]
---

# ADR-06 AI Skill 邊界與評測

## 背景

Designer 透過自然語言操作 WDA。會話對話記憶無法跨 session 或跨工具保留。Skill 是引導 AI 操作 CLI 的手冊。Skill 的定位需要明確區分為文件或發布產物，並定義版本原則與規格邊界。Skill 的行為驗收（L3 評測）原本規定需以 transcript harness 進行真實觸發重播與 recall/precision 量化計算。在 harness 工具缺席時，Skill 需要可執行的替代驗證機制。

## 選項

**開發獨立 GUI 應用程式、Skill 放在文件目錄當作文件維護、無 harness 時暫停驗收或填入估算數字。** GUI 增加各平台打包維護成本且不符 AI 操作習慣。將 Skill 視為文件會混淆規範與執行程序的邊界。若無 harness，這個選項會卡死交付或以虛報數字掩蓋量化缺口。

**純對話依賴 AI 記憶、Skill 擁有獨立版本與矩陣、以一般文字比對宣稱為完整評測。** 對話記憶無法作為持久權威。獨立版本矩陣徒增維護負擔。這個選項將缺乏 undertriggering 檢驗的文字比對等同於行為評測，會掩蓋漏觸發的靜默風險。

**AI 透過 Skill 操作 CLI 且專案檔案為唯一真實來源、Skill 隨 release 發布且版本對齊 release、無 harness 時採逐條文字比對並明確標示 not-measured。** Designer 用自然語言。AI 依 Skill 程序呼叫 CLI 並將所有決策寫回專案文件。Skill 放在 skills/wda/ 作為發布產物。Skill 的版本與 release 一致，不保存 Core 規則。在缺乏可執行 harness 時，L3 評測採確定性 fixture 逐條文字比對。recall 與 precision 在缺乏可執行 harness 時如實記錄為 not-measured，不虛報數據。

## 決策

Designer 以自然語言與 AI 對話。AI 載入 Skill 後操作 wda CLI。所有規格與決策必須寫回專案文件。專案檔案為唯一真實來源。AI 負責讀寫專案檔案，不得要求 Designer 先行手動閱讀檔案。

Skill 是隨 release 發布的產物，而非文件。實體位於 skills/wda/，與 Core 原始碼及文件層並列。Skill 承載操作程序與路由指標。Skill 不保存 Core 內部規則，不形成第二套產生器。Skill 無獨立版本號。Skill 的版本完全對齊所屬 WDA release。發布套件佈局遵循 Agent Plugins 1.0.0 標準。

在缺乏可執行 transcript harness 的限制下，Skill L3 行為評測暫以逐條文字比對執行。recall 與 precision 在無即時判定工具時一律記錄為 `not-measured`。評測絕不以估算值、前輪數據或通過率充當 recall 或 precision。評測結果僅對實際測試的 AI host（Claude Code）成立。

## 後果

- 專案交接與跨 session 連續性皆有檔案可循。這樣沒有決策丟失風險。
- release package 包含 skills/wda/ 與 plugin.json，標準 client 可直接安裝，Core 不需額外提供 skill 指令。
- Skill 比 binary 新時必須停止並提示升級，不得靜默降級；Skill 內容受白名單限制，防止規則洩漏。
- 逐條文字比對無法檢驗漏觸發（undertriggering）風險，為已知缺口；一旦可執行 harness 具備，必須回歸真實觸發重播評測。

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
