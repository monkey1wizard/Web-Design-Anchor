---
title: "Generated Project Architecture Template"
status: stable
updated: 2026-10-15
type: Template
description: "Describes how a generated WDA project is organized and how its source paths are used."
tags: []
---

# Architecture

> This document describes the Generated Project structure, technical baselines, and the responsibility of each source path.

## Created by wda init

`wda init` creates these project paths:

```text
.
├── .git/                    (present when Git is usable)
├── .gitignore
├── README.md
├── anchor.json
├── package.json
├── docs/
│   ├── architecture.md
│   ├── design.md
│   └── glossary.md
├── pages/
│   └── index.html
└── tokens/
    └── tokens.json
```

`anchor.json` and `package.json` are written for every project. `scripts/main.ts` is written only for the Spectrum 2 path. A WDA Minimal project has no website dependencies, so its `package.json` has an empty `dependencies` object and no `packageManager` field.

The source baseline is HTML5, Modern CSS, TypeScript, and native browser capabilities. HTML owns structure and static content. CSS owns presentation, layout, responsive behavior, and CSS-manageable states. TypeScript owns interaction, necessary state, and browser APIs.

The default page architecture is MPA. The project leads with semantic HTML and native browser capabilities. `wda build` produces the independent deployment tree at `dist/`; generated token CSS appears only in `dist/styles/tokens.css`.

## Manifests, packages, and tools

- `anchor.json` owns the WDA settings, the Design System that is actually in use, and the `initialization.fallback` record. It does not declare dependency versions.
- `package.json` is the only website dependency source. When `wda init` installs Spectrum 2 packages with a manager, the `packageManager` field records that manager name and exact version.
- The external package manager, npm or Bun, installs and updates website packages and owns the lockfile. Add or remove a package with that manager, for example `npm install <package>` or `bun add <package>`. WDA never writes, rewrites, or migrates a manager lockfile, and no lockfile exists until the first external install.
- `wda check` and `wda build` never install website packages. When `package.json` declares a package that is not installed, `wda check` reports a Warning that names the package.
- `wda tools fetch` restores the embedded TypeScript tool cache for the current platform. It takes no arguments and does not change `anchor.json` or `package.json`.

When the requested Design System cannot be set up during `wda init`, init creates a complete WDA Minimal project and states the reason it reported. Init then records the requested system and that reason in the `initialization.fallback` field of `anchor.json`. The Skill can add Spectrum 2 and Lit to the project later. It checks the project first, asks for consent, and removes the fallback record only after the integration verifies. Do not run `wda init` again for this purpose.

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
- Projects may use vendor elements and APIs directly when appropriate. Do not add a universal wrapper solely to hide a vendor name.

### Directory responsibilities

- `pages/` contains page entry structure and page metadata, not complex shared CSS or TypeScript.
- `components/` contains reusable fragments or behavior-oriented components without framework-specific requirements.
- `styles/` contains shared presentation rules and semantic CSS Custom Properties.
- `scripts/` contains independent TypeScript modules and browser behavior; it does not become a global application runtime by default.
- `tokens/` contains the project token source of truth.
- `assets/` contains static project assets.
- `docs/` records project-specific architecture, design decisions, project terms and naming conventions, and conditional development workflow.

### Custom Elements and Lit

Lit is the standard library for custom elements when the project uses them. A Spectrum 2 project declares Lit in `package.json`. A WDA Minimal project has no website dependencies, so Lit is absent until you add it with the external package manager or until the WDA Skill adds Spectrum 2 and Lit. Both Design System paths share the same custom element authoring standard.

#### Semantic HTML first

Native semantic HTML remains the first choice for layout, text, images, forms, and static content. Author a custom element only when native HTML is insufficient for encapsulated behavior or genuinely reusable product semantics. When a custom element is required, Lit is the standard library used to author it.

#### Component authoring without decorators

Author Lit components using standard JavaScript or TypeScript classes and explicit custom element registration. Do not use experimental class or property decorators. Keep any TypeScript configuration of the project free of decorator compiler settings.

```typescript
import { LitElement, html, css } from "lit";

export class ActionBadge extends LitElement {
  static properties = {
    count: { reflect: true },
    active: { reflect: true },
  };

  static styles = css`
    :host {
      display: inline-block;
    }
  `;

  constructor() {
    super();
    this.count = 0;
    this.active = false;
  }

  render() {
    return html`<span class="badge" aria-live="polite">${this.count}</span>`;
  }
}

customElements.define("action-badge", ActionBadge);
```

#### Managing the Lit dependency

Lit is a website dependency, so the external package manager of the project adds and removes it. To add Lit, run `npm install lit` or `bun add lit`. Run `wda check` after the change.

If a project uses only native HTML and CSS and does not require custom elements, remove Lit with the same manager:

```bash
npm uninstall lit
```

or, with Bun:

```bash
bun remove lit
```

#### Accessibility and manual review

Hand-written Lit elements must preserve accessibility across the component boundary:

- **Semantic HTML within templates**: Use native elements (`<button>`, `<a>`, `<input>`) inside component templates rather than simulating interactive controls with `<div>` or `<span>`.
- **Accessible name, role, and state**: Ensure all interactive controls have accessible names, proper roles, and communicate dynamic state changes (such as `aria-expanded`, `aria-busy`, and `aria-live`).
- **Keyboard operation and focus**: Provide complete keyboard navigation (Tab, Enter, Space, and arrow keys) and visible, high-contrast focus indicators for all interactive states.
- **Shadow DOM boundaries**: Ensure accessibility relationships (labels, descriptions, and ARIA references) correctly span or respect Shadow DOM boundaries.
- **Manual review**: All custom elements must conform to WCAG 2.2 AA. Because automated test suites only guard contractual constraints and cannot claim WCAG conformance, accessible behavior requires manual human review in browser preview, including keyboard navigation, visual focus inspection, and screen reader testing.

### Deployment boundary

`dist/` is the only deployment output. WDA rebuilds it from source. It contains no secrets. Teams can deploy it without WDA or the source project. URL rewriting and fallback routing belong to the deployment environment; this template makes no URL-shape guarantee.

When the project outgrows a standards-first source, an engineer may introduce Astro, React, Vue, or another framework. Preserve semantic HTML, CSS tokens and styles, Web Components, static assets, and project documentation where practical; migration is an engineering change, not an automatic transformation contract.
