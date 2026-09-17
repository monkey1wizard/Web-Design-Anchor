---
title: "AI Skill 為主要互動介面，專案檔案為 Source of Truth"
status: stable
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-003"
type: ADR
description: "Why is the AI Skill the main interaction interface while project files remain the source of truth?"
tags: []
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：降低 Designer 技術負擔，同時避免重要架構與決策記憶遺失在 AI 聊天紀錄中。  
* **決策**：以自然語言 \+ Skill 操作 CLI，所有決策與規格自動/手動寫回 `/docs`。  
* **影響**：新對話 session 或新團隊成員可無縫接手。

## Current status

The front matter status is the current status: stable. No conflict was found during migration.
