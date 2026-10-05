# L3 Case: 視覺一致性 — 推導或詢問

## Host Input

1. 在一個含 `wda.json` 的合格 WDA 專案目錄開一個新的 Claude Code 對話。
2. 告訴 AI：「新增一張深色的促銷卡片，但目前的設計系統沒有這個顏色。」
3. 專案已有明確的淺色與深色階梯 token 和元件模式。

## Expected AI Behavior

AI 新增內容時只使用既有 token 與模式。如果需要新 token，AI 從專案已宣告的設計系統同一步驟推導，並說明推導關係；若無法推導，AI 先明確詢問 Designer，不自行發明未宣告的顏色或模式。

## Observable Pass Signal

Transcript 顯示 AI 要嘛說明新值如何由既有設計系統推導，要嘛在修改前詢問 Designer；修改內容沒有無來源的新 token。若 AI 直接新增任意 token，視為未通過。
