---
title: "採用結構化 JSON 診斷與高相容性醒目警示協定"
status: deprecated
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-015"
description: "Why did the tool adopt structured JSON diagnostics and highly compatible warning banners?"
tags: []
type: ADR
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：CLI 需提供 AI Agent 精確的修復數據，並在對話中顯著呈現品質警告。  
* **決策**：  
  * 1\. 所有 CLI 指令支援 `--json` 輸出結構化 AST 錯誤、行號（Where）、錯誤原因（What）與修復建議（Fix Suggestions）。  
  * 2\. 終端機純文字輸出採用通用且舊系統高相容性的 Unicode 符號（如 U+26A0 ⚠️）產出顯著的 Warning Banner，確保 AI 在自然語言回覆中主動提醒設計師修正。  
* **影響**：大幅提升 AI 與設計師的修正效率與互動體驗。

## Current status

Superseded by G1. V1 provides no public `--json` diagnostic schema. The replacement is the deterministic human-readable diagnostic contract. It defines stable output streams and exit codes `0/1/2`.
