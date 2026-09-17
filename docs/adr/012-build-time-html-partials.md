---
title: "採用標準 Web 語意標籤的 Build-time HTML Partials 語法"
status: deprecated
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-012"
description: "Why does the tool expand semantic HTML partials at build time?"
tags: []
type: ADR
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：Source 端需要模組化 HTML 片段（如導覽列、頁尾），需兼顧標準 Web 語感與 AI/設計師直覺。  
* **決策**：採用語意化標籤 `<include src="components/nav.html"></include>` 與 `<slot name="content"></slot>`。  
* **影響**：不破壞 HTML 語法高亮，無特殊模板引擎學習負擔，編譯時無損展開為純靜態 HTML。

## Current status

Superseded by: 只有 `<wda-include>`。
