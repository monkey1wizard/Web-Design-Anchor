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

This project was initialized by `wda init`. When init created the Git repository, the project is already a local git repository. Its first commit covers the whole directory (including any files that were in place), and the project has no configured remote. There is no separate step to set up version control. If init printed a Git warning instead, the project has no repository yet. Follow that warning before you edit the project.

### Design System in Use

Read `anchor.json` to find the Design System that this project actually uses. The `designSystem.name` field names it. A WDA Minimal project may also have an `initialization.fallback` record. That record means init first tried to create Spectrum 2 and Lit, could not set them up, and created WDA Minimal instead. The `reason` field records what init reported.

To add Spectrum 2 and Lit later, ask the WDA Skill. The Skill checks the project and asks for consent before it changes anything. Do not run `wda init` again for this.

## Development Commands

Website packages belong to the external package manager of the project, npm or Bun. `package.json` records them. Add or remove a package with that manager, for example `npm install <package>` or `bun add <package>`. `wda check` and `wda build` never install packages.

- `wda check` — Validate project structure, required documents, DTCG tokens, and HTML compliance.
- `wda build` — Compile static deployment artifacts into `dist/`.
- `wda tools fetch` — Restore the embedded TypeScript tool cache for this platform. It takes no arguments.

## Preview

After `wda build` succeeds, preview the project through the AI host and inspect the first viewport. A successful build is not visual acceptance.

- Ask your AI host or browser to open the absolute path to `dist/index.html` and confirm the first viewport.

## Project Structure

- `anchor.json` — WDA settings, the Design System in use, and the initialization fallback record when one applies.
- `package.json` — Website dependencies. The external package manager owns its lockfile.
- `pages/` — Semantic HTML entry points for MPA routes.
- `tokens/` — Design token definitions in W3C DTCG format (`tokens/tokens.json`).
- `docs/` — Core project documentation (`architecture.md`, `design.md`, `glossary.md` for project terms and naming conventions).
- `dist/` — `wda build` produces isolated static output (untracked in Git).
