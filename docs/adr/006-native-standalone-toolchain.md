---
title: "選用 Native Standalone 工具鏈，免除 Node.js 前置依賴"
status: stable
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-006"
type: ADR
description: "Why does the toolchain use native standalone tools without a Node.js prerequisite?"
tags: []
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：Node.js 環境過於龐大且對設計師存在安裝門檻，V1 不希望手寫編譯器，也不希望強制依賴 Node.js。  
* **決策**：Core Tooling 尋求既有的 Rust 原生 binary / crate（如 LightningCSS 處理 CSS、成熟的原生獨立執行檔/crate 處理 TS/JS），杜絕 Node.js 的龐大負擔，V1 不從零手寫編譯器。  
* **影響**：設計師開箱即用，建置速度快，無 npm 安裝與版本衝突負擔。

## Current status

The front matter status is the current status: stable. The historical toolchain example is retained for traceability; the current contract mandates no CSS engine.
