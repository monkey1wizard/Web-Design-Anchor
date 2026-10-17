---
title: "Generated Project README Template"
status: stable
updated: 2026-10-15
sourceRefs:
  - "docs/architecture.md"
type: Template
description: "Provides the documentation entry point and getting started instructions for a generated WDA project."
tags: []
---

# Web Design Anchor Project

> Project documentation entry point and getting started instructions.

## Getting Started

This Web Design Anchor (WDA) project uses standards-first HTML5, Modern CSS, and TypeScript.

### Project Initialization and Version Control

This project was initialized by `wda init`. The project is already a local git repository. Its first commit covers the whole directory (including any files that were in place), and the project has no configured remote. There is no separate step to set up version control.

## Development Commands

Lit is installed on a best-effort basis during initialization and may be absent when initialization runs offline. To install it later, run `wda deps add lit`.

- `wda check` — Validate project structure, required documents, DTCG tokens, and HTML compliance.
- `wda build` — Compile static deployment artifacts into `dist/`.

## Preview

After `wda build` succeeds, preview the project through the AI host and inspect the first viewport. A successful build is not visual acceptance.

- Ask your AI host or browser to open the absolute path to `dist/index.html` and confirm the first viewport.

## Project Structure

- `wda.json` — WDA project manifest and configuration.
- `pages/` — Semantic HTML entry points for MPA routes.
- `tokens/` — Design token definitions in W3C DTCG format (`tokens/tokens.json`).
- `docs/` — Core project documentation (`architecture.md`, `design.md`, `glossary.md` for project terms and naming conventions).
- `dist/` — `wda build` produces isolated static output (untracked in Git).
