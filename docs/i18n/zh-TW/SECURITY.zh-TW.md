---
title: "Security"
status: stable
updated: 2026-11-12
type: Reference
description: "WDA 的安全漏洞回報流程、支援版本與已落地的安全模型。"
tags: []
source: SECURITY.md
lang: zh-TW
source_sha256: 6e262d027ce264ad2c6800163d5f62a102bb864128e275a5d488d7fbd964f272
translated_at: 2026-11-12
---

本文件涵蓋 WDA 工具與其公開原始碼的安全漏洞回報流程，不涵蓋使用 WDA 產出的個別網站或部署環境。

## Supported Versions

公開 GitHub Releases 提供穩定版 `v0.1.3` 與 prerelease。Prerelease 是測試版本，可能變更。請至 [GitHub Releases](https://github.com/monkey1wizard/Web-Design-Anchor/releases) 查看版本及資產。

`gal-last-good` 是開發用 tag，不是 release。

安全修補一律進入私有來源儲存庫的 `main`。`docs/architecture.md` 的「Version Compatibility」一節的四級格式相容階梯只描述專案格式相容性，不代表任何安全支援等級。`Supported legacy format` 不等於安全支援層級。

## Reporting a Vulnerability

公開儲存庫目前未啟用 GitHub private vulnerability reporting，因此沒有可用的私密回報入口。請不要在公開 issue 揭露漏洞細節。

我們採 best-effort 的協調揭露方式處理回報，不提供固定天數的 SLA，也不另設 email 回報管道。

## Security Model

本節只整理已落地的安全邊界。

規則本身以 [`docs/architecture.md`](../../architecture.md) 為唯一來源，不在此重述。

### Assets

- Project contract、source tree 與 `dist/` deployment output，依 `docs/generated-project.md`、`docs/commands.md` 的「What It Creates」一節與 `docs/design-system.md` 保護其邊界與完整性。
- `package.json` 中的網站套件宣告、由外部套件管理工具擁有的 lockfile 與 `node_modules`，以及原生 TypeScript 工具快取，依 `docs/commands.md` 的「Website Packages」與「wda build」兩節保護其來源與解析結果。
- Skill package 與其 host process contract，依 `docs/skill-spec.md` 保護其責任邊界。

### Trust Roles

- Designer 是 project source 與設計決策的擁有者；AI／Skill 是依權威文件路由操作的協作者，Core CLI 是執行 validation、init、dependency 與 build 責任的工具。角色分工見 `docs/architecture.md` 的「Core Principles」一節、`docs/commands.md` 與 `docs/skill-spec.md`。
- AI host 是執行 Skill 的外部環境，不被視為 WDA 的可信 runtime；其能力假設限於執行 native binary 並捕捉 stdout、stderr、exit status，見 `docs/skill-spec.md`。

### Trust Boundaries

- Designer／AI／Skill 與 WDA Core CLI 之間以 CLI 責任與輸出契約分隔，見 `docs/commands.md` 的「Diagnostic Output」一節。
- Source、generated `dist/` 與 deployment environment 是分離的樹與責任邊界，見 `docs/generated-project.md`。
- Skill 與 AI host 之間是安裝副本與 process contract 邊界；`scripts/` 與 `assets/` 不屬於 Skill 產物，見 `docs/skill-spec.md`。
- 私有 GitHub 來源儲存庫與公開 GitHub 發布儲存庫是兩個不同的信任邊界。公開儲存庫取得無父精選快照。匯出程式會排除私有內容並掃描殘留；這份快照不是私有儲存庫的鏡像。
- 私有 controller 只在必要的 Linux 與 Windows 檢查通過後發布快照。公開 `main` 與匹配的版本 tag 必須在同一次 Git 原子推送中指向同一個快照 commit。既有版本 tag 不得覆寫。
- 跨儲存庫憑證只提供給發布步驟的暫時性 askpass helper。憑證不得寫入 remote URL、精選快照或儲存庫設定。
- 公開 release workflow 只接受公開儲存庫的版本 tag。它建置六個目標，產生 checksums 與 cosign 簽章，再建立 GitHub Release。穩定版才執行 Homebrew 與 WinGet 發布階段。
- TypeScript 工具是每個 WDA 發行版內嵌的單一官方版本，並以每個平台套件的 `sha512` SRI 綁定。內嵌的 SRI 是工具唯一的信任根，專案無法覆寫。快取項目在解壓前會以該 SRI 驗證，建置永不下載工具。
- 安裝網站套件會透過 npm 或 Bun 等外部套件管理工具執行第三方程式碼。這個步驟與 `wda check`、`wda build` 分開，後兩者永不安裝套件，也不使用網路。WDA 不承諾第三方安裝腳本的副作用可以還原。

### Invariants

- 只接受 `docs/architecture.md` 的「Stability Layers」一節所定義的長期 contract；`dist/` 是唯一 deployment output，並遵守 `docs/generated-project.md`。
- `anchor.json` 是 strict WDA 設定合約。`package.json` 是唯一宣告網站套件的地方，lockfile 由外部套件管理工具擁有。WDA 不解析版本，見 `docs/commands.md` 的「What It Creates」與「Website Packages」兩節。
- Skill 不成為第二套規格來源，且 `scripts/` 不建立；其 host 能力限於 native binary process contract，見 `docs/skill-spec.md`。

### Attack Surface

- 不可信輸入 parser 是 `Cargo.toml` 中的 `jsonschema`、`json-sourcemap`、`marked-yaml`、`html5gum` 與 `serde_json` 依賴。它們分別涵蓋 schema／JSON、YAML frontmatter 與 HTML 輸入，對應 `docs/commands.md` 的「Diagnostic Output」一節、「What It Creates」一節與文件／source 邊界。
- `serde_json` 直接解析 `anchor.json`、`package.json` 與 generated `tokens.json`；這些輸入分別落在 `docs/commands.md` 的「What It Creates」與「Website Packages」兩節，以及 `docs/design-system.md`。
- `wda init` 會寫入 `.git/`、`.gitignore`、`README.md`、`anchor.json`、`package.json`、`docs/architecture.md`、`docs/design.md`、`docs/glossary.md`、`pages/index.html` 與 `tokens/tokens.json`。Spectrum 2 路徑另會寫入 `scripts/main.ts`。只有 `init` 所執行的套件管理工具會寫入它自己的 lockfile 與 `node_modules`。其碰撞、rollback 與本地 Git 行為依 `docs/commands.md` 的「wda init」一節。
- 外部套件管理工具（例如 npm 或 Bun）只在 `wda init` 安裝預設套件時執行。管理工具會解析版本並執行它自己的 lifecycle script。WDA 不執行自己的套件解析器。
- Spectrum online resolution 會透過使用者指定的外部套件管理工具，從 registry 解析 exact version；其 Design System 來源與離線退回見 `docs/design-system.md`。
- Skill host capability 僅為執行 native binary、捕捉 stdout／stderr／exit status；`scripts/` 為空且禁止建立，見 `docs/skill-spec.md`。

## Security Verification Limits

每個 GitHub Release 的 build、簽章與驗證結果，應以該次 Release 連結的 Actions 執行紀錄和資產為準。Prerelease 不會執行 Homebrew 發布或 WinGet 提交；這些套件管理器路徑屬於穩定版發布流程。

私有來源儲存庫採 GitHub Free 與單一維護者信任模型。可修改私有 workflow 與其 secret 的維護者，也能授權發布。GitLab 備份的維護政策由私有維護程序管理。
