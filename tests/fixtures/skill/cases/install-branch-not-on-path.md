# L3 Case: Install 分支 — wda 不在 PATH

**Status: not-run** — 阻塞於發佈套件計畫尚未完成，`wda` 執行檔還沒有可安裝的發佈物。本 fixture 目前只凍結建構步驟；發佈物落地後，由後續的執行任務實際跑這個 fixture，並把 `not-run` 換成通過/失敗結論。

## Host Input

1. 將 `skills/wda/` 複製為個人安裝，複製到 `~/.claude/skills/wda/`（Claude Code 2.1.260 的個人安裝路徑），frontmatter `metadata.version` 維持原值 `0.1.0`，不做修改。
2. 開一個新的 shell，把 `wda` 執行檔所在目錄從 `PATH` 移除（例如以一個不含 `wda` 的乾淨 `PATH` 值啟動 shell），使該 shell 內 `wda --version` 回報「command not found」或等義錯誤。
3. 在此 shell 啟動的 Claude Code 對話中，於一個含 `wda.json` 的合格 WDA 專案目錄，貼上 `tests/fixtures/skill/prompts.md` 的 should-trigger 清單裡任一句。

## Expected AI Behavior

AI 嘗試執行 `wda --version`（或其他 `wda` 子指令）失敗，判定 `wda` 不在 PATH，依決策路由表載入 `references/install.md`；`references/install.md` 只做偵測與路由，不含安裝步驟本身，因此 AI 接著把使用者導向 `README.md` 的「安裝」一節。AI 可以逐字引用 `README.md`「安裝」一節中的安裝指令，但不得展示來自 README 以外任何來源的安裝指令，也不得自行發明安裝指令。

## Observable Pass Signal

Transcript 顯示 `wda` Skill 被載入後，出現讀取 `references/install.md` 的紀錄或其路由文字，且最終文字包含指向 `README.md`「安裝」一節的說明。通過條件一：AI 可以逐字引用 README 該小節中的安裝指令。通過條件二：AI 未展示任何來自 README 以外來源的安裝指令，也未自行生成安裝指令。README 以外的指令例子包括 `curl`、`wget`、`cargo install`、`Invoke-WebRequest`、`winget`、`scoop`、`brew`。若 AI 略過 `references/install.md` 直接自行生成安裝步驟，或完全未提及 `README.md` 的「安裝」一節，視為未通過。
