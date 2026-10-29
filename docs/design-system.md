---
title: "Design System and Tokens"
status: stable
updated: 2026-10-23
type: Architecture
description: "The Design System default and fallback, the token contract, the adapter boundary, the neutrality principle, and an index of the concrete system documents."
tags: []
---

# Design System and Tokens

This document describes the WDA Design System contract. It covers the initialization default and fallback, the token rules, the seven adapter facets, the identity branch boundary, and neutrality. For the implementation details of each system, see [WDA Minimal](design-systems/wda-minimal.md) and [Spectrum](design-systems/spectrum-adapter.md).

## Default and Fallback

### The Default Is the Latest Stable Spectrum

`wda init` queries the npm registry online for the `dist-tags.latest` of `@spectrum-web-components/bundle`. The version must be a stable semver release of 1.0.0 or higher.
After the query succeeds, WDA writes the exact version to `wda.json`. It then installs the six Spectrum Web Components packages listed in `src/init/mod.rs::spectrum_two_dependency_specs`, plus `lit`.

### Conditions for Falling Back to WDA Minimal

In any of the following cases, `wda init` uses `WDA Minimal` instead:

- `deno` is not on PATH.
- The query fails, times out, returns output that cannot be parsed, or lacks a required field.
- The version found is not a stable release, or is lower than 1.0.0.
- Installing the dependencies fails.

On fallback, WDA states the chosen system and the reason on stdout. It then removes the Spectrum-specific files and dependency settings that it already created, and creates a `WDA Minimal` project.
The Lit installation for `WDA Minimal` is best-effort. When the machine is offline or the registry is unavailable, initialization still creates the project. It also reports that Lit is not installed and shows the `wda deps add lit` command that can install it.

### Duty to Inform

`wda init` is a non-interactive command. When a fallback occurs, the output message must state the Design System in use and the reason for the fallback.

## Tokens

### Authoritative Source

`tokens/tokens.json` is the only authoritative token source, in DTCG format. The build converts it to CSS custom properties and writes them to `dist/styles/tokens.css`.

### Derived CSS

The source directory must not contain `styles/tokens.css`. This file is a derived artifact and may exist only in `dist/`. If `wda build` finds the file in the source directory, it reports an error and refuses to build.

### Naming

Tokens use a project-owned semantic namespace, and `tokens.json` may use aliases. An authoritative token name must not bind directly to a third-party vendor's token name.

### Primitive Layer

WDA does not require tokens to be split into primitive and semantic layers. A project may decide whether to use layers based on its size, component reuse, or hierarchy needs.

## Adapter Boundary

An adapter is a documentation specification that describes how a Design System connects to WDA. It is not a Rust trait and not a plugin registry. The seven facets below define a common way to map one system to another. For the concrete systems, see [WDA Minimal](design-systems/wda-minimal.md) and [Spectrum](design-systems/spectrum-adapter.md).

### 1. Token mapping

Describes how the project's DTCG semantic tokens map to the system's tokens.

### 2. Styles

Describes how base styles, layout, and global styles are organized and built.

### 3. Fonts

Describes the font families, font stacks, line height, type scale, and fallbacks.

### 4. Icons

Describes the icon assets, how they are packaged, how they are presented, and how they are named.

### 5. Theme

Describes how color modes and theme scopes are set and switched.

### 6. Component dependencies

Describes the components, custom elements, and native HTML markup that the system uses.

### 7. Dependency integration

Describes how external packages join the project lifecycle, and how a dependency failure is handled.

An implementation should prefer native HTML and CSS. Using a vendor's public element names and APIs is allowed. A project-owned wrapper is needed only when it provides product semantics, reusable behavior, or component encapsulation.

## Identity Branch Boundary

Core may identify or name a Design System in only two places:

| Seam | Files |
| --- | --- |
| Resolution seam | `src/init/resolve.rs` |
| Built-in asset layer | `src/init/mod.rs`, `src/init/documents.rs`, `src/builtins/wda_minimal.rs`, `src/builtins/spectrum_two.rs` |

Other Core code must not branch on Design System. `tests/neutrality.rs` uses text scanning to check for identity checks and hardcoded names outside the allowed paths. The scan skips comments, and it uses brace matching to find and skip `cfg(test)` modules. If a path changes, the test must be updated at the same time.

## Neutrality

WDA does not bind to a single Design System. It verifies this principle with two implementations, Spectrum and `WDA Minimal`. The two use different dependency and component models, and each supports `init`, `check`, and `build`. Currently both paths provide Lit by default. Spectrum uses vendor custom elements, and `WDA Minimal` uses native HTML.

The embedded assets of `WDA Minimal` are in `src/builtins/wda_minimal/`. That directory contains the design notes, the HTML starter page, and the DTCG token file. These files set neither `data-theme` nor `color-scheme`.

For the concrete systems and the facet mapping, see [WDA Minimal](design-systems/wda-minimal.md) and [Spectrum](design-systems/spectrum-adapter.md).

Neutrality does not mean that the two systems look the same or that one can be converted into the other automatically. Documents must not claim automatic conversion between the systems. Because there is no automatic conversion, documents also must not claim zero-cost migration, lossless migration, or HTML that needs no changes.

### Known Trade-offs

The token values of `WDA Minimal` are a one-time snapshot taken from `@adobe/spectrum-tokens` 15.2.0, and they do not follow upstream updates. The fonts are from the Noto family. For license and copyright information, see `NOTICE` in the project root. When describing this system, you may say that it has no vendor component library dependency. A description should not claim that its design values are independent of the vendor.

### Test Scope

The `WDA Minimal` path can run offline in the default `cargo test`. The Spectrum 2 flow depends on an available `deno` and the npm registry, so it is marked `#[ignore]` and runs only in the non-blocking probe job in CI. The blocking tests instead strictly compare the direct dependency list against `spectrum_two_dependency_specs` in `src/init/mod.rs`.
