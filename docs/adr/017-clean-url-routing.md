---
title: "階層目錄索引靜態路由規範（Clean URLs）"
status: deprecated
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-017"
description: "Why does WDA not guarantee clean URLs for generated sites?"
tags: []
type: ADR
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：多頁面網站部署到 GitHub Pages、Cloudflare Pages 或 S3 時，需要乾淨無後綴的 URL。  
* **決策**：`pages/about.html` 或 `pages/about/index.html` 統一輸出為 `dist/about/index.html`，以目錄索引形式支援乾淨網址。  
* **影響**：輸出物具備 100% 的靜態主機相容性與現代 Web 路由體驗。

## Current status

Superseded by: **全面不提供 Clean URL guarantee**。
