---
title: "Glossary"
status: stable
updated: 2026-11-12
type: Reference
description: "Core terms used by the WDA specification and architecture documents. Each entry gives a definition only, not implementation rules."
tags: []
---

# Glossary

This document defines the terms used by the WDA specification and architecture documents. For procedures and validation logic, see the dedicated documents.

## Roles and Entities

| Term | Definition |
| :--- | :--- |
| Designer | The target user of WDA. A Designer states design and feature requirements in natural language and does not write code directly |
| AI host | The external environment that runs the AI conversation and loads the Skill, for example Claude Code. WDA does not trust this environment |
| Skill | The AI's operating manual and workflow guide. The files are in `skills/wda/` and ship with each release. For the specification, see `docs/skill-spec.md` |
| Core | The WDA CLI tool itself. It is a single Cargo package that contains the `wda_core` library (lib) and the `wda` executable (bin) |
| Generated Project | The project repository that `wda init` creates and that the Designer and the AI maintain over the long term. For the contract, see `docs/generated-project.md` |
| Standard Web Project | A web project that uses HTML5, Modern CSS, and TypeScript and is not tied to any front-end framework. A Generated Project is this kind of project |
| `dist/` | The only deployment directory, produced by `wda build`. It contains plain static files that work on their own |
| Build Only | The architecture commitment that WDA only builds static files that can be deployed on their own. It provides no hosting or deployment service |
| Process Contract | The capability agreement between the Skill and the AI host. The host only needs to run native binaries and capture stdout, stderr, and the exit status code |
| Lit | A package for building Web Components. Spectrum installs it as a website dependency, and the Skill may add it to a `WDA Minimal` project later |

## Stability Layers

| Term | Definition |
| :--- | :--- |
| Invariant | A long-term commitment that no version plans to change, for example that `dist/` is the only deployment artifact |
| Default | A project default value. A project can customize it in `anchor.json` |
| Tool Form | The execution form of the Core tool. Currently this is a single standalone CLI executable |
| Tooling Choice | The technology used to implement the Tool Form, for example Rust, Rolldown, and the embedded TypeScript tool. WDA may replace it in a later release without affecting the project contract |
| Implementation Detail | The code organization and implementation inside Core. It is not directly visible from outside |

## Project Contract

| Term | Definition |
| :--- | :--- |
| `anchor.json` | The WDA metadata file in the project root. It records the WDA settings, the actual Design System, and the initialization fallback record. It is validated against `schemas/anchor.schema.json` |
| `package.json` | The only website dependency declaration in a Generated Project. The external package manager, npm or Bun, owns its dependency changes. A successful Spectrum init records the chosen manager and exact version in `packageManager`, even when only one is usable. Initial Minimal omits the field |
| External Package Manager | The npm or Bun tool that the Designer's environment provides. It owns `package.json` changes, its lockfile, version solving, transitive versions, and installation. WDA never installs Node.js or Bun |
| Manager Lockfile | The lockfile that the external package manager writes. The manager owns it. WDA does not write, rewrite, or migrate it, and `wda check` and `wda build` only read installed content |
| Tool Cache | The local copy of the single TypeScript tool that a WDA release embeds, kept for the current platform. `wda tools fetch` restores it without arguments. It does not change `anchor.json` or `package.json` |
| Fallback | The `initialization.fallback` record in `anchor.json`. It holds the `requestedDesignSystem` and the `reason` when `wda init` creates `WDA Minimal` instead of the intended system. After verified Spectrum integration, the Skill removes its `initialization` wrapper and reruns `wda check`. Lit-only additions preserve it |
| `wdaVersion` | An `anchor.json` field. It records the WDA version that most recently processed this project successfully. It is not used to decide format compatibility |
| Source Baseline | The range of technology that the project source code may use: HTML5 and semantic markup, Modern CSS, TypeScript, and native browser capabilities |
| Deployment Baseline | The file types that the `dist/` artifact may contain: HTML, CSS, JavaScript, and static assets |
| Baseline Widely Available | The default browser compatibility baseline. It refers to web platform features that the major browser engines have supported for at least 30 months |
| WCAG 2.2 AA | The accessibility standard that a project adopts by default for web content |
| MPA | Multi-Page Application. Each page uses its own HTML file, and the browser's native navigation switches between pages. This is the default architecture of a WDA project |
| SPA | Single-Page Application. It switches views dynamically through client-side routing. WDA allows a project to adopt it but does not make it the default |
| Strict Schema | `anchor.json` accepts only declared fields. An unknown field is reported as an Error. There is no custom extension area and no metadata field |
| Clean URL | A path form that removes the file extension from the URL or simulates routes with nested `index.html` directories. WDA does not support this form. The deployment host handles it |

## Build

| Term | Definition |
| :--- | :--- |
| `wda-include` | The only custom build-time HTML tag that WDA supports. The build expands it into the component content, and it is not written to `dist/` |
| Build Staging Directory | The fresh temporary directory that `wda build` fills with the bundler output before it commits the listed entries, chunks, and static assets to `dist/`. It is not part of the project and is removed when the build ends |
| Collision Rule | The pre-flight safety check of `wda init`. If a path that the command plans to create already exists, the command refuses to run immediately |
| Rollback Baseline | The first initial commit of the local Git repository that `wda init` creates in the project directory |
| Resolution Layer | The first allowlisted interface where Core branches its logic by Design System: `src/init/resolve.rs` |
| Built-in Asset Layer | The second allowlisted interface where Core branches its logic by Design System. It consists of specified files in `src/init/` and `src/builtins/` |

## Design System

| Term | Definition |
| :--- | :--- |
| Design Token | The smallest design value, referred to by a semantic identifier name, for example a color, a spacing value, or a font size |
| DTCG | The standard JSON format defined by the Design Tokens Community Group. This project uses `tokens/tokens.json` as the only authoritative source |
| `WDA Minimal` | The minimal Design System built into WDA for offline environments. It creates no website dependencies at init, so it works without a network connection or a package manager. The Skill may later add Spectrum and Lit after the user consents |
| Spectrum / Spectrum 2 | The Design System from Adobe. It is the default option when `wda init` queries online and a usable external package manager installs it |
| Adapter | The interface specification that describes how a Design System connects to a WDA project. It consists of seven facets |
| Facet | One facet of the Adapter interface specification. The facets are Token mapping, style publishing and organization, fonts, icon assets, color themes, component dependency model, and package integration |
| Neutrality | The architecture commitment that Core is not tied to a single Design System. A qualifying system must support different dependency models and component models, and each must complete init, check, and build |
| Identity Branch | Logic in Core that identifies or branches on a Design System name. It may appear only in allowlisted files |

## Validation and Diagnostics

| Term | Definition |
| :--- | :--- |
| Diagnostic | One problem record that `wda check` or `wda build` outputs when it checks a project. When available, its source location uses 1-based lines and UTF-8 byte columns |
| Severity | The classification of a Diagnostic: an Error, which blocks the build, or a Warning, which does not block the build |
| Error | A serious problem that means the build result is incorrect or violates the project contract. It blocks the build. When it is reported through `ValidationReport`, the command exit status code is 1 |
| Warning | A problem that does not block the build output but indicates a potential risk and prompts the user. In human output, a Warning is written to stderr |
| ToolFault | The tool error that is returned when an external dependency tool or the file system fails unexpectedly or severely. When a command returns a ToolFault, its exit status code is 2 |
| Exit Status | The status that a CLI command returns after it runs: `0` means success or only Warnings; `1` means the `ValidationReport` contains an Error; `2` means a usage error or a ToolFault |
| Single Validation Seam | The only validation function that Core exposes: `validate_project(root)`. The binary cannot use the individual internal validation rule modules directly |
| Probe Test | A test that confirms whether the behavior or capability of a specific execution environment matches expectations |

## Skill and Acceptance

| Term | Definition |
| :--- | :--- |
| Allowlisted Content | The six kinds of guidance that `SKILL.md` may contain: role boundaries, version comparison steps, decision routing tables, workflow stop conditions, reporting duties, and authority pointers |
| Undertriggering | The case where the AI should load the Skill but does not, and so edits files without following the Core specification |
| Four-Tier Acceptance | The Skill quality check framework. It has L1 specification compliance, L2 boundary protection, L3 behavior evaluation, and L4 deletion test |
| Deletion Test | The L4 acceptance method. A person removes the whole Skill and then confirms that Core and the project documents still let the user operate WDA correctly |
| Authority Pointer | A reference marker in the same paragraph as a Skill rule sentence that points to a document path, for example `docs/`, the section sign `§`, or `ADR-` |
| Craft Floor | The section of the Generated Project design document that sets the minimum quality standard. It lists the basic conditions that a design deliverable must meet |

## Documents

| Term | Definition |
| :--- | :--- |
| Required Documents | The four core documents that a Generated Project must have at initialization: `README.md`, `docs/architecture.md`, `docs/design.md`, and `docs/glossary.md` |
| Conditional Documents | Documents that a Generated Project creates only when a special workflow appears that the other documents do not cover, for example `docs/development.md` |
| Canonical Source | The only source of a given project state or specification. Other documents may only summarize it and must not copy its content |
| `sourceRefs` | An array field in the document front matter. It lists the paths of the canonical sources that the document is based on or derived from |
| Front Matter | The YAML structured metadata at the start of a Markdown file, enclosed by `---` |
| ADR | Architecture Decision Record. It records the background, options, decision, and consequences of a single technical decision |
| Thematic ADR | An ADR document that groups related architecture decisions by topic |
| OKF | Open Knowledge Format. It is the specification basis for the front matter field structure and the document standard of this project |
| restore point | A version-control record point that can be used to restore the project state |
| SEEN | A status marker that means the Designer has seen the current result |
| ACCEPTED | A status marker that means the Designer has explicitly accepted the current result |
