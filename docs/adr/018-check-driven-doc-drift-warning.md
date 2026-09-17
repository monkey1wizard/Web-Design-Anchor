---
title: "檢核驅動之文件飄移警示協定（Check-driven Doc Drift Warning）"
status: deprecated
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-018"
description: "What document drift does the V1 check command detect?"
tags: []
type: ADR
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：確保 AI 或設計師在修改程式碼後，專案文件（/docs）不發生脫節（Drift）。  
* **決策**：`check` 命令比對專案結構、Token 定義與依賴變更，若發現未記錄於 `/docs` 則觸發醒目警示，由 AI 在當次對話中自動補齊文件。  
* **影響**：制度化保障「Project files 為唯一 Source of Truth」的原則。

## Current status

Superseded by B3. V1 `check` validates document structure and metadata only; it does not compare prose semantics.
