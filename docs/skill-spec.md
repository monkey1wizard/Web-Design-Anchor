---
title: "Skill Specification"
status: stable
updated: 2026-10-15
type: Architecture
description: "The definition of the Skill, the reason WDA keeps a single Skill, the directory and file layout, ownership and release, versioning, the ban on executable scripts, cross-host portability, and the four acceptance tiers."
tags: []
---

# Skill Specification

This document is the contract for the Skill layer of Core. The Skill lives in the `skills/wda/` directory. This document does not repeat the Skill's content. It specifies only what the Skill may and must not contain, and the acceptance standards that apply to it.

## Role and Responsibility Boundary

The Skill is the AI's operating manual and workflow layer. It guides the AI on how to recognize and install `wda`, how to read the project structure, when to call a given command, how to change the project, and how to maintain the project documents.

The Skill is not a source of the specification. It does not keep a copy of the core rules, and it is not a second generator. The rules exist only in the project documents and in Core. The contract between Core and the project manifests must not assume that only the Skill operates WDA.

The two layers divide duties by one principle: **workflow procedure belongs to the Skill, and quality criteria belong to the project documents.** How to ask about brand and visual style, and in what order, is workflow procedure, so it belongs to the Skill. The criteria for visual quality are quality criteria, so the project's `docs/design.md` sets them.

## Why WDA Keeps a Single Skill

### The Split Basis Is How Well a Trigger Can Be Told Apart

The CLI runs the selected command only after the caller states a clear intent. If the caller picks the wrong command, the system returns an error at once. The AI picks a Skill by meaning, and it may not yet know the exact intent at that moment. If it picks the wrong Skill, it receives no error and may follow the wrong workflow. For this reason the rule "a different duty justifies a new command" does not apply to Skills. A Skill may be split in two only when the AI can say, without any ambiguity and before it loads a Skill, which one to load.

### Three Split Options That Were Evaluated and Rejected

| Split option | Reason for rejection |
| :--- | :--- |
| Split by CLI command | One user request can span several commands |
| Split by Designer work phase | The trigger semantics of the phases overlap heavily. If every Skill must restate the same rules, the repeated text becomes the seed of a second generator |
| Split by user role | A separate Engineer Skill would admit that Engineers also depend on a Skill to operate WDA |

The project therefore keeps a single Skill, named `wda`.

## Directory and File Structure

### Directory and File Layout

```text
skills/wda/
├── SKILL.md
└── references/
    ├── install.md          when wda is not on the PATH environment variable
    ├── intake.md           before creating a new project, or before a major style change
    ├── assets.md           when the Designer provides assets
    ├── reversibility.md    before the first visual change, or when a revert is requested
    ├── iteration.md        before every visual change and explicit design review
    ├── reporting.md        when reporting a CLI result or design review, or judging preview ability
    └── troubleshooting.md  when check or build reports an Error
```

The `scripts/` and `assets/` directories must not be created.

### Layout Standard

The directory layout follows the Agent Plugins 1.0.0 open standard at [agent-plugins.org](https://agent-plugins.org/specification). The discovery rule `skills/<name>/SKILL.md` is supported by both that standard and Claude Code. The root of the release package holds a minimal `plugin.json` that contains only the `$schema` and `name` fields. The whole package therefore conforms to the standard package specification, and any compatible client can install it directly. Installation belongs to the AI host, and this is the only mechanism involved. Core needs no install command of its own.

### The SKILL.md Content Allowlist

`SKILL.md` may contain only the following six kinds of content. Anything else is a violation.

1. Role and boundary declarations
2. Execution steps for version comparison
3. Decision routing tables
4. Workflow stop conditions
5. Reporting and declaration duties
6. Pointers to authoritative sources, called authority pointers

`SKILL.md` must not contain the content of the specification rules themselves.

A sentence in the Skill that states a rule with a normative marker must have an authority pointer in the same paragraph. The normative markers depend on the `metadata.language` value in the `SKILL.md` front matter. For `metadata.language: en`, the markers are must, must not, never, and only. For `metadata.language: zh-TW`, the markers are 必須, 不得, and 只能.

### Line Limit and Review Threshold

The hard limit is 500 lines or 5,000 tokens. The review trigger threshold is 300 lines. Going over 300 lines is not a build failure, but it triggers an architecture review that asks whether specification rules were copied into the Skill. The routing guidance for the four commands and the seven reference documents should not exceed 300 lines.

## Ownership and Release

The split among the entity, the specification, and the copy is as follows.

| Item | Owning layer |
| :--- | :--- |
| Skill content | `skills/wda/`, next to `src/` and `docs/`, and not inside `docs/` |
| Skill specification | This document and the related ADR |
| Installed copy | A deployed copy, with the same standing as the binary on the PATH environment variable and with no separate owner |

Core produces the Skill and releases it together with each release. Installing the Skill into an AI host is the host's duty. A `wda skill` command must not be added for this purpose.

## Versioning

### The Version Number Matches the Release

The Skill has no version number of its own. Its version is the same as the WDA release version, and it is recorded in `metadata.version` in the front matter. The project does not maintain a compatibility matrix between the Skill and the CLI.

### The Three-Branch Handling Policy

The real risk is drift of the installed copy. The Skill copy stays in the AI host, the binary sits on the PATH environment variable, and the two can be updated separately.

| Comparison result | Handling |
| :--- | :--- |
| Equal | Report that the versions match and continue the workflow |
| Skill is older | Report a Warning and continue when no known conflict affects the workflow |
| Skill is newer | Report an Error, stop at once, and tell the user to update the CLI. The AI must not silently downgrade to a best-effort role-play |

A known conflict in release source or observed CLI capabilities overrides the equal and older branches. Report the conflict, stop the affected workflow, and direct the user to the matching release or source Skill. Equal version text alone cannot exclude such a conflict.

Comparison steps: read `metadata.version` in the `SKILL.md` front matter and compare it with the output text of `wda --version`. The AI reads the string text directly and does not rely on the host to parse front matter fields by meaning. The comparison is therefore not affected by differences among AI host implementations.

### The Skill Must Not Interpret wdaVersion

The Skill must not read `wdaVersion` in `anchor.json` to decide whether the project format is compatible. Compatibility checking is the exclusive duty of Core. The Skill may only relay the diagnostic output of `wda check` faithfully, so that it does not grow a second validator.

## Executable Scripts Are Banned

The `scripts/` directory must not be created.

### Conflict with the Zero Extra Runtime Premise

One of the main reasons for building the CLI in Rust is that users do not need to install and manage a Node.js environment to run WDA. The languages a Skill script can use are limited by the AI host. Any Python or Node.js script would bring back a runtime requirement. Users would think that installing `wda` is enough, when they would also need an interpreter.

### The Red Line Criterion

An operation that must run correctly belongs to the `wda` binary, and the project should evaluate whether it deserves a new command. An operation that needs judgment, translation, and decisions belongs to the AI and is recorded in `SKILL.md`. There is no third, gray category. The main argument for scripts is determinism, but that argument supports moving the capability into Core. It does not support writing an add-on script. If a Skill script took on a deterministic need, it would add a Core capability that has no command name, no side-effect contract, and no version compatibility guarantee.

### Exception Procedure

To add any script, the proposal must meet the architecture premise of zero language runtime, give sufficient proof that the work is not a duty of Core, and carry a dedicated ADR and a recorded architect review approval.

## Cross-Host Portability

### Front Matter Allows Only Five Fields

Only `name`, `description`, `compatibility`, `metadata`, and `license` are allowed. The `allowed-tools` field is explicitly banned. It is an experimental property, AI hosts support it to different degrees, and relying on it would bind the Skill to one implementation.

### The Only Capability Assumption

The Skill makes only one capability assumption about the host. The host can run a native binary and can correctly capture stdout, stderr, and the exit status. This is the same process contract that reliable execution of `wda check` and `wda build` requires. The Skill adds no API that is specific to one host. Even when the exit status is 0, the Skill must read stderr if a Warning may exist. It must never judge that a run succeeded from exit 0 or stdout alone.

### Diagnostic Reporting and Contextual Review

Capture stdout, stderr, and exit status for each `wda` call. Human mode retains its diagnostic channels. In JSON mode parse schema version 1 from stdout and combine exit status, status, diagnostics, and summary. Report warnings even on exit 0. Missing, malformed, or unsupported JSON is not success. Report errors and ToolFault without masking them. Report a build artifact as an absolute path only when this run succeeds and supplies `dist_path`, never from an old `dist/`.

For design iteration or explicit review, route to `references/iteration.md` for accessibility, fixed CSS values, and Token relationships. Separate CLI diagnostics, source-based AI suggestions, observed browser results, and unrun checks. Identify files, basis, and uncertainty. CLI success is not review completion. Without a usable browser or after failure, list unrun screen and keyboard checks and manual guidance. Do not force installation or claim full WCAG conformance.

Judge fixed values against first-party CSS, existing Tokens, purpose, theme, and cascade. Equal values alone do not prove semantic equivalence. Do not scan packages or `dist/` for suggestions. Review primitive, semantic, and component reference direction only when explicitly adopted by the project or requested by the user, using recorded project rules. Review alone does not authorize edits. Preserve edit scope, new-Token consent, and accessibility tradeoff disclosure. These reviews add no Core diagnostics.

## Do Not Depend on External Skills

| Behavior | Ruling |
| :--- | :--- |
| A missing specific external skill prevents the workflow from finishing | Not allowed |
| Naming a specific external skill as a recommendation | Not allowed. External skill names can change at any time and stop working, and naming them would bind WDA to one vendor's ecosystem |
| The host environment happens to have other skills that work alongside | Allowed. The WDA Skill stays transparent about their presence and does not interfere |

## Skill Discovery and Installation Guidance

An existing project is discovered by `anchor.json`. Core requires both `anchor.json` and `package.json`. When no `anchor.json` exists, the Skill is discovered before the project is created, and only when the user explicitly asks to install or set up WDA, or to create a new WDA project. A general website development request that is not in a WDA project must not load the Skill. Once the AI is inside a WDA project, the scope of the Skill stays the same. An AI that does not have the Skill installed should follow the official README guidance. An AI that does not have the Skill installed must not assume that the Skill is available.

The Skill must not list install commands again. When the AI needs to install `wda` or handle a `wda` that is not on the PATH, the Skill must cite the existing `references/install.md` and follow its guidance.

Before installation, read a reachable official README. Probe `wda --version` as required, Git as needed, and npm or Bun only for a supplement. Missing Git permits project creation without a local baseline. Obtain separate consent for each install command and for project creation and integration. Record launch errors separately from exit statuses and re-probe after installation.

The README maintains the install commands for each platform. The Skill and this document must not copy platform-specific commands.

The Skill must not contain executable scripts. The `scripts/` directory must not be created. This limit applies to the Skill and to its reference documents.

The "Versioning" section defines how the Skill version is handled against the CLI. The discovery or installation flow does not change that handling.

### Supplement After a Minimal Fallback

When `wda init` falls back to the Minimal project, it records `initialization.fallback` in `anchor.json`, with the requested system and the reason. The Skill owns the later supplement that adds the default Spectrum and Lit packages. The supplement is not a CLI subcommand, and `wda init` is not rerun for it.

The supplement runs in this order: read the recorded state, recheck the network, the manager, and the existing content, propose the change scope and the revert method, obtain consent, install with the manager's native commands, integrate, and verify. The Skill does not upgrade the project on reconnect by itself. The Skill does not overwrite existing pages, and it does not install Node.js or Bun.

After Spectrum integration and successful `wda check` and `wda build`, update `anchor.json` to the actual design system, remove the `initialization` wrapper, and rerun `wda check`. If that final check fails, restore only the prior `designSystem` and `initialization` within the consented scope, retain the successful build’s `wdaVersion`, report the failure, and check again. Lit-only changes retain both fields. A failed or refused supplement preserves the prior records. Manager package and lifecycle side effects follow the rollback limits in the revert rules. A usable baseline can come from init or be created later by the Designer. Without one, pause writes and explain the limit until explicit consent. The Skill runs no Git commands. The detailed steps are in `skills/wda/references/install.md`, and the revert rules are in `skills/wda/references/reversibility.md`.

## Four Acceptance Tiers

| Acceptance tier | Content and standard | Nature of execution |
| :--- | :--- | :--- |
| L1 Specification compliance | `tests/skill.rs` checks that every key in the `SKILL.md` front matter is one of the five allowed fields | Blocking CI gate |
| L2 Boundary protection, also called the guardrail | Confirms that the Skill text contains no literal diagnostic code value from `src/codes.rs` | Blocking CI gate that prevents regression |
| L3 Behavior evaluation | Runs text matching against a prompt question bank. This method cannot measure undertriggering, and ADR-06 lists it as a known gap | Text-matching evaluation |
| L4 Deletion Test | After the whole Skill is removed, a person confirms whether Core and the project documents are enough for a person to operate WDA correctly | Manual test, not yet run |

The L3 evaluation decides whether the Skill qualifies for delivery. L1 and L2 decide whether the Skill can enter the CI flow. Even when L2 passes in full, the Skill must not be delivered if L3 misses its target.

The L2 ban on creating `assets/` is an architecture rule specific to WDA, not a general standard. `wda init` is responsible for generating project files. If the Skill carried templates that could be copied into project files, it would form a second source of generation that conflicts with `init`.

L3 cannot currently check undertriggering. The AI may fail to load a Skill that it should use. When that happens, the AI may edit project files from general web development knowledge, skip `wda check` and `wda build`, and give no warning. Writing and evaluating the `description` field is therefore a separate core task.
