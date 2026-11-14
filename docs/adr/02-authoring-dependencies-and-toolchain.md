---
title: "Authoring, Dependencies, and Toolchain"
status: stable
updated: 2026-11-12
type: ADR
description: "Two project manifests with external npm or Bun package managers, a Rust build engine that resolves installed packages, and one embedded TypeScript native compiler verified by its SRI."
tags: []
absorbs: ["006", "011", "013"]
---

# ADR-02 Authoring, Dependencies, and Toolchain

## Context

WDA is a native executable, but it must still resolve dependencies and transform TypeScript into JavaScript. Modern CSS that the Designer writes must also integrate with tokens. The project needs a reliable way to declare, resolve, and lock dependencies without placing external tool installation burdens on the Designer.

On 2026-10-09, Deno and Cloudflare published official announcements stating that the Deno team is joining Cloudflare:
- Deno announcement "Deno is joining Cloudflare", dated October 9, 2026, by Ryan Dahl: https://deno.com/blog/cloudflare
- Cloudflare announcement "Deno is joining Cloudflare", published 2026-10-09: https://blog.cloudflare.com/deno-joins-cloudflare

The Deno announcement states that the team will support the Deno runtime for another year with monthly releases containing bug fixes and security updates, and after that year will end development of the Deno runtime. Deno remains open source. From the one-year support period, the end of Deno runtime development is derived as 2027-10 (a derived month, not a quoted date in the announcement).

Because WDA requires a lasting architecture that outlives short maintenance windows, WDA must not depend on an external runtime whose development is scheduled to end. Core needs to handle authoring, dependency resolution, script transformation, and type checking natively without requiring users to install external toolchains.

## Options

**Continue using Deno until its end of support in 2027-10.** This requires no immediate architecture change. However, it defers an inevitable migration, forces Designers to install a runtime nearing its end of life, and exposes projects to unaddressed security vulnerabilities after 2027-10.

**Require users to install Node.js and npm or pnpm, and process files with standard front-end tooling.** This aligns with standard JavaScript ecosystem conventions. But it forces Designers to manage Node.js versions, PATH configuration, and `node_modules` directories, violating the zero-external-tool prerequisite principle.

**Require users to install Bun.** Bun provides fast execution, TypeScript support, and package management in a single executable. However, it replaces one external runtime dependency with another, requires cross-platform installation management by the user, and introduces potential runtime incompatibilities.

**Rely on an unofficial community fork of Deno after 2027-10.** An open-source community fork could theoretically continue maintaining the runtime. But relying on uncertain community governance, unverified maintenance capacity, and unpredictable release cadence creates unacceptable long-term project risk.

**Adopt a Rust-native workflow with a self-built vendor directory, an import map downloader, unbundled transformation, and an embedded native type checker.** This option was adopted on 2026-11-06 and is superseded by the decision below. It attempted custom npm package resolution, version deduplication, and remote module delivery inside Core, and it depended on an external module CDN and a type-declaration mirror. The costs it accepted were increased `wda` binary size from embedding the toolchain, potential network waterfalls on unbundled deep module graphs, and a supply-chain trust boundary on the module CDN.

## Decision

Users install no external tools for `wda init`, `wda check`, or `wda build`, except an external package manager when they want Spectrum and Lit. `git` is optional. Release binaries embed the platform TypeScript native compiler. Source builds can restore it with `wda tools fetch`. `wda check` and `wda build` never install or update website packages.

Project manifests and format:

- WDA uses two manifests in the project root. `anchor.json` owns WDA settings, the actual Design System, and the minimal init fallback state. It is validated by `schemas/anchor.schema.json`. The current schema has no `schemaVersion`. A future breaking format change must define one explicitly. `package.json` is the only website dependency source. Its root object, optional `dependencies`, and optional `packageManager` are checked directly by `src/validation/project_contract.rs`, with no schema file.
- `anchor.json` does not duplicate dependencies, and it does not override native tool versions or the embedded SRI.
- Every new project receives both manifests. A Minimal project has empty dependencies and no `packageManager`, and it has no manager lockfile until the first external install.
- Both manifests are required. Old `wda.json`, `deno.jsonc`, and `deno.json` cannot replace either one. Core reports an Error for a missing manifest and does not migrate old formats automatically. Legacy files may coexist with both current manifests.

Website dependencies and package managers:

- The external manager owns `package.json` changes, its lockfile, version solving, transitive versions, and installation. The supported managers are npm and Bun.
- `wda init` detects a manager through a finite, tested seam in `src/init/resolve.rs`. The seam chooses the manager, runs its fixed argv in an explicit working directory, and reports the result. It does not solve versions, keep a cache, or download packages itself.
- On successful Spectrum init, the `packageManager` field of `package.json` records the actual manager name and exact version, even when only one manager is usable. They are not duplicated in `anchor.json`.
- With no reachable and usable manager, or when offline, `wda init` creates WDA Minimal directly. A failed Spectrum and Lit resolution or install removes that run's Spectrum-only artifacts and dependency settings, and then produces a complete Minimal. The Designer sees the real reason from the manager.
- Website package add, remove, and update use the manager's own commands. WDA has no `wda deps` command and no alias for it.

Build and static output:

- `wda build` uses the Rolldown 1.3.1 Rust API with the browser platform and ESM output. It builds all HTML page entries together, resolves installed modules through their exports, subpaths, and dynamic imports, and writes local chunks and assets.
- The HTML references the engine's actual local entries. Third-party modules are not loaded from a CDN, and there is no vendor directory, no site-module lock file, and no type-declaration mirror.
- The build writes to a fresh staging directory first and commits only manifest-listed entries, chunks, and required static assets. It never copies `node_modules` wholesale, dotfiles, or `.env*` files. Host-path debug information is disabled.
- WDA checks the full resolution graph and the typed output imports. Remote modules, Node-only modules, and dynamic references that cannot be fully emitted are reported as explicit diagnostics. WDA does not trust the engine exit code alone.
- A successful `init` writes `wdaVersion`. A successful `build` updates it in `anchor.json` and writes `dist/`. A failed build and `check` leave it unchanged.
- `dist/` is the only deployment artifact.
- CSS is copied as written. WDA designates no CSS processing engine, and only design tokens are converted into CSS Custom Properties.

Type checking and the embedded tool:

- `wda build` always type-checks. Type errors are an Error and block the build. Core never silently skips the type check.
- Each released `wda` binary binds exactly one TypeScript version. `packaging/toolchain.json` records the schema version, the supported manager names, the official package identity, the executable path, and the six-platform `sha512` SRI. The file is embedded with `include_str!` and validated by `src/tools.rs`.
- The embedded SRI is the only trust root for the tool. Projects cannot override the version, the SRI, or the executable path. The tool descriptor contains no arbitrary executable, argument list, or download command.
- The only tool entry point is `wda tools fetch` with no arguments. It restores the cache for the embedded version and this platform. Extra arguments exit with usage status `2`. There is no cross-version switch and no rewrite of `anchor.json` or `package.json`.
- A download is limited to 32 MiB per response, with connect and request timeouts. It is accepted only from the trusted HTTPS registry, each redirect is revalidated, and the embedded SRI is verified before any unpacking. Archive entries with absolute paths, parent traversal, links, or escapes are rejected. The cache is keyed by the embedded SRI and is published atomically, so concurrent readers never see partial content.
- `wda build` never uses the network. It checks the embedded or cached platform package first. A pure HTML Minimal project with no script entries needs no cache and does not launch the tool. A project with script entries and no cached tool is a project Error with exit status `1`, and its output names `wda tools fetch`.
- A parseable type error is a project Error with exit status `1`. A launch failure, a crash, a run longer than 30 seconds, or unparseable output is a ToolFault with exit status `2`.
- The TypeScript compiler runs under WDA's fixed options. These options are strict, `noEmit`, `isolatedModules`, `verbatimModuleSyntax`, `skipLibCheck`, and the browser libraries. A project `tsconfig.json` cannot weaken them.

## Consequences

- The Designer installs no Deno, Node.js, Bun, or esbuild for `wda check` or `wda build`. `git` is optional. Release binaries embed the platform TypeScript compiler. Source builds can restore it with `wda tools fetch`.
- Website packages need an external manager. Without one, the project stays on WDA Minimal and the Designer keeps working. Spectrum and Lit need a usable manager and network access at init time.
- Network access is restricted to explicit hosts and commands:
  - The npm registry is accessed by `wda init` when it resolves Spectrum and Lit through the chosen manager.
  - The npm registry is accessed by `wda tools fetch` when the platform TypeScript tool must be restored, including for source builds.
  - `wda check` and `wda build` never use the network.
- The supply-chain trust boundary is the npm registry, the chosen package manager, and the embedded SRI. Core runs the manager during `init` with `--ignore-scripts`. Separately approved manager commands use their own arguments and may run lifecycle scripts. WDA does not promise that their side effects can be reverted.
- The engine is a large dependency, and its output is a local ESM graph. Browser compatibility of a given npm package is not guaranteed. Packages outside the supported input set receive an explicit diagnostic, and WDA does not claim that every npm package works in a browser.
- Embedding the platform-specific type checker increases the binary size, estimated between 6 MB and 16 MB and measured before release. Every release packs six per-platform tools, and release artifacts must carry the third-party tool license notice.
- Six-platform execution, musl execution, and real network probes are verified by named release checks and by ignored probes. This decision does not claim their results by itself.
- Deno is not a runtime requirement for WDA or for generated projects. The repository's writing checker still runs under Deno.
- Online resolution failure during `wda init` triggers fallback to WDA Minimal, with the reason recorded in `anchor.json`.

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
- 2026-11-06: The Project Owner reversed the 2026-09-02 "always bundle" ruling, adopted unbundled ESM with import maps, moved dependency resolution to esm.sh, and removed Deno in favor of an embedded native type compiler.
- 2026-11-12: Updated the current decision to two manifests, external npm or Bun managers, a Rolldown build of installed packages, and one embedded TypeScript version verified by its SRI. Offline and no-manager init produce WDA Minimal.
- 2026-11-12: Removed the esm.sh, vendor directory, import map, type-declaration mirror, cross-version tool switching, and Deno-necessity statements from the current decision and consequences.
- 2026-11-14: Aligned manifest, version writeback, manager, and embedded tool guidance with the native CLI.
