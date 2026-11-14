---
title: "Spectrum Adapter Mapping"
status: stable
updated: 2026-11-12
type: Reference
description: "How the Spectrum 2 implementation maps to the seven WDA adapter facets."
tags: []
---

# Spectrum Adapter Mapping

This document records the Spectrum 2 integration that the code implements. For the general definition of the seven facets, see [docs/design-system.md](../design-system.md).

Spectrum 2 was added to the project on 2026-09-07. The commit history is in `src/builtins/spectrum_two/`.

## The Seven Facets

### 1. Token mapping

The code does not define a mapping between Spectrum tokens and WDA tokens.

### 2. Styles

At build time, the native build in `src/build/` bundles the script entry with a native bundler. It then type-checks the entry with the verified native TypeScript tool. No Deno or Node.js runtime is required.
This describes the bundling flow in the code. It does not mean that a separate Spectrum style integration exists.

### 3. Fonts

The initial dependency list has no font package. The build code also has no step that resolves font assets.

### 4. Icons

The initial dependency list has no separate icon package. The build code also has no step that resolves icon assets.

### 5. Theme

The initial dependency list includes `@spectrum-web-components/theme`. The code defines no other theme setting or switching behavior.

### 6. Component dependencies

The starter page uses the `@spectrum-web-components/*` packages. The table below lists the packages and how the other dependencies are integrated.

### 7. Dependency integration

`src/init/mod.rs` (`spectrum_two_dependencies`) pins the six Spectrum packages to the same resolved Spectrum version. It pins `lit` to its own resolved version.
Initialization writes these exact versions to the project's `package.json`. The project's package manager then installs them into `node_modules` and writes its lockfile.
The native build resolves each bare or subpath import in `scripts/main.ts` from the installed packages. It does not fetch packages at build time, and it does not fetch them at runtime in the browser.

## Summary Matrix

| Facet | What is implemented |
| --- | --- |
| Token mapping | No token mapping is defined yet |
| Styles | The native build bundles the script entry at build time |
| Fonts | No font package and no font resolution at build time |
| Icons | No separate icon package and no icon resolution at build time |
| Theme | The dependency list includes the theme package |
| Component dependencies | Spectrum Web Components packages |
| Dependency integration | `package.json` declares the exact versions, and the project's package manager installs them and writes its lockfile |

| Package | Version source |
| --- | --- |
| `@spectrum-web-components/theme` | The resolved Spectrum version |
| `@spectrum-web-components/button` | The resolved Spectrum version |
| `@spectrum-web-components/textfield` | The resolved Spectrum version |
| `@spectrum-web-components/picker` | The resolved Spectrum version |
| `@spectrum-web-components/card` | The resolved Spectrum version |
| `@spectrum-web-components/dialog` | The resolved Spectrum version |
| `lit` | Resolved independently |

The six Spectrum packages above are the direct dependencies listed in `src/init/mod.rs` (`SPECTRUM_TWO_DIRECT_DEPENDENCY_PACKAGES`). `lit` is a separate direct dependency with its own version.

Support limits:

- The starter supports only the packages listed above and the modules they expose to the starter script.
- The project does not claim that every npm package works in a browser. A package works only when the native build can bundle it into local files.
- A bare or subpath import that no installed package provides fails the build with an error that names the source file.

If the initialization install fails, WDA removes the Spectrum-specific files and dependency settings that it already created, and then creates a WDA Minimal project.
