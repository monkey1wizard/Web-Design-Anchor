---
title: "Spectrum Adapter Mapping"
status: stable
updated: 2026-10-23
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

At build time, `src/build/scripts.rs` runs esbuild through Deno. The command uses `--bundle` to bundle the script entry.
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

`src/init/mod.rs::spectrum_two_dependency_specs` pins the package versions to the resolved Spectrum version.
Initialization writes the list to the project's `deno.jsonc`, and `deno.lock` then locks it.

## Summary Matrix

| Facet | What is implemented |
| --- | --- |
| Token mapping | No token mapping is defined yet |
| Styles | esbuild `--bundle` bundles the script entry at build time |
| Fonts | No font package and no font resolution at build time |
| Icons | No separate icon package and no icon resolution at build time |
| Theme | The dependency list includes the theme package |
| Component dependencies | Spectrum Web Components packages |
| Dependency integration | `deno.jsonc` declares the dependencies, and `deno.lock` locks the versions |

| Package | Version source |
| --- | --- |
| `@spectrum-web-components/theme` | The resolved Spectrum version |
| `@spectrum-web-components/button` | The resolved Spectrum version |
| `@spectrum-web-components/textfield` | The resolved Spectrum version |
| `@spectrum-web-components/picker` | The resolved Spectrum version |
| `@spectrum-web-components/card` | The resolved Spectrum version |
| `@spectrum-web-components/dialog` | The resolved Spectrum version |
| `lit` | Resolved independently |

The seven direct dependency packages above are listed in `src/init/mod.rs::spectrum_two_dependency_specs`.
If the initialization install fails, WDA removes the Spectrum-specific files and dependency settings that it already created, and then creates a WDA Minimal project.
