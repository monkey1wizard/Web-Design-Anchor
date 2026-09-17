---
title: "輸出標準 Web (Framework-neutral) 與雙 Baseline 架構"
status: stable
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-002"
type: ADR
description: "Why does the output use framework-neutral standard Web technologies and dual baselines?"
tags: []
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：避免初期為 landing page 等需求過早引入框架複雜度，同時確保長期維護性與可讀性。  
* **決策**：Source 為 HTML5 \+ CSS \+ TS，Output 為 HTML \+ CSS \+ JS \+ assets。  
* **影響**：降低 Designer 與 AI 的維護門檻，並保留未來由工程師引入框架的乾淨遷移起點。

## Current status

The front matter status is the current status: stable. No conflict was found during migration.
