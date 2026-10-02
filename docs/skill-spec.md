---
title: "Skill 規格"
status: stable
updated: 2026-09-20
type: Architecture
description: "Skill 的定義、維持單一 Skill 的理由、目錄與檔案配置、歸屬權責劃分、版本控制、禁止收錄可執行腳本、跨平台可攜性與四層驗收標準。"
tags: []
---

# Skill 規格

本文件為 Core 對 Skill 層的契約規範。Skill 的實體位於 `skills/wda/` 目錄。本文件不重複贅述其具體內容，僅規範其允許與禁止包含的項目，以及相應的驗收標準。

## Skill 的定位與職責邊界

Skill 為 AI 的操作手冊與工作流程層。它指引 AI 如何辨識與安裝 `wda`、如何解析專案結構、何時呼叫特定指令、如何修改專案，以及如何維護專案文件。

Skill 並非規格來源。它不重複保存核心規則，亦不作為第二套產生器（generator）。規範規則僅存在於專案文件與 Core 之中。Core 與 `wda.json` 的契約設計絕不得預設僅有 Skill 會進行操作。

兩者的職責劃分原則為：**流程程序歸入 Skill，品質判準歸入專案文件。** 品牌與視覺風格的詢問方式與先後順序屬於流程程序，歸入 Skill；何謂符合視覺美感則屬於品質判準，歸由專案的 `docs/design.md` 規範。

## 僅維持單一 Skill 的理由

### 切分依據為觸發條件（Trigger）之可鑑別度

CLI 指令是在呼叫方已明確操作意圖後選取執行；一旦選錯，系統會立即回傳錯誤。Skill 則由 AI 在尚未確定具體意圖前依語意觸發選取；即便選錯也不會回報錯誤，而是靜默地走錯流程。因此，「職責不同即可新增指令」之原則並不適用於 Skill。唯有在「AI 載入任何一個 Skill 之前，即能毫無歧義判定應載入何者」的前提下，拆分出兩個獨立的 Skill 方能成立。

### 三種經評估否決之拆分方式

| 拆分方式 | 否決理由 |
| :--- | :--- |
| 依 CLI 指令拆分 | 單一使用者請求可能跨越數個指令，且 `deps` 指令對 Designer 而言極少主動觸發 |
| 依 Designer 工作階段拆分 | 各工作階段之觸發語意高度重疊；且若每份 Skill 皆需重述相同規則，該重複文字即成為第二套產生器的起點 |
| 依使用者角色拆分 | 若另設 Engineer 專用 Skill，等同承認 Engineer 亦需仰賴 Skill 方能操作 WDA |

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
    ├── iteration.md        每一次視覺變更執行之前，每次都載入
    ├── reporting.md        觸發回報義務時
    └── troubleshooting.md  check 或 build 回報 Error 時
```

`scripts/` 與 `assets/` 嚴禁建立。

### 配置規範標準

目錄配置遵循 Agent Plugins 1.0.0 開放標準[agent-plugins.org](https://agent-plugins.org/specification)。`skills/<name>/SKILL.md` 為該標準與 Claude Code 共同支援的探索規則。release package 根目錄配置最小化之 `plugin.json`，僅包含 `$schema` 與 `name` 兩個欄位。整個 package 因而符合標準套件規格。任何相容之用戶端（client）皆可直接安裝。安裝程序屬於 AI host 之範疇，機制僅在於此。Core 內部不需另設安裝指令。

### SKILL.md 內容白名單

`SKILL.md` 僅允許收錄以下六類內容，其餘項目一律視為違規：

1. 角色職責與邊界宣告
2. 版本比對之執行步驟
3. 決策路由表
4. 流程停止條件
5. 回報與宣告義務
6. 指向權威來源之指標（authority pointer）

嚴禁收錄規範規則內容本身。

### 行數上限與審查門檻

硬性上限為 500 行或 5,000 tokens；審查觸發門檻為 300 行。超過 300 行不視為建置失敗，但將觸發「是否將規範規則抄入 Skill」的架構審查。四個指令加上七份參考文件的路由引導，篇幅不應超過 300 行。

## 歸屬權責與釋出流程

實體、規範與副本三者之劃分如下：

| 實體項目 | 歸屬層級 |
| :--- | :--- |
| Skill 實體內容 | `skills/wda/`，與 `src/` 及 `docs/` 並列，不置於 `docs/` 目錄下 |
| Skill 之規範標準 | 本文件與相應 ADR |
| 安裝至 AI host 的副本 | 部署副本，地位等同安裝至 PATH 環境變數之 binary，無獨立歸屬 |

Core 產出並隨版本發布（release）一同釋出 Skill。將 Skill 安裝至 AI host 屬於 host 端的職責，嚴禁為此新增 `wda skill` 指令。

## 版本控制

### 版本號碼與所屬 release 相同

Skill 不具備獨立的版本號碼。其版本與所屬之 WDA release 版本完全一致，記載於 front matter 的 `metadata.version`。專案不維護 Skill 與 CLI 之間的相容性矩陣。

### 三分支處置策略

實質風險在於安裝副本的漂移：Skill 副本駐留於 AI host，binary 位於 PATH 環境變數中，兩者可能各自更新。

| 比對結果 | 處置方式 |
| :--- | :--- |
| 相等 | 繼續執行流程 |
| Skill 較舊 | 回報 Warning，可繼續執行。CLI 具備向下相容性，Skill 僅無法使用新功能 |
| Skill 較新 | 回報 Error，立即停止並提示使用者更新 CLI。嚴禁靜默降級為盡力而為的角色扮演（role-play） |

比對步驟：讀取 `SKILL.md` front matter 之 `metadata.version`，並與 `wda --version` 的輸出文字進行比對。AI 直接讀取字串文字，不仰賴 host 對 front matter 欄位進行語意解析，使比對過程不受各 AI host 實作差異影響。

### 嚴禁自行解讀 wdaVersion

Skill 不得自行讀取 `wda.json` 中的 `wdaVersion` 來判定專案格式相容性。相容性驗證為 Core 的專屬責任，Skill 僅能忠實轉述 `wda check` 的診斷輸出，以避免滋生第二套驗證邏輯（validator）。

## 禁止收錄可執行腳本

`scripts/` 目錄嚴禁建立。

### 與零額外 Runtime 前提之衝突

選用 Rust 開發 CLI 的核心理由之一，在於使用者不需為了執行 WDA 而額外安裝與管理 Node.js 環境。Skill 腳本所能執行的語言完全受限於 AI host 環境，任何 Python 或 Node.js 腳本皆會讓本已消除的 runtime 前提變相復辟：讓使用者誤以為僅需安裝 `wda`，實際上卻仍需直譯器（interpreter）支援。

### 紅線判準

凡需保證正確執行的操作，皆屬於 `wda` binary 的範疇，應評估是否具備收錄為新指令之價值。凡需判斷、轉譯與決策之操作，方屬於 AI 的範疇，記載於 `SKILL.md`。兩者之間不存在第三類灰色地帶。支持引入腳本最強烈的論點在於追求確定性，然而確定性所推導出的正當結論為「應納入 Core」，而非「撰寫外掛腳本」。藉由 Skill 腳本承接確定性需求，等同變相滋生出既無指令名稱、無 side-effect 契約，亦無版本相容性保證的第五個 Core 能力。

### 例外程序

若需新增任何腳本，必須同時滿足零語言 runtime 之架構前提、提出不屬於 Core 職責之充分證明，並附帶專屬 ADR 與 architect review 批准記錄。

## 跨 Host 可攜性

### Front Matter 僅限定五個欄位

僅允許使用 `name`、`description`、`compatibility`、`metadata`、`license`。明確禁止使用 `allowed-tools`。該欄位為實驗性屬性，各 AI host 支援程度不一，依賴它等同綁定特定實作。

### 唯一能力假設

Skill 對 host 的唯一能力假設如下。host 具備執行原生二進位檔案（native binary）的能力，並能正確捕捉 stdout、stderr 與結束狀態碼（exit status）。這與可靠執行 `wda check` 及 `wda build` 遵循相同的行程契約（process contract）。Skill 不新增任何特定 host 的專屬 API。即使結束狀態碼為 0（exit 0），若存在 Warning 亦必須讀取 stderr，絕不可僅憑 exit 0 或 stdout 判定執行成功。

## 不得依賴外部 Skill

| 行為型態 | 判定規範 |
| :--- | :--- |
| 缺少特定外部 skill 導致流程無法完成 | 嚴格禁止 |
| 具名推薦特定外部 skill | 嚴格禁止。外部 skill 名稱可能隨時變更而失效，且會將 WDA 綁定於特定廠商之生態系 |
| host 環境恰好有其他 skill 協同運作 | 允許。WDA Skill 對其存在保持透明且不予干涉 |

## Skill 發現與安裝指引

在尚無 `wda.json` 的情況下，只有使用者明確要求安裝或設定 WDA，或建立新的 WDA 專案時，才於專案建立前發現 Skill。一般網站開發請求若不在 WDA 專案中，不得載入 Skill。進入 WDA 專案後，Skill 的適用範圍維持不變。未安裝 Skill 的 AI 應依官方 README 指引操作。未安裝 Skill 的 AI 不得假設 Skill 已可用。

Skill 不得重複列出安裝指令。需要安裝 `wda` 或處理 `wda` 未加入 PATH 的情況時，Skill 必須引用既有的 `references/install.md`，並遵循該文件的指引。

各平台的安裝指令由 README 維護。Skill 與本文件不得複製平台專屬指令。

Skill 不得包含可執行腳本；`scripts/` 目錄嚴禁建立。此限制適用於 Skill 及其參考文件。

Skill 版本與 CLI 的相容性處置維持「版本控制」一節所定義的語意，不因發現或安裝流程而改變。

## 驗收四層

| 驗收層級 | 驗收內容與標準 | 執行性質 |
| :--- | :--- | :--- |
| L1 規格合規 | 通過 Agent Skills 規範之驗證工具檢驗 | CI 阻擋型閘門 |
| L2 邊界防護（Guardrail） | `scripts/` 與 `assets/` 不存在；`SKILL.md` 與 `references/**` 中，語言標記為 `html`、`json`、`css`、`toml` 且超過 5 行的程式碼區塊（code block）數量為零，僅允許 `text` 流程圖與 `bash` 指令呼叫；凡包含「必須、不得、只能」等規範句，同段落必須具備權威來源指標；Skill 內文字串與 Core 的 schema 或診斷訊息無重複；行數未超過上限 | CI 阻擋型閘門，專責防止回歸 |
| L3 行為評測 | 測試 prompt 題庫、執行、質性與量化評估、反覆迭代。必須包含觸發不足（undertriggering）之檢驗 | 主力驗收標準 |
| L4 刪除測試（Deletion Test） | 移除整個 Skill 後，Core 與專案文件仍足以供人類使用者正確操作 WDA | 人工執行，驗收等級 |

L3 評測決定 Skill 是否具備交付資格。L1 與 L2 僅決定能否進入 CI 流程；即使 L2 檢查全數合格，若 L3 未能達標，依然視為不可交付之產物。

L2 針對 `assets/` 目錄之禁令為 WDA 專屬之架構收緊規範，而非通用標準。專案檔案的產生為 `wda init` 之獨佔責任；若 Skill 攜帶可複製為專案檔案之範本，將與 `init` 形成衝突的兩套產生來源。

L3 必須包含觸發不足（undertriggering）檢驗。AI 常傾向不主動觸發應當觸發的 skill。對 WDA 而言此為產品層級的重大風險：若 Skill 未能觸發，AI 將逕自依據通用 Web 開發常識修改專案檔案，繞過 `wda check` 與 `wda build`，產出違反契約規範的專案內容，且全程靜默無警示。因此，撰寫與評測 `description` 欄位是一項獨立的核心工作，絕非附帶項目。
