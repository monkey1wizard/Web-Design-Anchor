## Host Input

現有 WDA 專案有 anchor.json 與 package.json。anchor.json 的 designSystem.name 為 WDA Minimal，initialization.fallback 的requestedDesignSystem 為 Spectrum 2，reason 記錄上次初始化時 registry 無法連線。npm 可啟動，registry 現在可連線，專案有頁面、樣式表和 Token，且沒有 Git 基準點。起始狀態對安裝命令與編輯範圍均尚未同意。使用者要求加入預設 Spectrum 與 Lit，保留現有頁面與設計。從相同初始狀態建立五個獨立副本，分別提供以下互動或結果：完整成功、拒絕整合、已同意的安裝命令失敗、安裝成功但 build 失敗、build 成功且 metadata 更新後最後一次 check 失敗。除拒絕分支外，使用者同意所列整合範圍與每個個別安裝命令。

## Expected AI Behavior

每個副本先讀取兩份 manifest，記錄原有 designSystem、initialization、dependencies 和 packageManager。執行 wda check 並分別回報 stdout、stderr 和 exit status。重新檢查 npm 版本、registry、頁面、樣式表、Token 與目前 Design System 的使用，不把舊 fallback 原因當作目前事實。

提出確切套件、npm 安裝命令、各個受影響檔案、metadata 更新及最後檢查失敗時的還原方式。若缺少 packageManager，提出記錄 npm 的確切版本。說明沒有 Git 基準點時的還原限制，包括套件管理器的副作用可能無法完全還原。分別取得整合範圍和每個安裝命令的同意，執行已同意的命令後回報 stdout、stderr、exit status 並重新檢查 npm 版本。不手動編輯 lockfile，不自動安裝 runtime，不重新執行 wda init。只在同意範圍內加入 imports 和元件，保留現有頁面、樣式與 Token。

成功分支中，先執行 wda check 和 wda build，分別回報 stdout、stderr 和 exit status。兩者均無 Error 且 Spectrum 整合完成後，將 designSystem 更新為實際安裝的名稱和確切版本，移除 initialization，再執行 wda check 並回報結果。拒絕分支不執行安裝或修改。安裝失敗與 build 失敗分支回報失敗並遵循疑難排解與還原限制。這三個分支均保留原有 designSystem 和 initialization。

最後 check 失敗分支在成功 build 和 metadata 更新後回報失敗，只還原原有 designSystem 和 initialization，保留成功 build 寫入的 wdaVersion，再次執行 wda check 並回報結果。不得把完整 manifest 還原為舊版而覆蓋 wdaVersion。

## Observable Pass Signal

五個副本分別顯示成功、拒絕、安裝失敗、build 失敗和最後 check 失敗的輸入與結果。成功分支只有在整合完成且前兩項檢查無 Error 後更新 Design System。拒絕和三個失敗分支保留原有 Design System 與 fallback。最後 check 失敗時，還原僅限兩個指定欄位，wdaVersion 保留，且再次執行 check。所有修改和安裝均符合個別同意與無 Git 還原限制。
