---
title: "Skill 規格"
status: stable
updated: 2026-10-15
type: Architecture
description: "Skill 的定義、維持單一 Skill 的理由、目錄與檔案配置、歸屬權責劃分、版本控制、禁止收錄可執行腳本、跨平台可攜性與四層驗收標準。"
tags: []
source: docs/skill-spec.md
lang: zh-TW
source_sha256: 962b1d6e39fa1240733250f3fd6dbabc03d81165761547abf1bef9cc278b7fe2
translated_at: 2026-10-23
---

# Skill 規格

本文件為 Core 對 Skill 層的契約規範。Skill 的實體位於 `skills/wda/` 目錄。本文件不重複贅述其具體內容，僅規範其允許與禁止包含的項目，以及相應的驗收標準。

## Skill 的定位與職責邊界

Skill 為 AI 的操作手冊與工作流程層。它指引 AI 如何辨識與安裝 `wda`、如何解析專案結構、何時呼叫特定指令、如何修改專案，以及如何維護專案文件。

Skill 並非規格來源。它不重複保存核心規則，也不作為第二套產生器（generator）。規範規則只存在於專案文件與 Core。Core 與專案清單的契約不能預設只有 Skill 會進行操作。

兩者的職責劃分原則為：**流程程序歸入 Skill，品質判準歸入專案文件。** 品牌與視覺風格的詢問方式及順序屬於流程程序，歸入 Skill。視覺美感的判準則屬於品質判準，由專案的 `docs/design.md` 規範。

## 僅維持單一 Skill 的理由

### 切分依據為觸發條件（Trigger）的可鑑別度

呼叫方明確表達操作意圖後，CLI 才會執行所選指令。若選錯，系統會立即回傳錯誤。AI 則依語意選擇 Skill，當下可能還不清楚具體意圖。即使選錯，AI 也不會收到錯誤，而可能直接走錯流程。因此，「職責不同即可新增指令」的原則不適用於 Skill。只有 AI 在載入 Skill 前能毫無歧義地判斷該載入哪一個時，才適合拆成兩個 Skill。

### 三種經評估否決的拆分方式

| 拆分方式 | 否決理由 |
| :--- | :--- |
| 依 CLI 指令拆分 | 單一使用者請求可能跨越數個指令 |
| 依 Designer 工作階段拆分 | 各工作階段的觸發語意高度重疊；若每份 Skill 都要重述相同規則，重複文字就會成為第二套產生器的起點 |
| 依使用者角色拆分 | 若另設 Engineer 專用 Skill，就等於承認 Engineer 也要依賴 Skill 才能操作 WDA |

因此專案僅維持單一 Skill，其名稱為 `wda`。

## 目錄與檔案結構

### 目錄與檔案配置

```text
skills/wda/
├── SKILL.md
└── references/
    ├── install.md          wda 未加入 PATH 環境變數時
    ├── intake.md           建立新專案、或重大風格轉向之前
    ├── assets.md           Designer 提供素材時
    ├── reversibility.md    第一次視覺變更之前、或要求還原時
    ├── iteration.md        每次視覺變更前及明確設計審查時載入
    ├── reporting.md        觸發回報義務時
    └── troubleshooting.md  check 或 build 回報 Error 時
```

`scripts/` 與 `assets/` 不得建立。

### 配置規範標準

目錄配置遵循 Agent Plugins 1.0.0 開放標準[agent-plugins.org](https://agent-plugins.org/specification)。`skills/<name>/SKILL.md` 為該標準與 Claude Code 共同支援的探索規則。release package 根目錄配置最小化的 `plugin.json`，僅包含 `$schema` 與 `name` 兩個欄位。整個 package 因而符合標準套件規格。任何相容的用戶端都可直接安裝。安裝程序屬於 AI host 的範疇，機制僅在於此。Core 內部不需另設安裝指令。

### SKILL.md 內容白名單

`SKILL.md` 僅允許收錄以下六類內容，其餘項目一律視為違規：

1. 角色職責與邊界宣告
2. 版本比對的執行步驟
3. 決策路由表
4. 流程停止條件
5. 回報與宣告義務
6. 指向權威來源的指標（authority pointer）

不得收錄規範規則內容本身。

在 Skill 中，以規範標記陳述規則的句子，必須在同一段落內附有權威指標。規範標記取決於 `SKILL.md` front matter 中 `metadata.language` 的值。`metadata.language: en` 的標記為 must、must not、never 與 only。`metadata.language: zh-TW` 的標記為 必須、不得 與 只能。

### 行數上限與審查門檻

硬性上限為 500 行或 5,000 tokens。審查觸發門檻為 300 行。超過 300 行不視為建置失敗，但會觸發「是否將規範規則抄入 Skill」的架構審查。四個指令加上七份參考文件的路由引導，篇幅不應超過 300 行。

## 歸屬權責與釋出流程

實體、規範與副本三者的劃分如下：

| 實體項目 | 歸屬層級 |
| :--- | :--- |
| Skill 實體內容 | `skills/wda/`，與 `src/`、`docs/` 並列，不放在 `docs/` 內 |
| Skill 規範 | 本文件與相應 ADR |
| 安裝副本 | 部署副本，地位等同 PATH 環境變數中的 binary，無獨立歸屬 |

Core 產出並隨版本發布（release）一同釋出 Skill。將 Skill 安裝至 AI host 屬於 host 端的職責，不得為此新增 `wda skill` 指令。

## 版本控制

### 版本號碼與所屬 release 相同

Skill 不具備獨立的版本號碼。其版本與 WDA release 版本一致，記載於 front matter 的 `metadata.version`。專案不維護 Skill 與 CLI 的相容性矩陣。

### 三分支處置策略

實質風險在於安裝副本的漂移：Skill 副本駐留於 AI host，binary 位於 PATH 環境變數中，兩者可能各自更新。

| 比對結果 | 處置方式 |
| :--- | :--- |
| 相等 | 回報版本相同並繼續執行流程 |
| Skill 較舊 | 回報 Warning。若沒有影響該工作流程的已知衝突，則繼續執行 |
| Skill 較新 | 回報 Error，立即停止並提示使用者更新 CLI。不得靜默降級為盡力而為的角色扮演（role-play） |

若 release 來源或觀察到的 CLI 能力顯示已知衝突，即使版本文字相同，也應回報衝突、停止受影響的工作流程，並引導使用者取得相符的 release 或來源 Skill。此規則也適用於較舊的 Skill。

比對步驟：讀取 `SKILL.md` front matter 的 `metadata.version`，並與 `wda --version` 的輸出文字進行比對。AI 直接讀取字串文字，不仰賴 host 對 front matter 欄位進行語意解析，使比對過程不受各 AI host 實作差異影響。

### 不得自行解讀 wdaVersion

Skill 不得自行讀取 `anchor.json` 中的 `wdaVersion` 來判定專案格式相容性。相容性驗證為 Core 的專屬責任，Skill 僅能忠實轉述 `wda check` 的診斷輸出，以避免滋生第二套驗證邏輯（validator）。

## 禁止收錄可執行腳本

`scripts/` 目錄不得建立。

### 與零額外 Runtime 前提的衝突

選用 Rust 開發 CLI 的主要原因之一，是使用者不必為了執行 WDA 而額外安裝與管理 Node.js 環境。Skill 腳本能使用的語言受限於 AI host。任何 Python 或 Node.js 腳本都會重新引入 runtime 前提，讓使用者以為只要安裝 `wda`，實際上還需要直譯器（interpreter）。

### 紅線判準

需要確保正確執行的操作屬於 `wda` binary 的範圍，應評估是否值得新增指令。需要判斷、轉譯與決策的操作則屬於 AI，應記載於 `SKILL.md`。兩者沒有第三類灰色地帶。支持引入腳本的主要理由是追求確定性，但這個理由支持將能力納入 Core，而不是撰寫外掛腳本。若 Skill 腳本承接確定性需求，就會增加一種沒有指令名稱、side-effect 契約或版本相容性保證的 Core 能力。

### 例外程序

若需新增任何腳本，必須同時滿足零語言 runtime 的架構前提、提出不屬於 Core 職責的充分證明，並附帶專屬 ADR 與 architect review 批准記錄。

## 跨 Host 可攜性

### Front Matter 僅限定五個欄位

僅允許使用 `name`、`description`、`compatibility`、`metadata`、`license`。明確禁止使用 `allowed-tools`。該欄位為實驗性屬性，各 AI host 支援程度不一，依賴它等同綁定特定實作。

### 唯一能力假設

Skill 對 host 的唯一能力假設如下。host 具備執行原生二進位檔案（native binary）的能力，並能正確捕捉 stdout、stderr 與結束狀態碼（exit status）。這與可靠執行 `wda check` 及 `wda build` 遵循相同的行程契約（process contract）。Skill 不新增任何特定 host 的專屬 API。即使結束狀態碼為 0（exit 0），若存在 Warning 亦必須讀取 stderr，絕不可僅憑 exit 0 或 stdout 判定執行成功。

### 診斷回報與情境審查

每次 `wda` 呼叫都擷取 stdout、stderr 與結束狀態。人類可讀模式維持原本的診斷通道。JSON 模式解析 stdout 的 schema version 1，合併判斷結束狀態、status、diagnostics 與 summary。即使 exit 0 也回報 Warning。JSON 缺失、格式錯誤或版本不支援都不是成功。確實回報錯誤與 ToolFault。只有本次 build 成功且提供 `dist_path`，才回報其絕對路徑，不得從舊的 `dist/` 推定。

設計迭代或明確審查時，透過 `references/iteration.md` 審查無障礙、固定 CSS 值與 Token 關係。區分 CLI 診斷、來源分析、瀏覽器實測與未執行項目，指出檔案、依據與不確定處。CLI 成功不代表審查完成。瀏覽器不可用或失敗時，列出未執行的畫面與鍵盤檢查及手動指引，不強制安裝，也不宣稱完整符合 WCAG。

依第一方 CSS、現有 Token、用途、主題與 cascade 判斷固定值。相同數值不足以證明語意相同。不掃描套件或 `dist/` 提供建議。僅在專案明確採用 primitive、semantic、component 分層或使用者要求時，依專案記錄的規則審查參照方向。審查本身不授權編輯，仍須遵守編輯範圍、新 Token 同意與無障礙取捨揭露。這些審查不新增 Core 診斷。

## 不得依賴外部 Skill

| 行為型態 | 判定規範 |
| :--- | :--- |
| 缺少特定外部 skill 導致流程無法完成 | 不允許 |
| 具名推薦特定外部 skill | 不允許。外部 skill 名稱可能隨時變更而失效，也會將 WDA 綁定於特定廠商的生態系 |
| host 環境恰好有其他 skill 協同運作 | 允許。WDA Skill 對其存在保持透明且不予干涉 |

## Skill 發現與安裝指引

以 `anchor.json` 探索既有專案。Core 要求 `anchor.json` 與 `package.json` 兩份清單。在尚無 `anchor.json` 的情況下，只有使用者明確要求安裝或設定 WDA，或建立新的 WDA 專案時，才於專案建立前發現 Skill。一般網站開發請求若不在 WDA 專案中，不得載入 Skill。進入 WDA 專案後，Skill 的適用範圍維持不變。未安裝 Skill 的 AI 應依官方 README 指引操作。未安裝 Skill 的 AI 不得假設 Skill 已可用。

Skill 不得重複列出安裝指令。需要安裝 `wda` 或處理 `wda` 未加入 PATH 的情況時，Skill 必須引用既有的 `references/install.md`，並遵循該文件的指引。

安裝前先讀取可存取的官方 README。依需要探測 `wda --version` 與 Git，僅在補充時探測 npm 或 Bun。缺少 Git 時仍可建立專案，但沒有本機基準。每個安裝指令、專案建立與整合均須分別取得同意。區分啟動錯誤與結束狀態，並在安裝後重新探測。

各平台的安裝指令由 README 維護。Skill 與本文件不得複製平台專屬指令。

Skill 不得包含可執行腳本。`scripts/` 目錄不得建立。此限制適用於 Skill 及其參考文件。

「版本控制」一節定義 Skill 版本與 CLI 的相容性處置。發現或安裝流程不會改變這項處置。

### Minimal 退回之後的補充

當 `wda init` 退回 Minimal 專案時，會在 `anchor.json` 記錄 `initialization.fallback`，內含原本要求的系統與原因。之後加入預設 Spectrum 與 Lit 套件的補充工作由 Skill 負責。補充不是 CLI 子命令，也不重新執行 `wda init`。

補充依下列順序執行：讀取已記錄的狀態、重新檢查網路、manager 與既有內容、提出變更範圍與還原方式、取得同意、使用 manager 原生命令安裝、整合、驗證。Skill 不會因網路重新連線而自行升級專案。Skill 不得覆寫既有頁面，也不得安裝 Node.js 或 Bun。

Spectrum 整合完成且 `wda check` 與 `wda build` 均成功後，更新 `anchor.json` 的實際 Design System，移除 `initialization` 包裝欄位，再執行 `wda check`。若最後檢查失敗，只在同意範圍內還原先前的 `designSystem` 與 `initialization`，保留成功 build 寫入的 `wdaVersion`，回報失敗並再次檢查。僅補充 Lit 時保留兩個欄位。補充失敗或被拒絕時保留先前記錄。manager 的套件與生命週期副作用受還原規則限制。可用基準可由 init 建立，也可由 Designer 稍後建立。沒有基準時暫停寫入並說明限制，直到取得明確同意。Skill 不執行 Git 指令。詳細步驟見 `skills/wda/references/install.md`，還原規則見 `skills/wda/references/reversibility.md`。

## 驗收四層

| 驗收層級 | 驗收內容與標準 | 執行性質 |
| :--- | :--- | :--- |
| L1 規格合規 | `tests/skill.rs` 檢查 `SKILL.md` front matter 的 key 是否都在五個允許欄位內 | CI 阻擋型閘門 |
| L2 邊界防護（Guardrail） | 確認 Skill 文字不含 `src/codes.rs` 的診斷碼字面值 | CI 阻擋型閘門，防止回歸 |
| L3 行為評測 | 以 prompt 題庫執行文字比對。此方法無法測量漏觸發（undertriggering），ADR-06 將此列為已知缺口 | 文字比對評估 |
| L4 刪除測試（Deletion Test） | 移除整個 Skill 後，由人工確認 Core 與專案文件是否足以讓人正確操作 WDA | 人工測試，尚未執行 |

L3 評測決定 Skill 是否具備交付資格。L1 與 L2 決定能否進入 CI 流程。即使 L2 全數通過，L3 未達標仍不得交付。

L2 禁止建立 `assets/` 是 WDA 專屬的架構規範，不是通用標準。`wda init` 負責產生專案檔案。若 Skill 攜帶可複製成專案檔案的範本，就會形成另一個產生來源，與 `init` 衝突。

L3 目前無法檢驗漏觸發（undertriggering）。AI 可能沒有載入應使用的 Skill。若發生這種情況，AI 可能依一般 Web 開發知識修改專案檔案，略過 `wda check` 與 `wda build`，而且不會發出警示。因此，撰寫和評測 `description` 欄位是獨立的核心工作。
