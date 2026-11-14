---
title: "Spectrum Adapter 對照"
status: stable
updated: 2026-11-12
type: Reference
description: "Spectrum 2 實作與 WDA Adapter 七個 facet 的對照。"
tags: []
source: docs/design-systems/spectrum-adapter.md
lang: zh-TW
source_sha256: a28518d6dfcc81b176e7a644821dbdf539b016c5a2b1fe9183f192802170151c
translated_at: 2026-11-12
---

# Spectrum Adapter 對照

本文件記錄程式碼已實作的 Spectrum 2 整合。七個 facet 的通用定義見 [docs/design-system.md](../../design-system.md)。

Spectrum 2 於 2026-09-07 加入專案。提交紀錄位於 `src/builtins/spectrum_two/`。

## 七個 facet

### 1. Token mapping

目前程式碼未定義 Spectrum token 與 WDA token 的對應。

### 2. Styles

建置時，`src/build/` 內的原生建置以原生打包器打包 script entry，再以已驗證的原生 TypeScript 工具進行型別檢查。不需要 Deno 或 Node.js 執行環境。
這描述程式碼的打包流程，不代表另有 Spectrum 樣式整合。

### 3. Fonts

初始化相依清單沒有字型套件。建置程式也沒有解析字型資源的步驟。

### 4. Icons

初始化相依清單沒有獨立圖示套件。建置程式也沒有解析圖示資源的步驟。

### 5. Theme

初始化相依清單包含 `@spectrum-web-components/theme`。程式碼未定義其他主題設定或切換行為。

### 6. Component dependencies

啟動頁面使用 `@spectrum-web-components/*` 套件。下表列出套件與其他相依整合方式。

### 7. Dependency integration

`src/init/mod.rs` 的 `spectrum_two_dependencies` 會把六個 Spectrum 套件固定為同一個已解析的 Spectrum 版本，並把 `lit` 固定為它自己的已解析版本。
初始化會把這些精確版本寫入專案的 `package.json`。接著專案的套件管理工具把它們安裝到 `node_modules` 並寫入鎖定檔。
原生建置會從已安裝的套件解析 `scripts/main.ts` 中的裸匯入與子路徑匯入。建置時不抓取套件，瀏覽器執行時也不抓取。

## 對照摘要表

| Facet | 已實作內容 |
| --- | --- |
| Token mapping | 尚未定義 token 對應 |
| Styles | 建置時由原生建置打包 script entry |
| Fonts | 沒有字型套件或建置時字型解析 |
| Icons | 沒有獨立圖示套件或建置時圖示解析 |
| Theme | 相依清單包含 theme 套件 |
| Component dependencies | Spectrum Web Components 套件 |
| Dependency integration | `package.json` 宣告精確版本，套件管理工具安裝並寫入鎖定檔 |

| 套件 | 版本來源 |
| --- | --- |
| `@spectrum-web-components/theme` | 已解析的 Spectrum 版本 |
| `@spectrum-web-components/button` | 已解析的 Spectrum 版本 |
| `@spectrum-web-components/textfield` | 已解析的 Spectrum 版本 |
| `@spectrum-web-components/picker` | 已解析的 Spectrum 版本 |
| `@spectrum-web-components/card` | 已解析的 Spectrum 版本 |
| `@spectrum-web-components/dialog` | 已解析的 Spectrum 版本 |
| `lit` | 獨立解析 |

以上六個 Spectrum 直接相依套件列於 `src/init/mod.rs` 的 `SPECTRUM_TWO_DIRECT_DEPENDENCY_PACKAGES`。`lit` 是另一個直接相依，版本獨立。

支援限制：

- 啟動頁面只支援上列套件，以及它們提供給啟動腳本的模組。
- 專案不主張所有 npm 套件都能在瀏覽器中運作。只有原生建置能打包成本地檔案的套件才能運作。
- 沒有任何已安裝套件提供的裸匯入或子路徑匯入，會讓建置失敗，並且錯誤訊息會指出來源檔案。

若初始化安裝失敗，WDA 會清除已建立的 Spectrum 專屬檔案與相依設定，再建立 WDA Minimal 專案。
