---
title: "Generated Project README Template"
status: stable
updated: 2026-08-24
sourceRefs:
  - "ROADMAP.md §2.13"
  - "ROADMAP.md §4 P4"
  - "docs/architecture.md §2.8"
type: Template
description: "Provides the documentation entry point and getting started instructions for a generated WDA project."
tags: []
---

# Web Design Anchor Project

> Project documentation entry point and getting started instructions.

## Getting Started

This is a Web Design Anchor (WDA) project built with standards-first HTML5, Modern CSS, and TypeScript.

### Project Initialization and Version Control

`wda init` obeys the collision rule: it accepts a directory containing existing files, but will refuse to initialize if any target path it plans to create (including `.git/`) is already present. Initialize the WDA project:

```bash
wda init
```

The project is already a local git repository once `wda init` succeeds: its first commit covers the whole directory (including any files you already had in place), and no remote is configured. There is no separate step to set up version control.

## Development Commands

- `wda check` — Validate project structure, required documents, DTCG tokens, and HTML compliance.
- `wda build` — Compile static deployment artifacts into `dist/`.

## Preview

After `wda build` succeeds, preview the project through the AI host and inspect the first viewport. A successful build is not visual acceptance.

- **Claude Code** — Ask the host to open the absolute path to `dist/index.html` and confirm the first viewport. Verified on host Claude Code, version `2.1.260`, date `2026-09-09`.

## Project Structure

- `wda.json` — WDA project manifest and configuration.
- `pages/` — Semantic HTML entry points for MPA routes.
- `tokens/` — Design token definitions in W3C DTCG format (`tokens/tokens.json`).
- `docs/` — Core project documentation (`architecture.md`, `design.md`, `naming.md`).
- `dist/` — Isolated static output produced by `wda build` (untracked in Git).
