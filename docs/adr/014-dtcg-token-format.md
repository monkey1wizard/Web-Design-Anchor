---
title: "以 W3C DTCG JSON 作為 Design Token 核心交換標準"
status: stable
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-014"
description: "Why does the tool use W3C DTCG JSON as the core design token exchange format?"
tags: []
type: ADR
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：需為 V2 的設計工具（Figma, Penpot 等）外掛預留 Integration Contract。  
* **決策**：Token 以 `tokens/tokens.json`（W3C Design Tokens Community Group 規範）為主要來源，Core 自動編譯生成 `styles/tokens.css` 的 CSS Custom Properties。  
* **影響**：與全球設計工具標準直接接軌，實現跨工具零阻礙匯出與轉換。

## Current status

stable. The historical source-side wording remains preserved above. The current rule is that generated token CSS exists only in `dist/`; `styles/tokens.css` is not a source-side generated artifact.
