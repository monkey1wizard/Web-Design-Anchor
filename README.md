# Web Design Anchor

**不熟悉網頁開發，也能透過 AI 做出好維護的網站。**

即使你不是工程師、不熟悉網頁開發，搭配 WDA 和 AI 聊天，就能做出簡單、好維護的靜態網頁。產出的網頁專案架構清楚，頁面、樣式和設計規範都有固定的位置。因此不只是內容好改，配色、字級等設計風格也能集中調整，網站改版多次也不會走樣。

WDA 提供標準化的 CLI 與專案規範，並以專案內的 `design.md` 定錨設計邏輯。AI 每次產出元件時，都能嚴格遵循正確的 Design System 與 Token 規格，省去過去繁瑣的溝通成本。

在技術架構上，WDA 回歸標準 Web，不依賴前端框架。HTML5 與 Modern CSS 的原生能力，足以處理豐富的版面與動態特效。腳本層則採用 TypeScript，讓程式碼好維護、不混亂。

在建置與交付上，WDA 選用 Deno 快速轉譯 TypeScript 腳本與專案資產，並統一輸出至 `dist/` 目錄。每位使用者搭配的 Coding Agent 或 IDE 各不相同，因此 WDA 本身不內建預覽伺服器，預覽交由使用者的環境自行處理。`dist/` 產出的是純靜態、零依賴的網頁，使用者能直接檢視成果，隨時以此部署。

## 快速開始

四個步驟即可完成第一個網站：

### 1. 安裝前置工具與 WDA

確認電腦已安裝 Git 與 Deno，兩者都必須加入 PATH 環境變數。WDA 不需要 Node.js。

安裝 WDA 後，執行版本檢查：

```sh
wda --version
```

詳細安裝方式請見下方的安裝指南章節。

### 2. 建立新專案

在空目錄中執行初始化指令：

```sh
wda init my-site
```

這會建立標準 WDA 專案結構，包含設定檔 `wda.json`、設計規範 `docs/design.md`、首頁 `index.html` 以及元件目錄。

### 3. 與 AI 協作設計

讓你的 AI 讀取專案內的 `docs/design.md`。AI 會依據定錨的 Design System 規範與 Token，產出符合標準 Web 的 HTML、CSS 與 Web Components 元件。

在修改過程中，隨時執行驗證：

```sh
wda check
```

WDA 會檢查合約、文件、HTML 標記與 Token 一致性，回報任何偏離規範的項目。

### 4. 建置可部署網站

完成設計後執行建置指令：

```sh
wda build
```

WDA 會轉譯 TypeScript 並將所有靜態資產打包輸出至 `dist/`。請透過本地 HTTP 靜態伺服器預覽，或將 `dist/` 部署至靜態網站主機。若產出物使用 ES module 等不支援 `file://` 的功能，直接開啟 HTML 檔案無法正常運作。

## 安裝指南

WDA 使用兩種不同層級的工具。Git 與 Deno 是電腦層級工具，兩者都必須在 PATH 中。Lit 是每個 WDA 專案自己的相依項目，不是全域工具，由 WDA 管理。

### 先安裝 Git 與 Deno

請依作業系統選擇一組偏好路徑：

| 平台 | Git 偏好路徑 | Deno 偏好路徑 | 官方替代來源 |
| --- | --- | --- | --- |
| Windows | `winget install --id Git.Git -e --source winget` | `winget install DenoLand.Deno` | [Git for Windows 安裝程式](https://git-scm.com/install/windows) 與 [Deno Windows 安裝選項](https://docs.deno.com/runtime/getting_started/installation/) |
| macOS | `brew install git` | `brew install deno` | [Git macOS 安裝選項](https://git-scm.com/download/mac) 與 [Deno 安裝選項](https://docs.deno.com/runtime/getting_started/installation/) |
| Debian／Ubuntu | `sudo apt-get install git` | `curl -fsSL https://deno.land/install.sh \| sh` | [Git Linux 安裝選項](https://git-scm.com/download/linux) 與 [Deno 手動下載](https://docs.deno.com/runtime/getting_started/installation/#manual-download) |
| Fedora | `sudo dnf install git` | `curl -fsSL https://deno.land/install.sh \| sh` | [Git Linux 安裝選項](https://git-scm.com/download/linux) 與 [Deno 手動下載](https://docs.deno.com/runtime/getting_started/installation/#manual-download) |

安裝後分別執行 `git --version` 與 `deno --version` 確認可用。如果指令找不到工具，請開啟新的終端機視窗再檢查。

### 安裝 WDA

公開 GitHub 儲存庫提供穩定版與 prerelease。請在 [GitHub Releases](https://github.com/monkey1wizard/Web-Design-Anchor/releases) 檢視可用版本。

依作業系統選擇一種方式安裝：

#### macOS 與 Linux

```sh
brew install monkey1wizard/tap/wda
```

或使用安裝腳本：

```sh
curl -fsSL https://raw.githubusercontent.com/monkey1wizard/web-design-anchor/main/packaging/install.sh | bash
```

#### Windows

```sh
winget install Monkey1Wizard.WDA
```

或使用 PowerShell 安裝腳本：

```sh
irm https://raw.githubusercontent.com/monkey1wizard/web-design-anchor/main/packaging/install.ps1 | iex
```

#### 已安裝 Rust

若已安裝 Rust 工具鏈，可使用 Cargo 安裝：

```sh
cargo install --git https://github.com/monkey1wizard/Web-Design-Anchor --locked
```

#### 確認安裝

完成安裝後，以唯讀方式執行 `git --version`、`deno --version` 與 `wda --version`，確認三個工具可啟動。例如：

```sh
wda --version
```

若 PATH 中找不到工具，請確認安裝位置並重新開啟終端機。若工具找到但無法啟動，請保留啟動錯誤，依實際錯誤排查。AI 不得虛構結束狀態，也不能把啟動失敗當作工具尚未安裝。

## 核心指令

WDA 提供四個指令，涵蓋專案生命週期：

| 指令 | 職責 | 說明 |
| :--- | :--- | :--- |
| `wda init` | 專案初始化 | 建立標準專案結構、`wda.json`、設計定錨文件與首頁範本 |
| `wda check` | 靜態驗證 | 驗證合約、文件結構、HTML 標籤規範與 Token 引用 |
| `wda build` | 建置交付 | 轉譯 TypeScript 與打包資產，統一輸出至 `dist/` 目錄 |
| `wda deps` | 相依管理 | 管理專案相依項目，例如安裝或更新 Lit 等必要函式庫 |

詳細參數與診斷說明請參閱 `docs/commands.md`。

## 搭配 AI 協作

WDA 專為 AI 協作設計。專案提供官方 Skill：

- **官方 Skill**：位在 `skills/wda/`。若你使用的 Coding Agent 支援 Skill，請載入此目錄。AI 會自動遵守 WDA 專案規範與工作流程。
- **無 Skill 環境**：如果 AI 沒有 WDA Skill，請先提供本 README 或另一份可讀取的官方 WDA 來源。若 AI 無法讀取，必須請你提供可存取的來源，然後停止。AI 不應自行猜測安裝指令。建立專案後，請 AI 依據該專案的 `docs/design.md` 產生程式碼，並在每階段執行 `wda check` 驗證。

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
`docs/design.md` 第 10 節「Craft Floor」改寫自 [impeccable](https://github.com/pbakaus/impeccable)，只採用其中一小部分判準，授權為 Apache-2.0。需要完整設計流程的讀者可參閱該專案。
