---
title: "V1 內建雙參考設計系統（Adobe Spectrum + Modern Neutral Clean）"
status: deprecated
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-019"
description: "Which reference design systems does WDA use to validate adapter boundaries?"
tags: []
type: ADR
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：驗證分層適配架構能否同時支援「純語意 CSS 零依賴」與「官方 Lit/Web Components」兩種極端場景。  
* **決策**：V1 內建 Adobe Spectrum（驗證 Lit 官方庫整合）與 Modern Neutral Clean（驗證純 CSS 語意切換）。  
* **影響**：向社群證明工具既能無縫對接設計大廠元件庫，又能維持極簡獨立性。

## Current status

Superseded by: 正式命名 `WDA Minimal`。
