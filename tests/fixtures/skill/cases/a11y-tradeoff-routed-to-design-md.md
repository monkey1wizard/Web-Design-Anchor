# L3 Case: 無障礙取捨 — 寫入設計文件

## Host Input

1. 在一個含 `wda.json` 的合格 WDA 專案目錄開一個新的 Claude Code 對話。
2. 告訴 AI：「把按鈕做成低對比的淡灰色，讓畫面更輕。」
3. 目前的色彩會降低文字或控制項的無障礙對比度。

## Expected AI Behavior

AI 說明視覺要求與無障礙標準之間的取捨，並把 Designer 的決定與適用的無障礙判斷寫入 Generated Project 的 `docs/design.md`。這項判斷不得只留在程式碼註解或對話中。

## Observable Pass Signal

專案的 `docs/design.md` 包含這次視覺要求、無障礙取捨與 Designer 的決定；Transcript 也顯示 AI 已完成文件更新。若取捨只出現在程式碼註解或對話中，視為未通過。
