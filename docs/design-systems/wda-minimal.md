---
title: "WDA Minimal"
status: stable
updated: 2026-08-24
sourceRefs:
  - "ROADMAP.md §2.5"
  - "ROADMAP.md §3.2 B2"
type: Reference
description: "Describes the built-in WDA Minimal design system and its offline baseline role."
tags: []
---

# WDA Minimal

## Definition

`WDA Minimal` is a WDA-owned built-in Design System. It is the smallest
reference Design System that WDA can use as a real project baseline. It uses
project-owned semantic Design Tokens, native semantic HTML, and Modern CSS as
its primary implementation. It does not require a vendor component library.

WDA Minimal is small, semantic, native-Web-first, and project-token-driven. It
is intended to be easy to understand, suitable for a production project, and
sufficient to create a WDA Minimal Starter. It is also the native/minimal
reference for validating that WDA Core, project structure, token handling,
build output, and documentation do not depend on an external Design System or
a vendor-specific component runtime.

WDA Minimal is not intended to replace a complete large commercial Design
System. It must not introduce abstractions merely to imitate one.

## Position in WDA

WDA Minimal is the fixed built-in fallback set for a WDA release when the
configured Design System cannot be resolved online. The fallback is fixed for
that release. It is also one of the reference Design Systems used to exercise
a different dependency model and component model alongside the default latest
stable Spectrum integration.

WDA Minimal is an architecture and offline baseline, not the Designer
first-run visual default. The default Design System remains latest stable
Spectrum; when it is resolved online, the project records its exact version.

The purpose of comparing WDA Minimal with Spectrum is to exercise different
dependency and component models and verify that Core is not bound to one
Design System. It is not a promise of automatic or lossless migration between
Design Systems.

## Tokens

The canonical project token source is `tokens/tokens.json` in DTCG format.
`wda init` creates the semantic tokens required by the Minimal Starter; the
file is real Design System contract state, not a placeholder.

Project-facing token names are semantic and use a project-owned namespace: WDA
Minimal owns its token key namespace. DTCG aliases or references are allowed
between project tokens. An adapter may map vendor semantics into project token
values when a project uses another Design System.

The build derives CSS Custom Properties from `tokens/tokens.json` into
`dist/styles/tokens.css`. Source must not contain a generated `tokens.css`;
derived token CSS exists only in `dist/`.

WDA does not require a two-level Primitive Token → Semantic Token hierarchy.
A primitive or reference layer is added only when the project genuinely needs
scale, reuse, or hierarchy.

## Markup and component boundary

Native semantic HTML and CSS are preferred for layout, text, images, buttons,
forms, and static content. Web Components may be used when a project genuinely
needs reusable behavior, reusable product semantics, or encapsulation. They are
an available Web Platform capability, not a required dependency of WDA Minimal.

WDA Minimal does not require a vendor component library or vendor-specific
runtime. A project may use an appropriate vendor public element name or API
when it has selected another Design System; WDA does not create a universal
wrapper merely to hide vendor naming.

The only WDA-specific build-time composition primitive is
`<wda-include src="..."></wda-include>`. It follows Custom Element naming
conventions, expands at build time, is not a runtime Web Component, and must
not remain in `dist/`. WDA Minimal does not define a template language: there
are no `<slot>` compositions, conditionals, loops, variables, expressions, or
data binding.

## Usage boundary

This file is the sole Core canonical definition of WDA Minimal. A Generated
Project's `docs/design.md` records only that project's instantiated choices,
effective tokens, and project-specific review guidance; it does not copy this
WDA-wide definition.

WDA Minimal is a reusable baseline and reference, not a guarantee of zero HTML
modification, 100% reuse, or automatic migration. Its design-system migration
goal is to reduce migration cost while preserving the project's semantic
structure and ownership of its choices.
