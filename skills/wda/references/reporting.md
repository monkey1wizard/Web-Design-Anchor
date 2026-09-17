# reporting（回報義務）

## 載入時機

只在觸發回報義務時載入本檔。觸發時機包含：`wda build` 執行完畢、任一 `wda` 呼叫的 stderr 出現 Warning、或需要判斷 host 有沒有內嵌預覽能力。除此之外不載入。

## 本檔只做一件事

規定觸發回報義務時必須回報的三類資訊：`wda build` 成功後的 `dist/` 絕對路徑、stderr 出現的任何 Warning、預覽結果（包含預覽位置、驗證內容，或 host 不具預覽能力時的明示降級）。權威來源：`docs/architecture.md` §2.12 Preview。

## 動作（依序執行）

1. **每次 `wda` 呼叫都要捕捉 stdout、stderr 與 exit status。** 即使指令以 exit 0 結束，也要讀 stderr，因為 Warning 只出現在 stderr。exit code 本身不代表成功，不得只看 exit code 或只看 stdout 判斷。
2. **`wda build` 成功後，回報 `dist/` 的絕對路徑。** 這是最低保證：任何能執行 `wda` 的 host 都具備接收這行輸出的能力，不得省略。
3. **stderr 出現 Warning 時，回報該 Warning。** exit 0 加上 stderr 有 Warning，仍要回報 Warning 本身，不得回報「一切正常」。
4. **技術檢查通過不等於設計完成。** 每次回報視覺變更都要附上預覽位置，並明確寫出「尚未經 Designer 看過，不算已接受」；未經使用者確認看過的變更，不計為已接受狀態。
5. **`wda build` 成功後，依 host 能力執行預覽。** 回報 `dist/` 的絕對路徑，並指出其中的 `index.html` 是入口。預覽必須服務專案根目錄，絕不以 `dist/` 作為 document root；只繫結 loopback，並使用 HTTP/1.1 keep-alive 或更新版本。host 有預覽能力時，用該能力開啟頁面，至少確認第一個 viewport；若 screenshot 不可靠，改以 DOM measurement 驗證，並說明仍須由 Designer 確認畫面。AI 不自行啟動長駐 server。
6. **host 沒有預覽能力時，明講降級。** 同時交付 `dist/` 的絕對路徑與上述四項規則，不得在使用者未看到變更的情況下宣告視覺變更完成。這是明示降級，不是靜默略過。
7. **狀態被接受後，請 Designer 將目前狀態記錄為新的 restore point。** 機制見 `references/reversibility.md`，本檔不重述。權威來源：`docs/architecture.md` §2.12。

## 邊界

本檔不判斷 host 是否具備預覽能力；呼叫方（Skill 主流程）依實際 host 環境選擇「開啟並驗證」或「明示降級」分支，本檔規定兩個分支各自的回報內容。本檔也不提供 `file://` fallback 或版本控制建議，那些分別是 Core 的既定限制與 `references/reversibility.md` 的權責。`check` 或 `build` 回報 Error 時的診斷程序不在本檔，路由到 `references/troubleshooting.md`。權威來源：`docs/architecture.md` §2.12。
