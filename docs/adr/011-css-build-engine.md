---
title: "採用雙原生建置引擎（LightningCSS Crate + esbuild Standalone Binary）"
status: deprecated
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-011"
description: "Why did the tool adopt native LightningCSS and standalone esbuild for CSS and script builds?"
tags: []
type: ADR
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：需要兼顧毫秒級打包效率、無 Node.js 依賴與最高度社群穩定性。  
* **決策**：CSS 由 Rust crate `lightningcss` 原生處理；TS/JS 打包直接調用官方 prebuilt 單一 esbuild standalone native binary（Go 實作，單檔數 MB，零 Node.js 依賴）。  
* **影響**：大幅降低 V1 自研打包器的風險，達到開箱即用與零環境污染。

## Current status

Superseded by: **不指定任何 CSS engine**。
