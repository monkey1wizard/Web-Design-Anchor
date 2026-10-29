---
title: "Build, Validation, and Diagnostics"
status: stable
updated: 2026-10-15
type: ADR
description: "Build-time fragment expansion, single-line deterministic diagnostics, impact-based severity, document structure validation, and the a11y check boundary."
tags: []
absorbs: ["007", "010", "012", "015", "018", "027"]
---

# ADR-03 Build, Validation, and Diagnostics

## Context

A multi-page website shares the navigation and the footer, so they must be expanded at build time to avoid extra runtime cost.
Validation and the build find different problems, so they need a clear severity criterion.
Both people and the AI must be able to read the diagnostic output clearly.
Document consistency checks and accessibility checks must divide the scope between automatic validation and manual review, so that heuristic rules do not cause false positives.

## Options

**Adopt a full template language, output --json diagnostics, fix the severity by problem category, compare documents and code in depth with heuristic rules, and build a browser engine into the CLI to simulate complex a11y scenarios.** This gives the greatest expressive power and the widest automation scope. The cost is that the template language and the JSON schema become a second heavy contract. Heuristic comparison and scenario-based a11y also cause many false positives, which badly damages the credibility and usefulness of a Warning.

**Copy the HTML into each page, show diagnostics only in the terminal, and check all a11y and documents manually.** This approach is the easiest to implement. The cost is that during maintenance it is easy to miss a shared block, and deterministic source syntax errors and accessibility problems may be overlooked.

**Expand a minimal tag at build time, output diagnostics as single-line deterministic plain text, and decide severity by impact.** The document check validates only the structure and the metadata.
The a11y check uses only deterministic source rules.
The only custom syntax is `<wda-include src="...">`.
Diagnostics are always single-line text, and `--json` is not supported.
Severity depends on whether the problem breaks the build or the correctness of the project.
The document check only confirms whether a document exists and what its front matter holds.
The automatic a11y check covers only three pure source rules. The remaining items are left to the manual preview check.

## Decision

Shared fragments are expanded at build time, and dist/ outputs only plain static HTML. The only custom syntax is `<wda-include src="..."></wda-include>`. The syntax does not support `<slot>`, conditions, loops, or data binding, and the artifact leaves no runtime.

Severity depends on impact and is not fixed by problem category. If a problem makes it certain that the artifact or the project is incorrect, it is marked as an Error and blocks the build. If a problem raises a quality or correctness concern, it is marked as a Warning, which stays visible but does not block the build.

Diagnostics are always deterministic single-line text, and `--json` is not provided. The public promise includes the severity, the code, the relative path, the line and column position, and a deterministic order.

wda check validates only the document structure and the required metadata, for example the title, status, and updated in the front matter. It does not use heuristic rules to compare the document content with the code.

The automatic a11y check is fixed to three pure source rules: `html[lang]` must not be empty, every `img` must have an `alt`, and every `id` in a document must be unique. A violation of any rule is marked as an Error. Items such as contrast, focus, and keyboard operation are left to a person to review during the preview. Passing the automatic check does not mean conformance to WCAG.

## Consequences

- The output adds no runtime dependency, and it causes no layout shift or SEO problem.
- The check rules do not produce false positives. A Warning gives guidance for action, and it is hard to ignore.
- The diagnostic format is simple and stable, and no public JSON schema needs to be maintained.
- The scope of automatic validation is clear.
- A person is responsible for WCAG conformance. It is not handed to a CLI tool that cannot understand context.

## Status History

- 2026-08-24: Migrated from the planning document.
- 2026-09-18: Rewritten as a full record. The decision is unchanged.
- 2026-08-24: Migrated from the planning document. At that time the decision was a hard split by category.
- 2026-08-24: Changed to judging by impact.
- 2026-09-18: Rewritten as a full record of the new decision.
- 2026-08-24: Migrated from the planning document. At that time the syntax was `<include>` plus `<slot>`.
- 2026-08-24: Changed to only `<wda-include>`.
- 2026-09-18: Rewritten as a full record of the new decision.
- 2026-08-24: Migrated from the planning document. At that time the decision was that all commands support `--json`.
- 2026-08-24: The Project Owner adopted the conclusions of the architect and adversarial review, and changed to not providing `--json`.
- 2026-09-18: Rewritten as a full record of the new decision.
- 2026-08-24: Migrated from the planning document. At that time the decision was to compare structure, tokens, and dependency changes.
- 2026-08-24: Changed to validate only structure and metadata.
- 2026-09-18: Rewritten as a full record of the new decision.
- 2026-08-24: Decided.
- 2026-08-26: The implementation of the three checks landed.
- 2026-09-18: Backfilled as an ADR.
- 2026-09-21: Consolidated from existing atomic decisions. The sources are listed in the front matter `absorbs`.
- 2026-10-23: Translated to English. The decision is unchanged.
