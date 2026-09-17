---
title: "採用固定結構的 Documentation-Driven Development"
status: stable
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-005"
type: ADR
description: "Why does the project use a fixed structure for documentation-driven development?"
tags: []
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：傳統前端專案文件容易脫節，需確保人與 AI 皆有統一步調。  
* **決策**：以 `README.md` 為單一入口，其餘依責任拆分至 `architecture.md`, `design.md`, `naming.md`, `development.md`, `handoff.md`。  
* **影響**：大幅降低交接與 AI context 探索成本。

## Current status

The front matter status is the current status: stable. B1 narrows the five-document set for the Generated Project: `docs/development.md` is conditional, and `wda init` never creates it.
