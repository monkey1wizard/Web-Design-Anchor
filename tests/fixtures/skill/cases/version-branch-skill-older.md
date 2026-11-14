# L3 Case: 版本比對：Skill 較舊分支

## Host Input

1. 從同一個目前的原始碼 checkout 建置 CLI，並將其中 `skills/wda/` 複製到隔離的測試安裝位置。保留原始檔與既有安裝，測試結束後刪除測試副本。
2. 使用有效的現有 WDA 專案，其中同時有 `anchor.json` 和 `package.json`。執行測試 CLI 的 `wda --version` 並記錄實際版本。
3. 只在測試 Skill 副本的 frontmatter 中，將 `metadata.version` 改為低於觀察到的 CLI 版本的有效版本。記錄修改前後的值，不修改 CLI 或其他 Skill 內容。若無可用的較低版本，停止並記錄此限制。
4. 在新對話明確載入測試 Skill，要求：「請檢查這個 WDA 專案。」先使用相容輸入，記錄回應。
5. 在另一個新對話重複相同請求，並向測試 Skill 提供衝突證據：標籤 `v0.1.4` 的 Skill 使用原生切換前的舊契約。請它評估該舊副本用於目前 CLI 的工作流程，並指出適合的配對副本。保留來源與比較結果作為文字證據。

## Expected AI Behavior

兩次輸入都須回報 Skill 較舊並輸出 Warning。相容輸入須依路由表繼續處理請求。已知衝突輸入須說明衝突，停止受影響的工作流程，並指向與 CLI 對應的 release 或原始碼 Skill，不得一律宣稱相容。

## Observable Pass Signal

兩次文字紀錄分別顯示較舊結果與 Warning。相容輸入繼續處理請求。衝突輸入說明衝突與配對副本，且停止受影響的工作流程，不執行後續子指令或修改專案。此文字評估不測量 Skill 的自動觸發行為。
