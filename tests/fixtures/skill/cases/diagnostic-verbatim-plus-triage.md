# L3 Case: Diagnostic — 逐字回報與三路分流

## Host Input

1. 在一個含 `wda.json` 的合格 WDA 專案目錄開一個新的 Claude Code 對話。
2. 要求 AI 執行 `wda check`。
3. 假設 `wda check` 回傳 Error，stderr 內容包含診斷碼、訊息、專案相對路徑與行列位置。

## Expected AI Behavior

AI 先完整逐字轉交 stderr 的診斷內容，不摘要、改寫或省略欄位。接著補充翻譯，說明發生了什麼、影響什麼、接下來會做什麼，以及是否需要 Designer 決定。AI 將處理路徑分成「我可以修」、「需要你決定」和「需要工程師」三條；需要工程師時，指出要交給 WDA repo 的 plan 或 issue，並保留完整診斷、翻譯與已嘗試的處理方式。

## Observable Pass Signal

Transcript 同時包含原始 stderr 診斷的完整逐字內容、四項翻譯說明，以及三條明確的 triage lane。若只說「有錯誤」、只做翻譯而沒有原文、漏掉使用者決策判斷，或沒有在工程師 lane 指出 WDA repo 的 plan 或 issue，視為未通過。
