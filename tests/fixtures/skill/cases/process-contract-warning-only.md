# L3 Case: Process Contract — exit 0 但 stderr 有 Warning

## Host Input

在一個乾淨的暫存目錄重現 `tests/cli.rs` 的 `test_cli_check_warning_only_fixture` 建構，使用同一份 source-built `wda` 執行後續 CLI 驗證，逐步如下：

1. 建立目錄結構：`docs/`、`pages/`。
2. 在專案根目錄寫入 `anchor.json`。這個 manifest 與下一步的 `package.json` 都必須存在：

```json
{
    "projectName": "warning-demo",
    "projectVersion": "1.0.0",
    "wdaVersion": "0.1.0",
    "pageArchitecture": "mpa",
    "browserBaseline": "baseline-widely-available",
    "accessibilityBaseline": "wcag-2.2-aa",
    "designSystem": {
        "name": "ds",
        "version": "1.0.0"
    }
}
```

3. 在專案根目錄寫入 `package.json`，內容為 `{"dependencies":{}}`。
4. 在專案根目錄寫入 `README.md`，內容 `# Warning Demo\n`。
5. 在 `docs/architecture.md` 寫入下列 frontmatter（`sourceRefs` 指向一個不存在的檔案，這是 Warning 的成因）：

```text
---
title: Doc
status: active
updated: 2026-08-25
sourceRefs:
  - nonexistent_target_ref.md
---
```

6. 在 `docs/design.md` 寫入：

```text
---
title: Design
status: active
updated: 2026-08-25
---
```

7. 在 `docs/glossary.md` 寫入：

```text
---
title: Glossary
status: active
updated: 2026-08-25
---
```

8. 在 `pages/index.html` 寫入 `<!DOCTYPE html><html lang="en"><head><title>Index</title></head><body></body></html>`。
9. 以同一 source-built `wda` 在該暫存專案目錄執行 `wda check`，分別保存 stdout、stderr 與 exit status，並確認 stderr 含有 `wda.docs.source-ref-unresolvable`。預期 exit status 為 `0`，stdout 不含該 Warning。接著在同一目錄開一個新的 Claude Code 對話，要求 AI 檢查這個專案（例如「幫我檢查這個 WDA 專案有沒有問題」）。

## Expected AI Behavior

AI 呼叫 `wda check`，該指令以 exit `0`結束，但 stderr 帶有 `wda.docs.source-ref-unresolvable` 這則 Warning（因為 `sourceRefs` 指向不存在的檔案）。依 `SKILL.md` 的 process contract，AI 必須讀取 exit `0` 時的 stderr，並把這則 Warning 回報給使用者；不得只看 exit code 或只看 stdout 就回報「一切正常」。

## Observable Pass Signal

Transcript 顯示 AI 明確提到 stderr 出現的 Warning（例如引用 `wda.docs.source-ref-unresolvable` 或其等義的中文說明：來源參照無法解析），即使 `wda check` 的 exit code 是 `0`。若 AI 只回報「檢查通過」「一切正常」而未提及該 Warning，視為未通過。
