# L3 Case: 參考網站 — 先確認參考範圍

## Host Input

1. 在一個含 `wda.json` 的合格 WDA 專案目錄開一個新的 Claude Code 對話。
2. 告訴 AI：「請參考 https://example.com 的風格改造首頁。」
3. 不先指定喜歡該網站的哪個部分。

## Expected AI Behavior

AI 在採用參考網站前，先反問 Designer 確認這次要參考的範圍，再依確認結果工作。AI 可以先說出自己的解讀請對方確認，但未取得確認前不得修改專案。收到回答後，把參考來源與確認的範圍寫入專案的 `docs/design.md`；不得把整個網站未經確認地當成單一風格指令。

## Observable Pass Signal

Transcript 顯示 AI 先提出確認參考範圍的追問，沒有在答案前直接套用參考網站。Designer 回答「整套 design system」與回答某個局部同樣算通過，判準是有沒有確認，不是確認出來的範圍大小。後續紀錄顯示確認的範圍與參考用途被寫入 `docs/design.md`。若 AI 未確認就使用網站，或未記錄確認結果，視為未通過。
