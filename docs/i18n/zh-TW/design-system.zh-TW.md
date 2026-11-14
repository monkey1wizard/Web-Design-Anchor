---
title: "Design System 與 Tokens"
status: stable
updated: 2026-11-12
type: Architecture
description: "Design System 預設與退回、依賴與清單職責、token 契約、adapter 邊界、中立性原則與實例文件索引。"
tags: []
source: docs/design-system.md
lang: zh-TW
source_sha256: 446156f630e0254f1081401f45ba69d7a459ddcbecd7dfef030655b90c8f7a06
translated_at: 2026-11-12
---

# Design System 與 Tokens

本文件說明 WDA 的 Design System 契約，包括初始化預設與退回、清單與依賴職責、token 規則、adapter 的七個 facet、身分分支邊界及中立性。各系統的實作細節見 [WDA Minimal](../../design-systems/wda-minimal.md) 與 [Spectrum](../../design-systems/spectrum-adapter.md)。

## 預設與退回

### 預設為 latest stable Spectrum

`wda init` 線上查詢 npm registry 的 `@spectrum-web-components/bundle` `dist-tags.latest`。版本須為 1.0.0 以上的正式 semver。
查詢成功且有可用的外部套件管理器時，`wda init` 會執行該管理器，將 Spectrum 套件與 `lit` 安裝為 `package.json` 中的網站依賴。管理器負責解析並記錄確切版本，並寫入它自己的 lockfile。接著 `anchor.json` 將 Design System 記為 Spectrum 與解析後的版本。

### 退回 WDA Minimal 的條件

遇到下列任一情況，`wda init` 會改用 `WDA Minimal`：

- 找不到可用的外部套件管理器。WDA 支援 npm 與 Bun。
- registry 查詢因任何原因失敗。原因包括 registry 無法連線、逾時、授權或速率限制回應、伺服器錯誤、TLS 失敗、輸出無法解析，以及缺少必要欄位。
- 查得的版本不是正式版本，或低於 1.0.0。
- 透過管理器安裝套件失敗。
- 以 `--package-manager` 指定的管理器不可用。此時仍會退回，WDA 不會改用其他管理器。

本機套件快取絕不會讓 `wda init` 改為 Spectrum。只有線上解析成功且管理器安裝成功，才會選用 Spectrum。

退回時，WDA 會在 stdout 說明採用的系統與原因。接著清除本次執行已建立的 Spectrum 專屬檔案與 Spectrum 依賴項目，再建立完整的 `WDA Minimal` 專案。若清除或必要寫入失敗，指令會回報明確錯誤，且不宣稱 Minimal 成功。

`WDA Minimal` 在沒有網路連線與套件管理器時仍可使用。初始化時它不建立網站依賴，因此沒有等待安裝的套件。`anchor.json` 將 Design System 記為 `WDA Minimal`，並在 `initialization.fallback` 記錄原本想採用的系統與原因，欄位為 `requestedDesignSystem` 與 `reason`。

### 之後補上預設

Skill 可以之後為 `WDA Minimal` 專案加入 Spectrum 與 Lit。它會先檢查網路、管理器與既有檔案，接著提出變更範圍與還原方式，並且只在使用者同意後才安裝。Skill 會在核准範圍內使用專案的外部套件管理器，並在不覆寫既有頁面的前提下整合套件。

此用途不會重新執行 `wda init`，專案也不會在網路恢復時自動升級。Spectrum 整合後，`wda check` 與 `wda build` 必須成功，Skill 才會將 `designSystem` 更新為實際系統，並移除僅含 `fallback` 欄位的 `initialization` 包裝物件。接著 Skill 再執行 `wda check`。若最後檢查失敗，Skill 在核准範圍內還原先前的 `designSystem` 與 `initialization` 記錄，並再次檢查。成功的建置仍會更新 `wdaVersion`。只加入 Lit 時，Design System 與退回記錄保持不變。整合被拒絕或失敗時，先前記錄保持不變，專案仍以 `WDA Minimal` 可用。

### 告知義務

發生退回時，`wda init` 的輸出必須說明採用的 Design System 與退回原因。輸出也必須說明之後可由 Skill 補上預設。

## 依賴與清單

### 網站套件

- `package.json` 是唯一的網站依賴來源。`anchor.json` 不重複任何依賴。
- Spectrum 初始化成功時，即使只有一個可用管理器，也會將所選管理器的名稱與確切版本記錄在 `package.json` 的標準 `packageManager` 欄位。初始 Minimal 不含此欄位。
- 外部管理器 npm 或 Bun 負責 `package.json` 的變更、其 lockfile、版本求解、遞移版本與安裝。WDA 絕不寫入、改寫或遷移管理器 lockfile。
- 新增、移除或更新網站套件使用管理器自己的指令。沒有任何 WDA 指令管理網站套件。
- `wda check` 與 `wda build` 只讀取已安裝的套件內容。它們絕不安裝或更新套件，也絕不為套件使用網路。
- 網站模組不從遠端 CDN 載入。WDA 不保留 import map、vendor 目錄或 CDN 鏡像。

### WDA 中繼資料

`anchor.json` 負責 WDA 設定、實際的 Design System，以及初始化退回記錄。它不宣告依賴版本，也不覆寫原生工具版本或其完整性值。

### 工具快取

TypeScript 工具是 WDA 發行版內嵌的單一版本。發行建置會內嵌各平台套件。原始碼建置會在 `wda init` 時嘗試取得同一版本，取得失敗時回報 Warning。`wda tools fetch` 不接受引數，會為目前平台還原該內嵌版本的快取。它不改變 `anchor.json` 或 `package.json`。

### 遷移

WDA 不會自動遷移既有專案。唯讀指令絕不改寫依賴宣告。使用已淘汰格式的專案，不宣稱可以建置。

## Token

### 權威來源

`tokens/tokens.json` 是採 DTCG 格式的唯一權威 token 來源。建置會將其轉為 CSS 自訂屬性，輸出至 `dist/styles/tokens.css`。

WDA 支援巢狀群組、繼承的 `$type`，以及整個值為 `{path.to.token}` 的別名。JSON 巢狀深度與別名鏈上限皆為 128 層。別名必須指向 token，且不能形成循環。支援的型別為 `color`、`dimension`、`fontFamily`、`fontWeight`、`lineHeight`、`number` 和 `string`。值必須是數字，或可構成單一 CSS 自訂屬性值的非空字串。陣列、物件、布林值、null 與其他型別皆不支援。

Token 名稱的各段可含字母、數字、連字號、底線與空格。空段和經 CSS 名稱正規化後衝突的名稱會被拒絕。字串不可含控制字元或 CSS 註解分隔符號。未跳脫的大括號、引號外的 `;` 與 `!`，以及不完整的跳脫序列都會被拒絕。引號、圓括號與方括號必須成對。不支援在值的一部分插入別名。Token 也不能包含子 token。

### 衍生 CSS

原始程式碼目錄不得包含 `styles/tokens.css`。此檔案是衍生產物，只能存在於 `dist/`。若 `wda build` 在原始程式碼目錄找到該檔案，便會回報錯誤並拒絕建置。

### 命名

Token 使用專案自有的語意命名空間，可以在 `tokens.json` 內使用別名。權威 token 名稱不得直接綁定第三方廠商的 token 名稱。

### Primitive layer

無論專案使用哪一套 Design System，token 分層都是自願的。WDA 不要求 token 必須分成 primitive 與 semantic 層。專案可按規模、元件重用或階層需求決定是否分層。

## Adapter 邊界

Adapter 是文件規格，用來說明 Design System 如何接入 WDA。它不是 Rust trait，也不是外掛註冊表。下列七個 facet 定義共同的對照方式。各系統實例見 [WDA Minimal](../../design-systems/wda-minimal.md) 與 [Spectrum](../../design-systems/spectrum-adapter.md)。

### 1. Token mapping

說明專案 DTCG 語意 token 如何對應至系統 token。

### 2. Styles

說明基準樣式、版面配置與全域樣式如何組織及建置。

### 3. Fonts

說明字型家族、字型堆疊、行高、字級比例與備援方式。

### 4. Icons

說明圖示資源、打包方式、呈現方式與命名。

### 5. Theme

說明色彩模式與主題範疇如何設定及切換。

### 6. Component dependencies

說明系統使用的元件、自訂元素及原生 HTML 標記。

### 7. Dependency integration

說明透過外部管理器安裝的外部套件如何納入專案生命週期，以及相依失敗時如何處理。

實作優先使用原生 HTML 與 CSS。使用廠商公開的元素名稱與 API 是允許的。只有在提供產品語意、可重用行為或元件封裝時，專案自訂 wrapper 才有必要。

## 身分分支邊界

Core 只有兩處可判斷或指定 Design System：

| 銜接介面 | 檔案 |
| --- | --- |
| 解析銜接層 | `src/init/resolve.rs` |
| 內建資產層 | `src/init/mod.rs`、`src/init/documents.rs`、`src/builtins/wda_minimal.rs`、`src/builtins/spectrum_two.rs` |

其他 Core 程式碼不得依 Design System 分支。`tests/neutrality.rs` 以文字掃描檢查允許路徑外的身分判斷與硬編碼名稱。掃描會略過註解，並以大括號配對找出及略過 `cfg(test)` 模組。若路徑改變，須同步更新測試。

## 中立性

WDA 不綁定單一 Design System，並以 Spectrum 及 `WDA Minimal` 兩種實作驗證這項原則。兩者使用不同的相依與元件模型，且各自支援 `init`、`check` 與 `build`。Spectrum 在初始化時透過外部管理器安裝 Lit。`WDA Minimal` 在初始化時不建立網站依賴，Skill 之後可加入 Spectrum 與 Lit。Spectrum 使用 vendor custom elements，`WDA Minimal` 使用原生 HTML。

`WDA Minimal` 的內嵌資產位於 `src/builtins/wda_minimal/`。該目錄包含設計說明、HTML 起始頁與 DTCG token 檔。這些檔案未設定 `data-theme` 或 `color-scheme`。

系統實例與 facet 對照請見 [WDA Minimal](../../design-systems/wda-minimal.md) 和 [Spectrum](../../design-systems/spectrum-adapter.md)。

中立性不代表兩套系統外觀相同，或可自動轉換。文件不得宣稱系統間可以自動轉換、零成本遷移、無損遷移或完全不需修改 HTML。

專案的 `anchor.json` 指名其他 Design System 時，也可以採用這些以外的系統。本文件的規則適用於每個專案，且不要求特定的 Design System。

### 已知取捨

`WDA Minimal` 的 token 數值是從 `@adobe/spectrum-tokens` 15.2.0 擷取的一次性快照，不會隨上游更新。字型採用 Noto 系列。授權與版權資訊見專案根目錄的 `NOTICE`。因此，描述此系統時可說它沒有 vendor 元件庫相依，不應聲稱其設計數值獨立於 vendor。

### 測試範圍

`WDA Minimal` 路徑可在預設 `cargo test` 中離線執行。Spectrum 2 流程依賴可用的 npm 或 Bun 管理器與 npm registry，因此標記為 `#[ignore]`，只在 CI 非阻斷性探針工作執行。
