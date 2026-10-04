---
title: "Generated Project Architecture Template"
status: stable
updated: 2026-09-22
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

The source baseline is HTML5, Modern CSS, TypeScript, and native browser capabilities. HTML owns structure and static content. CSS owns presentation, layout, responsive behavior, and CSS-manageable states. TypeScript owns interaction, necessary state, and browser APIs.

The default page architecture is MPA. The project leads with semantic HTML and native browser capabilities. `wda build` produces the independent deployment tree at `dist/`; generated token CSS appears only in `dist/styles/tokens.css`.

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
- `docs/` records project-specific architecture, design decisions, naming, and conditional development workflow.

### Custom Elements and Lit

New projects preinstall Lit by default across both Design System paths (Spectrum 2 and WDA Minimal), sharing a consistent custom element authoring standard.

#### Semantic HTML first

Native semantic HTML remains the first choice for layout, text, images, forms, and static content. Author a custom element only when native HTML is insufficient for encapsulated behavior or genuinely reusable product semantics. When a custom element is required, Lit is the standard library used to author it.

#### Component authoring without decorators

Author Lit components using standard JavaScript or TypeScript classes and explicit custom element registration. Do not use experimental class or property decorators. Generated projects keep `deno.jsonc` free of decorator compiler settings.

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

If a project uses only native HTML and CSS and does not require custom elements, Lit can be removed:

```bash
wda deps remove lit
```

To restore or install Lit later:

```bash
wda deps add lit
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
