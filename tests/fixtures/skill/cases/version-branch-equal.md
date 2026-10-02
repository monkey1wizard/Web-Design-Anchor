# L3 Case: 版本比對 — 相等分支

## Host Input

1. 將 `skills/wda/` 複製為個人安裝，複製到 `~/.claude/skills/wda/`（Claude Code 2.1.260 的個人安裝路徑）。**不得**修改 repo 內的 `skills/wda/`。
2. 確認已安裝副本 `~/.claude/skills/wda/SKILL.md` frontmatter 的 `metadata.version` 維持原值 `0.1.0`，不做修改。
3. 確認 PATH 上的 `wda --version` 印出 `wda 0.1.0`（與 `Cargo.toml` 的 `version = "0.1.0"` 一致）。
4. 在一個含 `wda.json` 的合格 WDA 專案目錄開一個新的 Claude Code 對話，貼上 `tests/fixtures/skill/prompts.md` 的 should-trigger 清單裡任一句。

## Expected AI Behavior

AI 讀取已安裝副本 `SKILL.md` frontmatter 的 `metadata.version`（`0.1.0`），執行 `wda --version` 取得 `0.1.0`，兩者相等，判定為「相等」分支：不輸出任何版本不符的 Warning 或 Error，直接依路由表繼續處理使用者的原始請求。

## Observable Pass Signal

Transcript 顯示 `wda` Skill 被載入（Skill tool call，或讀取 `SKILL.md` 的紀錄），且後續文字**不含**任何版本不符的 Warning 或 Error 陳述。若 transcript 出現版本相關的 Warning/Error 文字，視為未通過。
