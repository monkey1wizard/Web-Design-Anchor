---
title: "V1 採用 Build-Only 策略，不綁定 Hosting"
status: stable
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-004"
type: ADR
description: "Why does V1 build deployable static files without binding the tool to a hosting provider?"
tags: []
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：保持 Core Tooling 的純粹與職責單一。  
* **決策**：Core 只負責產出靜態可部署檔案，部署交由任何靜態空間（Vercel, Cloudflare, Netlify, S3 等）。  
* **影響**：不增加雲端運維與帳號綁定成本。

## Current status

The front matter status is the current status: stable. No conflict was found during migration.
