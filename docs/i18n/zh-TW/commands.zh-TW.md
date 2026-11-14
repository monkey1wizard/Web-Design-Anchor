---
title: "指令"
status: stable
updated: 2026-11-12
type: Reference
description: "指令的工具需求、行為、診斷與結束狀態。"
tags: []
source: docs/commands.md
lang: zh-TW
source_sha256: 64e2d337052b0313bc4c6f96438886cf0d677f9cd96cf316eb600759cf5ee777
translated_at: 2026-11-12
---

# 指令

Core 提供四個指令。Designer 與 AI 直接修改專案檔案，Core 不處理一般編輯工作。Designer 工作流程、Skill 安裝與 preview 不會新增其他指令。

```text
wda init    建立新專案，Git 可用時也建立本機 Git 儲存庫（Repository）
wda check   驗證，不產出
wda build   驗證並做型別檢查，通過後建置輸出 dist/
wda tools fetch   還原此平台的內嵌 TypeScript 工具快取
```

`wda --version` 只輸出版本資訊，供 Skill 比對版本。它不是第五個指令。

`wda tools fetch` 不接受任何引數。它只還原這個 WDA 版本內嵌的那一個 TypeScript 工具版本，且只針對目前平台。它不會切換版本，也不會變更 `anchor.json` 或 `package.json`。版本、tag、範圍、URL 或任何其他引數都是用法錯誤，結束狀態碼為 `2`。

WDA 沒有 `wda deps` 指令。網站套件屬於專案自己的套件管理器，例如 `npm install <package>` 或 `bun add <package>`。`wda deps`，不論有沒有子指令，都是用法錯誤，結束狀態碼為 `2`，並附上指向外部套件管理器的提示。WDA 不保留 `sync`、`refresh`、`add` 等別名。

直接執行 `wda` 時，程式會把各指令和 `--version` 的用法寫到 stdout，stderr 留空，並以狀態碼 `0` 結束。這段輸出是用法說明，不是另一個指令，也不是命令列用法錯誤。

## 外部工具

### git 與套件管理器的使用方式

Git 可用時，`wda init` 會建立本機儲存庫與第一次 commit。Git 不在 PATH 或無法啟動時，初始化仍會建立專案並發出 Warning，但不會建立本機儲存庫或第一次 commit。`wda check` 會探測 Git 並回報工具鏈狀態。

`wda check` 與 `wda build` 不會啟動套件管理器、不會安裝或更新套件，也不會使用網路。已準備好的專案不需要讓套件管理器在 PATH 上。它們只讀取 `node_modules` 中已安裝的內容和原生工具快取。`wda init` 是唯一會呼叫外部套件管理器的指令，而且只在安裝預設套件時呼叫。找不到可用的管理器，或機器連不到 registry 時，`wda init` 會建立完整的 `WDA Minimal` 專案。

`wda check`、`wda build` 與 `wda tools fetch` 不需要既有的 Git 儲存庫或 `.gitignore`。

### 哪個指令需要哪個工具

| 指令 | `git` | 套件管理器 | 原生 TypeScript 工具快取 |
| :--- | :--- | :--- | :--- |
| `init` | 選用；不在 PATH 或無法啟動時，仍建立專案，但不建立 `.git` 與第一次 commit，並發出 `wda.toolchain.git-missing` Warning | 選用；沒有可用管理器時，`init` 建立 `WDA Minimal` | 不需要；`init` 會嘗試取得，失敗時只發出 Warning |
| `check` | 選用；不在 PATH 時輸出 `wda.toolchain.git-missing` Warning，找到但無法啟動時以 `wda.tool.fault` Warning 回報；兩種情況都仍成功 | 絕不啟動；只缺管理器不算問題 | 只有頁面引用 `scripts/**/*.ts` 入口時才查詢；快取不存在時發出指向 `wda tools fetch` 的 `wda.build.tool-unavailable` Warning |
| `build` | 不需要 | 絕不啟動 | 只有頁面引用 script 入口時才需要；純 HTML 專案不需要快取即可建置；快取不存在是 `wda.build.tool-unavailable` Error |
| `tools fetch` | 不需要 | 不需要 | 還原快取；除 `init` 外，這是唯一為工具使用網路的指令 |

`wda check` 發現 `package.json` 宣告了套件、但 `node_modules` 沒有時，會回報 `wda.build.dependency-unlocked` Warning，指出套件名稱，並指向 `packageManager` 所記的管理器。有 script 入口但工具快取不存在時，會回報 `wda.build.tool-unavailable` Warning。這些 Warning 不會改變成功狀態，`check` 也不寫入任何內容，因此請自行執行管理器的安裝指令或 `wda tools fetch`。

`wda init` 缺少或無法啟動 Git 時，仍會建立專案與 `.gitignore`，略過 `.git` 與第一次 commit，並發出 `wda.toolchain.git-missing` Warning。缺少 Deno 在任何地方都不是條件：WDA 已不使用 Deno，沒有任何指令會發出 `wda.toolchain.deno-missing`。

## wda init

### 建立什麼

`wda init [--package-manager <name>]` 會在目前目錄建立新專案，直接採用預設值。支援的管理器名稱是 `npm` 與 `bun`。未知名稱是用法錯誤。未指定管理器時，`init` 只探測支援的管理器，探測時不下載任何東西，並選用唯一可用的那一個。有多個可用時，互動式終端機會顯示選單；非互動執行會以用法錯誤結束，並要求提供 `--package-manager`。取消選單會在寫入任何內容前結束指令。

`WDA Minimal` 路徑寫入：

```text
.git/                   本機 Git 儲存庫（Git 不可用時略過）
.gitignore              內容固定為 /dist/ 與 /node_modules/
README.md
anchor.json
package.json
docs/architecture.md
docs/design.md
docs/glossary.md
pages/index.html
tokens/tokens.json
```

`anchor.json` 保存 WDA 設定與實際使用的 Design System。`package.json` 是宣告網站套件的唯一位置。`WDA Minimal` 沒有相依套件、沒有 `packageManager` 欄位，在第一次外部安裝之前也沒有管理器鎖定檔。發生退回時，`anchor.json` 還會記錄 init 原本想用的 Design System 與文字形式的退回 `reason`。

Spectrum 2 路徑也會寫入 `scripts/main.ts`。只選定一個管理器且 registry 有回應時，`init` 會解析最新穩定版 Spectrum 的確切版本，在獨立的暫存目錄中用該管理器安裝六個 Spectrum 2 套件與 `lit`，並把管理器名稱與確切版本記入 `package.json` 的 `packageManager` 欄位。鎖定檔與 `node_modules` 由管理器負責。

機器離線、沒有可用管理器，或指定的管理器不可用時，`init` 會建立完整的 `WDA Minimal` 專案。本機套件快取不會讓 `init` 改用 Spectrum。`WDA Minimal` 不會執行管理器。

Git 可用時，程式會執行 `git init`，再將整個目錄提交為初始 commit。訊息固定為 `wda init`，程式不會設定遠端儲存庫。原有的 Designer 檔案也會納入提交。Git 不在 PATH 或無法啟動時，`init` 仍會建立專案與 `.gitignore`，略過 `.git` 與第一次 commit，並發出 `wda.toolchain.git-missing` Warning。

執行成功後，`init` 也會嘗試還原內嵌的 TypeScript 工具快取，但只在這次執行中 registry 有回應時才嘗試。失敗時發出指向 `wda tools fetch` 的 `wda.init.tool-unavailable` Warning，不會讓專案失敗。

`.gitignore` 只在初始化時建立。`wda init` 完成後，WDA 指令不會再讀取或寫入此檔案。

### 碰撞規則

目標目錄可以有其他檔案。若 `wda init` 要建立的檔案或目錄已存在，程式會在寫入前中止。檢查範圍包括上述項目、必要目錄、`.git/`、既有的 `package.json`、舊版 `wda.json`、`node_modules`，以及各個支援管理器的鎖定檔與狀態檔。程式不會讀取、修改或合併範圍外的檔案。`init` 不會採用既有的安裝結果。

### Design System 解析與退回

`wda init` 會先從 registry 解析最新穩定版 Spectrum 的確切版本。成功後，程式會把 Design System 寫入 `anchor.json`，並用選定的管理器安裝相依套件。若解析或安裝失敗，程式會移除這次執行為 Spectrum 建立的內容，改用 `WDA Minimal`，並在 stdout 說明採用的 Design System 和原因。原因是管理器回報的原因，輸出也會提示之後可請 Skill 補上 Spectrum 與 Lit。連不到 registry 時，說明用語是「registry unreachable」，而不是斷言機器離線。改用 Minimal 不算執行失敗。解析失敗的判定條件見 `docs/design-system.md`。

### 失敗時的還原清理

`wda init` 會一次完成整個初始化。任何步驟寫入失敗，或選單被取消時，程式會清除已寫入的內容，讓目錄回到執行前的狀態。程式依建立順序的反向次序移除檔案，而且只動這次執行建立的內容。若 Spectrum 2 安裝套件失敗而改用 WDA Minimal，程式會先清除 Spectrum 2 專屬檔案，再建立 Minimal 專案。清理失敗是明確的錯誤，絕不宣稱成功。管理器所執行的第三方套件腳本造成的影響，不承諾可還原。

### 拒絕條件與診斷碼

| 診斷碼 | 級別 | 條件 |
| :--- | :--- | :--- |
| `wda.init.tool-unavailable` | Warning | 專案寫入後，無法還原內嵌的 TypeScript 工具套件。專案已完整，可用 `wda tools fetch` 重試 |
| `wda.init.path-conflict` | Error | 觸發名稱或路徑衝突（碰撞規則）。不寫入任何內容 |
| `wda.init.git-identity-fallback` | Warning | 環境缺少 git 使用者身分設定，init 以臨時身分完成 commit |

## 網站套件

WDA 不解析、不下載、不鎖定也不更新網站套件。`package.json` 宣告它們，專案使用的套件管理器負責鎖定檔與 `node_modules`。請用管理器自己的指令新增、移除與更新套件，例如 `npm install <package>` 或 `bun add <package>`，再執行 `wda check` 或 `wda build`。

`wda check` 與 `wda build` 不會安裝或更新套件。套件與原生工具快取都就緒時，它們不需要網路。缺少套件或工具快取時會給出明確的診斷，指令不會暗中下載。套件安裝會執行第三方程式碼，因此它是另一個由你自己啟動的步驟。

`wda deps` 指令已移除。執行它會得到 `wda.cli.usage`、結束狀態碼 `2`，以及指向外部套件管理器的提示。

## wda check

### 與 build 共用同一套 validation engine

`wda check` 以唯讀方式檢查專案，不寫入檔案，也不更新 `wdaVersion`。它和 `wda build` 使用相同的專案契約、文件、HTML 與 Tokens 驗證規則。專案正確性不應依賴使用者先執行 `check`。

`check` 另外執行一項 preflight，檢查專案在這台電腦上需要什麼：

- Git 不在 PATH 時，程式會回報 `wda.toolchain.git-missing` Warning。Git 存在但無法啟動時，程式會回報 `wda.tool.fault` Warning。
- `package.json` 宣告了、但 `node_modules` 沒有的每個套件，程式會回報一則 `wda.build.dependency-unlocked` Warning，指出套件名稱與 `packageManager` 所記的管理器。
- 頁面引用 `scripts/**/*.ts` 入口、但原生 TypeScript 工具快取不存在時，程式會回報指向 `wda tools fetch` 的 `wda.build.tool-unavailable` Warning。沒有 script 入口的專案不需要快取，也不會收到這則 Warning。

這項 preflight 不會啟動套件管理器、不會安裝，也不會寫入。它不會探測 PATH 上有沒有套件管理器，因此只缺管理器不會讓已安裝的專案看起來有問題。這些 Warning 不會改變成功狀態。Build 不執行 PATH 工具檢查，而且因為缺少套件或工具快取時無法繼續，Build 會把它們回報為 Error。

Check 與 Build 都執行四個專案驗證規則模組。模組路徑如下：

- `src/validation/project_contract.rs`
- `src/validation/documents.rs`
- `src/validation/html.rs`
- `src/validation/tokens.rs`

Check 另執行 `src/validation/toolchain.rs`，其中是上述 preflight。Build 也會執行 `src/validation/build_output.rs` 的輸出規則。

### 規則模組：專案契約

驗證 `anchor.json` 與 `package.json` 是否存在且為合法 JSON。`anchor.json` 必須符合 `schemas/anchor.schema.json` 規範，且無未知欄位。`package.json` 的根必須是物件，`dependencies` 若存在必須是版本為字串的物件，`packageManager` 若存在必須是名稱、`@` 與確切版本。不讀取也不轉換 `wda.json`。缺少 `anchor.json` 或 `package.json` 任一檔的專案會得到 `wda.contract.missing` 錯誤。欄位規範詳見 `docs/generated-project.md`。

| 診斷碼 | 級別 |
| :--- | :--- |
| `wda.contract.missing` | Error |
| `wda.contract.malformed-json` | Error |
| `wda.contract.unknown-field` | Error |
| `wda.contract.schema-violation` | Error |

### 規則模組：文件

程式會檢查以下四份必要文件是否存在：`README.md`、`docs/architecture.md`、`docs/design.md`、`docs/glossary.md`。`docs/` 中的三份文件必須有 YAML front matter，且 `title`、`status`、`updated` 不得為空。`updated` 格式為 `YYYY-MM-DD`。其他 `docs/**/*.md` 只有在檔案存在時才檢查；front matter 有問題時會回報 Warning。若 front matter 有 `sourceRefs`，每個項目都必須能對應到專案內的檔案或章節。

文件一致性檢查只看結構與 metadata，不會比對文件內容和程式碼、`anchor.json`、tokens 或相依性狀態。人工審查負責發現內容差異。

| 診斷碼 | 級別 |
| :--- | :--- |
| `wda.docs.required-document-missing` | Error |
| `wda.docs.front-matter-missing` | Error |
| `wda.docs.front-matter-malformed` | 必要文件 Error，選用文件 Warning |
| `wda.docs.front-matter-field-invalid` | 必要文件 Error，選用文件 Warning |
| `wda.docs.source-ref-unresolvable` | Warning |
| `wda.docs.source-ref-escapes-root` | Warning |

### 規則模組：HTML

HTML 模組只檢查三項規則：`html[lang]` 不得為空、每個 `img` 都有 `alt` 屬性、文件內的 `id` 不重複。`alt` 檢查只確認屬性存在，不評估文字品質。

其他無障礙功能（a11y）由人工審查，包括 `alt` 文字是否有意義、疊加圖片後的對比度、焦點（focus）的可見性與順序、鍵盤操作、縮放與重排，以及動態效果。自動檢查全部合格，也不代表符合 WCAG 規範。`wda check` 不會因此產生「需要人工審查」的 Warning。

| 診斷碼 | 級別 |
| :--- | :--- |
| `wda.html.lang-missing` | Error |
| `wda.html.img-alt-missing` | Error |
| `wda.html.duplicate-id` | Error |

### 規則模組：Tokens

Tokens 模組會檢查 JSON，以及 [Design System](design-system.zh-TW.md) 所述的 WDA CSS token 支援範圍。建置輸出前會檢查值、型別、別名和正規化後的名稱。

| 診斷碼 | 級別 |
| :--- | :--- |
| `wda.tokens.malformed-json` | Error |
| `wda.tokens.value-missing` | Error |
| `wda.tokens.type-unresolvable` | Error |
| `wda.tokens.alias-unresolvable` | Error |
| `wda.tokens.unsupported` | Error |

## wda build

### 必要驗證與 build output 規則模組

`wda build` 先執行與 `check` 相同的四個專案驗證規則模組。只要有 Error，Build 就會中止，不會更動 `dist/`。Warning 不會阻擋建置。沒有 Error 時，Build 才進入產出階段，並執行 `src/validation/build_output.rs` 的輸出規則。

### wda-include 展開與不建立 template language

唯一的 WDA 自訂 build-time 語法是：

```text
<wda-include src="..."></wda-include>
```

它符合 Custom Element 的命名規範，在建置時展開為純 HTML。它並非執行階段（runtime）的 Web Component，`dist/` 中不得殘留此標籤。`src` 屬性僅能指向專案內的檔案，不得超出專案根目錄範圍，亦不得形成循環參照。

專案不提供樣板語言（template language），也不支援 `<slot>`、`if`、`for`、變數、運算式或資料繫結。一般版面和靜態內容應優先使用原生語意化 HTML 與 CSS。只有在需要重用行為、產品語意或封裝元件時，才建立元件。

### 型別檢查與模組建置

`wda build` 會用這個 WDA 版本內嵌的單一 TypeScript 工具版本，對頁面引用的每個 `scripts/**/*.ts` 入口做型別檢查。型別檢查一定先於打包，而且固定了專案 `tsconfig` 無法放寬的 strict 設定。可解析的型別錯誤是專案 Error，結束狀態碼為 `1`。工具無法啟動、當機或逾時，是結束狀態碼為 `2` 的 ToolFault。沒有頁面引用 script 入口時，建置不會查詢工具快取，也不會啟動工具。

工具來自內嵌或已快取的平台套件。快取不存在時，建置會回報指向 `wda tools fetch` 的 `wda.build.tool-unavailable` Error，並保持 `dist/` 不變。建置絕不下載工具。

WDA 接著用原生 Rust 打包器（Rolldown）以瀏覽器模式、ES module 輸出建置入口。打包器解析專案自己的套件管理器已安裝在 `node_modules` 的套件。WDA 不解析套件版本，也不維護 CDN 鏡像。遠端模組匯入、僅限 Node 的模組，以及 WDA 無法證明已完整輸出的動態匯入，都會回報 Error。建置不宣稱每個 npm 套件都能在瀏覽器運作。

### 暫存 build workspace

`wda build` 先把打包器輸出寫入全新的暫存目錄，再只把列出的入口、chunk 與靜態資源提交到 `dist/`。它不會整個複製 `node_modules`。`package.json` 與管理器鎖定檔在建置期間不會有任何位元組（byte）變更。建置結束後，程式會刪除暫存目錄，無論建置成功或失敗。

### 只使用已安裝的套件

`wda build` 只讀取 `node_modules` 中已安裝的內容。它不會安裝、挑選或升級任何套件，也不使用網路。script 匯入了未安裝的套件時，它會回報 `wda.build.dependency-unlocked` Error，要你用自己的套件管理器安裝專案套件。`dist/` 的輸出不留下遠端 runtime 相依項目，網站執行時也不需要 Node.js、Bun 或 WDA。

### secret 不進 dist/

建置時不會複製 dotfile 或 `.env*`。若頁面或樣式引用這些路徑，建置會回報 Error 並中止。這項排除規則只看檔案路徑，不檢查檔案內容。

### 成功時報出 dist/ 絕對路徑

建置成功時，程式會更新 `anchor.json` 的 `wdaVersion` 欄位，並在 stdout 印出 `dist/` 的絕對路徑。建置失敗時不會更新 `wdaVersion`。若程式偵測到 ES module、`fetch` 或其他已知不支援 `file://` 的特徵，也會提示產出物需要 HTTP host。

### 拒絕條件與診斷碼

| 診斷碼 | 級別 | 條件 |
| :--- | :--- | :--- |
| `wda.build.tool-unavailable` | build 為 Error，check 為 Warning | script 入口需要原生 TypeScript 工具快取，但快取不存在或無法定位。Build 失敗並指向 `wda tools fetch`。診斷碼與級別穩定，摘要文字不屬於穩定契約 |
| `wda.build.tokens-css-in-source` | Error | source 端存在 `styles/tokens.css` |
| `wda.build.include-unresolvable` | Error | `wda-include` 的 `src` 找不到檔案 |
| `wda.build.include-escapes-root` | Error | `wda-include` 的 `src` 超出專案根目錄範圍 |
| `wda.build.include-cycle` | Error | `wda-include` 形成循環參照 |
| `wda.build.type-error` | Error | TypeScript type-check 失敗 |
| `wda.build.secret-path-referenced` | Error | 頁面或樣式引用 dotfile 或 `.env*` |
| `wda.build.reference-unresolvable` | Error | 頁面或樣式引用的檔案不存在，或 script 匯入遠端模組 |
| `wda.build.unreferenced-asset` | Warning | `assets/` 目錄中存在未被任何頁面或樣式引用的檔案 |
| `wda.build.output-path-collision` | Error | 兩個 source 路徑對應到同一個 `dist/` 路徑 |
| `wda.build.dependency-unlocked` | build 為 Error，check 為 Warning | `package.json` 宣告或 script 匯入的套件沒有安裝在 `node_modules`。這個名稱是歷史遺留：此碼現在表示「未安裝」，不會查閱任何鎖定檔 |

## 診斷輸出

### Severity 依影響判斷

每筆診斷都包含 severity（嚴重度）和 diagnostic code（診斷碼）。程式依實際影響決定 severity，不會把問題類別直接對應到級別，例如把所有 a11y 問題列為 Warning，或把所有 script 問題列為 Error。只有在問題具體且能採取改善行動時，程式才會發出 Warning，也不會固定顯示無法消除的「需要人工審查」提示。

| 級別 | 意義 |
| :--- | :--- |
| Error | 問題將導致建置結果或專案不正確，嚴重至必須阻擋建置。只要能確定判定產出物將有錯誤，即可定為 Error |
| Warning | 不阻擋建置，但代表對正確性或品質的疑慮。必須具備足夠能見度並持續出現，使 Designer 與 AI 無法輕易忽略 |

### 對外穩定的語意

診斷輸出不採用特定結構化格式。對外維持穩定的項目固定為五項：

| 項目 | 內容 |
| :--- | :--- |
| severity | 嚴重度分級 |
| diagnostic code | 診斷碼本身 |
| 位置 | 正規化後的專案相對路徑，附帶行號與直欄位置（line:col） |
| 輸出流 | stderr 與 stdout 分流輸出 |
| exit status | 結束狀態碼（exit status） |

message（訊息內文）與下一步建議必須存在且清晰易懂，但逐字文字內容允許後續演進。標點符號、分隔符與欄寬不屬於契約，取用端（consumer）不得依賴切割文字欄位的方式進行解析。

### Exit status

`src/main.rs` 依回傳結果設定結束狀態。`init`、`check` 和 `build` 透過 `ValidationReport` 回報的診斷若含 Error，狀態碼為 `1`。`wda.cli.usage`、工具錯誤，以及其他 ToolFault，狀態碼都是 `2`。`wda tools fetch` 無法還原工具快取時，以狀態碼 `1` 結束。

`wda.init.path-conflict` 和（作為 Error 的）`wda.build.tool-unavailable` 透過 `ValidationReport` 回報，因此狀態碼為 `1`。`wda.init.tool-unavailable` 和 `check` preflight 的 Warning 絕不改變狀態碼。

```text
0   成功，允許存在 Warning
1   `ValidationReport` 含 Error
2   CLI 用法錯誤（含已移除的 `wda deps`）或 ToolFault
```

### 輸出流

Diagnostics 會寫入 stderr。成功結果和其他摘要訊息會寫入 stdout，包括 `wda init` 的 Design System 說明與 `wda build` 的 `dist/` 路徑。

### 單行、可判定、人和 AI 都能直接讀

每筆 diagnostic 都占一行。相同 binary 在輸入和 PATH 狀態相同時，會輸出相同的診斷集合與順序。Check 的工具鏈診斷則會依 Git 是否在 PATH 且能否啟動、`node_modules` 安裝了哪些套件，以及工具快取是否存在而變動。排序有明確的平手判定準則（tie-breaker），不受檔案系統走訪順序影響。取用端只需讀取 diagnostic code 和嚴重度，不必依固定欄位切割文字。每筆診斷都會提供 message 和下一步建議，文字可隨版本調整。

### JSON 輸出

`wda check --json` 與 `wda build --json` 在 stdout 輸出一份 UTF-8 JSON 文件，結尾帶換行。子程序輸出會被擷取，stderr 不會重印報告。作業系統終止程序或 stdout 寫入失敗時，無法保證文件完整。

文件包含 `schema_version` 1、`command`（`check` 或 `build`）、`status`（`success`、`project-error`、`usage-error` 或 `tool-fault`）、`diagnostics` 與 `summary`。每筆診斷包含 `code`、`severity`（`error` 或 `warning`）、`path`、`line`、`column`、`message`、`next_action`。缺少的位置欄位為 null，路徑使用斜線，訊息的控制字元保留在 JSON 值中。

`summary` 包含 `errors`、`warnings`、`dist_path`、`needs_http`。只有成功的 build 提供專案相對路徑與布林值。check 和所有失敗均為 null，即使舊 dist 仍存在。Warning 仍是成功，結束碼為 0。專案錯誤為 1，使用錯誤與工具故障為 2。JSON 與人類可讀輸出使用相同排序的診斷。

JSON 模式要求第一個引數為 `check` 或 `build`，且精確的 `--json` 出現在 `--` 終止符之前。此模式的未知或重複引數產生 JSON 使用錯誤。其他呼叫維持人類可讀的使用說明。

### 路徑正規化與 control character 防護

路徑會以相對專案根目錄的形式輸出，並以正斜線（`/`）分隔，不會洩漏 host 的絕對路徑。即使專案檔案含有換行或控制字元（control character），也不能藉此插入假的 diagnostic。

### 跨指令的診斷碼

| 診斷碼 | 級別 | 條件 |
| :--- | :--- | :--- |
| `wda.cli.usage` | Error | 命令列用法錯誤，結束狀態碼為 `2` |
| `wda.cli.json-not-supported` | Error | 在 check 或 build 以外傳入 `--json`，結束狀態碼為 `2` |
| `wda.tool.fault` | Error 或 Warning | 外部工具或檔案系統發生失敗。級別依指令與影響判定；Error 的結束狀態碼為 `2` |
| `wda.command.not-implemented` | Error | 保留碼，目前無任何指令會發出此診斷碼 |

### 全部診斷碼

`src/codes.rs` 是診斷碼的唯一宣告處。本文件是查閱表，每個碼都必須在本文件出現一次。

| 診斷碼 | 級別 | 所屬指令 |
| :--- | :--- | :--- |
| `wda.command.not-implemented` | Error | 跨指令 |
| `wda.cli.json-not-supported` | Error | 跨指令 |
| `wda.cli.usage` | Error | 跨指令 |
| `wda.tool.fault` | Error 或 Warning | 跨指令 |
| `wda.toolchain.git-missing` | Warning | check / init |
| `wda.toolchain.deno-missing` | 已淘汰 | 無。沒有任何指令會發出此碼；宣告保留在註冊表，待後續清理移除 |
| `wda.contract.missing` | Error | check / build |
| `wda.contract.malformed-json` | Error | check / build |
| `wda.contract.unknown-field` | Error | check / build |
| `wda.contract.schema-violation` | Error | check / build |
| `wda.docs.required-document-missing` | Error | check / build |
| `wda.docs.front-matter-missing` | Error | check / build |
| `wda.docs.front-matter-malformed` | Error 或 Warning | check / build |
| `wda.docs.front-matter-field-invalid` | Error 或 Warning | check / build |
| `wda.docs.source-ref-unresolvable` | Warning | check / build |
| `wda.docs.source-ref-escapes-root` | Warning | check / build |
| `wda.html.lang-missing` | Error | check / build |
| `wda.html.img-alt-missing` | Error | check / build |
| `wda.html.duplicate-id` | Error | check / build |
| `wda.tokens.malformed-json` | Error | check / build |
| `wda.tokens.value-missing` | Error | check / build |
| `wda.tokens.type-unresolvable` | Error | check / build |
| `wda.tokens.alias-unresolvable` | Error | check / build |
| `wda.init.tool-unavailable` | Warning | init |
| `wda.init.path-conflict` | Error | init |
| `wda.init.git-identity-fallback` | Warning | init |
| `wda.build.tool-unavailable` | Error 或 Warning | check / build |
| `wda.build.tokens-css-in-source` | Error | build |
| `wda.build.include-unresolvable` | Error | build |
| `wda.build.include-escapes-root` | Error | build |
| `wda.build.include-cycle` | Error | build |
| `wda.build.type-error` | Error | build |
| `wda.build.secret-path-referenced` | Error | build |
| `wda.build.reference-unresolvable` | Error | build |
| `wda.build.unreferenced-asset` | Warning | build |
| `wda.build.output-path-collision` | Error | build |
| `wda.build.dependency-unlocked` | Error 或 Warning | check / build |
