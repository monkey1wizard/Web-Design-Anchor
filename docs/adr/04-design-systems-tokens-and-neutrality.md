---
title: "Design Systems, Tokens, and Neutrality"
status: stable
updated: 2026-10-15
type: ADR
description: "The Design System adapter interface, DTCG token conversion, the Spectrum default with the WDA Minimal fallback, shared Lit authoring, the neutrality criterion, and the single definition."
tags: []
absorbs: ["008", "014", "019", "024", "025", "031"]
---

# ADR-04 Design Systems, Tokens, and Neutrality

## Context

Core must not be tied to a single Design System. To prove neutrality, Core must support two systems whose architectures differ enough, which are Spectrum 2 and WDA Minimal. The project must define how to handle the official vendor Web Components, the format and location of the design tokens, the standard for writing shared Custom Elements, and how to degrade and restore when offline.

## Options

**Build a generic wrapper, write tokens in CSS, or mix component libraries freely.** Requiring both systems to use a vendor library or exactly the same dependency model also belongs to this option. A generic wrapper may grow into a heavy second component library, and it cannot keep the promise of migrating without changing the HTML. If both systems depend on a vendor library, a reliable offline fallback is impossible. If components are hand-written freely, there is no consistent standard, and they are hard to maintain.

**Define the adapter in a document, adopt DTCG tokens, and limit the neutrality seams.** The build converts tokens into CSS. The default is Spectrum 2, and when offline the project falls back to WDA Minimal. Both systems share Lit as the default authoring standard for Custom Elements. The adapter defines seven facets. The token source is DTCG JSON, and the output goes only to dist/. The default system is the latest stable Spectrum, and the offline fallback is WDA Minimal. Both systems use Lit as the default authoring library for Custom Elements. WDA Minimal makes a best effort to install Lit, so that it remains usable offline.
The neutrality criterion covers only the resolution seam and the asset seam.

## Decision

The adapter is an interface specification defined in a document, and it covers seven facets. Native HTML and Modern CSS come first. A generic wrapper is not built to hide a vendor name.

The only source of tokens is tokens/tokens.json, and its format is DTCG. The build converts the tokens into CSS Custom Properties and outputs them to dist/styles/tokens.css. Derived files must not be stored in the source.

Core supports two Design Systems. The default is the latest stable Spectrum, with the npm packages pinned and a standalone Lit. The offline fallback option is WDA Minimal, and the values of its frozen Spectrum token snapshot are defined only once in Core.

Both systems share Lit as the default authoring library for Custom Elements. Layout and static content still prefer native semantic HTML. Lit is used only when behavior or product semantics need encapsulation, and it uses a standard class and customElements.define, without decorators.

Initializing WDA Minimal makes a best effort to install Lit. If the user is offline or has no network, initialization still succeeds and creates the project with plain semantic HTML. The summary says that Lit is not installed and gives the remedy command `wda deps add lit`.

The neutrality criterion is that the two systems each use a different dependency and component model and each complete init, check, and build. A Design System identity branch may appear only in the resolution seam (src/init/resolve.rs) and the asset seam (src/init/, src/builtins/). All other code must stay neutral.

## Consequences

- To add or replace a Design System, it is enough to extend the seams and the documents. The main flow of Core does not need to change.
- Design values are stored in a structured format, so both the AI and design tools can read them precisely, and the derived CSS does not pollute the source.
- Both systems use the same way of writing Custom Elements. WDA Minimal can still initialize successfully when there is no network.
- Automated tests guard neutrality. They keep the code and the documents from falsely claiming automatic conversion, because there is no guarantee of lossless migration.

## Status History

- 2026-08-24: Migrated from the planning document. At that time it said "layered adaptation".
- 2026-08-27: The definition of the seven facets landed.
- 2026-09-18: Rewritten as a full record.
- 2026-08-24: Migrated from the planning document. At that time it said to generate into `styles/tokens.css`.
- 2026-08-24: Changed to generate only into `dist/`.
- 2026-09-18: Rewritten as a full record.
- 2026-08-24: Migrated from the planning document. At that time the second system was called Modern Neutral Clean.
- 2026-08-24: Formally named `WDA Minimal`.
- 2026-08-27: The implementation of `WDA Minimal` landed.
- 2026-09-07: The implementation of Spectrum landed.
- 2026-09-18: Rewritten as a full record of the new decision.
- 2026-09-05: Decided by the Project Owner.
- 2026-09-18: Rewritten in the fixed five-section format. The decision is still in force.
- 2026-09-05: The Project Owner decided to move the real registry probe out of the blocking gate.
- 2026-09-07: Settled when the implementation of the second Design System landed.
- 2026-09-18: Rewritten in the fixed five-section format. The constraint is still in force.
- 2026-08-24: Decided.
- 2026-08-27: The definition document of `WDA Minimal` and the `design.md` template landed.
- 2026-09-18: Backfilled as an ADR.
- 2026-09-21: Consolidated from existing atomic decisions. The sources are listed in the front matter `absorbs`.
- 2026-10-23: Translated to English. The decision is unchanged.
