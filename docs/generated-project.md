---
title: "Generated Project"
status: stable
updated: 2026-11-12
type: Architecture
description: "The contract for the project that wda init produces: source baseline, language roles, project directory tree, the anchor.json and package.json contract files, the dist/ mapping, and the project document set."
tags: []
---

# Generated Project

The project that `wda init` produces is called a Generated Project. It is a Standard Web Project (defined in `docs/glossary.md`). This specification describes the technical baseline of the project, the role of each source type, the directory structure, the `anchor.json` and `package.json` contract files, the external lockfile duty, the mapping rules between source and `dist/`, the supported scope, and the document set. For command behavior and validation rules, see `docs/commands.md`.

## Source Baseline and Deployment Baseline

| Baseline | Content |
| :--- | :--- |
| Source | HTML5 and semantic HTML, Modern CSS, TypeScript, and native browser capabilities. Web Components are used when the project truly needs component encapsulation |
| Deployment | HTML, CSS, JavaScript, and assets |

A front-end framework is not a default option. The actual complexity of the application decides whether to introduce a framework. A routing need alone is not a sufficient reason. WDA promises to lower the cost of a later migration. It is not a promise of automatic migration, lossless migration, or zero-cost migration.

## Roles of HTML, CSS, and TypeScript

- HTML owns structure and static content.
- CSS owns visual presentation, layout, responsive design, and any state that CSS alone can handle.
- TypeScript owns interactive behavior, state management where it is needed, and calls to browser APIs.

TypeScript must have a real static type check. The transpile step of a bundler must not be counted as a type check.

## Project Directory Tree

### Tree Created by wda init

`wda init` creates only the following paths:

```text
project root/
├── .git/                 Local Git repository, when Git is usable
├── .gitignore            Content is fixed as /dist/
├── README.md
├── anchor.json           WDA project contract
├── package.json          Website dependency declaration
├── docs/
│   ├── architecture.md
│   ├── design.md
│   └── glossary.md
├── pages/
│   └── index.html
└── tokens/
    └── tokens.json
```

This directory structure is the `WDA Minimal` base structure. It is used for offline initialization and when dependency resolution fails. When online initialization succeeds, the created content depends on the Design System and on the dependency installation result. For details, see `docs/commands.md`.

A Generated Project declares its website dependencies only in `package.json`. WDA does not create `deno.jsonc`, `deno.json`, or `deno.lock`. Existing legacy files do not replace either required manifest.

### Directory Paths Created When Needed

The following paths are created only when the project needs them. `wda init` does not create placeholder directories.

```text
components/            Reusable static HTML fragments, or product-semantic components with real reuse
styles/                Shared source stylesheets, when a single stylesheet is not enough
scripts/               TypeScript interaction modules, when behavior logic needs source organization
assets/                Project-owned assets such as images, fonts, and icons
pages/<name>.html      Additional MPA page entry points
docs/development.md    Project-specific workflows that the four WDA commands and the required documents do not cover
```

`wda check` and `wda build` resolve `<wda-include src="...">` relative to the including file. Includes must stay inside the project root and must not form a cycle. Both commands report missing targets, root escapes, and the full chain of a cycle. They check IDs after expansion. Diagnostic locations identify the original source and use 1-based lines and UTF-8 byte columns. CRLF counts as one newline. `wda check` does not write `dist/`.

## Project Contract Files

Every Generated Project has two contract files in its root directory. Both are required.

| File | Role |
| :--- | :--- |
| `anchor.json` | The WDA project contract. It defines the project identity, the technical baseline, the Design System in use, and the initialization fallback record. It is validated against `schemas/anchor.schema.json` |
| `package.json` | The only website dependency declaration. It does not carry WDA metadata |

When either file is missing, `wda check` and `wda build` report a named Error for that file. A missing file is never filled in from another file, and a legacy `wda.json` is never read as a substitute. A project without both files cannot be built. WDA does not convert a legacy project automatically. A successful build updates `wdaVersion` in `anchor.json` and writes `dist/`. Validation and failed builds leave `wdaVersion` unchanged. To use the current format, create a new project with `wda init`, or rewrite the two files by hand.

## `anchor.json`

`anchor.json` is the core of the project contract. It defines the project identity, the technical baseline, and the Design System. Both `wda check` and `wda build` read this file first.

### Why the Schema Is Strict

`anchor.json` allows only explicitly defined fields, so that humans, AI, and Core can all understand the file. If custom content were added, other readers could not determine its meaning or purpose.

- There is no `metadata` field, no custom extension area, and no arbitrary top-level field.
- An unknown field is always an Error, not a Warning.
- The file content does not include a `$schema` field.
- The names `projectFormatVersion`, `deploymentDirectory`, `projectDescription`, and `wda.json` are outdated or non-contract names and are not used.

### Fields

| Field | Content | When it is written and who may change it |
| :--- | :--- | :--- |
| `projectName` | The default is `Web Design Anchor Project`. It is used to compose the page title | `init` writes the default value, and the project changes it afterward |
| `projectVersion` | The local time of the host at the moment `init` runs, with seconds and the UTC offset, without milliseconds. It is not incremented automatically afterward | Written once, by `init` only |
| `wdaVersion` | The WDA version used by the most recent successful `init` or `build` | `init` writes it, and every successful `build` updates it |
| `pageArchitecture` | The allowed values are `mpa` (multi-page architecture) and `spa` (single-page architecture). The default is `mpa` | `init` writes the default value, and the project changes it afterward |
| `browserBaseline` | The only allowed value is `baseline-widely-available` (Baseline Widely Available), which is also the default | Written by `init` only |
| `accessibilityBaseline` | The only allowed value is `wcag-2.2-aa` (WCAG 2.2 AA), which is also the default | Written by `init` only |
| `designSystem` | An object with the required `name` and `version` fields. The default is the latest stable Spectrum. When online resolution succeeds, `version` is the exact semver version number. When the fallback to `WDA Minimal` applies, `version` is the version built into that WDA release | Written by `init`. After verified Spectrum integration, the Skill updates it to the actual system. A Lit-only addition leaves it unchanged |
| `initialization.fallback` | Optional. Present only when `wda init` created `WDA Minimal` instead of the intended Design System. It holds `requestedDesignSystem`, the Design System that init tried first, and `reason`, a non-empty text that states why the fallback happened. Absent when no fallback occurred | Written by `init`. After verified Spectrum integration, the Skill removes the `initialization` wrapper. A Lit-only addition, refusal, or failed integration leaves it unchanged |

Whether the project may change a field depends on the stability layer of that field. For details, see "Stability Layers" in `docs/architecture.md`.

### How the Schema Is Stored

The canonical schema is `schemas/anchor.schema.json`, kept under version control. The binary embeds the same content directly, and the repository does not commit an automatically generated copy. The binary also validates the cross-field rules that JSON Schema cannot express, and it does not redefine fields that the schema already declares. No schema for the earlier `wda.json` format exists in the product, and the validator never reads that format.

### How wdaVersion Is Updated

`wdaVersion` records the WDA version that most recently processed this project successfully. It is not a project format version. Do not use this field to decide format compatibility. For the format compatibility rules, see "Version Compatibility" in `docs/architecture.md`.

| Command result | Handling of `wdaVersion` |
| :--- | :--- |
| `wda init` succeeds | Written |
| `wda build` succeeds | Updated in `anchor.json` |
| `wda build` fails | Unchanged |
| `wda check` | Unchanged |

## `package.json` and the External Lockfile

`package.json` is the only website dependency declaration. The external package manager, npm or Bun, owns its changes. WDA reads it and does not duplicate any dependency in `anchor.json`.

| Field | Content | Who writes it |
| :--- | :--- | :--- |
| `dependencies` | Optional object. Each value is a string version. A project with no website dependencies may omit it or leave it empty | The external manager, after an install. `init` writes the Spectrum and Lit entries only when online resolution succeeds |
| `packageManager` | Optional. A manager name, `@`, and an exact version, such as `npm@10.9.2`. Successful Spectrum init records the actual manager and exact version, even when only one is usable. Initial Minimal omits it | `init`, on successful Spectrum installation |
| `name`, `private`, `type` | Written by `init` as the project package identity. WDA does not check their values | `init` writes them, and the project may change them |

The root of `package.json` must be an object. WDA reports a named Error for any other shape, for a dependency value that is not a string, and for a `packageManager` value that does not follow the rule above.

The Manager Lockfile is the lockfile that the external package manager writes, such as `package-lock.json` for npm or `bun.lock` for Bun. The manager owns it. WDA never writes, rewrites, or migrates a Manager Lockfile. `wda check` and `wda build` only read installed content, and `wda tools fetch` does not change `anchor.json` or `package.json`.

## `dist/`

### The dist/ Directory Tree

`dist/` is the only deployment artifact directory. The following example shows the directory structure after a build:

```text
dist/
├── index.html
├── about.html
├── styles/
│   ├── main.css
│   └── tokens.css
├── scripts/
│   └── main.js
└── assets/
    └── images/
        └── hero.png
```

### Mapping Rules from Source to dist/

The path mapping from source to `dist/` is fixed and has no configuration option:

| Source path | `dist/` path | Rule |
| :--- | :--- | :--- |
| `pages/<rel>.html` | `dist/<rel>.html` | The file extension is kept. No nested `index.html` directory is created |
| `styles/<rel>.css` | `dist/styles/<rel>.css` | Mapped directly to the output |
| `scripts/<rel>.ts` | `dist/scripts/<rel>.js` | Type-checked first, then transpiled |
| `assets/<rel>` | `dist/assets/<rel>` | Only files that a page or stylesheet actually references are copied |
| `tokens/tokens.json` | `dist/styles/tokens.css` | Transpiled and produced during the build |

### Invariants of dist/

- `dist/` is a fixed output directory name and has no custom setting.
- Every build rebuilds from source and keeps no output from the previous build.
- `dist/` must deploy and run on its own in an environment that has no WDA, no source code, and no development tools.
- Derived artifacts may exist only in `dist/` and must not be written back to source. For example, the CSS transpiled from `tokens/tokens.json` exists only as `dist/styles/tokens.css`, and the source must not contain `styles/tokens.css`.
- The source side and `dist/` are two independent directory trees. Within one directory tree, a file identifier may exist in only one location.

### Items Not Written to dist/

The build does not write `components/`, `anchor.json`, `package.json`, the Manager Lockfile, `README.md`, `docs/`, `.gitignore`, hidden dotfiles, or `.env*` environment files to `dist/`. `dist/` is not part of source, and the build does not scan that directory.

## Non-Commitments

- **`file://` protocol**: WDA promises that `dist/` works when it is deployed on a standard static HTTP(S) host. WDA does not guarantee that it runs when opened directly on a local machine through `file://`. If `wda build` detects an ES module, `fetch`, or another known feature that cannot work through `file://`, it reminds the user to use an HTTP host. Receiving no reminder does not mean that WDA guarantees it works.
- **Clean URL**: WDA does not promise to remove file extensions, and it does not create nested `index.html` files to simulate front-end routing. The deployment host handles URL rewrite and fallback routing.
- Framework migration limits: for the related promise, see "Source Baseline and Deployment Baseline".

## Project Document Set

This section describes how the documents of a Generated Project divide their work. For how `wda check` checks documents, see "Rule Module: Documents" in `docs/commands.md`.

### README.md Is the Only Entry Point

`README.md` is the only reading entry of a Generated Project. Other specifications and documents are placed in `docs/` according to their duty.

### The Four Required Documents

The project has four required documents: `README.md`, `docs/architecture.md`, `docs/design.md`, and `docs/glossary.md`.

- `README.md` is the project entry. It explains the purpose and background of the project.
- `docs/architecture.md` explains the current composition of the site, the technical baseline, and the role of each source path.
- `docs/design.md` explains the Design System settings that the project adopts.
- `docs/glossary.md` explains the project terms and naming conventions.

`docs/naming.md` is not a supported legacy format, and the validator provides no compatibility handling for it. During migration, the project must rename the file to `docs/glossary.md` by hand.

### development.md Is a Conditional Document

`docs/development.md` is not a required document, and `wda init` does not create it. Create this file only when the project has a special workflow that neither the four WDA commands nor the four required documents cover. Examples are project-specific development commands, tool prerequisites, automation flows, or manual acceptance steps. `anchor.json` is the authoritative source for the browser baseline and the accessibility baseline. The project may summarize these two baselines in `docs/architecture.md` or `docs/design.md`, and it does not need to create this file for that reason.

### The Two Sections of the architecture.md Template and the Zero-Redundancy Principle

`docs/architecture.md` must be drafted from `docs/templates/generated-project/architecture.md`. The template has two main sections. `Created by wda init` lists the directories and files that `wda init` actually creates. `Allowed when needed` lists the optional extension paths and when to create them. The template also explains that `wda init` does not create placeholder directories. The architecture document does not restate the content of `README.md`. The project purpose and background are recorded only in `README.md`.
