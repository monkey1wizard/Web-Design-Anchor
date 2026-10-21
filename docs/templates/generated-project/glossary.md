---
title: "Generated Project Glossary Template"
status: stable
updated: 2026-10-15
sourceRefs:
  - "docs/glossary.md"
type: Template
description: "Defines project terms and naming conventions."
tags: []
---

# Glossary

> Defines project terms and naming conventions.

## Core Architectural Concepts

- **MPA (Multi-Page Application)**: The default architecture for WDA projects where each page is an independent HTML document located in `pages/`. Standard browser navigation handles page transitions without client-side routing frameworks.
- **SPA (Single-Page Application)**: An alternative application model driven by client-side routing and DOM manipulation. WDA defaults to MPA; SPA patterns are only adopted when explicitly required.
- **a11y (Accessibility)**: Digital accessibility ensuring websites are usable by everyone, including people with disabilities. The project adheres to WCAG 2.2 AA standards as its compliance baseline.

## Design System

- **Official Name**: `{{DESIGN_SYSTEM_NAME}}`
- The built-in baseline design system providing standard design tokens, reset styles, and semantic color palettes.

## Naming Conventions

- **HTML & Pages**: Use lowercase kebab-case for filenames in `pages/` (e.g., `pages/about-us.html`).
- **CSS Classes & Custom Properties**: Use semantic class names and DTCG-derived design token variables (e.g., `var(--color-...)`).
- **Components & Scripts**: Use descriptive names that indicate purpose, not presentation frameworks.
- **Design Tokens**: Define them in `tokens/tokens.json` following the W3C DTCG format with hierarchical grouping.
