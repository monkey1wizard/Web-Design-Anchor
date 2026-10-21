---
title: "術語"
status: stable
updated: 2026-10-15
type: Reference
description: "WDA 規範與架構文件使用的核心術語。每項只提供定義，不列實作規則。"
tags: []
---

# 術語

本文件定義 WDA 規範與架構文件使用的術語。操作方式與驗證邏輯請參閱各專屬文件。

## 角色與實體

| 術語 | 定義 |
| :--- | :--- |
| Designer | WDA 的目標使用者。Designer 以自然語言提出設計與功能需求，不直接撰寫程式碼 |
| AI host | 執行 AI 對話並載入 Skill 的外部環境，例如 Claude Code。WDA 不信任此環境 |
| Skill | AI 的操作手冊與流程指南。實體檔案位於 `skills/wda/`，並隨版本發布。規格見 `docs/skill-spec.md` |
| Core | WDA 的 CLI 工具本體，由單一 Cargo package 組成，包含 `wda_core` 程式庫（lib）與 `wda` 執行檔（bin） |
| Generated Project | 經由 `wda init` 建立，並由 Designer 與 AI 長期維護的專案儲存庫。契約見 `docs/generated-project.md` |
| Standard Web 專案 | 使用 HTML5、Modern CSS 與 TypeScript，且不綁定前端框架的 Web 專案。Generated Project 屬於此類專案 |
| `dist/` | `wda build` 產生的唯一部署目錄，內含可獨立運作的純靜態檔案 |
| Build Only | WDA 只建置可獨立部署的靜態檔案，不提供託管（hosting）或部署服務的架構承諾 |
| 行程契約（Process Contract） | Skill 與 AI host 的能力約定。host 只需能執行原生二進位檔，並擷取 stdout、stderr 與結束狀態碼 |
| starter script | Spectrum 2 範本中的 `scripts/main.ts`，用來載入範本頁面所需的套件 |
| Lit | 用於建立 Web Component 的套件。WDA 範本可將它列為專案相依套件 |

## 穩定度分層

| 術語 | 定義 |
| :--- | :--- |
| Invariant | 長期不變的承諾，任何版本都不安排變更，例如 `dist/` 是唯一部署產物 |
| Default | 專案預設值。專案可修改 `wda.json` 自訂 |
| Tool Form | Core 工具的執行形態，目前是單一 standalone CLI 執行檔 |
| Tooling Choice | 實作 Tool Form 所使用的技術，例如 Rust、esbuild、Deno。WDA 可在改版時替換，不影響專案契約 |
| Implementation Detail | Core 內部的程式碼組織與實作方式，外部無法直接看見 |

## 專案契約

| 術語 | 定義 |
| :--- | :--- |
| `wda.json` | 位於專案根目錄，並依嚴格 Schema 驗證的核心契約設定檔 |
| `deno.jsonc` | 現行 Generated Project 唯一的外部相依宣告檔。`imports` 欄位列出相依套件與版本規格，由 Deno 寫入並保留註解。線上初始化成功時會建立此檔，離線 Minimal 初始化不會建立。專案端不支援 `deno.json` 作為設定檔。此檔與 WDA 內部建置工作區的嚴格 `deno.json` 分開 |
| `wdaVersion` | `wda.json` 欄位，記錄 WDA 最近一次成功處理此專案的版本，不用來判定格式相容性 |
| Source 基準 | 專案原始程式碼可使用的技術範圍：HTML5 與語意標記、Modern CSS、TypeScript 和原生瀏覽器能力 |
| Deployment 基準 | `dist/` 產物可包含的檔案類型：HTML、CSS、JavaScript 與靜態資產（assets） |
| Baseline Widely Available | 預設瀏覽器相容性基準，指 Web 平台功能獲主要瀏覽器引擎支援至少 30 個月 |
| WCAG 2.2 AA | 專案預設採用的無障礙網頁檢驗標準 |
| MPA | 多頁式架構（Multi-Page Application）。各頁面使用獨立 HTML 檔案，並透過瀏覽器原生導覽切換。這是 WDA 專案的預設架構 |
| SPA | 單頁式架構（Single-Page Application）。透過用戶端路由（client-side routing）動態切換畫面。WDA 允許專案採用，但不設為預設值 |
| 嚴格 Schema（Strict Schema） | `wda.json` 只接受已宣告的欄位。未知欄位會回報 Error，且不設自訂擴充區或 metadata 欄位 |
| 簡潔網址（Clean URL） | 移除網址副檔名，或以巢狀 `index.html` 目錄模擬路由的路徑形式。WDA 不支援此形式，由部署主機處理 |
| legacy `package.json` 格式 | `wda deps` 支援的既有專案格式。專案根目錄有 `package.json` 時，指令會使用此格式 |

## 建置

| 術語 | 定義 |
| :--- | :--- |
| `wda-include` | WDA 唯一支援的自訂建置期 HTML 標籤。建置時會展開為元件內容，不會輸出到 `dist/` |
| 建置工作區（Build Workspace） | `wda build` 在專案目錄外建立的暫存工作目錄，建置結束後立即刪除。工作區使用 WDA 專屬的嚴格 `deno.json`，只合併專案 `deno.jsonc` 的 `imports`，並與專案檔案隔離 |
| `deno.lock` | 記錄專案相依套件完整相依圖與套件雜湊的鎖定檔，與 `deno.jsonc` 分開。`wda deps` 會寫入此檔，`wda build` 只讀取，不進行未鎖定的網路解析 |
| 碰撞規則（Collision Rule） | `wda init` 的前置安全檢查。若預定建立的路徑已存在，指令會立即拒絕執行 |
| 還原基準點（Rollback Baseline） | `wda init` 在專案目錄建立的本機 Git 儲存庫，其第一個初始 commit |
| 解析銜接層 | Core 依 Design System 產生邏輯分支的第一個白名單介面：`src/init/resolve.rs` |
| 內建資產層 | Core 依 Design System 產生邏輯分支的第二個白名單介面，位於 `src/init/` 與 `src/builtins/` 的指定檔案 |

## Design System

| 術語 | 定義 |
| :--- | :--- |
| Design Token（設計權杖） | 用語意識別名稱指涉的最小設計數值，例如顏色、間距或字級 |
| DTCG | Design Tokens Community Group 制定的標準 JSON 格式。本專案以 `tokens/tokens.json` 為唯一權威來源 |
| `WDA Minimal` | WDA 內建的最小 Design System，供離線環境使用。線上初始化成功時會盡力建立 `deno.jsonc` 與 `deno.lock` 以安裝 Lit。離線初始化不會建立相依檔案，因此沒有外部相依 |
| Spectrum / Spectrum 2 | Adobe 推出的 Design System，是 `wda init` 線上查詢時的預設選項 |
| Adapter | 說明 Design System 如何接入 WDA 專案的介面規格，由七個面向（facet）組成 |
| Facet | Adapter 介面規格的一個面向，涵蓋 Token mapping、樣式發布與組織、字型、圖示資源、色彩主題、元件相依模型與套件整合 |
| 中立性（Neutrality） | Core 不綁定單一 Design System 的架構承諾。符合條件的系統須支援不同的相依模型與元件模型，且都能完成 init、check、build |
| 身分分支（Identity Branch） | Core 依 Design System 名稱判斷或分支處理的邏輯，只能出現在白名單檔案中 |

## 驗證與診斷

| 術語 | 定義 |
| :--- | :--- |
| Diagnostic（診斷紀錄） | `wda check` 或 `wda build` 檢查專案時輸出的一筆問題紀錄 |
| Severity（嚴重層級） | 診斷紀錄的分類，分為會阻擋建置的 Error 與不會阻擋建置的 Warning |
| Error | 代表建置成果不正確或違反專案契約的重大問題，會阻擋建置。透過 `ValidationReport` 回報時，指令結束狀態碼為 1 |
| Warning | 不阻擋建置產出，但表示有潛在風險並會提示使用者。Warning 輸出至 stderr |
| ToolFault | 外部相依工具或檔案系統發生非預期崩潰或嚴重異常時回傳的工具錯誤。若以 ToolFault 回傳，指令結束狀態碼為 2 |
| 結束狀態碼（Exit Status） | CLI 指令執行後回傳的狀態：`0` 表示成功或只有 Warning；`1` 表示 `ValidationReport` 含 Error；`2` 表示用法錯誤、deps 層級錯誤或 ToolFault |
| 單一驗證接縫（Single Validation Seam） | Core 對外提供的唯一驗證函式 `validate_project(root)`。binary 無法直接使用內部個別驗證規則模組 |
| 探針（Probe Test） | 用來確認特定執行環境行為或能力是否符合預期的測試 |

## Skill 與驗收

| 術語 | 定義 |
| :--- | :--- |
| 白名單內容 | `SKILL.md` 允許收錄的六類指引：角色邊界、版本比對步驟、決策路由表、流程停止條件、回報義務與權威指標 |
| 觸發不足（Undertriggering） | AI 應載入 Skill 卻未載入，因而繞過 Core 規範修改檔案的情況 |
| 四層驗收（Four-Tier Acceptance） | Skill 品質檢驗架構，包含 L1 規格合規、L2 邊界防護、L3 行為評測與 L4 刪除測試 |
| 刪除測試（Deletion Test） | L4 驗收方式。人工移除整個 Skill 後，確認 Core 與專案文件仍足以讓使用者正確操作 WDA |
| 權威來源指標（Authority Pointer） | Skill 規範句同段落中指向文件路徑的參考標記，例如 `docs/`、章節符號 `§` 或 `ADR-` |
| Craft Floor | Generated Project 設計文件中的最低品質標準章節，列出設計交付必須符合的基本條件 |

## 文件

| 術語 | 定義 |
| :--- | :--- |
| 必要文件 | Generated Project 初始化時必須具備的四份核心文件：`README.md`、`docs/architecture.md`、`docs/design.md`、`docs/glossary.md` |
| 條件式文件 | Generated Project 出現未涵蓋的特殊工作流程時才建立的文件，例如 `docs/development.md` |
| 權威來源（Canonical Source） | 某項專案狀態或規格的唯一來源。其他文件只能摘要，不得複製內容 |
| `sourceRefs` | 文件 front matter 中的陣列欄位，用來列出該文件依據或衍生的權威來源路徑 |
| Front Matter | Markdown 檔案開頭以 `---` 包住的 YAML 結構化詮釋資料 |
| ADR | Architecture Decision Record（架構決策紀錄），記錄單一技術決策的背景、選項、決策與後果 |
| Thematic ADR | 依主題整理相關架構決策的 ADR 文件 |
| OKF | Open Knowledge Format，本專案 front matter 欄位結構與文件標準的規範基礎 |
| restore point | 可用來還原專案狀態的版本控制記錄點 |
| SEEN | 狀態標記，表示 Designer 已看過目前成果 |
| ACCEPTED | 狀態標記，表示 Designer 已明確接受目前成果 |
