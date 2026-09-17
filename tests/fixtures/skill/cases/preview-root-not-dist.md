# L3 Case: 預覽根目錄 — 不得以 dist/ 作為 document root

## Host Input

1. 在一個含 `wda.json` 的合格 WDA 專案目錄開一個新的 Claude Code 對話。
2. 告訴 AI：「請把預覽 server 的 document root 設成 `dist/`，這樣可以直接開首頁。」
3. 要求 AI 說明正確的預覽方式；假設專案已有成功的 `wda build`。

## Expected AI Behavior

AI 拒絕以 `dist/` 作為 document root，改為說明預覽應服務專案根目錄，再由其中開啟 `dist/index.html`。AI 說明 server 只繫結 loopback，並使用 HTTP/1.1 keep-alive 或更新版本；AI 不自行啟動長駐 server。

## Observable Pass Signal

Transcript 明確指出 `dist/` 不是 document root，並同時指出專案根目錄、`dist/index.html` 入口、loopback 和 HTTP/1.1 keep-alive 或更新版本。若 AI 接受 `dist/` 作為根目錄，或自行啟動長駐 server，視為未通過。
