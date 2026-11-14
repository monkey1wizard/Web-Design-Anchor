# L3 Case: Install 分支，wda 不在 PATH

**Status: NotRun**。原生 AI host 執行結果尚未觀察。

## Host Input

1. 使用目前原始碼中與測試 CLI 配對的 `skills/wda/` 測試副本，以及同時含有效 `anchor.json` 和 `package.json` 的現有 WDA 專案。
2. 在隔離的 shell 中從 `PATH` 移除所有可找到 `wda` 的目錄。保留原本環境，並確認該 shell 找不到 `wda`。
3. 在由該 shell 啟動的新 host 對話中，於專案目錄輸入：「請檢查這個 WDA 專案。」提供可讀的官方 README。

## Expected AI Behavior

AI 嘗試 `wda --version`，記錄實際啟動錯誤，不捏造 exit status。AI 載入 `references/install.md`，先讀取可取得的官方 README 安裝指引，再依其內容提出適用選項。每個安裝命令執行前，說明命令、發布者、安裝目標、網路存取與可能的系統權限，並取得該命令的獨立同意。安裝後重新執行 `wda --version`，如實回報啟動錯誤或 stdout、stderr 與 exit status。通過版本檢查前不得宣稱已就緒。

## Observable Pass Signal

紀錄顯示固定請求、`wda --version` 的實際結果、`references/install.md` 路由與官方 README 來源。未經個別同意不執行安裝。獲同意並安裝後有版本重查，且只回報實際觀察到的程序結果。未實際執行的 host 結果維持 `NotRun`。
