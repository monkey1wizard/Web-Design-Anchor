# iteration（視覺變更迭代）

## 載入時機

在任何視覺變更請求即將被執行之前，每一次都載入本檔，不只第一次。權威來源：`docs/architecture.md` §2.11。

## 本檔只做一件事

規定單一視覺變更的選擇階梯、影響範圍揭露、一致性判斷與決策標記；視覺審查判準仍屬 Generated Project 的 `docs/design.md`，本檔不重述。權威來源：`docs/architecture.md` §2.11。

## 動作（依序執行）

1. **選擇變更階梯。** 依序嘗試 token、元件樣式、頁面 CSS、inline style。動手前先說出這次選用哪一階；若選的不是 token，同時說明降階原因。權威來源：`docs/architecture.md` §2.11。
2. **請求點名參考網站時。** 動手前先反問 Designer，確認這次要參考的是什麼。若請求的措辭已經指明所指，可以把你的解讀說出來請 Designer 確認；措辭沒有指明時，就開放地問。不論解讀多確定，未取得確認前不得依它修改專案。確認後，把參考來源與確認的範圍寫進 Generated Project 的 `docs/design.md`，沒有對應章節就新增一節。完整的 intake 提問順序與內容／風格來源分工見 `references/intake.md`。權威來源：`docs/architecture.md` §2.11。
3. **處理隱藏狀態。** author-level 的 `display` 規則會覆寫 user-agent 的 `[hidden]` 規則，因此隱藏狀態需要 author-level 規則才能可靠維持。權威來源：`docs/architecture.md` §2.11。
4. **揭露影響範圍。** 若實作範圍超過 Designer 指定的視覺範圍，先詢問，或立即用 Designer 的語言列出受影響的頁面與區域。Designer 自己編輯造成的連帶影響要宣告，但不得未經要求自行修正。權威來源：`docs/architecture.md` §2.11。
5. **維持一致性。** 新內容只能使用既有 tokens 與 patterns。只有在同一步驟能從專案已宣告的 design system 推導時，才可新增 token；說明 light step 與 dark step等同的推導關係，其他情況先明確詢問 Designer。權威來源：`docs/architecture.md` §2.5。
6. **標記代替判斷。** 每個由 AI 代 Designer 作出的視覺決定，都標記「這是我決定的；可以修改」。若決定以視覺要求交換無障礙 criterion，寫回 Generated Project 的 `docs/design.md`，不可只留在程式碼註解。權威來源：`docs/architecture.md` §2.5。
7. **寫回文件。** 變更完成後把需要持續適用的決定寫回 `docs/`；不得要求 Designer 先閱讀文件才能繼續。權威來源：`docs/architecture.md` §2.11。

## 邊界

本檔不判斷專案的視覺是否符合判準，也不自行建立新的視覺判準；判準只由 Generated Project 的 `docs/design.md` 承載。權威來源：`docs/architecture.md` §2.5。

本檔不替 Designer 決定超出指定範圍的連帶修改，不把無障礙取捨藏在程式碼註解，也不以一致性為理由偷偷新增 token。權威來源：`docs/architecture.md` §2.5。
