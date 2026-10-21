---
title: "Spectrum Adapter 對照"
status: stable
updated: 2026-10-15
type: Reference
description: "Spectrum 2 實作與 WDA Adapter 七個 facet 的對照。"
tags: []
---

# Spectrum Adapter 對照

本文件記錄程式碼已實作的 Spectrum 2 整合。七個 facet 的通用定義見 [docs/design-system.md](../design-system.md)。

Spectrum 2 於 2026-09-07 加入專案。提交紀錄位於 `src/builtins/spectrum_two/`。

## 七個 facet

### 1. Token mapping

目前程式碼未定義 Spectrum token 與 WDA token 的對應。

### 2. Styles

建置時，`src/build/scripts.rs` 透過 Deno 執行 esbuild。命令使用 `--bundle` 打包 script entry。
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

`src/init/mod.rs::spectrum_two_dependency_specs` 依解析出的 Spectrum 版本固定套件版本。
初始化會把清單寫入專案的 `deno.jsonc`，再由 `deno.lock` 鎖定。

## 對照摘要表

| Facet | 已實作內容 |
| --- | --- |
| Token mapping | 尚未定義 token 對應 |
| Styles | 建置時以 esbuild `--bundle` 打包 script entry |
| Fonts | 沒有字型套件或建置時字型解析 |
| Icons | 沒有獨立圖示套件或建置時圖示解析 |
| Theme | 相依清單包含 theme 套件 |
| Component dependencies | Spectrum Web Components 套件 |
| Dependency integration | `deno.jsonc` 宣告相依，`deno.lock` 鎖定版本 |

| 套件 | 版本來源 |
| --- | --- |
| `@spectrum-web-components/theme` | 已解析的 Spectrum 版本 |
| `@spectrum-web-components/button` | 已解析的 Spectrum 版本 |
| `@spectrum-web-components/textfield` | 已解析的 Spectrum 版本 |
| `@spectrum-web-components/picker` | 已解析的 Spectrum 版本 |
| `@spectrum-web-components/card` | 已解析的 Spectrum 版本 |
| `@spectrum-web-components/dialog` | 已解析的 Spectrum 版本 |
| `lit` | 獨立解析 |

以上七個直接相依套件列於 `src/init/mod.rs::spectrum_two_dependency_specs`。
若初始化安裝失敗，WDA 會清除已建立的 Spectrum 專屬檔案與相依設定，再建立 WDA Minimal 專案。
