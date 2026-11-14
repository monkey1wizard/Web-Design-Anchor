---
title: "AI Skill Boundaries and Evaluation"
status: stable
updated: 2026-10-15
type: ADR
description: "The AI Skill as the primary operating interface, its position as an artifact shipped with each release, and the L3 line-by-line text comparison method used when no harness exists."
tags: []
absorbs: ["003", "022", "026"]
---

# ADR-06 AI Skill Boundaries and Evaluation

## Context

The Designer operates WDA in natural language, but conversation memory cannot persist across sessions or tools. The Skill guides the AI in operating the CLI. The project must state whether the Skill is a document or a shipped artifact, and it must define the versioning principle and the specification boundary. The original acceptance of Skill behavior (the L3 evaluation) required a transcript harness to replay real triggers and to quantify recall and precision. Without a harness tool, the Skill still needs an executable alternative way of validation.

## Options

**Develop a standalone GUI application, keep the Skill in a documentation directory, and pause acceptance or fill in estimated values when there is no harness.** A GUI adds packaging and maintenance cost on every platform, and it does not fit how an AI operates. Treating the Skill as an ordinary document blurs the line between the specification and the execution procedure. Without a harness, this approach either blocks delivery or hides the quantitative gap behind false numbers.

**Rely only on conversation memory, give the Skill its own version and a compatibility matrix, and claim that a general text comparison is a complete evaluation.** Conversation memory is not a durable and reliable basis. A separate version matrix only adds maintenance burden. A text comparison cannot check undertriggering, and if it is treated as a behavior evaluation, it hides the risk of a missed trigger.

**The AI operates the CLI through the Skill. The project files are the only source of truth.** The Skill ships with the release and uses the same version as the release. When there is no harness, a line-by-line text comparison is used instead and is clearly marked not-measured. The Designer uses natural language. The AI calls the CLI by following the Skill procedure and writes all decisions back to the project documents. The Skill is stored as a shipped artifact in skills/wda/. The Skill version matches the release, and the Skill does not hold Core rules. If there is no executable harness, the L3 evaluation uses deterministic fixtures to compare the text line by line. When there is no executable harness, recall and precision are honestly recorded as not-measured, and estimated figures must not be filled in.

## Decision

The Designer talks with the AI in natural language. After the AI loads the Skill, it operates the wda CLI and writes all specifications and decisions back to the project documents. The project files are the only source of truth. The AI is responsible for reading and writing the project files and must not require the Designer to read the files by hand first.

The Skill is an artifact that ships with each release, and it is not a document. The files are in skills/wda/, alongside the Core source code and the document layer. The Skill provides operating procedures and routing pointers. It does not hold the internal rules of Core, and it does not form a second generator. The Skill has no version number of its own, and its version is fully aligned with the WDA release it belongs to. The layout of the release package follows the Agent Plugins 1.0.0 standard.

On design iteration or user request, the Skill reviews accessibility, fixed CSS values and Token use in context. It reviews Token reference direction only when the project adopts layer rules or the user asks. These suggestions are separate from Core diagnostics. Review alone does not authorize edits, and new Tokens require user consent. Browser results must be observed, and unavailable checks must be reported as unrun.

If there is no executable transcript harness, the Skill L3 behavior evaluation temporarily uses a line-by-line text comparison instead. When there is no live judging tool, recall and precision are always recorded as `not-measured`. The evaluation must not substitute estimated values, data from an earlier round, or a pass rate for recall or precision. The evaluation result applies only to the AI host that was actually tested (Claude Code).

## Consequences

- The project files support handoff and continuation across sessions, which lowers the risk of losing decisions.
- The release package contains skills/wda/ and plugin.json, so a standard client can install it directly. Core does not need to provide an extra Skill command.
- If the Skill version is newer than the binary, the system must stop and prompt for an upgrade, and it must not downgrade silently. The Skill content is limited by an allowlist, to prevent rules from leaking.
- A line-by-line text comparison cannot check missed triggers (undertriggering), and this is a known gap. Once an executable harness exists, the evaluation that replays real triggers must be restored.

## Status History

- 2026-08-24: Migrated from the planning document.
- 2026-09-18: Rewritten as a full record. The decision is unchanged.
- 2026-08-24: The decision was settled. At that time the path was written in the singular as `skill/wda/`.
- 2026-08-26: The Project Owner corrected it to the plural `skills/wda/` to align with the discovery rule of Agent Plugins 1.0.0, and added `plugin.json` in the root directory.
- 2026-09-08: The Designer visual iteration workflow was unfrozen, and the seventh reference, `skills/wda/references/iteration.md`, was added. It is loaded before every visual change is run, not only the first one.
- 2026-09-18: Rewritten in the fixed five-section format. The decision is unchanged.
- 2026-09-05: The Project Owner decided that the target host is Claude Code.
- 2026-09-09: The Project Owner accepted this method as a known limitation and did not block delivery because of it.
- 2026-09-18: Rewritten in the fixed five-section format. It stays `draft` because this is a second-best choice made while the capability is absent. Once an executable harness exists, this record should be replaced by the approach that replays real triggers.
- 2026-09-21: Consolidated from existing atomic decisions. The sources are listed in the front matter `absorbs`.
- 2026-10-23: Translated to English. The decision is unchanged.
- 2026-11-13: Assigned contextual accessibility, CSS, and voluntary Token layer review to the Skill.
