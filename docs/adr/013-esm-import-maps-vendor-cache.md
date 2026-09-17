---
title: "採用標準 ESM Import Maps 與靜態 Vendor 快取管理依賴"
status: deprecated
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-013"
description: "Why does the tool use standard ESM import maps and a static vendor cache for dependencies?"
tags: []
type: ADR
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：當專案引入 Lit 或官方 Web Components 時，需在無 npm / node\_modules 的環境下維持離線可重現性。  
* **決策**：Core 在初始化或引入元件時將標準 ESM 模組儲存至 `scripts/vendor/`，並透過標準 `importmap` 映射。  
* **影響**：完全遵循 Web 標準，免除 npm 安裝負擔，且符合離線重現與安全要求。

## Current status

Superseded by: `package.json` + Deno / `deno.lock`。
