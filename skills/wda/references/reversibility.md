# reversibility（可逆變更與還原）

## 載入時機

只在下列兩種情況之一發生前載入本檔：即將進行第一次視覺變更，或使用者要求還原。除此之外不載入。

## 本檔只做一件事

規定視覺變更動手前要先確認 Core 已在 `wda init` 建立本機 repository 的 baseline，才能保證事後可還原；以及還原時要還原到哪個版本。暫存目錄永遠不算 baseline；如果 baseline 不存在，必須停止工作並把是否先建立 baseline 的選擇交給 Designer。權威來源：`docs/architecture.md` §2.12、ADR-023。

## 動作（依序執行）

1. **動手前，先確認有一個可還原點。** 在進行任何視覺變更之前，確認 `wda init` 已建立本機 repository 的 baseline。baseline 不存在時，立即停止工作，將是否先建立 baseline 的選擇交給 Designer；暫存目錄不得當作 baseline。這一步未經 Designer 明確確認，不得省略。已經有現成可還原點（例如上一輪變更就已記錄過、且中間沒有未記錄的變更）時，不必重複記錄。
2. **進行視覺變更。**
3. **還原時，還原到「上一個已被 Designer 看過並接受的版本」，不是任何更早或更新的版本。** 未經使用者確認看過的變更，不算已接受狀態；還原不得把專案帶回一個 Designer 從未看過的版本。判斷「Designer 看過並接受」的依據，是步驟 1 記錄可還原點當下的狀態，而不是還原當下專案的最新狀態。
4. **還原後驗證。** 執行 `wda check`，確認還原後的狀態本身沒有 Error。回報 Error 時，路由到 `references/troubleshooting.md`，本檔不重述診斷內容。權威來源：`docs/architecture.md` §2.12。
5. **接受狀態定義。** `SEEN` 只表示 Designer 已看過目前狀態；只有 Designer 明確確認接受後，才轉為 `ACCEPTED`。接受後要求 Designer 記錄新的 restore point 的請求，於 reporting 時發出；見 `references/reporting.md`。

## 邊界

baseline 由 Core 在 `wda init` 建立；Skill 不執行任何版本控制指令，baseline 不存在時即停止工作。權威來源：`docs/architecture.md` §2.12、ADR-023。本檔也不判斷「這次變更算不算視覺變更」，那是呼叫方（Skill 主流程與使用者對話）的判斷。
