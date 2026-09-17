---
title: "極簡核心 CLI 指令集（`init`, `check`, `build`）"
status: deprecated
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-016"
description: "Why was the core CLI reduced to a minimal set of project lifecycle commands?"
tags: []
type: ADR
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：在 Claude / Codex 等具備強大檔案讀寫能力的 AI Agent 環境中，過度膨脹的 CLI 命令只會增加維護成本。  
* **決策**：Core CLI 僅收斂為三個純粹指令：`init`（建立骨架）、`check`（多維度規則檢核與文件比對）、`build`（打包與資產編譯）。  
* **影響**：大幅精簡 Rust Core 的責任與程式碼體積，檔案生成與重構完全釋放給 AI 發揮。

## Current status

Superseded by: `init` / `deps` / `check` / `build`。
