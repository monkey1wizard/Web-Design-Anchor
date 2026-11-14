---
title: Glossary
status: active
updated: 2026-11-12
---

# Glossary

> Defines project terms and naming conventions.

## Core Architectural Concepts

- **MPA (Multi-Page Application)**: The default architecture for WDA projects where each page is an independent HTML document located in `pages/`. Standard browser navigation handles page transitions without client-side routing frameworks.
- **SPA (Single-Page Application)**: An alternative application model driven by client-side routing and DOM manipulation. WDA defaults to MPA; SPA patterns are only adopted when explicitly required.
- **a11y (Accessibility)**: Digital accessibility ensuring websites are usable by everyone, including people with disabilities. The project adheres to WCAG 2.2 AA standards as its compliance baseline.

## Design System

- **Official Name**: `WDA Minimal`
- The built-in baseline design system providing standard design tokens, reset styles, and semantic color palettes.

## Project Files and Package Management

- **anchor.json**: The WDA manifest. It records the WDA settings, the Design System in use, and the fallback record when one applies.
- **package.json**: The website dependency source. It declares the packages that the project uses. A WDA Minimal project starts with no entries in it.
- **External package manager**: The npm or Bun command that installs and updates website packages. The manager owns the lockfile, and WDA does not write it.
- **Fallback record**: The `initialization.fallback` field of `anchor.json`. It names the Design System that init requested and gives the reason init created WDA Minimal instead.
- **Skill supplement**: The process in which the WDA Skill adds Spectrum 2 and Lit to an existing WDA Minimal project after the user consents.

## Naming Conventions

- **HTML & Pages**: Use lowercase kebab-case for filenames in `pages/` (e.g., `pages/about-us.html`).
- **CSS Classes & Custom Properties**: Use semantic class names and DTCG-derived design token variables (e.g., `var(--color-...)`).
- **Components & Scripts**: Use descriptive names that indicate purpose, not presentation frameworks.
- **Design Tokens**: Define them in `tokens/tokens.json` following the W3C DTCG format with hierarchical grouping.
