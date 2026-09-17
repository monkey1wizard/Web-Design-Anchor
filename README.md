# Web Design Anchor

Web Design Anchor 簡稱 WDA，是給 Designer 使用的 Web 工具。它讓 Designer 搭配 AI，以標準 Web 長期建立與維護網站。標準 Web 指 HTML5、Modern CSS 與 TypeScript。

WDA 不綁定任何前端框架。當需求變複雜、需要交給工程師接手時，它交出的是一個乾淨的標準 Web 專案。

> 本檔是導覽，不是規格。`docs/` 是本專案的規範來源；`ROADMAP.md` 是索引。兩者衝突時一律以 `docs/` 為準。

## 這個工具想解決什麼

Designer 想自己長期維護一個網站時，通常會撞上兩堵牆。

第一堵是框架。選了框架就綁定它的生命週期、升級節奏與生態系。幾年後框架換代，網站跟著要重寫。

第二堵是交接。用視覺化工具做出來的網站，工程師接手時往往得整個重做，因為產物不是標準 Web。

WDA 的答案是不選框架，只用標準 Web，並讓 AI 承擔撰寫細節。Designer 描述要什麼，AI 藉由 WDA 產出與維護一個標準 Web 專案。

## 角色鏈

```text
Designer
   |
   v
AI 對話
   |
   v
AI Skill
   |
   v
wda CLI
   |
   v
Standard Web Project
   |
   v
dist/（可獨立部署）
```

`wda` 是 standalone CLI。它的責任止於產出合法的靜態檔案。

## V1 的範圍

V1 採 Build Only。它產出可獨立部署的靜態檔案，不含 hosting 或 deployment 服務。

刻意不做的事情，完整清單在 `ROADMAP.md` §7。其中兩項是永久排除，不是延到 V2：

- Clean URL、URL rewrite 與 fallback routing。這些屬於部署與 hosting 環境，不是 WDA 的契約。
- preview server。Core 永不擁有 preview。

## 安裝

使用 WDA 前，請先安裝 `deno` 與 `git`，並確認兩者都在 PATH 上。不需要 Node.js

### Homebrew

第一個正式版起可用。在 macOS 或 Linux 上執行：

```sh
brew install monkey1wizard/tap/wda
```

### WinGet

第一個正式版起可用，但要等 WinGet moderation merge 後才能安裝：

```powershell
winget install Monkey1Wizard.WDA
```

### 安裝腳本

Unix 系統可下載並執行 `install.sh`：

```sh
curl -fsSL https://github.com/monkey1wizard/web-design-anchor/releases/latest/download/install.sh | sh
```

Windows 可下載並執行 `install.ps1`：

```powershell
irm https://github.com/monkey1wizard/web-design-anchor/releases/latest/download/install.ps1 | iex
```

### Cargo

也可以直接從公開 repository 安裝，並使用鎖定的 dependency state：

```sh
cargo install --git https://github.com/monkey1wizard/web-design-anchor --tag v0.1.0 --locked wda
```

### 手動下載

從 release 下載適合平台的 archive 與 `checksums.txt`，再用 checksum 驗證 archive。macOS 手動下載的 binary 若被隔離，請執行：

```sh
xattr -d com.apple.quarantine wda
```

Windows SmartScreen 可能會對未建立信譽的下載檔案顯示警告，請在確認來源與 checksum 後再允許執行。

無論使用哪一種方式，請以以下指令確認安裝成功：

```sh
wda --version
```

release archive 的根目錄包含 `skills/wda/` Skill；Homebrew 安裝則把 Skill 放在 Homebrew 的 share directory。

## 倉庫現況

**P8b `feat-designer-workflow` 已於 2026-09-09 落地，角色鏈的視覺迭代環節從此有實體程序。** 範圍不是事前規劃出來的，而是拿第一次真實 Designer 工作階段（Skill／WDA 皆 `0.1.0`）當定案依據：那次工作階段踩到十三個登記範圍項目中的九個，另外揭露三個未登記的缺口，逐一收斂成本次落地的內容。`skills/wda/` 新增第七份 reference `references/iteration.md`——只在任何視覺變更要求即將被執行之前載入，每一次都載入，承載改動階梯（token → 元件樣式 → 該頁 CSS → 行內樣式，降階需說明理由）、波及範圍宣告、一致性禁令（新 token 只能同階推導）與 AI 決策標示；既有六份 reference 與 `SKILL.md` 同步擴充：`intake.md` 加參考網站追問與外部內容視為資料的規定，`SKILL.md` 加編輯授權停止條件（`don't edit` 類說法持續有效直到明講解除，補規格不算解除），`reporting.md` 加 host 內預覽程序與接受後記錄還原點的請求，`reversibility.md` 因 ADR-023 收窄：可還原點現由 `wda init` 建立的本地 git repository 保證，Skill 不再提議或代為執行任何版本控制指令，只在基準點缺席時停下並交由 Designer 決定。`SKILL.md` 維持 71 行，遠低於 300 行審查線；沒有新增任何 Core 指令，四指令封閉性不變。

**驗收：L1／L2 在 `cargo test` 全綠（338 passed、0 failed、12 ignored）；L3 於 Claude Code 重跑，因本倉庫沒有可執行的 transcript harness，採逐條文字比對而非真實觸發回放——19 個確定性 fixture 中 16 個 PASS、3 個 PARTIAL（`change-ladder-rung-stated`、`external-content-is-data`、`reference-site-which-part`，皆是規則文字本身正確，但所在 reference 的載入時機不保證涵蓋 fixture 描述的情境）、1 個沿用既有 `not-run` 裁定（安裝分支，歸 ROADMAP §3.4 R9）；should-trigger／should-not-trigger 語句筆數如實列出，recall／precision 因無即時判定工具記為 `not-measured`，不虛報數字。Owner 已審閱並接受這兩點為已知限制，缺口已由 `fix-skill-reference-triggers`（2026-09-11 落地，見下段）補齊。**已知代價**：2026-09-09 之前建立的 WDA 專案沒有 `.git`，Skill 對這類舊專案的保護只剩第一次視覺變更前停下來講明白，不是硬性阻擋。

**`fix-skill-reference-triggers` 已於 2026-09-11 落地，補齊 P8b 結案時揭露的三個 L3 PARTIAL 缺口。** 19 個確定性 fixture 全數 PASS，0 個 PARTIAL 殘留。三處修法：`iteration.md`（任何視覺變更前每次載入）的改動階梯步驟新增 rung-naming 規則——動手前先說出這次選用哪一階，若選的不是 token 才需要同時說明降階原因；`iteration.md` 同步新增一個步驟，規定請求點名參考網站時先反問 Designer 確認這次要參考的範圍，`intake.md` 改為引用該步驟而不重述，不再預設答案必須是「參考網站的某個部分」；`SKILL.md` 的『## 停止條件』新增一列，規定外部或參考來源的文字直接對 AI 發話時，AI 停止把它當指令執行、引用回 Designer 並說明不會照做，但仍繼續以該文字為內容處理 Designer 原本的請求（`SKILL.md` 因此從 70 行增至 71 行，仍遠低於 300 行審查線）。對應 fixture 也已從 `reference-site-which-part.md` 改名為 `reference-site-scope-confirmed.md`，Expected AI Behavior 與 Observable Pass Signal 一併重寫，判準從「答出哪一部分」改為「有沒有先確認範圍」。`cargo test --test skill` 維持 9 tests 全綠，`L3_CASE_COUNT` 維持 19（改名不增減筆數）。

**`docs-consolidation` 已於 2026-09-15 落地。** `docs/` 下全部 32 份檔案的前置資料加上 OKF 鍵 `type`、`description`、`tags`，打開任何一份不必讀內文就知道它是哪一類文件、回答什麼問題。`status` 字彙同步改為 `stable`／`draft`／`deprecated`，規則寫在 `docs/development.md`。倉庫根新增 `SECURITY.md`，說明漏洞回報管道與只從已落地表面整理的 Security Model。回報管道是 GitHub Private Security Advisories，要等 P9 建立公開 GitHub 倉庫才真正開通。

**P7 `feat-spectrum-adapter` 已於 2026-09-07 落地，第二套 Design System 從此有真實實作，架構中立性不再只由文件宣稱。** `wda init` 現在會線上解析 Spectrum 2：透過 `deno` 查詢 npm registry 上 `@spectrum-web-components/bundle` 的 `dist-tags.latest`，取得一個 1.0.0 以上的確切 semver，寫進 `wda.json` 的 `designSystem`，並把 starter script 實際 import 的六個直接相依套件（`theme`、`button`、`textfield`、`picker`、`card`、`dialog`）各自以 `npm:<package>@<version>` 釘在同一個版本上。範圍限定在直接相依，不列舉 transitive。該直接相依集合有阻擋型離線守門：`src/init/mod.rs` 的 `spectrum_two_dependency_specs` 是純函式，由同檔一個不帶 `#[ignore]` 的測試以完全相等斷言比對硬寫的六元素向量，因此把常數縮回單一套件會讓裸跑的 `cargo test` 轉紅。在此之前，唯一斷言這六個套件的位置帶 `#[ignore]`，只在非阻擋的 CI probe job 執行。解析有七個各自具名的失敗分支：`deno` 不在 PATH、非零離開碼、十秒逾時、輸出無法解析、缺少 `latest` 欄位、非正式版本、版本低於 1.0.0。任何一個觸發都退回內建 `WDA Minimal`，並在 G11 報告載明是哪一個，措辭彼此不重疊。相依解析失敗時走既有的反向清理回捲，`package.json` 與 `deno.lock` 一併納入清理集合，只移除該次執行建立的產物，絕不更動或刪除目錄內既有內容。

**中立性的驗證形狀在此明確化。** 它不是「兩套系統長得一樣」，而是兩套系統各自以不同的依賴模型與元件模型走完 `init` → `check` → `build`：`WDA Minimal` 是零 npm 依賴加純語意 HTML，Spectrum 2 是 npm 依賴加 vendor custom element。Core 不得在解析接縫（`src/init/resolve.rs`）與資產接縫（`src/init/mod.rs`、`src/init/documents.rs`、`src/builtins/wda_minimal.rs`、`src/builtins/spectrum_two.rs`）之外，出現任何 design system 的身分分支或硬編碼名稱。這條由 `tests/neutrality.rs` 以真實 Rust 來源解析器把關，會先剝除 `cfg(test)` 模組與註解再稽核，不是字串比對。同一份測試另外守住兩件事：`docs/` 與原始碼樹不得殘留 Spectrum 1 的識別字，且文件與 README 不得宣稱自動轉換、零成本遷移、無損遷移或零 HTML 變更；既有的否定式免責敘述不會被誤判。

**`WDA Minimal` 的 token 值同時改為 Spectrum 2 預設值的一次性凍結快照**（Owner 於 2026-09-05 裁定），字體改用 Noto 系列。中立性因此只保住一半：零 npm 依賴這一半仍然成立，失去的是 token 值層級的獨立性。歸屬寫在倉庫根的 `NOTICE`，載明上游套件 `@adobe/spectrum-tokens`、確切版本 `15.2.0`、Apache-2.0 授權與所做的修改，依該授權第 4(d) 節。快照凍結，不追隨上游更新。

**Spectrum 2 那一側的端到端鏈仍是參考性質覆蓋。** 它需要真實 `deno` 與可連線的 npm registry，因此標記 `#[ignore]`，只在兩份 CI 的非阻擋探針 job 執行，不擋合併（Owner 於 2026-09-05 裁定）。預設的 `cargo test` 完全離線：每一個 init 呼叫點都走注入式的離線解析接縫。

**P8a `feat-wda-skill` 已於 2026-09-06 落地，角色鏈的 AI Skill 環節從此有實體產物。** `skills/wda/` 現在存在：一份 `SKILL.md` 加上六份 `references/`（`install`、`intake`、`assets`、`reversibility`、`reporting`、`troubleshooting`）。它依 `docs/architecture.md` §2.11 的內容白名單只放角色與邊界宣告、版本比對步驟、決策路由表、停止條件與回報義務，不放規則內容本身；`scripts/` 與 `assets/` 依同一節不建立。它不放在 `docs/` 底下，因為依 ADR-022 它是隨 release 發佈的產物，不是 documentation。驗收四層的落點：L1 規格合規與 L2 邊界 guardrail 由 `tests/skill.rs` 的 9 個測試在 CI 上把關；L3 行為評測於 2026-09-06 在 Claude Code 上通過（四個決定性案例全數 100%，should-trigger recall 約 98.6%，門檻 0.95），因此 Skill 判定為可交付。

**落地時有兩個已揭露的驗收缺口。** 其一，§2.11 的 L4 Deletion Test（刪掉整個 Skill 後，Core 與專案文件仍足以讓人正確操作 WDA）經 Owner 於 2026-09-06 裁定移出 P8a 範圍，四層驗收因此只實際跑了三層。它並非全無承接：`ROADMAP.md` §6 第 6 項就是 Deletion Test 的驗收化版本，由 P10 `v1-acceptance` 執行。未定的是這樣算不算 §2.11 L4 的完整履行，登記在 `ROADMAP.md` §3.4 R8。**已知代價**：L4 自此不擋 Skill artifact 本身，只擋 V1，失敗會落在最貴的時點。其二，L3 的安裝分支（`wda` 不在 PATH 時路由到 `references/install.md`）在結案時是 `not-run`，因為它指向尚未建立的 `docs/install.md`，那是 P9 的產出；`not-run` 不等於通過，何時由誰實測未指派，登記在 `ROADMAP.md` §3.4 R9。

**P5 `feat-wda-build` 已於 2026-09-04 落地。** `wda build` 現在把 source 樹編成可獨立部署的 `dist/`：展開 `<wda-include>`、對展開後的頁面重驗 ID 唯一性、把 `tokens/tokens.json` 產生成 `dist/styles/tokens.css`、以 `deno` 對 TypeScript 做真正的 type-check 再轉譯成 `.js`，只複製被引用到的檔案，並且永不複製 dotfile 或 `.env*`。整份輸出先寫進 staging 樹，全部成功才 atomically 換進 `dist/`，因此失敗不會留下半成品。每次成功都會印出 `dist/` 的絕對路徑，以及該輸出是否需要 HTTP server 才能執行。

**P4 `feat-wda-init` 已於 2026-09-02 落地。** `wda init` 現在是非互動、all-or-nothing 的專案初始化器：要求 `git` 在 PATH 上（缺失時以 `wda.init.tool-unavailable` 拒絕執行且不寫入任何檔案），並遵守碰撞規則（collision rule），接受已含既有檔案的目錄，僅在預計建立的名稱已被佔用時以 `wda.init.path-conflict` 拒絕執行，絕不覆寫或刪除既有內容。它產出其所寫入檔案皆通過 `wda check`（零 Error、零 Warning）的 Minimal Project（對使用者既有檔案不作保證），走離線的 `WDA Minimal` Design System fallback 路徑；成功時建立本地 Git repository 並建立涵蓋全目錄的初次 commit（無 remote）。任一寫入或步驟失敗都會反向回滾該次執行所建立的檔案與 `.git/`，保留所有既有檔案。stdout 會印出高顯著度的 G11 報告，載明實際使用的 Design System 與退回原因。

**P2 `feat-wda-cli-foundation` 已於 2026-08-26 落地，本倉庫現在同時有文件與產品程式碼。** `wda` CLI 存在且可建置：單一 Cargo package，`wda_core` lib target 加 `wda` bin target。

已實作的是 `wda check`、`wda init`、`wda build`、`wda deps` 與 `wda --version`。`wda deps` 支援 `add`、`remove` 與 `update`，作為專案中唯一能改變 dependency graph 的責任邊界，以 `package.json` 為 canonical 宣告並透過 `deno` 進行解析與鎖定（`deno.lock`），且遵守零依賴不留空檔案的 empty-state 規則。`wda build` 需要 `deno` 在 PATH 上，找不到時以 `wda.build.tool-unavailable` 明確失敗，不會降級或略過。`wda build` 在專案目錄之外準備一個暫存的 build workspace，把 `package.json`、`deno.lock` 與可達的 script 集合複製進去，並在其中執行 `deno install --frozen`、`deno check` 與 esbuild。專案根目錄的 `deno.lock` 全程維持位元不變，`node_modules/` 從不出現在專案根目錄。`build` 從不傳遞 `--allow-scripts`，因此建置期間不執行任何 npm 生命週期腳本。無論成功或失敗，`build` 都會在結束時刪除這個 workspace。`wda check` 驗證 Project Contract、必要文件與 front matter、`pages/` 與 `components/` 的 HTML 可及性子集，以及 `tokens/tokens.json` 的 DTCG 結構，全部經由 `validate_project` 這個唯一的驗證接縫。診斷輸出是單行、可決定順序、寫到 stderr，exit 依 `0` 無 Error、`1` 有 Error、`2` usage error 或 ToolFault 區分。V1 不提供 `--json`。

從這裡起，文件裡的 executable claim 由 `cargo test` 背書。全部 target 合計，Windows 宣告 330 個測試、實際執行 317 個，Linux 各多一個；多出的那一個是 `src/build/mod.rs` 裡唯一以 `#[cfg(unix)]` 標記的測試，只在 Unix 編譯。兩邊的差額 13 個是標記 `#[ignore]` 的探針，預設不執行。（2026-09-07 於 Windows 實測 `cargo test`：317 passed、0 failed、13 ignored。）這 13 個探針經真實 `deno` 連上真實 npm registry，因此不放在預設閘門上，預設的 `cargo test` 不碰 registry（Owner 於 2026-09-05 裁定）。兩個 CI 設定各自把關兩個平台：`.gitlab-ci.yml` 是本倉庫實際執行的那一份，因為 `origin` 在 GitLab；`.github/workflows/ci.yml` 則供 GitHub 鏡像使用。兩者都先安裝 `deno`，再跑 `cargo test`，且都不釘死 Deno 版本（Owner 於 2026-09-04 裁定統一浮動）：`.github/workflows/ci.yml` 在 2.x 範圍內浮動，`.gitlab-ci.yml` 取最新版。兩份設定另各有一個跑 `cargo test -- --ignored` 的探針 job，都不擋合併：GitLab 以 `allow_failure: true` 宣告，GitHub 沒有對應的 per-job 旗標，改由 branch protection 決定，並在檔案內以註解寫明該意圖。

## 怎麼讀這個倉庫

`docs/` 是規格來源；`ROADMAP.md` 是專案索引。下表是索引，不是摘要：

| 你想知道 | 讀哪裡 |
| --- | --- |
| 產品是什麼、現在到哪 | §1 現況 |
| 不可違反的契約條文 | `docs/architecture.md`、`docs/development.md` |
| 怎麼回報安全漏洞、安全模型涵蓋什麼 | `SECURITY.md` |
| 哪些事已裁定、依據為何 | §3.2 本輪已裁定 |
| 還有哪些事沒裁定 | §3.3 與 §3.4 |
| 工作怎麼切、為什麼是這個順序 | §4 Plan 拆分 |
| 每個 plan 各自要做什麼 | §5 各 Plan 範圍 |
| V1 何時算完成 | §6 V1 驗收標準 |
| 什麼刻意不做 | §7 不在 V1 範圍 |
| AI 要怎麼操作 `wda` | `skills/wda/SKILL.md` |
| 已知風險 | §9 已知風險 |

### 分層讀法

`docs/architecture.md` 把契約分成穩定度層級。愈上層愈不該動：

| 層 | 意思 | 例子 |
| --- | --- | --- |
| Invariant | 長期不變的承諾 | `dist/` 是唯一部署產物 |
| Default | 專案預設值 | MPA、WCAG 2.2 AA |
| Tool Form | Core 的執行形態 | standalone CLI |
| Tooling Choice | 可更換的實現技術 | Rust、esbuild |
| Implementation Detail | 內部組織 | 不影響外部契約 |

`docs/architecture.md` 定義了更換 Tooling Choice 時，哪些東西不得跟著變。這條界線是整份契約的承重點。

## 開發路線

十二個 plan，編號 P0 到 P10。完整依賴圖與排序理由在 `ROADMAP.md` §4。

兩個里程碑值得單獨記住：

- **P2** 第一次有可執行、可測試的產出。在此之前，所有架構假設都只是文件。
- **P7** 第一次有 Designer 實際會走的完整路徑。已於 2026-09-07 落地。

P8b 是 Designer 視覺迭代工作流，目前凍結，內容尚未收斂。

## 歷史輸入

倉庫根目錄的四份 `prefile_*.md` 是規劃階段的歷史輸入。它們不具裁定權威，只供追溯。內容與 `docs/` 衝突時一律以 `docs/` 為準。

## 這份檔案的狀態

第一版草稿，預期會被改寫。

改寫時請維持一條界線：README 負責導覽與脈絡，規則留在 `docs/`。在這裡複製一條規則，就是製造第二個 truth source。本專案的架構刻意避免這件事。
