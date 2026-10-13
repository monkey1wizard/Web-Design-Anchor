本文件涵蓋 WDA 工具與其公開原始碼的安全漏洞回報流程，不涵蓋使用 WDA 產出的個別網站或部署環境。

## Supported Versions

公開 GitHub Releases 提供穩定版 `v0.1.0` 與 prerelease。Prerelease 是測試版本，可能變更。請至 [GitHub Releases](https://github.com/monkey1wizard/Web-Design-Anchor/releases) 查看版本及資產。

`gal-last-good` 是開發用 tag，不是 release。

第一次正式發行前，安全修補一律進入私有來源儲存庫的 `main`。`docs/architecture.md` 的「版本相容性」一節的四級格式相容階梯只描述專案格式相容性，不代表任何安全支援等級。`Supported legacy format` 不等於安全支援層級。

## Reporting a Vulnerability

公開儲存庫目前未啟用 GitHub private vulnerability reporting，因此沒有可用的私密回報入口。請不要在公開 issue 揭露漏洞細節。

我們採 best-effort 的協調揭露方式處理回報，不提供固定天數的 SLA，也不另設 email 回報管道。

## Security Model

本節只整理已落地的安全邊界。

規則本身以 [`docs/architecture.md`](docs/architecture.md) 為唯一來源，不在此重述。

### Assets

- Project contract、source tree 與 `dist/` deployment output，依 `docs/generated-project.md`、其「wda.json」一節與 `docs/design-system.md` 保護其邊界與完整性。
- Dependency declaration、lock state 與 build-time composition，依 `docs/commands.md` 的「wda build」一節與 `docs/commands.md` 的「wda deps」一節保護其來源與解析結果。
- Skill package 與其 host process contract，依 `docs/skill-spec.md` 保護其責任邊界。

### Trust Roles

- Designer 是 project source 與設計決策的擁有者；AI／Skill 是依權威文件路由操作的協作者，Core CLI 是執行 validation、init、dependency 與 build 責任的工具。角色分工見 `docs/architecture.md` 的「核心原則」一節、`docs/commands.md` 與 `docs/skill-spec.md`。
- AI host 是執行 Skill 的外部環境，不被視為 WDA 的可信 runtime；其能力假設限於執行 native binary 並捕捉 stdout、stderr、exit status，見 `docs/skill-spec.md`。

### Trust Boundaries

- Designer／AI／Skill 與 WDA Core CLI 之間以 CLI 責任與輸出契約分隔，見 `docs/commands.md` 的「診斷輸出」一節。
- Source、generated `dist/` 與 deployment environment 是分離的樹與責任邊界，見 `docs/generated-project.md`。
- Skill 與 AI host 之間是安裝副本與 process contract 邊界；`scripts/` 與 `assets/` 不屬於 Skill 產物，見 `docs/skill-spec.md`。
- 私有 GitHub 來源儲存庫與公開 GitHub 發布儲存庫是兩個不同的信任邊界。公開儲存庫取得無父精選快照。匯出程式會排除私有內容並掃描殘留；這份快照不是私有儲存庫的鏡像。
- 私有 controller 只在必要的 Linux 與 Windows 檢查通過後發布快照。公開 `main` 與匹配的版本 tag 必須在同一次 Git 原子推送中指向同一個快照 commit。既有版本 tag 不得覆寫。
- 跨儲存庫憑證只提供給發布步驟的暫時性 askpass helper。憑證不得寫入 remote URL、精選快照或儲存庫設定。
- 公開 release workflow 只接受公開儲存庫的版本 tag。它建置六個目標，產生 checksums 與 cosign 簽章，再建立 GitHub Release。穩定版才執行 Homebrew 與 WinGet 發布階段。

### Invariants

- 只接受 `docs/architecture.md` 的「穩定度分層」一節 所定義的長期 contract；`dist/` 是唯一 deployment output，並遵守 `docs/generated-project.md`。
- `wda.json` 是 strict project contract，dependency graph 由 `package.json` 宣告並由 `deno.lock` 鎖定，見 `docs/generated-project.md` 的「wda.json」一節與 `docs/commands.md` 的「wda deps」一節。
- Skill 不成為第二套規格來源，且 `scripts/` 不建立；其 host 能力限於 native binary process contract，見 `docs/skill-spec.md`。

### Attack Surface

- 不可信輸入 parser 位於 `Cargo.toml` 15–19：`jsonschema`、`json-sourcemap`、`marked-yaml`、`html5gum`、`serde_json`。它們分別涵蓋 schema／JSON、YAML frontmatter 與 HTML 輸入，對應 `docs/commands.md` 的「診斷輸出」一節、`docs/generated-project.md` 的「wda.json」一節與文件／source 邊界。
- `serde_json` 直接解析 `wda.json`、`package.json`、generated `tokens.json` 與 `deno.lock`；這些輸入分別落在 `docs/generated-project.md` 的「wda.json」一節、`docs/design-system.md` 與 `docs/commands.md` 的「wda deps」一節。
- `wda init` 寫入 `.gitignore`、`README.md`、`wda.json`、`docs/architecture.md`、`docs/design.md`、`docs/naming.md`、`pages/index.html` 與 `tokens/tokens.json`；其碰撞、rollback 與本地 Git 行為依 `docs/generated-project.md` 與 `docs/commands.md`。
- `deno run -A npm:esbuild@…` 會下載工具並啟動 all-permissions subprocess；這是 P5／P6 已落地的 build-time boundary，相關責任依 `docs/architecture.md` 的「穩定度分層」一節與 `docs/commands.md` 的「wda deps」一節。
- Spectrum online resolution 會透過 Deno 解析 npm registry 的 exact version；其 Design System 來源與 offline fallback 見 `docs/design-system.md`。
- Skill host capability 僅為執行 native binary、捕捉 stdout／stderr／exit status；`scripts/` 為空且禁止建立，見 `docs/skill-spec.md`。

## Security Verification Limits

每個 GitHub Release 的 build、簽章與驗證結果，應以該次 Release 連結的 Actions 執行紀錄和資產為準。Prerelease 不會執行 Homebrew 發布或 WinGet 提交；這些套件管理器路徑屬於穩定版發布流程。

私有來源儲存庫採 GitHub Free 與單一維護者信任模型。可修改私有 workflow 與其 secret 的維護者，也能授權發布。GitLab 備份的維護政策由私有維護程序管理。
