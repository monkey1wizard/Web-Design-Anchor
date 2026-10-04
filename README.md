# Web Design Anchor

Web Design Anchor（WDA）是專為建立「最小 Web」而生的 CLI 工具。它為所有想快速建立網站的人設計，使用者只要與 AI 對話，就能快速完成現代 Web 設計。

WDA 提供標準化的 CLI 與專案規範，並以專案內的 `design.md` 定錨設計邏輯。AI 每次產出元件時，都能嚴格遵循正確的 Design System 與 Token 規格，省去過去繁瑣的溝通成本。

在技術架構上，WDA 回歸標準 Web，不依賴前端框架。HTML5 與 Modern CSS 的原生能力，足以處理豐富的版面與動態特效；腳本層則採用 TypeScript，讓程式碼好維護、不混亂。

在建置與交付上，WDA 選用 Deno 快速轉譯 TypeScript 腳本與專案資產，並統一輸出至 `dist/` 目錄。每位使用者搭配的 Coding Agent 或 IDE 各不相同，因此 WDA 本身不內建預覽伺服器，預覽交由使用者的環境自行處理。`dist/` 產出的是純靜態、零依賴的網頁，使用者能直接檢視成果，隨時以此部署。

## 安裝與首次使用

WDA 使用兩種不同層級的工具。Git 與 Deno 是電腦層級工具，兩者都必須在 PATH 中。Lit 是每個 WDA 專案自己的相依項目，不是全域工具。WDA 不需要 Node.js。

本節在 2026-09-24 依 Git 與 Deno 的官方安裝頁重新查核。安裝指令會下載軟體，且部分指令可能要求系統管理員權限。執行前，先確認指令、發行者、安裝範圍與網路存取符合你的預期。請一次只執行一個指令，並在完成後驗證。

### 先安裝 Git 與 Deno

請依作業系統選擇一組偏好路徑。這些指令只安裝 Git 或 Deno，不會建立 WDA 專案。

| 平台 | Git 偏好路徑 | Deno 偏好路徑 | 官方替代來源 |
| --- | --- | --- | --- |
| Windows | `winget install --id Git.Git -e --source winget` | `winget install DenoLand.Deno` | [Git for Windows 安裝程式](https://git-scm.com/install/windows) 與 [Deno Windows 安裝選項](https://docs.deno.com/runtime/getting_started/installation/) |
| macOS | `brew install git` | `brew install deno` | [Git macOS 安裝選項](https://git-scm.com/download/mac) 與 [Deno 安裝選項](https://docs.deno.com/runtime/getting_started/installation/) |
| Debian／Ubuntu | `sudo apt-get install git` | `curl -fsSL https://deno.land/install.sh \| sh` | [Git Linux 安裝選項](https://git-scm.com/download/linux) 與 [Deno 手動下載](https://docs.deno.com/runtime/getting_started/installation/#manual-download) |
| Fedora | `sudo dnf install git` | `curl -fsSL https://deno.land/install.sh \| sh` | [Git Linux 安裝選項](https://git-scm.com/download/linux) 與 [Deno 手動下載](https://docs.deno.com/runtime/getting_started/installation/#manual-download) |

如果偏好路徑的套件管理工具不存在，請使用同列的官方替代來源。不要為了安裝 WDA 而先安裝另一個套件管理工具。Deno 的 Linux 指令會從 Deno 官方網站下載並執行安裝程式，預設會將 Deno 放在使用者目錄。Deno 官方文件也指出，安裝後通常必須開啟新的終端機，新的 PATH 才會生效。

安裝每個工具後，分別執行：

```sh
git --version
```

```sh
deno --version
```

只有指令成功輸出版本時，才代表該工具已能從目前的 PATH 啟動。安裝程式顯示成功不代表目前的終端機已可使用它。如果指令找不到工具，請先開啟新終端機再檢查。若工具可找到但無法啟動，請記錄實際錯誤，並不要把它誤判成尚未安裝。

### WDA 發布狀態

WDA 的安裝途徑必須和 Git、Deno 分開判斷。公開 GitHub 儲存庫已有 prerelease 與對應的發行資產。Prerelease 用於測試，可能變更，且不代表穩定版已發布。請在 [GitHub Releases](https://github.com/monkey1wizard/Web-Design-Anchor/releases) 查看可用版本及其資產；目前沒有穩定版可用。

儲存庫已具備 GitHub 發布鏈。私有來源儲存庫的 controller 會先在 Linux 與 Windows 執行必要檢查，再將同一個精選快照 commit 的公開 `main` 與版本 tag 原子推送至公開儲存庫。公開儲存庫的 release workflow 接著建置、簽署及驗證發行資產。精選快照不含私有維護文件或私有 controller workflow。安全與供應鏈邊界詳見 `SECURITY.md`。私有來源儲存庫的維護者操作程序位於 `.dev/release.md`，該檔不會進入公開快照。

你可以從 Releases 頁面下載符合平台的 `wda-<版本>-<平台>-<架構>` 資產與 `checksums.txt`。Linux 與 macOS 可使用 `packaging/install.sh`，Windows 可使用 `packaging/install.ps1`。安裝程式預設尋找最新穩定版；若要安裝 prerelease，請依該 Release 頁面所列的版本設定 `WDA_VERSION`，再執行安裝程式。安裝程式會驗證資產的 SHA-256 checksum；若安裝環境有 cosign，且簽章檔可取得，也會驗證簽章。

有 Rust 工具鏈的使用者也可從公開 GitHub 儲存庫安裝 Cargo 套件：

```sh
cargo install --git https://github.com/monkey1wizard/Web-Design-Anchor wda --locked
```

這個 Cargo 指令使用公開儲存庫的目前 `main`，不會選擇特定 prerelease。Homebrew 與 WinGet 僅用於穩定版發布流程，目前不提供穩定版套件安裝途徑。

完成安裝後，執行：

```sh
wda --version
```

### 沒有 WDA Skill 時，請讓 AI 依這個順序協助

如果 AI 沒有 WDA Skill，請先提供本 README 或另一份可讀取的官方 WDA 來源。AI 不能讀取來源時，必須請你提供可存取的來源，然後停止。AI 不應自行猜測安裝指令。

1. AI 先以唯讀方式執行 `git --version`、`deno --version` 與 `wda --version`。
2. AI 對每個已啟動的指令記錄 stdout、stderr 與結束狀態。指令無法啟動時，AI 只記錄啟動錯誤，不得虛構結束狀態。
3. AI 用一份摘要分開說明「可用」、「PATH 中找不到」與「找到但無法啟動」。工具無法啟動本身不是安裝建議的依據。
4. 工具確實不在 PATH 時，AI 從本節選出一個精確指令，說明發行者、安裝目標、網路存取與可能的系統權限，再為該指令單獨詢問一次同意。AI 不得要求密碼。
5. 使用者同意後，AI 才能執行該一個指令，並立即重新執行相同的 `--version` 檢查。每個後續安裝或專案建立動作都需要新的同意。
6. 使用者只要求安裝時，AI 在工具驗證後停止，不得執行 `wda init`。使用者明確要求建立專案時，AI 必須再次詢問同意，且只有 Git 可用時才可執行 `wda init`。

Deno 尚未可用時，WDA 的功能受限。`wda check` 仍可使用，且 `wda init` 可建立 WDA Minimal 專案，但不能解析線上 Design System、安裝 Lit、執行 `wda deps` 或執行 `wda build`。AI 必須清楚說明這個限制，並另外取得建立專案的同意。

建立專案後，AI 必須分開回報專案是否建立，以及 Lit 的狀態。WDA 會嘗試在專案內加入 Lit。只有在 `deno.jsonc` 宣告 `lit` 且 `deno.lock` 存在時，才能稱為「已宣告並鎖定」。這不代表 registry 一定可連線。Lit 需要修復時，AI 必須先取得新的同意，才可以執行 `wda deps add lit`。

## 文件導覽

| 想了解的內容 | 參閱文件 |
| :--- | :--- |
| WDA 由什麼組成、邊界在哪、開發驗證與文件規則 | `docs/architecture.md` |
| `wda init` 產出的專案：`wda.json` 欄位、專案樹、`dist/` 對應 | `docs/generated-project.md` |
| 四個指令做什麼、診斷輸出怎麼讀 | `docs/commands.md` |
| Design System 的預設與退回、Token、Adapter 邊界 | `docs/design-system.md` |
| Skill 能放什麼、如何驗收 | `docs/skill-spec.md` |
| 術語與詞彙定義 | `docs/glossary.md` |
| 哪些決策已裁定與其背景原因 | `docs/adr/` |
| WDA Minimal 與 Spectrum 的實例定義 | `docs/design-systems/` |
| `wda init` 寫進專案的文件範本 | `docs/templates/generated-project/` |
| 安全模型涵蓋範圍與漏洞回報方式 | `SECURITY.md` |
| AI 如何操作 `wda` | `skills/wda/SKILL.md` |

## 授權

Apache-2.0。第三方素材與元件授權請參閱 `NOTICE`。
產生的 `docs/design.md` 第 10 節「Craft Floor」改寫自 [impeccable](https://github.com/pbakaus/impeccable)，只採用其中一小部分判準，授權為 Apache-2.0。需要完整設計流程的讀者可參閱該專案。
