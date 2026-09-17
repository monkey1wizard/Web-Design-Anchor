---
title: "Design System Adapter Boundary"
status: stable
updated: 2026-08-27
sourceRefs:
  - "ROADMAP.md §2.5"
  - "docs/adr/008-layered-design-system-adapter.md"
  - "docs/design-systems/wda-minimal.md"
type: Reference
description: "Defines the documentation contract between WDA and design system adapters."
tags: []
---

# Design System Adapter Boundary

## Definition and Purpose

The Design System Adapter boundary is the documented public contract interface defining how design systems interface with Web Design Anchor (WDA). It establishes a standard structural boundary across seven design system facets, allowing WDA project configuration, asset validation, and build tooling to operate neutrally regardless of whether the active design system is built-in (`WDA Minimal`) or external (`Spectrum`).

This boundary is documented as a specification and architectural contract. It is explicitly **not** a Rust trait, plugin registry, or runtime code abstraction. Keeping the boundary as documentation preserves project flexibility without introducing premature code abstractions into WDA Core.

## Architectural Boundaries

A Design System Adapter must describe how a design system satisfies seven core boundary facets:

1. **Token Mapping** — Project-owned DTCG semantic token contract and vendor mapping.
2. **Styles** — Baseline CSS, resets, derived CSS custom properties, and layout utility rules.
3. **Fonts** — Typographic scales, font stacks, web font loading, and fallbacks.
4. **Icons** — Icon resolution, asset packaging, SVG inline/sprite mechanisms, and naming conventions.
5. **Theme** — Light/dark mode switching, color scheme variants, and runtime attribute bindings.
6. **Component Dependencies** — Available UI components, custom elements, or native HTML markup requirements.
7. **Dependency Integration** — Package management, build-time asset resolution, vendor import maps, and offline fallback behavior.

## Public Boundary Facets

### 1. Token Mapping

Token mapping defines the contract between project-owned canonical DTCG semantic tokens (`tokens/tokens.json`) and design system token definitions.

- **WDA Minimal**: Consumes project-owned DTCG semantic tokens directly without intermediate vendor layers. Token aliases resolve internally within `tokens.json`. Canonical project tokens use semantic naming (e.g., `color.background.surface`, `spacing.md`).
- **Spectrum**: Maps canonical project DTCG semantic tokens to Spectrum design tokens (e.g., Spectrum CSS variables or Spectrum design token aliases such as `--spectrum-gray-100` or `--spectrum-background-base-color`).

### 2. Styles

Styles define CSS distribution, baseline resets, layout systems, and global stylesheet compilation.

- **WDA Minimal**: Derived token CSS is generated from `tokens/tokens.json` to `dist/styles/tokens.css`. Includes a native CSS baseline reset and Modern CSS layout rules in project stylesheets without external preprocessors or frameworks.
- **Spectrum**: Imports Spectrum CSS stylesheets and packages (e.g., `@spectrum-css/page`, `@spectrum-css/tokens`). Derived token variables integrate directly with Spectrum CSS custom property layers and global theme styles.

### 3. Fonts

Fonts define typography families, font stacks, line heights, typographic scales, and fallback strategies.

- **WDA Minimal**: Uses native system font stacks (`system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif`). Requires zero network requests or external web font loading.
- **Spectrum**: Uses Adobe Clean font stack (`adobe-clean, sans-serif`) with system font fallbacks. Defines `@font-face` declarations or loads font assets via npm/web font distribution.

### 4. Icons

Icons define icon asset packaging, SVG embedding/spriting, naming conventions, and icon element syntax.

- **WDA Minimal**: Uses native inline SVG elements or simple local SVG assets. Requires no external icon web components, icon fonts, or icon libraries.
- **Spectrum**: Uses Spectrum Workflow Icons (`@spectrum-css/icon` or `@adobe/spectrum-css-workflow-icons`) integrated via inline SVG symbols or Spectrum custom icon elements.

### 5. Theme

Theme defines color scheme switching (light, dark, high contrast), theme attribute bindings, and container scoping.

- **WDA Minimal**: Manages themes through the standard CSS `color-scheme` property and data attribute bindings (`data-theme="light"` / `data-theme="dark"`).
- **Spectrum**: Uses Spectrum theme containers and attributes (`data-theme`, `color-scheme`, Spectrum 2 `sp-theme` attributes like `system="spectrum-two"`, `color="light"`/`color="dark"`, `scale="medium"`).

### 6. Component Dependencies

Component dependencies define the component model, custom element registries, and markup requirements.

- **WDA Minimal**: Uses native semantic HTML5 elements (`<header>`, `<main>`, `<nav>`, `<article>`, `<button>`). Requires zero vendor component dependencies, Web Components, or custom element runtimes.
- **Spectrum**: Uses Adobe Spectrum Web Components (`@spectrum-web-components/*`) or Spectrum CSS element markup (e.g., `<sp-button>`, `<sp-textfield>`, `<sp-theme>`), requiring Web Component registration and element dependencies.

### 7. Dependency Integration

Dependency integration defines how external packages, build tools, import maps, and offline fallbacks are integrated into the WDA project lifecycle.

- **WDA Minimal**: Zero external npm or network dependencies. Asset sources are release-fixed, embedded into the WDA binary (`src/builtins/wda_minimal/`), and available offline as a built-in fallback set.
- **Spectrum**: Uses npm package-based dependency integration (`@spectrum-web-components/bundle`, Spectrum CSS npm packages), vendor import maps in `wda.json`, and CDN or local vendor asset caching.

## Adapter Facet Summary Matrix

| Facet | WDA Minimal Case | Spectrum Case |
| --- | --- | --- |
| **Token Mapping** | Direct project semantic DTCG tokens | Mapped from project DTCG tokens to Spectrum token aliases |
| **Styles** | Derived `dist/styles/tokens.css` + native modern CSS reset | Spectrum CSS (`@spectrum-css/*`) style packages |
| **Fonts** | System font stack (`system-ui`) | Adobe Clean font stack with fallbacks |
| **Icons** | Native inline SVGs | Spectrum Workflow Icons (`@spectrum-css/icon`) |
| **Theme** | Data attribute (`data-theme`) & `color-scheme` CSS | Spectrum theme attributes and Spectrum theme classes |
| **Component Dependencies** | Pure native semantic HTML5 | Spectrum Web Components (`@spectrum-web-components/*`) |
| **Dependency Integration** | Zero external dependencies (embedded built-in binary fallback) | NPM package dependencies & vendor import maps |
