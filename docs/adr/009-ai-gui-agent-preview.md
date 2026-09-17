---
title: "整合 AI GUI Agent 原生預覽能力"
status: stable
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-009"
description: "Why does the tool use AI GUI Agent native preview capabilities instead of binding Core to a local server?"
tags: []
type: ADR
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：設計師主要在 Claude、Codex 等現代 AI GUI 介面中協作，若需細部微調再進入熟悉的設計或程式工具。  
* 決策：Core Tooling 專注產出標準、乾淨的靜態 Web 產物與明確的 Entrypoint，直接交由 AI Agent 的 Webview / Artifact 原生渲染機制進行即時預覽，不需在 Core 綁定笨重的本地伺服器。  
* 影響：流程更加輕盈，符合現代 AI 輔助開發體驗。

## Current status

The front matter status is the current status: stable. No conflict was found during migration.
