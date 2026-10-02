---
title: "指令"
status: stable
updated: 2026-09-22
type: Reference
description: "四個指令各自的前置條件、讀寫範圍、規則清單、拒絕條件，以及診斷輸出的細則。"
tags: []
---

# 指令

Core 僅有四個指令。指令集採封閉設計，不得為了 Designer 工作流程、Skill 安裝或 preview 需求新增第五個指令。Core 並非通用編輯器，日常的 source 與文件編輯皆由 Designer 與 AI 直接修改專案檔案。

```text
wda init    建立新專案與本機 Git 儲存庫（Repository）
wda deps    管理 dependency graph
wda check   驗證，不產出
wda build   驗證，通過後建置輸出 dist/
```

`wda --version` 僅印出版本資訊，供 Skill 進行版本比對，並非第五個指令。

## 外部工具

### deno 與 git 的 PATH 使用方式

`deno` 負責相依性解析（dependency resolution）與轉譯器呼叫。`git` 由 `wda init` 用以建立本機 Git 儲存庫；`wda check` 也會探測 Git 並回報工具鏈狀態。`wda init` 在寫入前探測 Git 是否可啟動；若 Git 不在 PATH 或雖已找到但無法啟動，指令會中止。Minimal 初始化可在沒有 `deno` 的環境中完成。若初始化路徑需要 Deno 解析或安裝相依套件，Deno 不在 PATH 或無法啟動時會退回 `WDA Minimal`。

`wda check`、`wda build` 與 `wda deps` 不要求既有的 Git 儲存庫（repository），亦不要求 `.gitignore` 存在。

### esbuild 由 WDA 釘死版本、經 deno 呼叫

TypeScript 轉譯器由 WDA 以 `deno run -A npm:esbuild@<釘死版本>` 呼叫。其版本由 WDA 決定，非由執行環境決定，因此 `esbuild` 絕非 PATH 前置條件，Node.js 亦同。

### 哪個指令需要哪個工具

| 指令 | `deno` | `git` |
| :--- | :--- | :--- |
| `init` | 選用；Deno 不在 PATH 或無法啟動時，以 WDA Minimal 有限初始化，不進行線上 Design System 解析或相依套件安裝 | 需要；不在 PATH 或無法啟動時，以 Error 中止且不寫入任何內容 |
| `deps` | 需要 | 不需要 |
| `check` | 選用；不在 PATH 時輸出 `wda.toolchain.deno-missing` Warning，找到但無法啟動時以 `wda.tool.fault` Warning 回報；兩種情況皆仍成功 | 選用；不在 PATH 時輸出 `wda.toolchain.git-missing` Warning，找到但無法啟動時以 `wda.tool.fault` Warning 回報；兩種情況皆仍成功 |
| `build` | 需要 | 不需要 |

`wda init` 缺少或無法啟動 Git 時會發出 `wda.init.tool-unavailable` Error，並在寫入前中止。Deno 不在 PATH 或無法啟動時，`wda init` 會以 WDA Minimal 有限初始化。此路徑不進行線上 Design System 解析或相依套件安裝。Deno 不在 PATH 時另發出 `wda.toolchain.deno-missing` Warning。`wda check` 對 Git 或 Deno 不在 PATH、或找到但無法啟動，皆只發出 Warning，不會單獨導致失敗。

## wda init

### 建立什麼

`wda init` 於目前目錄建立新專案。執行時不進行任何互動提問，全數採用預設值。

`WDA Minimal` 路徑寫入：

```text
.git/                   本機 Git 儲存庫（Repository）
.gitignore              內容固定為 /dist/
README.md
wda.json
docs/architecture.md
docs/design.md
docs/naming.md
pages/index.html
tokens/tokens.json
```

線上初始化時，`WDA Minimal` 路徑會以 best-effort 嘗試安裝 `lit`。安裝成功時產生 `deno.jsonc` 與 `deno.lock`。若離線或安裝失敗，初始化仍順利完成，但不產生任何相依檔案，並於 stdout 輸出缺少 Lit 的退回原因與 `wda deps add lit` 復原建議。`WDA Minimal` 預設不包含 starter script。

Spectrum 2 路徑則額外寫入 `scripts/main.ts`。線上解析時，路徑會安裝六個 Spectrum 2 套件與 `lit`，產生 `deno.jsonc` 與 `deno.lock`，並將 starter script 實際 import 的套件鎖定於相同版本。

檔案寫入成功後執行 `git init`，並以涵蓋整個目錄的初始 commit 收尾。該 commit 訊息固定為 `wda init`，不設定遠端儲存庫（remote）。Designer 原先留存於目錄中的既有檔案亦會一併納入此 commit 中。

`.gitignore` 僅為單次建立的便利產物。在 `wda init` 完成後，所有 WDA 指令皆不再對其進行讀取或寫入。

### 碰撞規則

目標目錄中允許預先存在其他檔案。然而，只要本次欲建立的任何檔案或目錄名稱已被佔用，`wda init` 在進行任何磁碟寫入前便會拒絕並中止。欲建立的名稱包含上述清單中的各檔案、其所需目錄與 `.git/`。所有路徑皆無條件將 `deno.jsonc`、專案端 `deno.json` 與 `deno.lock` 列入碰撞檢查集合中。在此集合以外的既有檔案內容一律不讀取、不更動，亦不進行合併。

### Design System 解析與退回

`wda init` 首先線上解析 latest stable Spectrum 的確切版本。解析成功時將該版本寫入 `wda.json` 並安裝相依套件；解析失敗或相依安裝失敗時，則退回使用 `WDA Minimal`，並於 stdout 說明實際採用的 Design System 與退回原因（包含相依安裝失敗的復原指示）。退回並不視為執行失敗。有關解析失敗的判定條件，詳見 `docs/design-system.md`。

### 失敗時的還原清理

`wda init` 採不可分割（all-or-nothing）機制。若任一步驟寫入失敗，即會反向清理所有已寫入的內容，使目錄完整復原至執行前的初始狀態。新建立的 `deno.jsonc` 與 `deno.lock`（無論是 Spectrum 2 或 WDA Minimal 路徑）皆納入還原清理集合中，在失敗時依序反向移除；若 Spectrum 2 相依安裝失敗退回 WDA Minimal，亦會清理 Spectrum 2 專屬檔案後再重新建立 Minimal 專案。

### 拒絕條件與診斷碼

| 診斷碼 | 級別 | 條件 |
| :--- | :--- | :--- |
| `wda.init.tool-unavailable` | Error | `git` 不在 PATH，或雖已找到但無法啟動；訊息指出實際原因。不寫入任何內容 |
| `wda.init.path-conflict` | Error | 觸發名稱或路徑衝突（碰撞規則）。不寫入任何內容 |
| `wda.init.git-identity-fallback` | Warning | 環境缺少 git 使用者身分設定，init 以臨時身分完成 commit |

## wda deps

### 宣告與鎖定

現行專案以 `deno.jsonc` 的 `imports` 欄位作為 npm 相依套件的宣告處，使用穩定的 `npm:<package>@<version>` 規格，並鎖定於 `deno.lock`。同時，WDA 完整保留對既有 `package.json` 專案的向下相容支援。`wda deps` 在執行前自動判斷專案格式。若專案根目錄存在 `package.json`，則走 legacy 分支。否則走現行 `deno.jsonc` 分支。

`wda deps` 藉由 `deno` 進行解析（現行格式明確指定 `--config deno.jsonc`，legacy 格式指定 `--package-json`），並鎖定於 `deno.lock`。檔案寫入權全權由 Deno 擁有（Deno-owned writes），Deno 會妥善保留 `deno.jsonc` 中的註解、尾端逗號與其他自訂欄位。此指令為唯一能改變相依關係圖（dependency graph）的指令。`wda build` 僅使用已鎖定的結果，絕不自動挑選新版本。

### add、remove、update 的各自職責

```text
wda deps add <套件>      加入一個相依套件並鎖定
wda deps remove <套件>   移除一個相依套件並更新鎖定
wda deps update [套件]   在既有宣告內重新解析並鎖定
```

### 暫存驗證與不可分割提交

`wda deps` 在變更專案檔案前，一律先在隔離的暫存目錄中執行 Deno 指令，並以 `deno install --frozen` 進行前置圖驗證（staged validation）。執行時絕不帶入 `--allow-scripts` 參數，因此嚴格禁用任何套件的 npm 生命週期腳本（lifecycle scripts）。

驗證成功後，變更以不可分割事務方式提交回專案，並遵循嚴格的固定順序。WDA 先寫入、更名或刪除 `deno.lock`，再更新相依宣告檔（`deno.jsonc` 或 `package.json`）。若中途發生 I/O 錯誤，會自動還原檔案內容至操作前的原始位元組，避免留下損毀或不一致的狀態。

### 語意空狀態規則

在無實際相依套件時，專案不保留空白的宣告檔或鎖定檔：

- 對於現行 `deno.jsonc` 專案：當移除最後一個相依套件時，WDA 檢查根層級的語意結構。若檔案中除空 `imports` 之外已無其他語意欄位，則將 `deno.jsonc` 刪除（單純註解不視為保留檔案的依據）；若仍包含其他自訂根欄位，則保留檔案僅清除 `imports`。當 `deno.lock` 僅剩版本標頭（`{"version": ...}`）時，一併刪除空白的 `deno.lock`。
- 對於 legacy `package.json` 專案：移除最後一個相依套件後，一併刪除 `package.json` 與 `deno.lock`。
- 離線建立的 `WDA Minimal` 專案預設即不包含這兩個檔案。

### 拒絕條件、復原與診斷碼

`wda deps` 無專屬之診斷碼，皆以跨指令診斷碼回報：

- 若專案端存在 `deno.json`（或同時存在 `deno.json` 與 `deno.jsonc`），`wda deps` 會以 `wda.tool.fault` 拒絕並中止，要求使用者將設定檔更名或調整為 `deno.jsonc`。WDA 絕不指示使用者建立或編輯專案端 `deno.json`。
- 若命令用法錯誤，以 `wda.cli.usage` 拒絕（結束狀態碼為 `2`）。
- 若環境缺少 `deno`、解析失敗、或暫存驗證失敗，以 `wda.tool.fault` 拒絕（結束狀態碼為 `2`）。執行失敗時不寫入亦不更動專案檔案。
- 復原方式：若專案初始化時因離線而未安裝 Lit，使用者可在網路連線後執行 `wda deps add lit` 補行安裝。

## wda check

### 與 build 共用同一套 validation engine

`wda check` 為唯讀指令。它不寫入任何專案檔案，亦不更新 `wdaVersion`。它與 `wda build` 共用專案契約、文件、HTML 與 Tokens 專案驗證規則。`check` 另會檢查 PATH 中的可選工具。Git 或 Deno 不在 PATH 時會以對應的 `wda.toolchain.*-missing` Warning 回報；工具已找到但無法啟動時會以 `wda.tool.fault` Warning 回報。這些 Warning 不會改變成功狀態。Build 不執行這項 PATH 檢查。專案正確性不能依賴「使用者記得先執行 check」來維持。

Check 與 Build 都執行四個專案驗證規則模組，對應 `src/validation/` 的四個檔案；Check 另依 PATH 狀態產生工具鏈診斷。Build 不執行 PATH 工具檢查。Build 需要 Deno 時若 Deno 不可用，會以 `wda.build.tool-unavailable` Error 中止；現有錯誤摘要不構成穩定文字契約。

### 規則模組：專案契約

驗證 `wda.json` 是否存在、為合法 JSON 格式、符合 `schemas/wda.schema.json` 規範，且無未知欄位。欄位規範詳見 `docs/generated-project.md` 的 `wda.json` 欄位表。

| 診斷碼 | 級別 |
| :--- | :--- |
| `wda.contract.missing` | Error |
| `wda.contract.malformed-json` | Error |
| `wda.contract.unknown-field` | Error |
| `wda.contract.schema-violation` | Error |

### 規則模組：文件

驗證四份必要文件是否存在：`README.md`、`docs/architecture.md`、`docs/design.md`、`docs/naming.md`。位於 `docs/` 目錄下的三份文件必須具備 YAML front matter，並包含不得為空的 `title`、`status`、`updated`。其中 `updated` 格式須為 `YYYY-MM-DD`。其餘 `docs/**/*.md` 僅於檔案存在時才驗證，若 front matter 存在問題則降級為 Warning 回報。若 front matter 包含 `sourceRefs`，各項目必須能解析為專案內的檔案或章節。

文件一致性檢查僅針對結構與 metadata。不比對文件內文與程式碼、`wda.json`、tokens 或相依性狀態的差異。內容漂移由人工審查負責。

| 診斷碼 | 級別 |
| :--- | :--- |
| `wda.docs.required-document-missing` | Error |
| `wda.docs.front-matter-missing` | Error |
| `wda.docs.front-matter-malformed` | 必要文件 Error，選用文件 Warning |
| `wda.docs.front-matter-field-invalid` | 必要文件 Error，選用文件 Warning |
| `wda.docs.source-ref-unresolvable` | Warning |
| `wda.docs.source-ref-escapes-root` | Warning |

### 規則模組：HTML

僅驗證三條可由原始程式碼直接明確判斷的規則：`html[lang]` 不得為空、每個 `img` 標籤皆具備 `alt` 屬性、文件內的 `id` 具唯一性。`alt` 檢查僅驗證屬性是否存在，不評判文字內容優劣。

其餘無障礙功能（a11y）判定皆交由人工審查：`alt` 文字是否具實質意義、疊加圖片後的對比度、焦點（focus）的可見性與順序、鍵盤操作支援、縮放與重排表現，以及動態效果。即使所有自動檢查皆合格，亦不代表符合 WCAG 規範。`wda check` 亦不會因此產生「需要人工審查」的 Warning 提示。

| 診斷碼 | 級別 |
| :--- | :--- |
| `wda.html.lang-missing` | Error |
| `wda.html.img-alt-missing` | Error |
| `wda.html.duplicate-id` | Error |

### 規則模組：Tokens

驗證 `tokens/tokens.json` 為合法 JSON 格式、符合 DTCG 規範、每個 token 皆具備設定值、型別可判定，且別名（alias）可正常解析。

| 診斷碼 | 級別 |
| :--- | :--- |
| `wda.tokens.malformed-json` | Error |
| `wda.tokens.value-missing` | Error |
| `wda.tokens.type-unresolvable` | Error |
| `wda.tokens.alias-unresolvable` | Error |

## wda build

### 必要驗證與 build output 規則模組

`wda build` 首先執行與 `check` 相同的四個專案驗證規則模組。若出現任何 Error 即立即中止，不更動 `dist/` 目錄；Warning 則不阻擋建置。在驗證確認無 Error 後方進入產出階段；產出階段另包含第五個規則模組 build output，對應 `src/validation/build_output.rs`。

### wda-include 展開與不建立 template language

唯一的 WDA 自訂 build-time 語法是：

```text
<wda-include src="..."></wda-include>
```

它符合 Custom Element 的命名規範，在建置時展開為純 HTML。它並非執行階段（runtime）的 Web Component，`dist/` 中不得殘留此標籤。`src` 屬性僅能指向專案內的檔案，不得超出專案根目錄範圍，亦不得形成循環參照。

專案不提供樣板語言（template language），不支援 `<slot>`、`if`、`for`、變數、運算式或資料繫結。一般版面與靜態內容應優先使用原生語意化 HTML 與 CSS。唯有確實需要可重用行為、可重用產品語意或元件封裝時，方建立元件。

### 暫存 build workspace

`wda build` 於專案目錄之外建立暫存工作區（build workspace）。內部工作區由 WDA 寫入專屬且嚴格的 JSON 格式 `deno.json`，承載 WDA 要求的編譯與模組設定（包括 `nodeModulesDir: "manual"` 與標準 `lib` 清單）。

針對現行格式專案，WDA 解析專案端 `deno.jsonc` 的 `imports` 映射，並將其合併至工作區的 `deno.json` 中；專案端其餘自訂欄位一律不帶入工作區。若專案端存在專案層級的 `deno.json`，建置將回報 ToolFault 錯誤並拒絕執行。針對既有 `package.json` 專案，則將 `package.json` 原樣複製至工作區。

僅當選定格式的相依宣告非空（現行 `imports` 非空，或 legacy `dependencies` 非空）時，方於工作區內執行 `deno install --frozen`。若相依宣告為空，則跳過安裝步驟且不要求 `deno.lock` 存在。執行時絕不帶入 `--allow-scripts` 參數，因此不執行任何套件的 npm 生命週期腳本（lifecycle scripts）。建置結束後，不論成功或失敗，皆會刪除此暫存工作區。專案根目錄的 `deno.lock` 在整個建置過程中不會有任何位元組（byte）的變更。

### 只取已鎖定的 dependency

`wda build` 僅使用 `deno.lock` 所鎖定的版本。它不挑選新版本、不變更相依關係圖（dependency graph），亦不進行任何靜默升級。建置時會將所有相依模組一同打包（bundle）至 `dist/`，不殘留任何遠端 runtime 相依性。

### secret 不進 dist/

建置時絕不複製 dotfile 或 `.env*`。若頁面或樣式引用此類路徑，建置將回報 Error 並拒絕執行。此為檔案層級的排除機制，不進行檔案內容偵測。

### 成功時報出 dist/ 絕對路徑

建置成功時將更新 `wda.json` 的 `wdaVersion` 欄位，並於 stdout 印出 `dist/` 的絕對路徑；建置失敗時則不更新 `wdaVersion`。若偵測到 ES module、`fetch` 或其他已知不相容於 `file://` 協定的特徵，將同時提示產出物需要 HTTP host 伺服環境。

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

每筆診斷皆包含 severity（嚴重度）與 diagnostic code（診斷碼）。severity 依據實際影響判定，不依問題類別武斷劃分，亦嚴禁硬性規定「所有 a11y 皆為 Warning」或「所有 script 問題皆為 Error」。唯有存在具體且可採取改善行動的條件時方發出 Warning，不得每次皆產生無法消除的「需要人工審查」提示。

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

message（訊息內文）與下一步建議必須存在且清晰易懂，但逐字文字內容允許後續演進。標點符號、分隔符與欄寬皆不構成契約，取用端（consumer）不得依賴切割文字欄位的方式進行解析。

### Exit status

```text
0   無 Error，允許存在 Warning
1   驗證結果包含任何 Error
2   CLI 用法錯誤、指令未執行完成，或發生 ToolFault
```

### 輸出流

Diagnostics 寫入 stderr。成功結果與非 diagnostic 的摘要訊息則寫入 stdout：包含 `wda init` 的 Design System 說明，以及 `wda build` 的 `dist/` 路徑。

### 單行、可判定、人和 AI 都能直接讀

每筆 diagnostic 皆為單行文字。同一 binary 面對相同輸入與相同 PATH 狀態時，必產生完全相同的集合與順序；Check 的工具鏈診斷會依 PATH 中 Git 與 Deno 是否存在及能否啟動而改變。排序具備明確的平手判定準則（tie-breaker），不依賴檔案系統的走訪順序。診斷的解析僅依據 diagnostic code 與嚴重度級別，不仰賴固定欄位切割。message 與下一步建議必須存在，但逐字文字內容允許隨版本演進。

### 不提供 `--json` 選項

不提供 `--json` 選項，亦不承諾任何結構化的診斷格式。若使用者傳入 `--json`，將於 stderr 明確拒絕，stdout 不輸出任何 JSON，並以狀態碼 `2` 結束，診斷碼為 `wda.cli.json-not-supported`。

若未來欲提出具版本標示的結構化輸出，必須先具備具名的機器端取用者（consumer），並附上可重現的失敗案例，證明純文字介面確實缺少必要資訊。唯有滿足此項條件後，方能另案提出並重新循 ADR 流程審議。「consumer 不想處理純文字」並非充分理由。首批可考慮納入的範圍僅限 `check` 與 `build`。

### 路徑正規化與 control character 防護

路徑一律輸出為相對於專案根目錄的相對路徑，以正斜線（`/`）分隔，絕不洩漏 host 環境的絕對路徑。即使專案檔案內容包含換行或控制字元（control character），亦不得注入偽造的第二筆 diagnostic。

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
