---
title: "Authoring、相依與工具鏈"
status: stable
updated: 2026-10-15
type: ADR
description: "外部工具前置條件、TypeScript 與 CSS 處理管道、Deno 原生相依性管理與向下相容分支。"
tags: []
absorbs: ["006", "011", "013"]
---

# ADR-02 Authoring、相依與工具鏈

## 背景

WDA 是原生執行檔，但仍須解析 npm 相依性並將 TypeScript 轉成 JavaScript。Designer 撰寫的 Modern CSS 也須與 token 整合。專案需要可靠的相依性宣告、解析與鎖定方式，也需要決定外部工具需求及既有專案的相容方式。

## 選項

**要求使用者安裝 Node.js 與 npm/pnpm，再由獨立 esbuild 執行檔或 lightningcss 處理。** 這符合常見的前端開發方式。
但這會把 Node.js 版本管理與 node_modules 的負擔交給 Designer。
esbuild 也須為多個平台分發執行檔。

**自建套件下載器，將套件存入 vendor 目錄並使用 import map，再自製編譯引擎。** 這種方式不需要外部工具，但 Core 必須自行實作套件解析、版本去重與編譯器，複雜度過高。

**外部工具只要求 deno 與 git。** 新專案以 deno.jsonc 宣告相依性，並由 Deno 管理。既有專案仍可使用 package.json。Deno 負責保留註解與寫入檔案。TypeScript 先通過靜態型別檢查，再由固定版本的 esbuild 轉譯。CSS 不另指定處理引擎，並維持原樣。

## 決策

外部工具只要求 `deno` 與 `git` 位於 PATH，不要求 Node.js。`deno` 負責解析相依性及呼叫轉譯器。`git` 負責在初始化時建立本機版本控制基準點。執行 `wda init` 時，若缺少 `deno`，指令會改用 WDA Minimal。若缺少 `git`，指令會在寫入前中止。`wda check` 會針對缺少的工具回報 Warning。`wda build` 與 `wda deps` 則會以具名診斷失敗。

WDA 不指定 CSS 引擎。`styles/**/*.css` 會原樣複製到 `dist/styles/`，只有 token 會轉成 Custom Properties。TypeScript 必須先通過靜態型別檢查，再由 `deno run -A npm:esbuild@0.25.5` 轉譯成 JavaScript。

相依性宣告與管理：

- 現行專案使用 deno.jsonc 的 imports 宣告相依性，並由 Deno 解析及鎖定至專案根目錄的 deno.lock。WDA 不接受專案端的 deno.json，以免設定不一致。
- 既有專案若使用 package.json 宣告相依性，仍保留原有宣告與建置方式。`wda check` 與 `wda build` 不會暗中遷移檔案結構。
- `wda deps` 是唯一可變更相依性的指令。它呼叫 Deno 時會指定 `--config`，並停用第三方 lifecycle scripts。Deno 負責寫入檔案及保留註解，WDA 只清理空狀態。
- 所有相依套件都會在建置時打包進 `dist/`，不留下 remote runtime 相依性。

## 後果

- Designer 只需安裝 `deno`、`git` 與 `wda` 三個執行檔。
- 轉譯器與相依解析版本受到控管，不會隨本機環境改變。
- 現行專案可使用 JSONC 註解與 Deno 模組。既有專案不會因破壞性更新而失去支援。
- Designer 撰寫的 CSS 會原樣輸出至 `dist/`。所有相依套件都會打包發布，因此不必在執行時連線至 CDN。
- 缺少 `deno` 時，`wda init` 會改用 WDA Minimal，`wda check` 會回報 Warning，而 `wda build` 與 `wda deps` 會以具名診斷失敗。缺少 `git` 時，`wda init` 會在寫入前中止，其他三個指令則分別回報 Warning 或具名診斷。

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
- 2026-10-14：補上缺少 `deno` 或 `git` 時各指令目前採用的處理方式。
- 2026-09-21：本記錄由既有原子決策收斂而成，來源見 front matter absorbs。
