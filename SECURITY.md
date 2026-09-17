本文件涵蓋 WDA 工具與其公開原始碼的安全漏洞回報流程，不涵蓋使用 WDA 產出的個別網站或部署環境。

## Supported Versions

目前尚未發行任何版本。`gal-last-good` 是開發用 tag，不是 release。

在 release pipeline 落地前，安全修補一律進入 `main`。`docs/architecture.md` §2.10 的四級格式相容階梯只描述專案格式相容性，不代表任何安全支援等級；`Supported legacy format` 不等於安全支援層級。

## Reporting a Vulnerability

目前唯一的漏洞回報管道是 GitHub Private Security Advisories。這個管道會與公開 GitHub 倉庫同時開通；在公開 GitHub 倉庫建立前，尚無可用的回報入口。

我們採 best-effort 的協調揭露方式處理回報，不提供固定天數的 SLA，也不另設 email 回報管道。

## Security Model

本節只整理已落地的安全邊界；規則本身以 [`docs/architecture.md`](docs/architecture.md) 為唯一來源，不在此重述。

### Assets

- Project contract、source tree 與 `dist/` deployment output，依 [`docs/architecture.md` §2.4](docs/architecture.md#24-source-與-deployment)、`docs/architecture.md` §2.9 與 [`§2.5`](docs/architecture.md#25-design-system-與-tokens) 保護其邊界與完整性。
- Dependency declaration、lock state 與 build-time composition，依 [`docs/architecture.md` §2.6](docs/architecture.md#26-components-與-build-time-composition) 與 [`§2.7`](docs/architecture.md#27-dependencies) 保護其來源與解析結果。
- Skill package 與其 host process contract，依 [`docs/architecture.md` §2.11](docs/architecture.md#211-skill-層) 保護其責任邊界。

### Trust Roles

- Designer 是 project source 與設計決策的擁有者；AI／Skill 是依權威文件路由操作的協作者，Core CLI 是執行 validation、init、dependency 與 build 責任的工具。角色分工見 [`docs/architecture.md` §2.1](docs/architecture.md#21-核心原則)、[`§2.3`](docs/architecture.md#23-cli-責任) 與 [`§2.11`](docs/architecture.md#211-skill-層)。
- AI host 是執行 Skill 的外部環境，不被視為 WDA 的可信 runtime；其能力假設限於執行 native binary 並捕捉 stdout、stderr、exit status，見 [`docs/architecture.md` §2.11](docs/architecture.md#skill-boundary)。

### Trust Boundaries

- Designer／AI／Skill 與 WDA Core CLI 之間以 CLI 責任與輸出契約分隔，見 [`docs/architecture.md` §2.3](docs/architecture.md#23-cli-責任) 與 [`§2.8`](docs/architecture.md#28-診斷與嚴重度)。
- Source、generated `dist/` 與 deployment environment 是分離的樹與責任邊界，見 [`docs/architecture.md` §2.4](docs/architecture.md#24-source-與-deployment)。
- Skill 與 AI host 之間是安裝副本與 process contract 邊界；`scripts/` 與 `assets/` 不屬於 Skill 產物，見 [`docs/architecture.md` §2.11](docs/architecture.md#skill-boundary)。

### Invariants

- 只接受 [`docs/architecture.md` §2.2](docs/architecture.md#22-分層) 所定義的長期 contract；`dist/` 是唯一 deployment output，並遵守 [`§2.4`](docs/architecture.md#24-source-與-deployment)。
- `wda.json` 是 strict project contract，dependency graph 由 `package.json` 宣告並由 `deno.lock` 鎖定，見 `docs/architecture.md` §2.9 與 [`§2.7`](docs/architecture.md#27-dependencies)。
- Skill 不成為第二套規格來源，且 `scripts/` 不建立；其 host 能力限於 native binary process contract，見 [`docs/architecture.md` §2.11](docs/architecture.md#skill-boundary)。

### Attack Surface

- 不可信輸入 parser 位於 `Cargo.toml` 15–19：`jsonschema`、`json-sourcemap`、`marked-yaml`、`html5gum`、`serde_json`。它們分別涵蓋 schema／JSON、YAML frontmatter 與 HTML 輸入，對應 [`docs/architecture.md` §2.8](docs/architecture.md#28-診斷與嚴重度)、`docs/architecture.md` §2.9 與文件／source 邊界。
- `serde_json` 直接解析 `wda.json`、`package.json`、generated `tokens.json` 與 `deno.lock`；這些輸入分別落在 `docs/architecture.md` §2.9、[`§2.5`](docs/architecture.md#25-design-system-與-tokens) 與 [`§2.7`](docs/architecture.md#27-dependencies)。
- `wda init` 寫入 `.gitignore`、`README.md`、`wda.json`、`docs/architecture.md`、`docs/design.md`、`docs/naming.md`、`pages/index.html` 與 `tokens/tokens.json`；其碰撞、rollback 與本地 Git 行為依 [`docs/architecture.md` §2.4](docs/architecture.md#24-source-與-deployment) 與 [`§2.3`](docs/architecture.md#23-cli-責任)。
- `deno run -A npm:esbuild@…` 會下載工具並啟動 all-permissions subprocess；這是 P5／P6 已落地的 build-time boundary，相關責任依 [`docs/architecture.md` §2.2](docs/architecture.md#22-分層) 與 [`§2.7`](docs/architecture.md#27-dependencies)。
- Spectrum online resolution 會透過 Deno 解析 npm registry 的 exact version；其 Design System 來源與 offline fallback 見 [`docs/architecture.md` §2.5](docs/architecture.md#25-design-system-與-tokens)。
- Skill host capability 僅為執行 native binary、捕捉 stdout／stderr／exit status；`scripts/` 為空且禁止建立，見 [`docs/architecture.md` §2.11](docs/architecture.md#skill-boundary)。

## Not Yet Covered

P9 `release-pipeline` 尚未落地：目前沒有公開 GitHub repository、release artifact 或 artifact verification。因此本文件的 GitHub Private Security Advisories 管道仍是待開通的聲明；P9 落地時必須重寫本文件。
