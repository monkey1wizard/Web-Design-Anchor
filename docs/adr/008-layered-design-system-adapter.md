---
title: "分層式 Design System Adapter 架構（Token + Semantic CSS + 可選 Web Components）"
status: stable
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-008"
type: ADR
description: "Why does the design system adapter use layered tokens, semantic CSS, and optional Web Components?"
tags: []
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：專案需要能夠切換不同的 Design System，且在有官方 Web Components（如 Lit 函式庫）時希望能善用其能力，但又不能將專案架構硬綁死在單一系統。  
* **決策**：分層適配機制。以語意化 Tokens 與原生 CSS 樣式為切換設計系統的基石；當有官方成熟的 Web Components（如 Lit-based 函式庫）時亦能無縫引入並包裝，兼具官方元件能力與設計系統切換彈性。  
* **影響**：專案既能維持輕量與切換靈活性，又能相容官方 Web Components 生態。

## Current status

The historical decision remains Accepted. Under the current §2.5 Adapter boundary, vendor public element names and public APIs are legitimate dependency usage. WDA does not build a universal wrapper merely to hide vendor names; project-specific wrappers remain appropriate only when they add product semantics, reusable behavior, or meaningful encapsulation.
