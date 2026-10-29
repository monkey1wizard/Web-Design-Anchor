---
title: "WDA Minimal"
status: stable
updated: 2026-10-23
type: Reference
description: "The minimal Design System built into WDA: its definition, token provenance, starter structure, and the source of the design.md criteria."
tags: []
---

# WDA Minimal

## Definition

WDA Minimal is the minimal Design System built into WDA. It is also the reference that WDA uses to validate a real-project baseline. It uses project-owned semantic Design Tokens, native semantic HTML, and Modern CSS, and it does not depend on a vendor component library. It adopts Lit, which it shares with Spectrum 2, as the standard for writing Custom Elements.

WDA Minimal centers on being small, semantic, native to the Web, and driven by project tokens. A real project can use it, and it can also create a WDA Minimal Starter. WDA Core, the project structure, token handling, the build result, and the documentation do not depend on the vendor component library or the dedicated runtime of any external Design System.

When online initialization succeeds, WDA Minimal installs `lit` on a best-effort basis and produces `deno.jsonc` and `deno.lock`. The online path that installs Lit successfully is therefore not a zero-dependency path. WDA Minimal contains no vendor component library and produces no starter script (`scripts/main.ts`).

WDA Minimal does not replace a full, large commercial Design System. It also adds no abstraction layer to imitate another system.

## Position in WDA

When no configured Design System can be found online, or when the Spectrum 2 dependency install fails, WDA falls back to the fixed built-in WDA Minimal set. The same release uses the same set. WDA Minimal is also a reference Design System. It runs alongside the default latest stable Spectrum and tests a different dependency model and component model. Spectrum 2 contains six vendor component packages and a separate Lit. WDA Minimal installs only Lit on a best-effort basis and contains no vendor component library.

WDA Minimal is the architectural and offline baseline. It is not the visual default that a Designer sees at first launch. The default Design System remains the latest stable Spectrum. When online resolution succeeds, the project records the exact version. When the Spectrum 2 dependency install fails, WDA uses the indivisible mechanism to remove the Spectrum-specific files and dependency settings that it already wrote, and then creates a WDA Minimal project.

If `wda init` runs while offline or without access to the npm registry, the Lit install is handled on a best-effort basis. Initialization still creates the project but does not produce `deno.jsonc` or `deno.lock`. stdout shows the difference caused by the offline fallback and gives `wda deps add lit` as the recovery command. This keeps the project usable offline and does not weaken the Standard Web deployment output.

WDA compares WDA Minimal with Spectrum to test the dependency model and the component model, and to confirm that Core is not bound to any specific Design System. This comparison is not a promise of automatic or lossless migration.

## Token Provenance

The token values come from a frozen snapshot of `@adobe/spectrum-tokens` 15.2.0 and do not track upstream updates. The snapshot is based on the `dist-tags.latest` of the npm registry, and its version is 15.2.0.

WDA copies the token values, not the names. The project still defines the names as semantic names, for example `color.brand.primary` and `spacing.md`.

The font family tokens use font stacks that start with Noto. Noto is licensed under SIL OFL 1.1.

The `NOTICE` file in the repository root records the copyright attribution. Following section 4(d) of Apache-2.0, it lists the upstream package name, the exact version, the license, and the modifications.

The snapshot does not update automatically. To refresh the snapshot, read the new version and update `NOTICE`.

## Tokens

`tokens/tokens.json` is the DTCG source of the project's canonical tokens. `wda init` creates the semantic tokens that the Minimal Starter needs. This file represents the actual contract of the Design System. It is not placeholder content.

The project owns its semantic token names and namespaces. Project tokens can use DTCG aliases or references among themselves. If the project switches to another Design System, an adapter can map the vendor semantics to the project token values.

The build converts `tokens/tokens.json` into CSS Custom Properties and writes them to `dist/styles/tokens.css`. The source must not contain a generated `tokens.css`. The derived token CSS appears only in `dist/`.

WDA does not require tokens to be split into a Primitive Token layer and a Semantic Token layer. When a project needs a scale, reuse, or hierarchy, it can then add a primitive or reference layer.

## Starter Structure

The source of the WDA Minimal Starter is the embedded assets in `src/builtins/wda_minimal/`, which `include_str!` compiles into the binary:

- `tokens/tokens.json`: the DTCG tokens.
- `pages/index.html`: the single-page Starter.
- `docs/design.md`: the parameterized source of the visual review criteria.

The test fixture is in `tests/fixtures/wda_minimal_starter/`. The three files above are byte-for-byte identical to the embedded assets. The fixture also contains `README.md`, `wda.json`, `docs/architecture.md`, and `docs/glossary.md`.

## The Parameterized design.md Criteria Source

`src/builtins/wda_minimal/docs/design.md` is the parameterized source of the visual review criteria. The file marks the block for each Design System with `{{#if_design_system "WDA Minimal"}}` and `{{#if_design_system "Spectrum 2"}}`. Rendering keeps only the blocks that match the condition.

`## Attribution` and `## 10. Craft Floor` do not belong to a single Design System. They sit outside the conditional blocks, so every render keeps them.

Section 10, "Craft Floor", is the last section and contains 11 visual quality review criteria. The content of section 10 is the same for WDA Minimal and Spectrum 2. A Designer can change or remove any criterion and record the change in that section. These criteria are not mandatory checks of `wda check`.

WDA selects and rewrites only part of the impeccable text. It does not bring in its Skill, commands, or dependencies. This Apache-2.0 text snapshot does not track upstream updates. `NOTICE` records the source and the exact commit.

The criteria have two replacement markers. `{{DESIGN_SYSTEM_NAME}}` becomes the Design System name, and `{{ACCESSIBILITY_BASELINE}}` becomes the accessibility baseline. After rendering, no unexpanded `{{` or `}}` marker should remain.

`render_criteria` in `src/builtins/wda_minimal.rs` renders the criteria. `src/init/documents.rs` writes the result to `docs/design.md` of the Generated Project.

## Markup and Component Boundary

For layout, text, images, buttons, forms, and static content, prefer native semantic HTML and CSS. Use Web Components only when the project needs reusable behavior, product semantics, or encapsulation. Both Design System paths in WDA adopt Lit. Lit is the standard for writing Custom Elements.

A standard LitElement class declaration and a `customElements.define` registration both meet this standard. The standard does not use a decorator compile flag. WDA Minimal contains no vendor component library and produces no starter script (`scripts/main.ts`).

Lit is a lightweight wrapper over the Web platform and does not bind a project to a specific front-end framework. If a project does not need Lit, run `wda deps remove lit` to remove it. If a project needs Lit after it was created offline, or wants to restore a Lit that it removed earlier, run `wda deps add lit`.

WDA Minimal needs no vendor component library or its dedicated runtime. When a project switches to another Design System, it can use the element names or APIs that the vendor publishes. WDA does not create a generic wrapper to hide vendor names.

`<wda-include src="..."></wda-include>` is the only custom build-time composition primitive in WDA. It follows the Custom Element naming rules and expands at build time. It is not a runtime Web Component and does not remain in `dist/`.

WDA Minimal defines no template language. It has no `<slot>`, conditions, loops, variables, expressions, or data binding.

## Usage Boundary

This document is the complete definition of WDA Minimal in Core. The `docs/design.md` of a Generated Project records only the tokens that the project actually adopts and the project-specific review criteria. It does not copy this WDA-level definition.

WDA Minimal is a reusable baseline and reference. It is not a guarantee of "zero HTML modification", "100% reuse", or "automatic migration". Its migration goal is to lower the cost while it keeps the project's semantic structure and its freedom of choice.
