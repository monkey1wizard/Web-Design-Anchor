---
title: "Product Scope and Output Boundaries"
status: stable
updated: 2026-10-15
type: ADR
description: "The Core execution form, the Standard Web output boundary, the closed command set, and the deployment environment assumptions."
tags: []
absorbs: ["001", "002", "016", "017", "028"]
---

# ADR-01 Product Scope and Output Boundaries

## Context

WDA lets a Designer build and maintain a website over the long term together with an AI.
The Designer's environment usually has no development toolchain, and the Designer should not have to maintain one.
The website must be easy to maintain over time and must not be forced into a rewrite when a front-end framework changes generation.
When the AI can read and write files directly, too many CLI commands only add maintenance cost.
Deterministic code must make sure that critical operations are correct.
The website output must state the supported deployment environments and the URL behavior, so that it does not promise results that Core cannot control.

## Options

**Implement the CLI in Node.js, Python, or Go, or adopt a front-end framework or a static site generator.** These choices have a rich ecosystem and ready-made solutions.
Users must handle Node.js versions and package conflicts.
The project is also limited by the upgrade pace and the complexity of the framework, and it loses the advantage of not depending on a runtime.

**Open the command set, guarantee Clean URL rewrites, and guarantee that the file:// protocol works.** This fits particular workflows or local previews. The cost is that the responsibility of Core keeps growing. Core must generate dedicated rewrite settings for different hosts, or it must limit the ES module and fetch capabilities of Standard Web to stay compatible with file://.

**Write the CLI in Rust, output plain Standard Web, limit the commands to four, and guarantee only static HTTP hosts.** Core is a single standalone native binary.
The output follows HTML5, Modern CSS, TypeScript, and native browser capabilities.
The commands are fixed to four: init, deps, check, and build.
The output is guaranteed to deploy only to a general static HTTP(S) host. It does not guarantee support for file:// or Clean URL.

## Decision

Core is compiled from Rust into a standalone native binary. The user only needs to put `wda` on the PATH and does not need to install any language runtime.

The project and its output use only Standard Web. The source uses HTML5, Modern CSS, TypeScript, and native browser capabilities. The deployment artifact contains HTML, CSS, JavaScript, and assets. The project is not tied to a front-end framework, and the website does not depend on WDA at run time.

The commands are fixed to four: init, deps, check, and build. A fifth command must not be added for the Designer workflow, the Skill installation, or the preview. Only an operation that represents a different responsibility and needs guaranteed correct execution qualifies for a new command. The AI is responsible for operations that need judgment or a decision.

WDA does not guarantee Clean URL. It also does not promise to remove file extensions or to generate nested directories that simulate routing. The deployment environment is responsible for URL rewrites and fallback routing.

dist/ is guaranteed to deploy only to a general static HTTP(S) host. It is not guaranteed to open directly through file://. If wda build detects a known incompatible feature, it raises a warning, but it does not provide a fallback.

## Consequences

- Users can run Core without installing Node.js or any other language runtime.
- The website does not depend on any framework or on WDA. An engineer who takes over faces Standard Web.
- Dependency changes have a single entry point, and the build uses a trusted lock state. The host is responsible for the preview and the Skill installation, so they do not add CLI commands.
- The output from source to dist/ can be predicted one to one. A project that needs Clean URL must configure it in the deployment environment.
- Standard Web capabilities are not restricted by file://. If the host does not support the preview, it must say so clearly, and it must not declare a change complete when no one has reviewed it.

## Status History

- 2026-08-24: Migrated from the planning document. The original was a four-line list.
- 2026-09-18: Rewritten as a full record. The decision is unchanged.
- 2026-08-24: Migrated from the planning document.
- 2026-09-18: Rewritten as a full record. The decision is unchanged.
- 2026-08-24: Migrated from the planning document. At that time there were three commands.
- 2026-08-24: Added `deps`, which made four commands.
- 2026-09-18: Rewritten as a full record of the new decision.
- 2026-08-24: Migrated from the planning document. At that time the decision was to output nested `index.html`.
- 2026-08-24: Changed to no guarantee.
- 2026-09-18: Rewritten as a full record of the new decision.
- 2026-08-24: Decided.
- 2026-09-18: Backfilled as an ADR.
- 2026-09-21: Consolidated from existing atomic decisions. The sources are listed in the front matter `absorbs`.
- 2026-10-23: Translated to English. The decision is unchanged.
