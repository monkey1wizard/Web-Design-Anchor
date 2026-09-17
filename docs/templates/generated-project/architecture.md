---
title: "Generated Project Architecture Template"
status: stable
updated: 2026-08-24
sourceRefs:
  - "ROADMAP.md §2.13"
  - "ROADMAP.md §3.2 G6"
type: Template
description: "Describes how a generated WDA project is organized and how its source paths are used."
tags: []
---

# Architecture

> Describe how the Generated Project is organized, its technical baselines, and the responsibility of each source path.

## Created by wda init

`wda init` creates these project paths:

```text
.
├── .gitignore
├── README.md
├── wda.json
├── docs/
│   ├── architecture.md
│   ├── design.md
│   └── naming.md
├── pages/
│   └── index.html
└── tokens/
    └── tokens.json
```

The source baseline is HTML5, Modern CSS, TypeScript, and native browser capabilities. HTML owns structure and static content; CSS owns presentation, layout, responsive behavior, and CSS-manageable states; TypeScript owns interaction, necessary state, and browser APIs.

The default page architecture is MPA. The project uses semantic HTML and native browser capabilities first. `wda build` produces the independent deployment tree at `dist/`; generated token CSS belongs only in `dist/styles/tokens.css`.

## Allowed when needed

The following paths and patterns are optional. Create them only when the project has the corresponding need. `wda init` does not create any placeholder directories.

```text
components/       Reusable static fragments or genuinely reusable product semantics.
styles/            Shared source styles when one stylesheet is no longer sufficient.
scripts/           TypeScript interaction modules when behavior needs source organization.
assets/            Images, fonts, and icons supplied by the project.
pages/<name>.html  Additional MPA page entry points.
docs/development.md
                  Project-specific commands, prerequisites, automation, or manual
                  verification not covered by the WDA commands or required documents.
```

### Technical baselines

- Keep source framework-neutral and standards-first.
- Use semantic classes and CSS Custom Properties for project styling.
- Use Web Components only when native HTML is insufficient for reusable behavior or encapsulation.
- Use the WDA build-time include primitive only for composition that genuinely needs it; it is not a runtime Web Component and must not remain in `dist/`.
- Keep `tokens/tokens.json` in W3C DTCG format. Build derives CSS Custom Properties into `dist/`; source does not contain generated token CSS.
- Vendor elements and APIs may be used directly when appropriate. Do not add a universal wrapper solely to hide a vendor name.

### Directory responsibilities

- `pages/` contains page entry structure and page metadata, not complex shared CSS or TypeScript.
- `components/` contains reusable fragments or behavior-oriented components without framework-specific requirements.
- `styles/` contains shared presentation rules and semantic CSS Custom Properties.
- `scripts/` contains independent TypeScript modules and browser behavior; it does not become a global application runtime by default.
- `tokens/` contains the project token source of truth.
- `assets/` contains static project assets.
- `docs/` records project-specific architecture, design decisions, naming, and conditional development workflow.

### Deployment boundary

`dist/` is the only deployment output. It is rebuilt from source, contains no secrets, and can be deployed without WDA or the source project. URL rewriting and fallback routing belong to the deployment environment; this template makes no URL-shape guarantee.

When the project outgrows a standards-first source, an engineer may introduce Astro, React, Vue, or another framework. Preserve semantic HTML, CSS tokens and styles, Web Components, static assets, and project documentation where practical; migration is an engineering change, not an automatic transformation contract.
