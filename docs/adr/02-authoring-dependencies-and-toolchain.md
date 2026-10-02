---
title: "Authoring、相依與工具鏈"
status: stable
updated: 2026-09-21
type: ADR
description: "外部工具前置條件、TypeScript 與 CSS 處理管道、Deno 原生相依性管理與向下相容分支。"
tags: []
absorbs: ["006", "011", "013"]
---

# ADR-02 Authoring、相依與工具鏈

## 背景

Core 本身是 native binary，但需要解析 npm 相依性並將 TypeScript 轉為 JavaScript。同時 Designer 撰寫的 Modern CSS 需要與 token 整合，外部相依性需要可靠的宣告、解析與鎖定機制。專案需要決定依賴哪些外部工具，如何宣告相依性，以及如何維持既有專案的相容性。

## 選項

**要求使用者安裝 Node.js 與 npm/pnpm，由 esbuild 獨立 binary 或 lightningcss 處理。** 最常見的前端生態做法。代價是將 Node.js 版本管理與 node_modules 的重量帶給 Designer，且 esbuild 需針對多平台分發 binary。

**自建套件下載器存入 vendor 目錄加 import map，並自製編譯引擎。** 完全無需外部工具。代價是 Core 必須重寫套件解析、版本去重與編譯器，複雜度不可承受。

**外部工具僅要求 deno 與 git。** 現行相依性由 deno.jsonc 宣告並由 Deno 管理。
保留 legacy package.json 分支。
TypeScript 經 Deno 呼叫釘版 esbuild 轉譯。
CSS 原樣保留。
deno 與 git 是唯二 PATH 前置工具。
新專案相依性採用 Deno 原生 deno.jsonc 的 imports 宣告並鎖定於 deno.lock。
Deno 負責檔案寫入與註解保留。
舊專案繼續支援 package.json。
TypeScript 經靜態型別檢查後由釘死版本的 esbuild 轉譯。
CSS 原樣複製，不指定 CSS 引擎。

## 決策

外部工具僅要求 deno 與 git 存在於 PATH 上，不要求 Node.js。deno 負責相依性解析與呼叫轉譯器；git 負責初始化時建立本機版本控制基準點。缺任一工具時明確失敗，不靜默降級。

不指定 CSS 引擎。styles/**/*.css 原樣複製到 dist/styles/，只有 token 轉成 Custom Properties。TypeScript 必須通過靜態型別檢查，再由 `deno run -A npm:esbuild@0.25.5` 轉譯為 JavaScript。

相依性宣告與管理：

- 現行專案統一採用 deno.jsonc 的 imports 宣告相依性，並由 Deno 解析鎖定於專案根目錄的 deno.lock。拒絕專案端 deno.json 以避免設定歧異。
- 保留向下相容分支：既有以 package.json 宣告相依性的專案維持原有宣告與建置行為，check 與 build 絕不暗中遷移檔案結構。
- wda deps 是唯一改變相依性的指令入口，呼叫 Deno 時帶入明確 `--config` 且停用第三方 lifecycle scripts。檔案寫入與註解保留由 Deno 擁有，WDA 僅處理空狀態清理。
- 所有相依套件在建置時一律 bundle 進 dist/，不留 remote runtime 依賴。

## 後果

- Designer 僅需安裝 deno、git 與 wda 三個執行檔。
- 轉譯器與相依解析版本受控，不隨本機環境漂移。
- 現行專案享有 JSONC 註解與原生的 Deno 模組生態，舊專案不受破壞性更新影響。
- Designer 撰寫的 CSS 在 dist/ 一字不差；所有相依性打包發布，無 CDN 連線風險。

## 狀態沿革

- 2026-08-24：從企劃文件遷入，當時寫的是「尋求 Rust 原生 binary 或 crate」。
- 2026-09-02：Project Owner 裁定 `deno` 是唯一 PATH 前置工具，轉譯器經 `deno run` 釘版呼叫。
- 2026-09-08：`git` 加入為第二個前置工具。
- 2026-09-18：改寫成完整紀錄。
- 2026-08-24：從企劃文件遷入，當時的決策是 lightningcss 加 esbuild 獨立 binary。
- 2026-08-24：改為不指定 CSS 引擎。
- 2026-09-02：轉譯器改為經 `deno run` 釘版呼叫。
- 2026-09-18：改寫成新決策的完整紀錄。
- 2026-08-24：從企劃文件遷入，當時的決策是 vendor 目錄加 importmap。
- 2026-08-24：改為 `package.json` 加 `deno.lock`。
- 2026-09-02：Project Owner 裁定一律 bundle，remote 之後再看需求決定。
- 2026-09-18：改寫成新決策的完整紀錄。
- 2026-09-21：本記錄由既有原子決策收斂而成，來源見 front matter absorbs。
