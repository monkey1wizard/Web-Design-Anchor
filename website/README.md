# Web Design Anchor Project

> Project documentation entry point and getting started instructions.

## About this website

This project is the official Web Design Anchor (WDA) website and a reference implementation. It is written in zh-TW and built with WDA and the Spectrum 2 design system.

- **Audience:** designers who work with an AI coding agent and do not write code, and engineers who arrive from GitHub or a shared link. Both matter equally.
- **Desired action:** the visitor installs WDA and starts a first project.
- **Positioning:** the output is clean and does not lock you in. `wda build` emits a plain static `dist/` with no framework, and the deployed site does not depend on WDA at runtime. The page proves this with real `wda build` output instead of claims.

## Getting Started

This Web Design Anchor (WDA) project uses standards-first HTML5, Modern CSS, and TypeScript.

### Project Initialization and Version Control

This project was initialized by `wda init`. The project is already a local git repository. Its first commit covers the whole directory (including any files that were in place), and the project has no configured remote. There is no separate step to set up version control.

## Development Commands

- `wda check` — Validate project structure, required documents, DTCG tokens, and HTML compliance.
- `wda build` — Compile static deployment artifacts into `dist/`.

## Preview

After `wda build` succeeds, preview the project through the AI host and inspect the first viewport. A successful build is not visual acceptance.

- Ask your AI host or browser to open the absolute path to `dist/index.html` and confirm the first viewport.

## Project Structure

- `wda.json` — WDA project manifest and configuration.
- `pages/` — Semantic HTML entry points for MPA routes.
- `tokens/` — Design token definitions in W3C DTCG format (`tokens/tokens.json`).
- `docs/` — Core project documentation (`architecture.md`, `design.md`, `glossary.md`).
- `dist/` — `wda build` produces isolated static output (untracked in Git).
