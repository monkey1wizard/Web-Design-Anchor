---
title: "ADR-022 Skill 邊界與發佈歸屬"
status: stable
updated: 2026-08-26
sourceRefs:
  - "ROADMAP.md §3.2 G2"
  - "docs/architecture.md#skill-boundary"
description: "What belongs to the WDA Skill release artifact instead of Core documentation?"
tags: []
type: ADR
---

## Historical decision (frozen)

The Skill is a release artifact rather than documentation. Its distributable source lives at `skill/wda/`, alongside Core source and documentation, and is published with the WDA release. The Skill stores no Core rules and does not form a second generator. Its version equals the WDA release version and is carried in the Skill front matter.

## Current status

Accepted. The Skill boundary is normative: `skills/wda/` owns the release artifact, while `docs/architecture.md` owns the Core-side boundary specification. The Skill may provide procedures, routing, and pointers to authoritative project documents, but the Core contract remains outside the Skill and has one canonical source. The Skill has no independent version; it uses the version of its containing WDA release.

On 2026-08-26 the owner corrected the path from the singular `skill/wda/` (as frozen above) to the plural `skills/wda/`, aligning it with the `skills/<name>/SKILL.md` discovery rule shared by Claude Code and the Agent Plugins 1.0.0 open standard (https://agent-plugins.org/specification). The release package additionally carries a minimal root `plugin.json` (`$schema` and `name` only), making the package installable by any conformant client. Every other clause of the frozen decision is unchanged.

On 2026-09-08, when P8b was unfrozen, the seventh Skill reference `skills/wda/references/iteration.md` was added. It loads before any visual change request is acted on, every time, not only the first.
