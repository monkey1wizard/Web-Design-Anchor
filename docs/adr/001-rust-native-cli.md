---
title: "核心工具使用 Rust / Cargo 實作 Native CLI"
status: stable
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-001"
type: ADR
description: "Why does the core tool use Rust and Cargo for a native CLI?"
tags: []
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：需要一套穩定、跨平台、高效能且不侵入生成網站 runtime 的工具鏈。  
* **決策**：Core Tooling 與 CLI 以 Rust 開發，提供 `cargo install` 與 Prebuilt binary。  
* **影響**：工具獨立於 Web 專案的 npm/pnpm 套件生態，避免 Node.js 版本或套件衝突。

## Current status

The front matter status is the current status: stable. No conflict was found during migration.
