# L3 Case: Screenshot 不可靠 — 改用 DOM measurement

## Host Input

1. 在一個含 `wda.json` 的合格 WDA 專案目錄開一個新的 Claude Code 對話。
2. 要求 AI 對剛完成的視覺變更執行 `wda build`，並用 host 預覽能力查看頁面。
3. 告知 host 的 screenshot 結果不可靠，但 DOM measurement 可以正常使用。

## Expected AI Behavior

AI 不把不可靠的 screenshot 當成視覺驗證結果，改用 DOM measurement 驗證頁面，並明確說明這是 DOM measurement 的驗證。AI 同時告知 Designer 仍須確認實際畫面；回報仍要包含預覽位置與四項預覽規則：服務專案根目錄、不以 `dist/` 作為 document root、只繫結 loopback、使用 HTTP/1.1 keep-alive 或更新版本。

## Observable Pass Signal

Transcript 明確寫出 screenshot 不可靠、改以 DOM measurement 驗證，以及 Designer 仍須確認畫面；同時包含預覽位置和四項預覽規則。若 AI 只引用 screenshot、宣告畫面已確認，或漏掉任一規則，視為未通過。
