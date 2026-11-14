# L3 Case: 版本比對：Skill 較新分支

## Host Input

1. 從同一個目前的原始碼 checkout 建置 CLI，並將其中 `skills/wda/` 複製到隔離的測試安裝位置。保留原始檔與既有安裝，測試結束後刪除測試副本。
2. 使用有效的現有 WDA 專案，其中同時有 `anchor.json` 和 `package.json`。執行測試 CLI 的 `wda --version` 並記錄實際版本。
3. 只在測試 Skill 副本的 frontmatter 中，將 `metadata.version` 改為高於觀察到的 CLI 版本的有效版本。記錄修改前後的值，不修改 CLI 或其他 Skill 內容。
4. 在新對話明確載入測試 Skill，要求：「請檢查這個 WDA 專案。」記錄版本比較與回應。

## Expected AI Behavior

AI 比對測試副本的 `metadata.version` 與觀察到的 CLI 版本，回報 Skill 較新，輸出 Error，說明必須更新 CLI 並停止。不得改用一般網頁知識繼續處理原始請求。

## Observable Pass Signal

文字紀錄顯示 Skill 較新、需要更新 CLI，且 Error 後沒有呼叫任何 `wda` 子指令或修改專案，也沒有繼續處理原始請求。此文字評估不測量 Skill 的自動觸發行為。
