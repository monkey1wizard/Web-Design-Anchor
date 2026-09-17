---
title: "採用輕量級 Build-time HTML Partials 機制"
status: stable
updated: 2026-08-24
sourceRefs:
  - "prefile_Web Design Anchor 企劃文件與決策紀錄 (ADR & Glossary).md#ADR-007"
type: ADR
description: "Why does the project expand lightweight HTML partials at build time?"
tags: []
---

## Historical decision (frozen)

* **狀態**：Accepted  
* **背景**：多頁面專案需要共用導覽列、頁尾與版面結構，純複製不易維護，而客戶端渲染有 CLS 與 SEO 疑慮。  
* **決策**：Rust Core 實作極簡的靜態 Build-time Partial 展開功能，Source 保持 DRY，Deployment 輸出為 100% 乾淨標準靜態 HTML。  
* **影響**：兼顧開發期結構模組化與輸出期的標準 Web 零依賴特性。

## Current status

The front matter status is the current status: stable. No conflict was found during migration.
