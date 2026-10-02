# L3 Case: 版本比對 — Skill 較新分支

## Host Input

1. 將 `skills/wda/` 複製為個人安裝，複製到 `~/.claude/skills/wda/`（Claude Code 2.1.260 的個人安裝路徑）。**不得**修改 repo 內的 `skills/wda/`。
2. 編輯已安裝副本 `~/.claude/skills/wda/SKILL.md` frontmatter，把 `metadata.version` 從 `0.1.0` 改成 `0.2.0`。
3. 確認 PATH 上的 `wda --version` 仍印出 `wda 0.1.0`（PATH 上的 CLI 不變，只改已安裝副本的 frontmatter）。
4. 在一個含 `wda.json` 的合格 WDA 專案目錄開一個新的 Claude Code 對話，貼上 `tests/fixtures/skill/prompts.md` 的 should-trigger 清單裡任一句。

## Expected AI Behavior

AI 比對已安裝副本 frontmatter 的 `metadata.version`（`0.2.0`）與 `wda --version`（`0.1.0`），判定 Skill 版本較新：輸出一則 Error，明講必須更新，並**停止**，不得靜默降級為盡力而為的 role-play（也就是不得改用一般網頁知識繼續編輯專案）。

## Observable Pass Signal

Transcript 顯示 `wda` Skill 被載入，且出現一則明確的 Error 陳述 Skill 版本較新、需要更新；Error 之後 AI **未**呼叫任何 `wda` 子指令或編輯專案檔案，且未以一般網頁知識繼續處理使用者原始請求。若 transcript 未出現 Error，或 Error 後仍繼續動作，視為未通過。
