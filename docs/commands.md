---
title: "指令"
status: stable
updated: 2026-10-15
type: Reference
description: "四個指令的工具需求、行為、診斷與結束狀態。"
tags: []
---

# 指令

Core 提供四個指令。Designer 與 AI 直接修改專案檔案，Core 不處理一般編輯工作。Designer 工作流程、Skill 安裝與 preview 不會新增其他指令。

```text
wda init    建立新專案與本機 Git 儲存庫（Repository）
wda deps    管理 dependency graph
wda check   驗證，不產出
wda build   驗證，通過後建置輸出 dist/
```

`wda --version` 只輸出版本資訊，供 Skill 比對版本。它不是第五個指令。

直接執行 `wda` 時，程式會把四個指令和 `--version` 的用法寫到 stdout，stderr 留空，並以狀態碼 `0` 結束。這段輸出是用法說明，不是第五個指令，也不是命令列用法錯誤。

## 外部工具

### deno 與 git 的 PATH 使用方式

`deno` 會解析相依套件並呼叫轉譯器。`wda init` 需要 `git` 建立本機儲存庫，也會先確認 Git 能否啟動，再寫入檔案。若 Git 不在 PATH 或無法啟動，初始化會中止。`wda check` 會探測 Git 並回報工具鏈狀態。沒有 `deno` 時仍可執行 Minimal 初始化。若初始化需要 Deno 解析或安裝相依套件，但 Deno 不在 PATH 或無法啟動，程式會改用 `WDA Minimal`。

`wda check`、`wda build` 與 `wda deps` 不需要既有的 Git 儲存庫或 `.gitignore`。

### esbuild 由 WDA 釘死版本、經 deno 呼叫

WDA 以 `deno run -A npm:esbuild@<釘死版本>` 呼叫 TypeScript 轉譯器。由於 WDA 固定 esbuild 版本，PATH 不需要 `esbuild` 或 Node.js。

### 哪個指令需要哪個工具

| 指令 | `deno` | `git` |
| :--- | :--- | :--- |
| `init` | 選用；Deno 不在 PATH 或無法啟動時，以 WDA Minimal 有限初始化，不進行線上 Design System 解析或相依套件安裝 | 需要；不在 PATH 或無法啟動時，以 Error 中止且不寫入任何內容 |
| `deps` | 需要 | 不需要 |
| `check` | 選用；不在 PATH 時輸出 `wda.toolchain.deno-missing` Warning，找到但無法啟動時以 `wda.tool.fault` Warning 回報；兩種情況都仍成功 | 選用；不在 PATH 時輸出 `wda.toolchain.git-missing` Warning，找到但無法啟動時以 `wda.tool.fault` Warning 回報；兩種情況都仍成功 |
| `build` | 需要 | 不需要 |

`wda check` 對 Git 或 Deno 不在 PATH、或找到但無法啟動，都只發出 Warning，不會單獨導致失敗。
`wda init` 缺少或無法啟動 Git 時會發出 `wda.init.tool-unavailable` Error，並在寫入前中止。Deno 不在 PATH 或無法啟動時，`wda init` 會以 WDA Minimal 有限初始化。此路徑不進行線上 Design System 解析或相依套件安裝。Deno 不在 PATH 時另發出 `wda.toolchain.deno-missing` Warning。

## wda init

### 建立什麼

`wda init` 會在目前目錄建立新專案，直接採用預設值。

`WDA Minimal` 路徑寫入：

```text
.git/                   本機 Git 儲存庫（Repository）
.gitignore              內容固定為 /dist/
README.md
wda.json
docs/architecture.md
docs/design.md
docs/glossary.md
pages/index.html
tokens/tokens.json
```

有網路時，`WDA Minimal` 會嘗試安裝 `lit`。安裝成功後會產生 `deno.jsonc` 和 `deno.lock`。離線或安裝失敗時，初始化仍會完成，但不會產生相依檔案。stdout 會說明原因，並建議執行 `wda deps add lit`。`WDA Minimal` 預設不含 starter script。

Spectrum 2 路徑也會寫入 `scripts/main.ts`。線上解析成功時，初始化會安裝六個 Spectrum 2 套件與 `lit`，產生 `deno.jsonc` 和 `deno.lock`，並以相同版本鎖定 starter script 實際匯入的套件。

檔案寫入後，程式會執行 `git init`，再將整個目錄提交為初始 commit。訊息固定為 `wda init`，程式不會設定遠端儲存庫。原有的 Designer 檔案也會納入提交。

`.gitignore` 只在初始化時建立。`wda init` 完成後，WDA 指令不會再讀取或寫入此檔案。

### 碰撞規則

目標目錄可以有其他檔案。若 `wda init` 要建立的檔案或目錄已存在，程式會在寫入前中止。檢查範圍包括上述項目、必要目錄、`.git/`、`deno.jsonc`、專案端 `deno.json` 和 `deno.lock`。程式不會讀取、修改或合併範圍外的檔案。

### Design System 解析與退回

`wda init` 會先線上解析最新穩定版 Spectrum 的確切版本。成功後，程式會把版本寫入 `wda.json` 並安裝相依套件。若解析或安裝失敗，程式會改用 `WDA Minimal`，並在 stdout 說明採用的 Design System 和原因。安裝失敗時也會提供復原步驟。改用 Minimal 不算執行失敗。解析失敗的判定條件見 `docs/design-system.md`。

### 失敗時的還原清理

`wda init` 會一次完成整個初始化。任何步驟寫入失敗時，程式會清除已寫入的內容，讓目錄回到執行前的狀態。清理範圍包含 Spectrum 2 與 WDA Minimal 所建立的 `deno.jsonc` 和 `deno.lock`。程式依建立順序的反向次序移除檔案。若 Spectrum 2 安裝相依套件失敗而改用 WDA Minimal，程式會先清除 Spectrum 2 專屬檔案，再建立 Minimal 專案。

### 拒絕條件與診斷碼

| 診斷碼 | 級別 | 條件 |
| :--- | :--- | :--- |
| `wda.init.tool-unavailable` | Error | `git` 不在 PATH，或雖已找到但無法啟動；訊息指出實際原因。不寫入任何內容 |
| `wda.init.path-conflict` | Error | 觸發名稱或路徑衝突（碰撞規則）。不寫入任何內容 |
| `wda.init.git-identity-fallback` | Warning | 環境缺少 git 使用者身分設定，init 以臨時身分完成 commit |

## wda deps

### 宣告與鎖定

現行專案在 `deno.jsonc` 的 `imports` 欄位宣告 npm 套件，使用固定版本的 `npm:<package>@<version>` 規格，並將版本鎖定在 `deno.lock`。WDA 也支援既有的 `package.json` 專案。`wda deps` 會先判斷專案格式：根目錄有 `package.json` 時使用 legacy 分支，否則使用現行 `deno.jsonc` 分支。

`wda deps` 會呼叫 `deno` 解析並更新 `deno.lock`。現行格式使用 `--config deno.jsonc`，legacy 格式使用 `--package-json`。Deno 會寫入檔案，並保留 `deno.jsonc` 的註解、尾端逗號和其他自訂欄位。只有這個指令會改變套件圖（dependency graph）。`wda build` 只使用已鎖定的版本，不會自動挑選新版本。

### add、remove、update 的各自職責

```text
wda deps add <套件>      加入一個相依套件並鎖定
wda deps remove <套件>   移除一個相依套件並更新鎖定
wda deps update [套件]   在既有宣告內重新解析並鎖定
```

### 暫存驗證與不可分割提交

`wda deps` 會先在隔離的暫存目錄執行 Deno 指令，再以 `deno install --frozen` 驗證相依圖（staged validation），之後才修改專案檔案。執行時不會傳入 `--allow-scripts`，因此不會執行 npm 生命週期腳本（lifecycle scripts）。

驗證成功後，WDA 會按固定順序一次提交變更：先寫入、更名或刪除 `deno.lock`，再更新相依宣告檔（`deno.jsonc` 或 `package.json`）。若過程發生 I/O 錯誤，程式會還原檔案的原始位元組，避免留下損壞或不一致的狀態。

### 語意空狀態規則

在無實際相依套件時，專案不保留空白的宣告檔或鎖定檔：

- 對於現行 `deno.jsonc` 專案：當移除最後一個相依套件時，WDA 檢查根層級的語意結構。若檔案中除空 `imports` 之外已無其他語意欄位，則將 `deno.jsonc` 刪除（單純註解不視為保留檔案的依據）；若仍包含其他自訂根欄位，則保留檔案僅清除 `imports`。當 `deno.lock` 僅剩版本標頭（`{"version": ...}`）時，一併刪除空白的 `deno.lock`。
- 對於 legacy `package.json` 專案：移除最後一個相依套件後，一併刪除 `package.json` 與 `deno.lock`。
- 離線建立的 `WDA Minimal` 專案預設即不包含這兩個檔案。

### 拒絕條件、復原與診斷碼

`wda deps` 使用跨指令診斷碼回報錯誤：

- 若專案端存在 `deno.json`，或同時存在 `deno.json` 與 `deno.jsonc`，`wda deps` 會以 `wda.tool.fault` 拒絕並中止。程式會要求使用者將設定檔更名或改用 `deno.jsonc`。WDA 不會要求使用者建立或編輯專案端 `deno.json`。
- 若命令用法錯誤，以 `wda.cli.usage` 拒絕（結束狀態碼為 `2`）。
- 若環境缺少 `deno`、解析失敗、或暫存驗證失敗，以 `wda.tool.fault` 拒絕（結束狀態碼為 `2`）。執行失敗時不寫入亦不更動專案檔案。
- 復原方式：若專案初始化時因離線而未安裝 Lit，使用者可在網路連線後執行 `wda deps add lit` 補行安裝。

## wda check

### 與 build 共用同一套 validation engine

`wda check` 以唯讀方式檢查專案，不寫入檔案，也不更新 `wdaVersion`。它和 `wda build` 使用相同的專案契約、文件、HTML 與 Tokens 驗證規則。`check` 也會探測 PATH 中的選用工具。若 Git 或 Deno 不在 PATH，程式會回報對應的 `wda.toolchain.*-missing` Warning。若工具存在但無法啟動，程式會回報 `wda.tool.fault` Warning。專案正確性不應依賴使用者先執行 `check`。
這些 Warning 不會改變成功狀態。Build 不執行 PATH 工具檢查。

Check 與 Build 都執行四個專案驗證規則模組。模組路徑如下：

- `src/validation/project_contract.rs`
- `src/validation/documents.rs`
- `src/validation/html.rs`
- `src/validation/tokens.rs`

Check 另執行 `src/validation/toolchain.rs`，依 PATH 狀態回報工具鏈診斷。Build 不探測 PATH 中的選用工具。Build 需要 Deno 卻無法使用時，會回報 `wda.build.tool-unavailable` Error。此診斷的摘要文字可能隨版本改變。Build 也會執行 `src/validation/build_output.rs` 的輸出規則。

### 規則模組：專案契約

驗證 `wda.json` 是否存在、為合法 JSON 格式、符合 `schemas/wda.schema.json` 規範，且無未知欄位。欄位規範詳見 `docs/generated-project.md` 的 `wda.json` 欄位表。

| 診斷碼 | 級別 |
| :--- | :--- |
| `wda.contract.missing` | Error |
| `wda.contract.malformed-json` | Error |
| `wda.contract.unknown-field` | Error |
| `wda.contract.schema-violation` | Error |

### 規則模組：文件

程式會檢查以下四份必要文件是否存在：`README.md`、`docs/architecture.md`、`docs/design.md`、`docs/glossary.md`。`docs/` 中的三份文件必須有 YAML front matter，且 `title`、`status`、`updated` 不得為空。`updated` 格式為 `YYYY-MM-DD`。其他 `docs/**/*.md` 只有在檔案存在時才檢查；front matter 有問題時會回報 Warning。若 front matter 有 `sourceRefs`，每個項目都必須能對應到專案內的檔案或章節。

文件一致性檢查只看結構與 metadata，不會比對文件內容和程式碼、`wda.json`、tokens 或相依性狀態。人工審查負責發現內容差異。

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

Tokens 模組會檢查 `tokens/tokens.json` 是否為合法 JSON、是否符合 DTCG 規範，以及每個 token 是否有設定值、型別能否判定、別名（alias）能否解析。

| 診斷碼 | 級別 |
| :--- | :--- |
| `wda.tokens.malformed-json` | Error |
| `wda.tokens.value-missing` | Error |
| `wda.tokens.type-unresolvable` | Error |
| `wda.tokens.alias-unresolvable` | Error |

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

### 暫存 build workspace

`wda build` 會在專案目錄外建立暫存工作區（build workspace）。WDA 會在工作區寫入專用 JSON 格式的 `deno.json`，其中包含編譯和模組設定，例如 `nodeModulesDir: "manual"` 與標準 `lib` 清單。

現行格式專案會將專案端 `deno.jsonc` 的 `imports` 合併到工作區 `deno.json`，其他自訂欄位不會帶入。若專案端有 `deno.json`，建置會回報 ToolFault 並中止。既有 `package.json` 專案則會將該檔原樣複製到工作區。

只有選定格式的相依宣告非空時，程式才會在工作區執行 `deno install --frozen`。也就是現行格式的 `imports` 或 legacy 格式的 `dependencies` 至少有一項。宣告為空時，程式會略過安裝，也不要求存在 `deno.lock`。執行時不會傳入 `--allow-scripts`，因此不會執行 npm 生命週期腳本（lifecycle scripts）。建置結束後，程式會刪除暫存工作區，無論建置成功或失敗。專案根目錄的 `deno.lock` 在建置期間不會有任何位元組（byte）變更。

### 只取已鎖定的 dependency

`wda build` 只使用 `deno.lock` 鎖定的版本，不挑選新版本、不變更套件圖（dependency graph），也不會靜默升級。建置時會把所有相依模組打包（bundle）到 `dist/`，不留下遠端 runtime 相依項目。

### secret 不進 dist/

建置時不會複製 dotfile 或 `.env*`。若頁面或樣式引用這些路徑，建置會回報 Error 並中止。這項排除規則只看檔案路徑，不檢查檔案內容。

### 成功時報出 dist/ 絕對路徑

建置成功時，程式會更新 `wda.json` 的 `wdaVersion` 欄位，並在 stdout 印出 `dist/` 的絕對路徑。建置失敗時不會更新 `wdaVersion`。若程式偵測到 ES module、`fetch` 或其他已知不支援 `file://` 的特徵，也會提示產出物需要 HTTP host。

### 拒絕條件與診斷碼

| 診斷碼 | 級別 | 條件 |
| :--- | :--- | :--- |
| `wda.build.tool-unavailable` | Error | 建置需要 Deno，但 Deno 不可用。建置失敗；診斷碼與級別穩定，摘要文字不屬於穩定契約 |
| `wda.build.tokens-css-in-source` | Error | source 端存在 `styles/tokens.css` |
| `wda.build.include-unresolvable` | Error | `wda-include` 的 `src` 找不到檔案 |
| `wda.build.include-escapes-root` | Error | `wda-include` 的 `src` 超出專案根目錄範圍 |
| `wda.build.include-cycle` | Error | `wda-include` 形成循環參照 |
| `wda.build.type-error` | Error | TypeScript type-check 失敗 |
| `wda.build.secret-path-referenced` | Error | 頁面或樣式引用 dotfile 或 `.env*` |
| `wda.build.reference-unresolvable` | Error | 頁面或樣式引用的檔案不存在 |
| `wda.build.unreferenced-asset` | Warning | `assets/` 目錄中存在未被任何頁面或樣式引用的檔案 |
| `wda.build.output-path-collision` | Error | 兩個 source 路徑對應到同一個 `dist/` 路徑 |
| `wda.build.dependency-unlocked` | Error | 相依宣告（deno.jsonc 或 package.json）有宣告或原始碼有引用，但 deno.lock 沒有對應的鎖定，或鎖定檔已過期 |

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

`src/main.rs` 依回傳結果設定結束狀態。`init`、`check` 和 `build` 透過 `ValidationReport` 回報的診斷若含 Error，狀態碼為 `1`。`wda.cli.usage` 和 `deps` 層級的診斷或工具錯誤，以及其他 ToolFault，狀態碼都是 `2`。

`wda.init.tool-unavailable` 和 `wda.init.path-conflict` 透過 `ValidationReport` 回報，因此狀態碼為 `1`。`wda.build.tool-unavailable` 以 ToolFault 回傳，因此狀態碼為 `2`。

```text
0   成功，允許存在 Warning
1   `ValidationReport` 含 Error
2   CLI 用法錯誤、`deps` 錯誤或 ToolFault
```

### 輸出流

Diagnostics 會寫入 stderr。成功結果和其他摘要訊息會寫入 stdout，包括 `wda init` 的 Design System 說明與 `wda build` 的 `dist/` 路徑。

### 單行、可判定、人和 AI 都能直接讀

每筆 diagnostic 都占一行。相同 binary 在輸入和 PATH 狀態相同時，會輸出相同的診斷集合與順序。Check 的工具鏈診斷則會依 Git 和 Deno 是否在 PATH 且能否啟動而變動。排序有明確的平手判定準則（tie-breaker），不受檔案系統走訪順序影響。取用端只需讀取 diagnostic code 和嚴重度，不必依固定欄位切割文字。每筆診斷都會提供 message 和下一步建議，文字可隨版本調整。

### 不提供 `--json` 選項

目前沒有 `--json` 選項，也不提供結構化診斷格式。若使用者傳入 `--json`，程式會在 stderr 拒絕，stdout 不會輸出 JSON，並以狀態碼 `2` 結束，診斷碼為 `wda.cli.json-not-supported`。

若要提議有版本標示的結構化輸出，提案必須指出機器端取用者（consumer），並附上可重現的失敗案例，證明純文字介面缺少必要資訊。符合條件後，可另案提交 ADR 審議。單純表示「consumer 不想處理純文字」並不足以支持提案。第一階段只考慮 `check` 和 `build`。

### 路徑正規化與 control character 防護

路徑會以相對專案根目錄的形式輸出，並以正斜線（`/`）分隔，不會洩漏 host 的絕對路徑。即使專案檔案含有換行或控制字元（control character），也不能藉此插入假的 diagnostic。

### 跨指令的診斷碼

| 診斷碼 | 級別 | 條件 |
| :--- | :--- | :--- |
| `wda.cli.usage` | Error | 命令列用法錯誤，結束狀態碼為 `2` |
| `wda.cli.json-not-supported` | Error | 傳入 `--json` 選項，結束狀態碼為 `2` |
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
| `wda.toolchain.git-missing` | Warning | check |
| `wda.toolchain.deno-missing` | Warning | check / init |
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
| `wda.init.tool-unavailable` | Error | init |
| `wda.init.path-conflict` | Error | init |
| `wda.init.git-identity-fallback` | Warning | init |
| `wda.build.tool-unavailable` | Error | build |
| `wda.build.tokens-css-in-source` | Error | build |
| `wda.build.include-unresolvable` | Error | build |
| `wda.build.include-escapes-root` | Error | build |
| `wda.build.include-cycle` | Error | build |
| `wda.build.type-error` | Error | build |
| `wda.build.secret-path-referenced` | Error | build |
| `wda.build.reference-unresolvable` | Error | build |
| `wda.build.unreferenced-asset` | Warning | build |
| `wda.build.output-path-collision` | Error | build |
| `wda.build.dependency-unlocked` | Error | build |
