---
title: "WDA Core 術語詞彙表"
status: stable
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md §2"
  - "ROADMAP.md §3.2 B1"
type: Reference
description: "本文件定義 WDA Core 使用的主要術語。"
tags: []
---

# WDA Core 術語詞彙表

本文件定義 WDA Core 使用的主要術語。

| 術語 | 定義與說明 |
| --- | --- |
| **Source Baseline** | 原始開發基準，定義為 HTML5、Modern CSS、TypeScript 與原生瀏覽器能力（必要時使用 Web Components）。 |
| **Deployment Baseline** | 最終輸出與部署基準，僅包含純 HTML、CSS、JavaScript 與靜態 assets，無額外 runtime 依賴。 |
| **Framework-neutral Output** | 不綁定任何前端框架（React、Vue、Astro 等）的標準 Web 輸出。 |
| **Integration Contract** | Core Tooling 對外開放的標準化資料交換規範，使設計工具（Figma、Penpot 等）能透過正規化數據對接，而不把各工具專屬 schema 帶入 Core。 |
| **Core Tooling (Rust / Cargo)** | 以 Rust 實作的本地原生命令列工具，負責專案生成、規範檢查、驗證與建置協調，不進入生產環境 runtime。 |
| **Documentation-Driven Development (DDD)** | 以 `README.md` 為唯一入口、`docs/` 為唯一真實來源的專案維護模式。Generated Project 實際的文件集合以 [`docs/templates/generated-project/architecture.md`](templates/generated-project/architecture.md) 為準：`wda init` 建立 `docs/architecture.md`、`docs/design.md` 與 `docs/naming.md`，`docs/development.md` 是 conditional 文件，`wda init` 永不建立。（裁定 B1）ADR-005 記錄的五份文件清單是歷史決策，不是現行文件集合。 |
| **Build-time HTML Partials** | Source 端撰寫的輕量化模組化 HTML 片段（如 header、footer、card），由 Rust Core 在編譯時期無損展開為純靜態 HTML，不引入客戶端 runtime 或繁重的模板引擎。 |
| **Layered Design System Adapter** | 分層式設計系統適配器架構，由底層的 Token 映射（CSS Custom Properties）與標準語意化 CSS 元件為基底，並在需要時向上相容官方 Web Components（如 Lit-based 函式庫），實現可在多設計系統間切換而不破壞專案結構。 |
