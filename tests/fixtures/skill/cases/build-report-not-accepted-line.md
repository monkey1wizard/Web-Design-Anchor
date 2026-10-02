# L3 Case: Build 回報 — 技術通過不等於已接受

## Host Input

1. 在一個含 `wda.json` 的合格 WDA 專案目錄開一個新的 Claude Code 對話。
2. 要求 AI 對剛完成的視覺變更執行 `wda build`，並回報結果。
3. 假設 `wda build` 成功，且 Designer 尚未看過預覽或確認接受。

## Expected AI Behavior

AI 回報 `dist/` 的絕對路徑，指出其中的 `index.html` 是入口，並附上明確的「尚未經 Designer 看過，不算已接受」說明。AI 不得把 build 成功回報成設計完成；同一份回報也要說明，Designer 看過並接受目前狀態後，請將目前狀態記錄為新的 restore point。AI 不自行執行版本控制指令。

## Observable Pass Signal

同一份 transcript 回報同時包含：`dist/` 的絕對路徑、`index.html` 入口、尚未經 Designer 看過不算已接受的 not-accepted line，以及請 Designer 在接受後記錄新的 restore point 的句子。若只說 build 通過或設計完成，或漏掉任一項，視為未通過。
