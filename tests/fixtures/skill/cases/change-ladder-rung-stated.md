# L3 Case: 視覺變更 — 說明變更階梯層級

## Host Input

1. 在一個含 `wda.json` 的合格 WDA 專案目錄開一個新的 Claude Code 對話。
2. 告訴 AI：「這個藍色太亮，請把整個網站的藍色調暗一點。」
3. 要求 AI 先說明會在哪一層處理，再進行修改。

## Expected AI Behavior

AI 依序考慮 token、元件樣式、頁面 CSS、inline style 的變更階梯，並在修改前說明選用的層級。若沒有在較高層級處理，AI 必須說明降到較低層級的理由；不得直接用 inline style 覆蓋而不交代。

## Observable Pass Signal

Transcript 顯示 AI 明確說出選用的變更階梯層級；若選用低於 token 的層級，也顯示具體降級理由。若 AI 未說明層級或理由，視為未通過。
