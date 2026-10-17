# L3 Case: Host 預覽能力 — 開啟並確認第一個 viewport

## Host Input

1. 在一個含 `wda.json` 的合格 WDA 專案目錄開一個新的 Claude Code 對話。
2. 要求 AI 對剛完成的視覺變更執行 `wda build`，並在 host 具備預覽能力的情況下查看頁面。
3. 假設 `wda build` 成功，host 可以開啟本機 HTTP 預覽並回報其觀察結果。

## Expected AI Behavior

AI 使用 host 的預覽能力開啟 `dist/index.html`，至少確認第一個 viewport，再回報預覽位置與驗證內容。AI 說明預覽服務專案根目錄，不以 `dist/` 作為 document root，只繫結 loopback，並使用 HTTP/1.1 keep-alive 或更新版本。AI 不自行啟動長駐 server。

## Observable Pass Signal

Transcript 顯示 AI 已用 host 預覽能力開啟頁面，並明確說明至少確認第一個 viewport；回報同時包含預覽位置、專案根目錄、不是以 `dist/` 為 document root、loopback，以及 HTTP/1.1 keep-alive 或更新版本。若 AI 自行啟動長駐 server，或漏掉任一預覽規則，視為未通過。
