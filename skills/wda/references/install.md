# install（安裝路由）

## 載入時機

只在 `wda` 不在 PATH 時載入本檔。判斷依據是呼叫 `wda`（例如 `wda --version`）時得到的失敗結果：shell 回報「command not found」（或 Windows 上的等義訊息）、或呼叫直接無法啟動，都算作「不在 PATH」的偵測訊號。exit code 非零但指令本身確實啟動（例如 `wda check` 因專案問題而失敗）不算這個情況，屬於 `references/troubleshooting.md` 的範圍。

## 本檔只做兩件事

1. 說明如何從失敗的呼叫辨識出「`wda` 不在 PATH」。
2. 指向 Core 的安裝文件 `docs/install.md`，由該文件承載實際安裝程序。

本檔不寫任何安裝指令、不寫任何下載步驟。安裝程序的權責在 `docs/install.md`（由 P9 撰寫與維護），本 Skill 不得複製、改寫或代為決定安裝方式。

## 動作

偵測到「`wda` 不在 PATH」後，讀取並依循 `docs/install.md`。若 `docs/install.md` 當下不存在或找不到，停止並回報這個缺口給使用者，不得自行編造安裝指令或下載步驟。
