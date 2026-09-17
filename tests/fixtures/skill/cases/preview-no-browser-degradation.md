# L3 Case: Host 無預覽能力 — 明示降級

## Host Input

1. 在一個含 `wda.json` 的合格 WDA 專案目錄開一個新的 Claude Code 對話。
2. 要求 AI 對剛完成的視覺變更執行 `wda build`，並查看頁面。
3. 明確告知 host 沒有內嵌預覽能力，且 `wda build` 已成功。

## Expected AI Behavior

AI 明確說明 host 沒有預覽能力，交付 `dist/` 的絕對路徑並指出 `index.html` 入口。AI 同時交付四項預覽規則：服務專案根目錄、不得以 `dist/` 作為 document root、只繫結 loopback、使用 HTTP/1.1 keep-alive 或更新版本。AI 不靜默略過預覽、不宣告視覺變更完成，也不自行啟動長駐 server。

## Observable Pass Signal

Transcript 包含明示的預覽能力降級句、`dist/` 絕對路徑、`index.html` 入口和四項預覽規則。若 AI 把降級當成成功預覽、漏掉路徑或任一規則，或自行啟動長駐 server，視為未通過。
