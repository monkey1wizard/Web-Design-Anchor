---
title: "嚴格腳本定位與顯著提示（Persistent Diagnostic Warnings）"
status: deprecated
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-010"
description: "Why does the tool keep diagnostic warnings persistent and highly visible without blocking the build?"
tags: []
type: ADR
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：需明確區分阻擋性錯誤與設計品質警示。  
* 決策：  
  * Script / 語法錯誤（Error）：必須精確指出發生檔案、行號、欄位（Where）以及具體錯誤原因（What），強制阻擋建置。  
  * Token 違規與無障礙（A11y）問題（Warning）：不阻擋建置以維持流程流暢，但採「高顯著度、強烈提示（Persistent & Annoying Prompts）」呈現於檢核結果中，使 AI 與設計師在對話中持續接收修正提醒。  
* 影響：兼顧了開發效率與高品質設計規範的落實。

## Current status

Superseded by: severity 依 **impact** 判斷。
