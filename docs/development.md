---
title: "WDA Core 開發與驗證"
status: stable
updated: 2026-08-24
sourceRefs:
  - "ROADMAP.md §2.13"
  - "ROADMAP.md §3.2 B1"
  - "ROADMAP.md §3.2 G4"
  - "ROADMAP.md §3.2 G5"
  - "ROADMAP.md §3.2 G6"
type: Guide
description: "本文件說明 WDA Core 的開發與驗證規則。"
tags: []
---

# WDA Core 開發與驗證

本文件是 WDA Core 的 unconditional development and validation guidance，承接
`ROADMAP.md` §2.13。它描述 Core repository 的文件與驗證規則，不是
Generated Project 的 project-specific development instructions。

## Core 文件的適用範圍

本文件是 unconditional 文件，Core repository 只要採用 WDA 的文件契約就適用，
不以某個 project 是否需要額外工作流為條件。

§2.13 的 B1 conditional rule 只約束 Generated Project 的同名
`docs/development.md`。只有 project 需要一個未被四個 WDA 指令或 required
project documents 涵蓋的 project-specific development command、tool
prerequisite、automation 或 manual verification workflow 時，才建立並維護
該 Generated Project 文件。Browser / a11y baselines 以 `wda.json` 為
canonical source，並摘要於 `docs/architecture.md` 或 `docs/design.md`；它們
本身不會觸發該文件。（裁定 B1）

`wda init` 永不建立 Generated Project 的 `docs/development.md`；這項條件不
影響本 Core 文件的 unconditional 適用範圍。（裁定 B1）

## WDA-managed Markdown metadata

新產出的 current-truth Markdown 文件位於 `docs/` 時，必須使用 YAML front
matter，包含非空的 `title`、`status` 與 `updated`（格式為 `YYYY-MM-DD`）。
文件若衍生自其他 canonical source，`sourceRefs` 必填，且每一項都必須是
project-root-relative 的檔案或章節參照。`README.md` 與歷史 `prefile_*` 文件不適用
此慣例。（裁定 G4）

對本 repo 自己的 `docs/` 檔案，OKF 的 `type`、`description` 與 `tags` 三個鍵也都是必填；其規格依 [canonical `open-knowledge-format` SPEC.md](https://github.com/GoogleCloudPlatform/open-knowledge-format/blob/main/SPEC.md)，不採用其他專案下已凍結的快照。OKF v0.2 將 `type` 定義為必填的自由字串，因此本 repo 使用的六值 `type` 字彙是本 repo 的慣例，不是 OKF 定義的字彙；只有 `status` 的 `draft`、`stable`、`deprecated` 字彙由 OKF 定義。`wda check` 對 Generated Project 的 frontmatter 契約維持不變，仍只要求非空的 `title`、`status` 與 `updated`。（裁定 A-03）

## ADR 命名

ADR 的 display identifier 使用 `ADR-NNN`；檔名使用
`docs/adr/NNN-<kebab-case-slug>.md`，並使用同一個 zero-padded 三位數編號。
這項規則同時適用於目前仍有效與已 superseded 的 ADR。（裁定 G5）

## Generated Project architecture template

Generated Project 的 `architecture.md` 範本分成以下兩節：

- `Created by wda init`：只列出 `wda init` 實際建立的 paths。
- `Allowed when needed`：列出 optional patterns 與其建立 trigger，並明載
  `wda init` 不建立任何 placeholder 目錄。

這是 Generated Project 範本的分層規則，不是本 Core 文件的條件式建立規則。
（裁定 G6）

## 外部工具前置需求（Prerequisites）

WDA Core 開發與建置環境中，`deno` 與 `git` 是兩個必須存在於系統 PATH 上的外部工具。

TypeScript 轉譯器（transpiler）由 WDA 直接以 `deno run -A npm:esbuild@0.25.5`（即 `deno run -A npm:esbuild@<pinned version>`）呼叫，其版本由 WDA 釘死而非由環境決定，因此 `esbuild` 本身永不是系統 PATH 上的前置需求。`deno` 用於相依解析（dependency resolution）與轉譯器呼叫；`git` 則因 Core 硬綁 git（見 ADR-023），由 `wda init` 建立本地 Git repository，且涵蓋其行為的測試亦會實際呼叫（shell out）真實 `git`。

`wda build` 在專案目錄之外準備一個暫存的 build workspace（build workspace，暫存建置工作區）。只有在專案根目錄存在 `package.json` 時，`build` 才會在這個 workspace 內執行 `deno install --frozen`；`build` 從不傳遞 `--allow-scripts`，因此這次安裝不會執行任一套件的 npm 生命週期腳本。建置完成後，不論成功或失敗，`build` 都會刪除這個 workspace。專案根目錄的 `deno.lock` 在整個 `build` 過程中維持位元不變，`build` 從不重寫它。
