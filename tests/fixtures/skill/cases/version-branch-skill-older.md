# L3 Case: 版本比對 — Skill 較舊分支

## Host Input

1. 將 `skills/wda/` 複製為個人安裝，複製到 `~/.claude/skills/wda/`（Claude Code 2.1.260 的個人安裝路徑）。**不得**修改 repo 內的 `skills/wda/`。
2. 編輯已安裝副本 `~/.claude/skills/wda/SKILL.md` frontmatter，把 `metadata.version` 從 `0.1.0` 改成 `0.0.9`。
3. 確認 PATH 上的 `wda --version` 仍印出 `wda 0.1.0`（PATH 上的 CLI 不變，只改已安裝副本的 frontmatter）。
4. 在一個含 `wda.json` 的合格 WDA 專案目錄開一個新的 Claude Code 對話，貼上 `tests/fixtures/skill/prompts.md` 的 should-trigger 清單裡任一句。

## Expected AI Behavior

AI 比對已安裝副本 frontmatter 的 `metadata.version`（`0.0.9`）與 `wda --version`（`0.1.0`），判定 Skill 版本較舊：輸出一則 Warning，說明 CLI 向後相容、Skill 只是少用到新能力，接著**繼續**處理使用者的原始請求，不停止。

## Observable Pass Signal

Transcript 顯示 `wda` Skill 被載入，且出現一則明確的 Warning 陳述 Skill 版本較舊；Warning 之後 AI 繼續依路由表往下執行（例如呼叫 `wda check`/`wda build` 或回應使用者的原始請求），而非停在 Warning 上不動。若 transcript 未出現 Warning，或出現 Warning 後未繼續處理，視為未通過。
