---
title: "Architecture"
status: stable
updated: 2026-11-12
type: Architecture
description: "Describes the composition of the WDA repository, its architecture rules, and its documentation rules."
tags: []
---

# Architecture

This document describes the composition of the WDA repository, its responsibility boundaries, and its architecture rules. For command behavior, see `docs/commands.md`. For the project that `wda init` produces, see `docs/generated-project.md`. For the Design System and Skill rules, see `docs/design-system.md` and `docs/skill-spec.md`. If a rule in this document conflicts with a dedicated document, the dedicated document prevails.

## Core Principles

### Standards First

WDA prefers Web platform standards. When no suitable standard exists, WDA uses a simple, explicit, and widely used approach. WDA adds a custom mechanism only when standards cannot meet the need, and it adds as little as possible.

### Simple, Explicit, Single Responsibility

Each command, file, and rule is responsible for one thing and is easy to understand. If a new command is responsible for different work, you may add it.

### Readable by Humans and AI

Important information must be understandable to the Designer, the AI, and the Engineer. Structured state may live in `anchor.json`, `tokens.json`, or a lockfile, but the documents must also explain the key state.

### WDA Is a Tool, Not Governance

WDA does not decide the governance process for a project owner. WDA can work with ADRs, but it does not require every decision to be written as an ADR.

### No Dependence on a Single AI Product

WDA must be easy for an AI to use, but it must not depend on a specific AI product, host, or ecosystem.

## System Composition

WDA has three parts: the specification, Core, and the Skill. The three work together, and none of them owns another.

| Physical component | Location / path | Nature and description |
| :--- | :--- | :--- |
| Specification | `docs/` | The only source of the specification. `README.md` is for navigation only. |
| Core | `src/`, `tests/`, `schemas/` | The executable tool itself. A single Cargo package that contains the `wda_core` library and the `wda` binary. |
| Skill | `skills/wda/` | An operating manual that ships with each release. It is not a source of the specification. |

The specification lives in `docs/` because the specification outlives any version of the code. The Skill and the binary are both deliverables, so the Skill does not live in `docs/`.

Core owns locatable source diagnostics, shared include analysis, Token validation and emission, and text or JSON reporting. Lines and UTF-8 byte columns are 1-based. The Skill owns contextual accessibility, fixed CSS and Token suggestions, and voluntary Token layer review. Review does not authorize edits. For the JSON schema and exit behavior, see [JSON Output](commands.md#json-output). For review conditions and consent, see [Skill Specification](skill-spec.md).

## Stability Layers

The WDA rules fall into five layers. The layer decides how a rule changes, who decides the change, and what the change costs. The higher a layer is, the more stable it is.

```text
Invariant              Never changes
Default                A project can customize it in anchor.json
Tool Form              Changes only in a major WDA redesign
Tooling Choice         WDA can replace it in a release; projects are not affected
Implementation Detail  Replaceable at any time; not visible from outside
```

### Invariant

An Invariant is a commitment that cannot change. Changing one of these commitments changes the core definition of the product.

- The deployment artifact is standard HTML, CSS, JavaScript, and assets.
- WDA is not a runtime dependency of the website.
- `dist/` is the only deployment artifact.
- Every Error that would make the build result incorrect blocks the build. For the definition of Error, see Diagnostic Output in `docs/commands.md`.

### Default

A project can set a different value explicitly in `anchor.json`. When the Designer makes no explicit choice, WDA uses the default value and does not ask. For the default values, see the `anchor.json` field table in `docs/generated-project.md`.

### Tool Form

Tool Form describes how Core runs. A change to how Core runs is a major architecture redesign.

- WDA is a standalone CLI.
- The user only needs to put one native binary on PATH. `wda init`, `wda check`, and `wda build` need no Node.js, Bun, Deno, or esbuild on PATH. A project with website packages needs an external package manager, and `wda init` calls it only to install Spectrum and Lit.

### Tooling Choice

Tooling Choice describes how WDA implements the Tool Form. WDA can replace the following tools in a release, but the replacement must not change the project contract, the responsibilities of the four commands, or the build result, and it must not require a project to change.

- The CLI binary is written in Rust.
- Rolldown 1.3.1, through its Rust API, builds the browser ESM output from the installed packages and the project's own pages and scripts.
- One TypeScript native compiler version is embedded in each release. `packaging/toolchain.json` binds its version, six-platform packages, and `sha512` SRI.
- npm or Bun, run by the external package manager, owns `package.json` changes, the manager lockfile, version solving, and installation.

### Implementation Detail

An Implementation Detail is the internal code organization of Core. You can replace it without changing the external contract.

- How the Rust modules are split.
- The rollback implementation of atomic init.

## Scope

WDA only builds. It produces static files that can be deployed on their own. WDA does not provide a hosting or deployment service. The Designer or the AI host can deploy the artifact to a static hosting service such as GitHub Pages, Cloudflare Pages, Netlify, or S3.

Built-in hosting would tie WDA to a cloud service and add maintenance work for accounts, keys, billing, and provider APIs. This work does not affect the standard Web artifact, so it is outside the scope of WDA.

WDA permanently excludes the following features:

- Clean URL, URL rewrite, and fallback routing. The deployment environment is responsible for these features.
- A preview server. Core never includes a preview server.

## Project Directory Structure

The following `src/` tree follows the current output of `git ls-files src`. The other entries list the top-level directories and files that the repository has now.

```text
web-design-anchor/
├── .github/                 CI, release workflows, and GitHub instructions
├── AGENTS.md                Project agent instructions and the authoritative check commands
├── CHANGELOG.md             Version change log
├── examples/                Example projects
├── tools/writing/           Writing quality checker for documents
├── Cargo.toml               Single package: the wda_core library and the wda binary
├── Cargo.lock               Rust dependency lock file
├── schemas/                 Schemas for the project manifest files
├── tests/                   Integration tests and test cases
├── docs/                    Project specification and reference documents
├── skills/wda/              Skill that ships with each release
├── packaging/               Release and installation files
├── src/
│   ├── build/               wda build: include, layout, tokens, scripts, report
│   ├── builtins/            Embedded WDA Minimal and Spectrum 2 assets
│   │   ├── spectrum_two.rs
│   │   ├── spectrum_two/pages/index.html
│   │   ├── spectrum_two/scripts/main.ts
│   │   ├── spectrum_two/tokens/tokens.json
│   │   ├── wda_minimal.rs
│   │   ├── wda_minimal/docs/design.md
│   │   ├── wda_minimal/pages/index.html
│   │   └── wda_minimal/tokens/tokens.json
│   ├── init/                 wda init: documents, collision scan, and the manager resolution seam
│   ├── validation/           Project rule validation
│   │   ├── build_output.rs   dist/ artifact checks
│   │   ├── documents.rs      Required document and front matter checks
│   │   ├── html.rs            HTML source checks
│   │   ├── project_contract.rs anchor.json, package.json, and schema checks
│   │   ├── tokens.rs          DTCG tokens checks
│   │   └── toolchain.rs       Toolchain checks
│   ├── builtins/mod.rs
│   ├── cli.rs                 Argument parsing
│   ├── codes.rs               Diagnostic codes
│   ├── diagnostics.rs         Diagnostic data model, ordering, and single-line output
│   ├── lib.rs                 External integration interface
│   ├── main.rs                stdout, stderr, and exit status
│   ├── tool_probe.rs          Git and tool launch probes
│   └── tools.rs               Embedded TypeScript descriptor, cache, and wda tools fetch
├── plugin.json                Agent Plugins declaration
├── README.md                  Project overview
├── SECURITY.md                Security model
├── NOTICE                     Third-party attribution
└── LICENSE                    License terms
```

`src/builtins/spectrum_two/` holds the Spectrum 2 starter page, tokens, and `scripts/main.ts`. `src/init/mod.rs` uses these assets to produce the initial project. The four validation modules are used through the shared interface that `src/lib.rs` provides, and `src/main.rs` contains no validation rules. This lets `wda check` and `wda build` share one validation engine.

## Version Compatibility

### Future Format Versions

The current schema has no `schemaVersion` field. WDA keeps backward compatibility with existing project formats first. A future breaking change to a format must define an explicit `schemaVersion`, and it must not apply silently during `check` or `build`. A new WDA version can build a project in a compatible older format, but it does not migrate the project structure automatically. A new implementation must keep the validators and dependency-handling branches for the older formats that WDA promises to support.

### Backward Compatibility and No Silent Migration

The dependency declaration format must stay backward compatible, and WDA must not rewrite project files silently.

- The current format declares WDA settings in `anchor.json` and website dependencies in `package.json`. A project that uses an unrecognized configuration has an unsupported format, and Core reports an Error.
- Both manifests are required. Old `wda.json`, `deno.jsonc`, and `deno.json` files cannot substitute for either manifest. Core reports an Error for a missing manifest and does not migrate old formats automatically. Legacy files may coexist with both current manifests.
- `wda check` does not write project files. A successful `wda build` updates `wdaVersion` in `anchor.json` and writes `dist/`. A failed build leaves `wdaVersion` unchanged. Neither command migrates the project format.

### Four-Level Validation Semantics

```text
Current format           → OK, for example a project uses the current configuration
Supported legacy format  → Warning, for example an existing project in a supported earlier format
Deprecated legacy format → A more explicit Warning
Unsupported format       → Error, for example an unrecognized format or dual configuration
```

### One-Time Exception (2026-11-06)

On 2026-11-06, the Owner ruled a one-time exception to the compatibility and Tooling Choice rules. Because the Owner is the only user of WDA at this stage, this migration is exempt from backward compatibility obligations and does not need to retain legacy validators or earlier dependency branches. The replacement of toolchain choices may require a project to change, may alter the build result layout in `dist/`, and waives the rule that projects are not affected. This exception applies only to the 2026-11-06 toolchain migration and does not extend to later changes.

## Dependency Model

Core handles dependencies under the following architecture rules:

- `package.json` is the only website dependency source, and `anchor.json` does not duplicate it.
- The external package manager, npm or Bun, owns `package.json` changes, its lockfile, version solving, transitive versions, and installation. WDA does not solve npm versions and does not keep a CDN mirror or a vendor directory.
- `wda check` and `wda build` read only installed package content. They never install, update, or rewrite packages, and they never use the network for packages.
- Website modules are not loaded from a remote CDN. Remote module imports are reported as diagnostics.
- `wda init` runs the manager only for the Spectrum and Lit install. Package install runs third-party code, so it is a separate step from `check` and `build`.
- WDA states the lifecycle script behavior of the external manager and does not promise that its side effects can be reverted.
- Read-only commands must not migrate or rewrite dependency declarations.

For the dependency behavior of each command, see `docs/commands.md`.

## Design System Boundary

The default Design System is the latest stable Spectrum. When a usable manager is available online, `wda init` installs Spectrum and Lit through that manager, and the manager records the exact version. When the network is unavailable, no usable manager is found, or resolution or installation fails, `wda init` produces the built-in `WDA Minimal` and explains the reason. A local package cache never switches init to Spectrum. `anchor.json` records the actual Design System, and on fallback it also records the originally intended system and the reason. The canonical source of tokens is `tokens/tokens.json`, and the derived CSS exists only in `dist/`.

The name and identity of a Design System may appear only in the resolution seam `src/init/` and the built-in asset layer `src/builtins/`. Other Core code must not branch on Design System identity. `IDENTITY_SEAM_FILES` in `tests/neutrality.rs` is the only allowlist. For the full specification, see `docs/design-system.md`.

## Skill Boundary

The Skill is the AI's operating manual. It is not a source of the specification and not a second generator. Core and the `anchor.json` contract must not assume that only the Skill operates WDA. WDA provides one Skill only, named `wda`, at `skills/wda/`, with the same version as the release it belongs to. The Skill assumes that the host can run a native binary and read stdout, stderr, and the exit status. For the full specification, see `docs/skill-spec.md`.

## Web Preview

### Core Provides No Preview

The AI host is responsible for preview. The AI host can use a WebView, an Artifact, or an external browser to show `dist/`. The CLI is not responsible for web preview and does not include a preview server.

A preview server would add maintenance work for a resident process, a port, and an HTTP server, and it does not fit the role of a standalone CLI. If a preview feature that works across AI hosts becomes necessary, a plugin or a design-tool integration should provide it.

### Three Baseline Rules

Not every AI host can display a web page. The following principles keep a Designer from accepting a visual result without seeing the change:

1. After `wda build` succeeds, it must print the absolute path of `dist/`, so the AI host can get the artifact.
2. When the AI host has no built-in preview, it must state that preview is limited, and it must not declare a visual change complete.
3. A change that the Designer has not seen must not count as accepted.

### Reversible Baseline

A change that nobody has seen does not count as accepted. So, when you restore the previous accepted state, you do not return to a version that the Designer has not seen.

When Git is available, the first commit of the project's local Git repository provides the restore baseline, created by `wda init` or by the user after installing Git later. Without Git, there is no restore baseline, and an AI host checkpoint, a file snapshot, or a manual restore can only be auxiliary.

## Development and Verification

The authoritative-check fence in `AGENTS.md` names two authoritative commands:

```sh
cargo test
deno run --cached-only --frozen --allow-read --allow-env --allow-sys --config tools/writing/deno.json tools/writing/check.ts --required
```

The writing checker needs Deno, and the authoritative writing check runs under it. Product `wda init`, `wda check`, and `wda build` do not need Deno. Tests marked `#[ignore]` do not run by default. They include the probes that need a real package manager or the real network, and their results do not gate release by themselves.

The writing checker for required documents lives in `tools/writing/`. Before first use, install the checker dependencies as `AGENTS.md` instructs.

## Documentation Rules for This Repository

The writing rules in this section apply to the required document set. The other rules in this section apply only to `docs/` in this repository. The `wda check` command validates the document rules for a project that `wda init` produces. For those rules, see Rule Module: Documents in `docs/commands.md`.

### Writing Rules

Documents give priority to accuracy, then clarity, then plain language. When requirements conflict, keep accuracy first and clarity second.

The required document set contains the following paths:

- en-US: `README.md`, `SECURITY.md`, `docs/*.md`, `docs/adr/*.md`, and `docs/design-systems/*.md`.
- en-US: `skills/wda/SKILL.md` and `skills/wda/references/*.md`.
- en-US: `docs/templates/generated-project/*.md` and `CHANGELOG.md`.
- en-US: `src/builtins/wda_minimal/docs/design.md`.
- zh-TW: `docs/i18n/zh-TW/*.zh-TW.md`, `AGENTS.md`, and `.dev/project.md`.
- ja: `docs/i18n/ja/*.ja.md`.

The checker lives in `tools/writing/`. For the first installation, run the following command from the repository root:

```sh
deno install --config tools/writing/deno.json
```

To check the required documents, run the following command from the repository root:

```sh
deno run --cached-only --frozen --allow-read --allow-env --allow-sys --config tools/writing/deno.json tools/writing/check.ts --required
```

Findings from `prh` and `no-unmatched-pair` are hard errors for zh-TW and ja. Findings from `sentence-length` and `write-good` are advisory for zh-TW. For ja, `sentence-length` is advisory. When you fix a zh-TW document, `zhtw-mcp` gives advice only and is not an authoritative check gate. When a project-specified term conflicts with a `zhtw-mcp` suggestion, the project-specified term prevails.

Every fix must preserve the facts, numbers, conditions, scope, and uncertainty. When you fix a document, check the invariants before and after the edit. If a needed edit would change the meaning, stop and report it.

Do not rewrite the Status History section or its content in an ADR. The checker still checks that section, but it lists the findings there as advisory only.

### Multilingual Policy

The canonical language of this repository is English. The canonical English files are `README.md`, `SECURITY.md`, the six contract files `docs/architecture.md`, `docs/commands.md`, `docs/design-system.md`, `docs/generated-project.md`, `docs/glossary.md`, and `docs/skill-spec.md`, the two files in `docs/design-systems/`, the six ADR files in `docs/adr/`, and `skills/wda/SKILL.md` with `skills/wda/references/*.md`. The files in `docs/templates/generated-project/` stay en-US, as before.

A translation lives at `docs/i18n/<lang>/<name>.<lang>.md`. The zh-TW set is closed and has exactly ten files:

- `docs/i18n/zh-TW/README.zh-TW.md`
- `docs/i18n/zh-TW/SECURITY.zh-TW.md`
- `docs/i18n/zh-TW/architecture.zh-TW.md`
- `docs/i18n/zh-TW/commands.zh-TW.md`
- `docs/i18n/zh-TW/design-system.zh-TW.md`
- `docs/i18n/zh-TW/generated-project.zh-TW.md`
- `docs/i18n/zh-TW/glossary.zh-TW.md`
- `docs/i18n/zh-TW/skill-spec.zh-TW.md`
- `docs/i18n/zh-TW/spectrum-adapter.zh-TW.md`
- `docs/i18n/zh-TW/wda-minimal.zh-TW.md`

The ja set is also closed and has exactly ten files:

- `docs/i18n/ja/README.ja.md`
- `docs/i18n/ja/SECURITY.ja.md`
- `docs/i18n/ja/architecture.ja.md`
- `docs/i18n/ja/commands.ja.md`
- `docs/i18n/ja/design-system.ja.md`
- `docs/i18n/ja/generated-project.ja.md`
- `docs/i18n/ja/glossary.ja.md`
- `docs/i18n/ja/skill-spec.ja.md`
- `docs/i18n/ja/spectrum-adapter.ja.md`
- `docs/i18n/ja/wda-minimal.ja.md`

Each translation carries the six required front matter keys and four provenance keys:

| Key | Content |
| :--- | :--- |
| `source` | The repository-relative path of the canonical file, for example `docs/architecture.md`. |
| `lang` | The language tag of the translation, for example `zh-TW` or `ja`. |
| `source_sha256` | The SHA-256 of the canonical file after line endings are normalized to LF, in lowercase hexadecimal. |
| `translated_at` | The date of the translation in `YYYY-MM-DD` format. |

The freshness rule: `tools/writing/check.ts --required` derives the source path from the file name of the translation. It fails when `source_sha256` differs from the SHA-256 of the LF-normalized canonical file. `EXPECTED_DOCS_MD_FILES` in `tests/docs.rs` fixes the set of files under `docs/`.

The tag in `docs/i18n/<lang>/<name>.<lang>.md` is the shortest BCP 47 tag that identifies the written variety.

ADRs are English-only and have no translation. The h2 headings of an ADR are `## Context`, `## Options`, `## Decision`, `## Consequences`, and `## Status History`.

### Required Front Matter Keys

Every Markdown file under `docs/` must contain the following six keys:

| Key | Content |
| :--- | :--- |
| `title` | Must not be empty. |
| `status` | Use `draft`, `stable`, or `deprecated`. |
| `updated` | Use the `YYYY-MM-DD` format. Update it when the content has a semantic change. |
| `type` | Use the vocabulary of this project: `Architecture`, `Reference`, `Guide`, `ADR`, `Template`, or `Checklist`. |
| `description` | Use a short, single-sentence description. |
| `tags` | Use an array. It can be empty. |

`type`, `description`, and `tags` follow the definitions in the SPEC.md of the open-knowledge-format (OKF). OKF defines `type` as a required free-form string, so the six values above are a convention of this project, not an OKF vocabulary. OKF defines only the three values of `status`.

### Status Vocabulary

| Value | Meaning |
| :--- | :--- |
| `draft` | The document is still being edited and is not a formal basis. |
| `stable` | The document records the current rules. |
| `deprecated` | Another document has replaced this content. The document stays for historical reference. |

### When sourceRefs Is Required

When a document derives from another canonical source, it must fill in `sourceRefs`. Each item uses a path or a section relative to the repository root. This rule does not apply to `README.md` or to the `prefile_*` files in the root directory. An ADR never fills in `sourceRefs`. The next section gives the reason.

### ADRs Are First-Hand Sources

Each ADR records one lasting architecture concern and is the first-hand source for that concern. So an ADR does not fill in `sourceRefs`, and its body does not refer to other ADRs, architecture documents, or `README.md`. A referenced document can be rewritten, moved, or deleted, and that breaks the reference. When an ADR needs an outside rule to be understood, it should summarize the rule instead of copying it word for word or leaving a reference that breaks easily.

An ADR can refer directly to code and physical files, for example a `src/` module, a `tests/` fixture, or the `NOTICE` file in the repository root. These paths point to files that exist, not to abstract documents.

### Admission Criteria

A new ADR must meet both of the following criteria:

1. The rejected alternative was implemented, scheduled, or used as a default.
2. The measurement that supports the decision must name a specific number, a failure message, or a premise that was overturned.

Current operating rules stay in the contract document they belong to and do not get a separate ADR.

### Numbering and File Names

The display number uses the `ADR-NN` format. The file name uses the `docs/adr/NN-<kebab-case-slug>.md` format. Both use two digits, with a leading zero when the number has fewer than two digits.

### Fixed Five Sections

Each ADR contains the following five sections in order. When no comparable alternative exists, there is no decision, and you do not need to write an ADR.

| Section | Content |
| :--- | :--- |
| Context | Describes the background of the problem and the constraints at the time. |
| Options | Lists at least two options and explains the cost and trade-offs of each. |
| Decision | Records the choice explicitly. |
| Consequences | Describes the benefits, the trade-offs given up, and the effect on other parts of the system. |
| Status History | Records dates and single-sentence notes to track replacement or revision. |

The `status` field in the YAML front matter records the current status. When another ADR replaces an ADR, set the status to `deprecated` and record the replacing source in the Status History.

Each consolidated ADR keeps, in order and word for word, all the status-history entries of the ADRs that it absorbed, and it appends one consolidation entry dated 2026-09-21 at the end. The sources of the consolidation entry are listed in `absorbs` in the front matter. The body does not write `ADR-<number>`. This is a one-time exception to the append-only rule at the single-file level. Git history still keeps the exact file archive.

On 2026-10-22, the Owner ruled a second one-time exception: the status-history entries of the ADRs are translated to English with the same dates and the same meaning, and each ADR appends one entry that states it was translated to English and that the decision is unchanged. Git history still keeps the exact file archive.

### Thematic Consolidation and Mapping Table

This repository consolidated 26 atomic decisions into 6 thematic ADRs. Each ADR records one lasting architecture concern. The mapping is as follows:

| New number | Theme | Absorbed old decisions (absorbs) | File name |
| :--- | :--- | :--- | :--- |
| ADR-01 | Product scope and output boundaries | 001, 002, 016, 017, 028 | `docs/adr/01-product-scope-and-output-boundaries.md` |
| ADR-02 | Authoring, dependencies, and toolchain | 006, 011, 013 | `docs/adr/02-authoring-dependencies-and-toolchain.md` |
| ADR-03 | Build, validation, and diagnostics | 007, 010, 012, 015, 018, 027 | `docs/adr/03-build-validation-and-diagnostics.md` |
| ADR-04 | Design System, tokens, and neutrality | 008, 014, 019, 024, 025, 031 | `docs/adr/04-design-systems-tokens-and-neutrality.md` |
| ADR-05 | Generated Project lifecycle and reversibility | 023, 029, 030 | `docs/adr/05-generated-project-lifecycle-and-reversibility.md` |
| ADR-06 | AI Skill boundaries and evaluation | 003, 022, 026 | `docs/adr/06-ai-skill-boundaries-and-evaluation.md` |

### Line Endings and Encoding

The Git repository uses LF line endings and UTF-8 encoding without a BOM. If the local setting is `core.autocrlf=true`, the CRLF that you see after checkout does not mean that the commit uses CRLF. When you edit a file, keep its existing line endings. Do not rewrite a whole file only to convert line endings.

## Retired ADR Numbers

The following numbers are retired. They will not be reassigned:

| Number | Reason |
| :--- | :--- |
| ADR-004 | The decision not to provide hosting is now part of Scope in this document. |
| ADR-005 | The document structure rules for a project that `wda init` produces are now part of `docs/generated-project.md`. |
| ADR-009 | Preview by the AI host and the three baseline rules are now part of Web Preview in this document. |
| ADR-020 | Deleted on 2026-10-13. It has no current location. |
| ADR-021 | The template rules for the architecture document of a project that `wda init` produces are now part of `docs/generated-project.md` and `docs/templates/generated-project/`. |
