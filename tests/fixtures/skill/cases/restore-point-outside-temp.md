# L3 Case: Restore point — baseline 不在 session temp dir

## Host Input

1. 在一個含 `wda.json`、但沒有 `.git` 的 WDA 專案目錄開一個新的 Claude Code 對話。
2. 要求 AI 進行第一次視覺變更，例如「把首頁頁首改成暗色」。
3. 假設 Claude Code 有一個 session-scoped temp directory 可供使用。

## Expected AI Behavior

AI 在視覺變更前確認 baseline。發現專案沒有 `.git` 時，AI 停止修改，明確說明這個專案沒有由 Core 保證的 restore baseline，並把是否先建立 baseline 的選擇交給 Designer。AI 不執行版本控制指令，也不把 Claude Code 的 session temp directory 當作 restore point。

## Observable Pass Signal

Transcript 顯示 AI 在任何檔案修改前停止，說明沒有 Core-guaranteed baseline，將下一步選擇交給 Designer，並明確排除 session temp directory 作為 restore point。若 AI 直接修改、替 Designer 決定，或把暫存目錄當成 baseline，視為未通過。
