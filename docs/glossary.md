---
title: "術語"
status: stable
updated: 2026-09-22
type: Reference
description: "WDA 規範與架構文件所使用之核心術語定義。每項條目僅提供界定定義，不收錄實作規則。"
tags: []
---

# 術語

本文件僅收錄 WDA 各項規範所涉及之術語與概念定義。本文件不收錄具體操作規則與驗證邏輯。請參閱各專屬文件以取得這些內容。

## 角色與實體

| 術語 | 定義 |
| :--- | :--- |
| Designer | WDA 的目標使用者。使用自然語言表達設計與功能需求，不直接撰寫程式碼 |
| AI host | 執行 AI 對話並載入執行 Skill 的外部宿主環境（例如 Claude Code）。WDA 將其視為不可信環境 |
| Skill | Skill 是 AI 的操作手冊與工作流程引導層。實體檔案位於 `skills/wda/` 目錄。Skill 隨版本發布（release）一同釋出。規格詳見 `docs/skill-spec.md` |
| Core | 可獨立執行的 CLI 工具本體。由單一 Cargo package 構成，包含 `wda_core` 程式庫（lib）與 `wda` 二進位執行檔（bin） |
| Generated Project | Generated Project 是經由 `wda init` 初始化建立的專案儲存庫。Designer 與 AI 共同長期維護此專案儲存庫。契約詳見 `docs/generated-project.md` |
| Standard Web 專案 | 僅使用 HTML5、Modern CSS 與 TypeScript 建構，且不綁定任何前端框架的 Web 專案。Generated Project 即屬於 Standard Web 專案 |
| `dist/` | `wda build` 產出之唯一部署目錄，存放可獨立運作之純靜態檔案 |
| Build Only | WDA 的架構承諾：只負責建置可獨立部署之靜態檔案，不提供託管（hosting）或部署服務 |
| 行程契約（Process Contract） | Skill 與 AI host 之間的能力約定：host 僅需具備執行原生二進位檔案，並捕捉 stdout、stderr 與結束狀態碼之能力 |

## 穩定度分層

| 術語 | 定義 |
| :--- | :--- |
| Invariant | 不可變更的長期承諾。任何版本皆不安排變更，例如「`dist/` 為唯一部署產物」 |
| Default | 專案預設值。專案可藉由修改 `wda.json` 自訂調整 |
| Tool Form | Core 工具的執行形態。目前為單一 standalone CLI 執行檔 |
| Tooling Choice | 實現 Tool Form 所選用的技術堆疊（如 Rust、esbuild、Deno）。WDA 改版時可替換，且不影響專案契約 |
| Implementation Detail | Core 內部的程式碼組織與實作方式，對外部完全透明且不可見 |

## 專案契約

| 術語 | 定義 |
| :--- | :--- |
| `wda.json` | 位於專案根目錄的核心契約設定檔，採用嚴格 Schema 驗證 |
| `deno.jsonc` | `deno.jsonc` 是現行 Generated Project 唯一的外部相依宣告檔案。它在 `imports` 欄位宣告相依套件與規格，由 Deno 原生管理寫入並保留註解。線上成功初始化時會寫入，離線 Minimal 初始化時則不建立。專案端不採用亦不支援 `deno.json` 作為設定檔。它與 WDA 內部建置工作區所使用的嚴格 `deno.json` 完全分離 |
| `wdaVersion` | `wda.json` 內的欄位，記錄最後一次成功處理此專案的 WDA 版本號碼，不用於判定格式相容性 |
| Source 基準 | 專案原始程式碼允許採用的技術範圍：HTML5 與語意化標記、Modern CSS、TypeScript 與原生瀏覽器能力 |
| Deployment 基準 | `dist/` 目錄產出物允許包含的檔案類型：HTML、CSS、JavaScript 與靜態資產（assets） |
| Baseline Widely Available | 預設的瀏覽器相容性基準，指 Web 平台功能已獲主要瀏覽器引擎支援達 30 個月以上 |
| WCAG 2.2 AA | 專案預設採納之無障礙網頁檢驗標準 |
| MPA | 多頁式架構（Multi-Page Application）。各頁面為獨立 HTML 檔案，經由瀏覽器原生導覽機制切換。WDA 專案之預設架構 |
| SPA | 單頁式架構（Single-Page Application）。藉由用戶端路由（client-side routing）動態切換畫面。WDA 允許專案自訂採用，但非預設值 |
| 嚴格 Schema（Strict Schema） | `wda.json` 所遵循之驗證原則：僅允許已宣告欄位，出現任何未知欄位皆回報 Error，不設自訂擴充區或 metadata 欄位 |
| 簡潔網址（Clean URL） | 移除網址副檔名或以巢狀 `index.html` 目錄模擬路由之路徑形態。WDA 明確排除此項支援，交由部署主機環境處理 |

## 建置

| 術語 | 定義 |
| :--- | :--- |
| 建置期組合（Build-Time Composition） | 於 `wda build` 執行期間將靜態片段展開為純 HTML 的機制，產出物不殘留任何執行期（runtime）邏輯 |
| `wda-include` | WDA 唯一支援之自訂建置期 HTML 標籤，於建置時展開為元件內容，不輸出至 `dist/` |
| 建置工作區（Build Workspace） | 建置工作區是 `wda build` 於專案目錄外所建立的暫存工作目錄。建置結束後立即刪除。工作區內部使用 WDA 專屬且嚴格的 `deno.json` 設定檔。該設定檔僅合併專案 `deno.jsonc` 的 `imports`，並與專案端檔案完全隔離 |
| `deno.lock` | `deno.lock` 是專案相依性套件的鎖定檔案。它記錄完整相依關係圖與套件雜湊，並與宣告檔 `deno.jsonc` 分離。`wda deps` 會寫入此檔案。`wda build` 僅以唯讀方式參照此檔案，不進行未鎖定的網路解析 |
| 碰撞規則（Collision Rule） | `wda init` 執行的前置安全檢查：當欲建立之任何目標路徑已存在於目錄中時，立即拒絕執行並退出 |
| 還原基準點（Rollback Baseline） | `wda init` 於專案目錄所建立之本機 Git 儲存庫的第一個初始 Commit |
| 不可分割初始化（Atomic Init） | `wda init` 的安全保證：若初始化過程中遭遇任何失敗，將自動復原並完整清除所有殘留檔案 |
| 解析銜接層 | Core 內部允許依據 Design System 產生邏輯分支的第一處白名單介面：`src/init/resolve.rs` |
| 內建資產層 | Core 內部允許依據 Design System 產生邏輯分支的第二處白名單介面：`src/init/` 與 `src/builtins/` 下之指定檔案 |

## Design System

| 術語 | 定義 |
| :--- | :--- |
| Design Token（設計權杖） | 設計決策的最小具體數值（例如顏色、間距或字級），以具備語意之識別名稱指涉 |
| DTCG | Design Tokens Community Group 制訂之標準 JSON 格式，專案以 `tokens/tokens.json` 作為唯一權威來源 |
| `WDA Minimal` | WDA 內建的最小化 Design System，作為離線環境時之退回選項。線上成功初始化時採 best-effort 建立 `deno.jsonc` 與 `deno.lock` 以預裝 Lit；離線初始化時則不建立相依檔案，維持零外部相依結構 |
| Spectrum / Spectrum 2 | Adobe 推出的 Design System，為 `wda init` 於線上查詢時的預設選項 |
| Adapter | 定義 Design System 如何接入 WDA 專案之介面規格，由七個面向（facet）組成 |
| Facet | Adapter 介面規範中的單一面向，涵蓋 Token mapping、樣式發布與組織、字型設定、圖示資源處理、色彩主題、元件相依模型與套件整合等七個面向 |
| 中立性（Neutrality） | Core 不綁定單一 Design System 之架構承諾。定義為同時支援兩套相依模型與元件模型具備實質差異之系統，且各自皆能走完 init/check/build |
| 離線退回（Offline Fallback） | 當 `wda init` 線上解析 Design System 失敗時，自動退回採用 `WDA Minimal` 之安全機制 |
| 身分分支（Identity Branch） | 於 Core 程式碼中依 Design System 名稱進行判斷或分支處理之邏輯，嚴格限定僅能存在於白名單檔案中 |

## 驗證與診斷

| 術語 | 定義 |
| :--- | :--- |
| Diagnostic（診斷紀錄） | `wda check` 或 `wda build` 檢核專案時所輸出的單筆問題紀錄 |
| Severity（嚴重層級） | 診斷紀錄之分類層級，分為阻擋建置的 Error 與不阻擋建置的 Warning 兩級 |
| Error | 會導致建置成果不正確或違反專案契約之重大問題，將阻擋建置流程並導致指令回傳狀態碼 1 |
| Warning | 不阻擋建置產出，但代表潛在風險且需持續提示使用者的問題，輸出於 stderr |
| ToolFault | 外部相依工具或檔案系統發生非預期崩潰或嚴重異常時擲出的工具錯誤，指令回傳狀態碼 2 |
| 結束狀態碼（Exit Status） | CLI 指令執行完畢時回傳之處理狀態：`0`（成功或僅含 Warning）、`1`（回報 Error）、`2`（用法錯誤或 ToolFault） |
| 原始程式碼檢查（Source-Check） | 僅需分析原始程式碼檔案即可確定判定結果，不需藉由真實瀏覽器進行動態渲染之靜態檢查 |
| 單一驗證接縫（Single Validation Seam） | Core 對外暴露之唯一驗證函式介面 `validate_project(root)`，binary 無法直接碰觸內部個別驗證規則模組 |
| 四大規則家族（Four Rule Families） | Core 內部組織驗證邏輯之四大模組：contract、documents、html、tokens |

## Skill 與驗收

| 術語 | 定義 |
| :--- | :--- |
| 白名單內容 | `SKILL.md` 僅允許收錄之六類指引內容：角色邊界宣告、版本比對步驟、決策路由表、流程停止條件、回報義務與權威指標 |
| 觸發不足（Undertriggering） | AI 在應當載入 Skill 的情境下未主動載入，導致繞過 Core 規範修改檔案之失誤行為 |
| 四層驗收（Four-Tier Acceptance） | Skill 的品質檢驗標準架構，包含 L1 規格合規、L2 邊界防護、L3 行為評測與 L4 刪除測試 |
| 刪除測試（Deletion Test） | L4 驗收層級：將整個 Skill 移除後，Core 與專案文件仍足以供人類使用者正確操作 WDA 的人工驗收測試 |
| 權威來源指標（Authority Pointer） | Skill 規範句同段落中必須具備之指向文件路徑（如 `docs/`、章節符號 `§` 或 `ADR-`）的參考標記 |

## 文件

| 術語 | 定義 |
| :--- | :--- |
| 必要文件 | Generated Project 初始化時必須具備的四份核心文件：`README.md`、`docs/architecture.md`、`docs/design.md`、`docs/naming.md` |
| 條件式文件 | 僅於專案出現未涵蓋之特殊工作流程時方行建立之文件，例如 Generated Project 的 `docs/development.md` |
| 權威來源（Canonical Source） | 某項專案狀態或規格的唯一真正來源。文件僅可對其進行摘要，嚴禁複製其內容 |
| `sourceRefs` | 文件 front matter 中的陣列欄位，宣告該文件所依據或衍生自哪些權威來源路徑 |
| Front Matter | Markdown 檔案開頭以 `---` 包覆之 YAML 結構化詮釋資料區塊 |
| ADR | Architecture Decision Record（架構決策紀錄），用於記錄單一技術決策之脈絡、選項、決策與後果之一手歷史文件 |
| OKF | Open Knowledge Format，本專案 front matter 欄位結構與文件標準之規範基礎 |
