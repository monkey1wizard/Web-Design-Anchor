---
title: "Paper Spectrum Adapter Mapping"
status: stable
updated: 2026-08-27
sourceRefs:
  - "ROADMAP.md §2.5"
  - "docs/adr/008-layered-design-system-adapter.md"
  - "docs/design-systems/adapter-boundary.md"
  - "docs/design-systems/wda-minimal.md"
type: Reference
description: "Maps the WDA adapter boundary to the Paper Spectrum design system."
tags: []
---

# Paper Spectrum Adapter Mapping

## Definition and Purpose

The Paper Spectrum Adapter Mapping is an architectural reference document that exercises the [Design System Adapter Boundary](adapter-boundary.md) against Adobe Spectrum, a comprehensive commercial design system.

This mapping serves as a formal proof that the WDA Adapter Boundary is design-system neutral and not tailored exclusively to `WDA Minimal`. It maps Spectrum's published design token structures, component models, theme scoping mechanisms, component dependencies, and npm package distribution models onto WDA's public adapter facets.

**Implementation Scope**: This document provides a paper mapping and structural specification only. Actual software implementation, runtime registry integration, and automated adapter transformation logic for Spectrum are explicitly deferred to Phase 7 (P7). `WDA Minimal` remains the release-fixed offline fallback set and native neutrality reference.

## Boundary Mapping Facets

### 1. Spectrum Tokens (Token Mapping)

Spectrum token mapping establishes the bridge between project-owned DTCG semantic tokens (`tokens/tokens.json`) and Adobe Spectrum design tokens.

- **Canonical Project Tokens**: Canonical project tokens remain semantic and project-owned (e.g., `color.background.primary`, `color.text.body`, `space.medium`). Project token contracts do not directly embed vendor token identifiers.
- **Spectrum 2 Design Tokens**: The Spectrum adapter maps canonical DTCG semantic tokens to the Spectrum 2 design token set exposed within the `<sp-theme system="spectrum-two">` scope. Spectrum 2 resolves those tokens through the theme and scale modules loaded for that scope (see Facet 5), not through a project-declared custom-property literal.
- **Token Resolution Layer**: In a Spectrum-adapted project, WDA derives a vendor token binding layer (`dist/styles/spectrum-tokens.css`) that maps the Spectrum 2 tokens resolved within the `<sp-theme system="spectrum-two">` scope into project semantic custom property declarations (e.g., `--wda-color-background-primary` bound to the corresponding Spectrum 2 token). This document does not fix the Spectrum 2 property names; they are resolved at build time from the theme module loaded for the scope.

### 2. Spectrum Components (Component Mapping)

Spectrum component mapping defines how native semantic HTML markup and WDA component roles correspond to Adobe Spectrum UI components and markup structures.

- **Web Component Mapping**: Native HTML elements are mapped to Adobe Spectrum Web Components (`@spectrum-web-components/*`):
  - Native `<button>` / CTA → `<sp-button variant="accent">`
  - Native `<input type="text">` → `<sp-textfield label="...">`
  - Native `<select>` → `<sp-picker label="...">`
  - Native `<article>` / Card container → `<sp-card heading="...">`
  - Native `<dialog>` → `<sp-dialog>` inside `<sp-dialog-wrapper>`
- **Spectrum CSS Class Mapping**: For projects utilizing Spectrum CSS directly without Web Components, native markup elements receive corresponding Spectrum CSS component class names (e.g., `.spectrum-Button`, `.spectrum-Textfield`, `.spectrum-Card`).
- **Composition & Slot Neutrality**: WDA build primitives (such as `<wda-include>`) expand statically at build time before component rendering. Spectrum components encapsulate internal layout via Shadow DOM slots (`slot="label"`, `slot="icon"`), leaving WDA build tooling decoupled from component-internal runtime rendering.

### 3. Spectrum Theme (Theme Mapping)

Spectrum theme mapping defines container scoping, theme switching (light, dark, high contrast), density scales, and system color-scheme synchronization.

- **Theme Container Elements**: Spectrum uses a container component to establish theme context. The adapter maps WDA's `data-theme` attribute to the `<sp-theme>` provider element carrying the Spectrum 2 system opt-in, `system="spectrum-two"`:
  - `<sp-theme system="spectrum-two" color="light" scale="medium">`
  - `<sp-theme system="spectrum-two" color="dark" scale="medium">`
  - Omitting the `system="spectrum-two"` attribute renders the Spectrum 1 default theme silently, so the attribute is mandatory wherever `<sp-theme>` is used against this adapter.
- **Attribute Synchronization**: Theme attribute bindings map standard Web Platform settings (`color-scheme: light dark`) and `data-theme` values directly into the `<sp-theme system="spectrum-two">` container's `color` and `scale` attributes, ensuring seamless light/dark mode toggling and high-contrast accessibility compliance.

### 4. Spectrum Component Dependencies (Component Dependencies)

Spectrum component dependencies document the explicit package, custom element, and asset requirements needed to render Spectrum components.

- **Custom Element Registrations**: Spectrum Web Components require explicit custom element registry definitions before custom tags render in the browser DOM (e.g., `window.customElements.define('sp-button', SpectrumButton)`).
- **Per-Component Package Dependencies**: Each component maps to a discrete component package dependency:
  - `@spectrum-web-components/button`
  - `@spectrum-web-components/textfield`
  - `@spectrum-web-components/picker`
  - `@spectrum-web-components/theme`
- **Dependency Tree & Peer Requirements**: `@spectrum-web-components/button` depends on `@spectrum-web-components/base`, `@spectrum-web-components/icon`, and `@spectrum-web-components/theme`. The adapter manifest explicitly tracks component dependency trees to ensure all required custom elements and asset resources are resolved during build time.
- **Component Version Floor**: Every `@spectrum-web-components/*` package in the direct-dependency set (Facet 5) is held at `1.0.0` or above. Spectrum 2 support was introduced at that floor; a package below it does not carry the `system="spectrum-two"` opt-in this adapter depends on.

### 5. Spectrum NPM Dependency Integration (NPM Dependency Integration)

Spectrum npm dependency integration specifies how external npm packages, import maps, vendor manifests, and offline fallback mechanisms operate within WDA project lifecycles.

- **Lockstep-Publishing Premise**: This adapter takes as an explicit premise that `@spectrum-web-components/*` packages publish in lockstep, that is, every package in the direct-dependency set below is released at the same version in the same release. That premise is a measured observation, not a contractual guarantee from the vendor, and the adapter's single-version pin (below) depends on it holding.
- **Direct-Dependency Set**: This is the complete set of `@spectrum-web-components/*` packages this adapter's consumers, including the Spectrum 2 starter script, may import as a non-relative specifier. A consumer must not import a package outside this set, and must not rely on a transitive-only package:
  - `@spectrum-web-components/theme` (theme provider, scale modules, color modules)
  - `@spectrum-web-components/button`
  - `@spectrum-web-components/textfield`
  - `@spectrum-web-components/picker`
  - `@spectrum-web-components/card`
  - `@spectrum-web-components/dialog`
- **NPM Package Management**: Spectrum dependencies are declared in the project's `package.json` and tracked in `wda.json` under the vendor dependency manifest, every entry pinned to the single version the lockstep premise resolves for that release, each held at the `1.0.0` floor stated in Facet 4:
  ```json
  {
    "designSystem": {
      "name": "Spectrum 2",
      "version": "1.0.0",
      "packages": {
        "@spectrum-web-components/theme": "^1.0.0",
        "@spectrum-web-components/button": "^1.0.0",
        "@spectrum-web-components/textfield": "^1.0.0",
        "@spectrum-web-components/picker": "^1.0.0",
        "@spectrum-web-components/card": "^1.0.0",
        "@spectrum-web-components/dialog": "^1.0.0"
      }
    }
  }
  ```
- **Browser Import Maps**: WDA generates ES module import maps during project build to resolve vendor package specifiers (`import { Button } from '@spectrum-web-components/button'`) to local node_modules vendor distributions (`/vendor/@spectrum-web-components/button/index.js`).
- **Spectrum 2 Theme and Scale Imports**: The theme provider and its Spectrum 2 modules are imported from the theme package's Spectrum 2 subpaths, not from a separate CSS-variables package:
  - `@spectrum-web-components/theme/sp-theme.js` (the `<sp-theme>` provider)
  - `@spectrum-web-components/theme/spectrum-two/scale-medium.js`, `@spectrum-web-components/theme/spectrum-two/scale-large.js` (scale modules)
  - `@spectrum-web-components/theme/spectrum-two/theme-light.js`, `@spectrum-web-components/theme/spectrum-two/theme-dark.js` (color modules)
- **Build-Time Asset Resolution**: icon SVG sprites (`@spectrum-css/icon`) and web font assets (`adobe-clean`) are resolved from npm package payloads during `wda build`.
- **Offline & Resolution Fallback**: If an external Spectrum npm package cannot be resolved online or during an offline build without cached node_modules, WDA's resolution engine gracefully falls back to the release-fixed `WDA Minimal` built-in fallback set (`src/builtins/wda_minimal/`).

## Spectrum Adapter Mapping Matrix

| Boundary Facet | Spectrum Adapter Target Concept | Concrete Mapping Mechanism |
| --- | --- | --- |
| **Tokens** | `Spectrum Tokens` (Spectrum 2 design tokens) | Canonical project DTCG semantic tokens mapped to the Spectrum 2 token set, resolved within the `<sp-theme system="spectrum-two">` scope |
| **Components** | `Spectrum Components` (Web Components & CSS) | Native HTML markup mapped to `<sp-button>`, `<sp-textfield>`, `<sp-card>`, or `.spectrum-*` CSS classes |
| **Theme** | `Spectrum Theme` (Color / Scale / System) | WDA `data-theme` & `color-scheme` mapped to `<sp-theme system="spectrum-two" color="light|dark" scale="medium|large">` |
| **Component Dependencies** | `Spectrum Component Dependencies` | Explicit registration of `@spectrum-web-components/*` custom elements & dependency trees, each package held at the `1.0.0` floor |
| **NPM Dependency Integration** | `Spectrum NPM Dependency Integration` | `package.json` & `wda.json` package manifests pinned under the lockstep-publishing premise, ES module import maps, npm asset resolution, and `WDA Minimal` offline fallback |
