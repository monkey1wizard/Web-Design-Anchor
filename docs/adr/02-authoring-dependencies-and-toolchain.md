---
title: "Authoring, Dependencies, and Toolchain"
status: stable
updated: 2026-10-15
type: ADR
description: "The external tool prerequisites, the TypeScript and CSS processing pipeline, Deno-native dependency management, and the backward-compatible branch."
tags: []
absorbs: ["006", "011", "013"]
---

# ADR-02 Authoring, Dependencies, and Toolchain

## Context

WDA is a native executable, but it must still resolve npm dependencies and transform TypeScript into JavaScript. The Modern CSS that the Designer writes must also integrate with tokens. The project needs a reliable way to declare, resolve, and lock dependencies. It also needs to decide the external tool requirements and how existing projects stay compatible.

## Options

**Require users to install Node.js and npm or pnpm, and then process the files with a standalone esbuild executable or lightningcss.** This matches common front-end development practice.
But it passes the burden of Node.js version management and node_modules to the Designer.
esbuild must also distribute executables for several platforms.

**Build a package downloader that stores packages in a vendor directory and uses an import map, and then build a custom compile engine.** This approach needs no external tools, but Core must implement package resolution, version deduplication, and a compiler by itself. The complexity is too high.

**Require only deno and git as external tools.** A new project declares dependencies in deno.jsonc, and Deno manages them. An existing project can still use package.json. Deno preserves comments and writes the files. TypeScript first passes a static type check, and then a pinned version of esbuild transforms it. CSS has no designated processing engine and stays as written.

## Decision

The external tools require only `deno` and `git` on the PATH. Node.js is not required. `deno` resolves dependencies and calls the transpiler. `git` creates the local version-control baseline at initialization. When `wda init` runs and `deno` is missing, the command falls back to WDA Minimal. If `git` is missing, the command aborts before it writes anything. `wda check` reports a Warning for a missing tool. `wda build` and `wda deps` fail with a named diagnostic.

WDA does not designate a CSS engine. `styles/**/*.css` is copied as written to `dist/styles/`, and only tokens are converted into Custom Properties. TypeScript must first pass a static type check, and then `deno run -A npm:esbuild@0.25.5` transforms it into JavaScript.

Dependency declaration and management:

- A current project declares dependencies in the imports of deno.jsonc. Deno resolves them and locks them into deno.lock in the project root. WDA does not accept a project-side deno.json, so that the settings stay consistent.
- An existing project that declares dependencies in package.json keeps its original declaration and build method. `wda check` and `wda build` do not migrate the file structure silently.
- `wda deps` is the only command that may change dependencies. When it calls Deno, it specifies `--config` and disables third-party lifecycle scripts. Deno writes the files and preserves comments, and WDA only cleans up the empty state.
- All dependency packages are bundled into `dist/` at build time, so no remote runtime dependency remains.

## Consequences

- The Designer only needs to install three executables: `deno`, `git`, and `wda`.
- The versions of the transpiler and the dependency resolution are controlled and do not change with the local environment.
- A current project can use JSONC comments and Deno modules. An existing project does not lose support because of a breaking update.
- The CSS that the Designer writes is output to `dist/` as written. All dependency packages are bundled and published, so no connection to a CDN is needed at run time.
- When `deno` is missing, `wda init` falls back to WDA Minimal, `wda check` reports a Warning, and `wda build` and `wda deps` fail with a named diagnostic. When `git` is missing, `wda init` aborts before it writes anything, and the other three commands report a Warning or a named diagnostic respectively.

## Status History

- 2026-08-24: Migrated from the planning document. At that time it said "seek a Rust native binary or crate".
- 2026-09-02: The Project Owner decided that `deno` is the only PATH prerequisite tool, and that the transpiler is called through a pinned `deno run`.
- 2026-09-08: Added `git` as the second prerequisite tool.
- 2026-09-18: Rewritten as a full record.
- 2026-08-24: Migrated from the planning document. At that time the decision was lightningcss plus a standalone esbuild binary.
- 2026-08-24: Changed to no designated CSS engine.
- 2026-09-02: Changed the transpiler to a pinned call through `deno run`.
- 2026-09-18: Rewritten as a full record of the new decision.
- 2026-08-24: Migrated from the planning document. At that time the decision was a vendor directory plus an importmap.
- 2026-08-24: Changed to `package.json` plus `deno.lock`.
- 2026-09-02: The Project Owner decided to always bundle, and to decide on remote later according to need.
- 2026-09-18: Rewritten as a full record of the new decision.
- 2026-10-14: Added the current handling in each command when `deno` or `git` is missing.
- 2026-09-21: Consolidated from existing atomic decisions. The sources are listed in the front matter `absorbs`.
- 2026-10-23: Translated to English. The decision is unchanged.
