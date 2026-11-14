---
title: "Design System and Tokens"
status: stable
updated: 2026-11-12
type: Architecture
description: "The Design System default and fallback, the dependency and manifest duties, the token contract, the adapter boundary, the neutrality principle, and an index of the concrete system documents."
tags: []
---

# Design System and Tokens

This document describes the WDA Design System contract. It covers the initialization default and fallback, the manifest and dependency duties, the token rules, the seven adapter facets, the identity branch boundary, and neutrality. For the implementation details of each system, see [WDA Minimal](design-systems/wda-minimal.md) and [Spectrum](design-systems/spectrum-adapter.md).

## Default and Fallback

### The Default Is the Latest Stable Spectrum

`wda init` queries the npm registry online for the `dist-tags.latest` of `@spectrum-web-components/bundle`. The version must be a stable semver release of 1.0.0 or higher.
After the query succeeds and a usable external package manager is available, `wda init` runs that manager to install the Spectrum packages and `lit` as website dependencies in `package.json`. The manager resolves and records the exact versions and writes its own lockfile. `anchor.json` then records the Design System as Spectrum with the resolved version.

### Conditions for Falling Back to WDA Minimal

In any of the following cases, `wda init` creates `WDA Minimal` instead:

- No usable external package manager is found. WDA supports npm and Bun.
- The registry query fails for any reason. The reasons include an unreachable registry, a timeout, an authorization or rate-limit response, a server error, a TLS failure, output that cannot be parsed, and output that lacks a required field.
- The version found is not a stable release, or is lower than 1.0.0.
- Installing the packages through the manager fails.
- A named manager, given with `--package-manager`, is unavailable. WDA then falls back and does not switch to another manager.

A local package cache never switches `wda init` to Spectrum. Only a successful online resolution and a successful manager install select Spectrum.

On fallback, WDA states the chosen system and the reason on stdout. It removes the Spectrum-specific files and the Spectrum dependency entries that this run created, and then creates a complete `WDA Minimal` project. If the removal or a required write fails, the command reports an explicit error and does not claim a Minimal success.

`WDA Minimal` is usable without a network connection and without a package manager. It creates no website dependencies at init, so no package is left waiting for installation. `anchor.json` records `WDA Minimal` as the Design System. It also records the originally intended system and the reason in `initialization.fallback`, with the fields `requestedDesignSystem` and `reason`.

### Adding the Defaults Later

The Skill can add Spectrum and Lit to a `WDA Minimal` project later. It checks the network, the manager, and the existing files first. It then proposes the change scope and a revert method, and installs only after the user consents. The Skill uses the project’s external package manager within the approved scope and integrates the packages without overwriting existing pages.

`wda init` is not rerun for this purpose, and a project does not upgrade automatically when the network returns. After Spectrum integration, `wda check` and `wda build` must succeed before the Skill sets `designSystem` to the actual system and removes the `initialization` wrapper, whose only field is `fallback`. The Skill then reruns `wda check`. If that final check fails, it restores the prior `designSystem` and `initialization` records within the approved scope and checks again. A successful build still updates `wdaVersion`. Adding Lit alone preserves the Design System and fallback. A refused or failed integration leaves the prior records unchanged, and the project stays usable as `WDA Minimal`.

### Duty to Inform

When a fallback occurs, the output of `wda init` must state the Design System in use and the reason for the fallback. The output must also say that the Skill can add the defaults later.

## Dependencies and Manifests

### Website Packages

- `package.json` is the only website dependency source. `anchor.json` does not duplicate any dependency.
- A successful Spectrum init records the chosen manager's name and exact version in the standard `packageManager` field of `package.json`, even when only one manager is usable. Initial Minimal omits the field.
- The external manager, npm or Bun, owns `package.json` changes, its lockfile, version solving, transitive versions, and installation. WDA never writes, rewrites, or migrates a manager lockfile.
- Adding, removing, or updating a website package uses the manager's own commands. No WDA command manages website packages.
- `wda check` and `wda build` read only installed package content. They never install or update packages, and they never use the network for packages.
- Website modules are not loaded from a remote CDN. WDA does not keep an import map, a vendor directory, or a CDN mirror.

### WDA Metadata

`anchor.json` owns the WDA settings, the actual Design System, and the initialization fallback record. It does not declare dependency versions, and it does not override the native tool version or its integrity value.

### Tool Cache

The TypeScript tool is the single version that a WDA release embeds. Release builds embed the platform package. Source builds try to fetch the same version at `wda init` and report a Warning when the fetch fails. `wda tools fetch` takes no arguments and restores the cache of that embedded version for the current platform. It does not change `anchor.json` or `package.json`.

### Migration

WDA does not migrate existing projects automatically. Read-only commands never rewrite dependency declarations. A project in a retired format is not claimed to build.

## Tokens

### Authoritative Source

`tokens/tokens.json` is the only authoritative token source, in DTCG format. The build converts it to CSS custom properties and writes them to `dist/styles/tokens.css`.

WDA supports nested groups, inherited `$type`, and whole-value `{path.to.token}` aliases. JSON nesting and alias chains are limited to 128 levels. Alias targets must be tokens, and cycles are rejected. Supported types are `color`, `dimension`, `fontFamily`, `fontWeight`, `lineHeight`, `number`, and `string`. Values must be numbers or nonempty strings that form one CSS custom-property value. Arrays, objects, booleans, null, and other types are rejected.

Token name segments may contain letters, numbers, hyphens, underscores, and spaces. Empty segments and names that collide after CSS normalization are rejected. Strings cannot contain control characters or CSS comment delimiters. Unescaped braces, unquoted `;` and `!`, and incomplete escapes are rejected. Quotes, parentheses, and brackets must balance. Partial alias interpolation is not supported. A token cannot also contain child tokens.

### Derived CSS

The source directory must not contain `styles/tokens.css`. This file is a derived artifact and may exist only in `dist/`. If `wda build` finds the file in the source directory, it reports an error and refuses to build.

### Naming

Tokens use a project-owned semantic namespace, and `tokens.json` may use aliases. An authoritative token name must not bind directly to a third-party vendor's token name.

### Primitive Layer

Token layering is voluntary for every project, whichever Design System it uses. WDA does not require tokens to be split into primitive and semantic layers. A project may decide whether to use layers based on its size, component reuse, or hierarchy needs.

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

Describes how external packages, installed through the external manager, join the project lifecycle, and how a dependency failure is handled.

An implementation should prefer native HTML and CSS. Using a vendor's public element names and APIs is allowed. A project-owned wrapper is needed only when it provides product semantics, reusable behavior, or component encapsulation.

## Identity Branch Boundary

Core may identify or name a Design System in only two places:

| Seam | Files |
| --- | --- |
| Resolution seam | `src/init/resolve.rs` |
| Built-in asset layer | `src/init/mod.rs`, `src/init/documents.rs`, `src/builtins/wda_minimal.rs`, `src/builtins/spectrum_two.rs` |

Other Core code must not branch on Design System. `tests/neutrality.rs` uses text scanning to check for identity checks and hardcoded names outside the allowed paths. The scan skips comments, and it uses brace matching to find and skip `cfg(test)` modules. If a path changes, the test must be updated at the same time.

## Neutrality

WDA does not bind to a single Design System. It verifies this principle with two implementations, Spectrum and `WDA Minimal`. The two use different dependency and component models, and each supports `init`, `check`, and `build`. Spectrum installs Lit through the external manager during init. `WDA Minimal` creates no website dependencies at init, and the Skill may add Spectrum and Lit later. Spectrum uses vendor custom elements, and `WDA Minimal` uses native HTML.

The embedded assets of `WDA Minimal` are in `src/builtins/wda_minimal/`. That directory contains the design notes, the HTML starter page, and the DTCG token file. These files set neither `data-theme` nor `color-scheme`.

For the concrete systems and the facet mapping, see [WDA Minimal](design-systems/wda-minimal.md) and [Spectrum](design-systems/spectrum-adapter.md).

Neutrality does not mean that the two systems look the same or that one can be converted into the other automatically. Documents must not claim automatic conversion between the systems. Because there is no automatic conversion, documents also must not claim zero-cost migration, lossless migration, or HTML that needs no changes.

A project may use a Design System other than these two when its `anchor.json` names it. The rules in this document apply to every project and do not require a particular Design System.

### Known Trade-offs

The token values of `WDA Minimal` are a one-time snapshot taken from `@adobe/spectrum-tokens` 15.2.0, and they do not follow upstream updates. The fonts are from the Noto family. For license and copyright information, see `NOTICE` in the project root. When describing this system, you may say that it has no vendor component library dependency. A description should not claim that its design values are independent of the vendor.

### Test Scope

The `WDA Minimal` path can run offline in the default `cargo test`. The Spectrum 2 flow depends on an available npm or Bun manager and the npm registry, so it is marked `#[ignore]` and runs only in the non-blocking probe job in CI.
