---
title: "Generated Project Lifecycle and Reversibility"
status: stable
updated: 2026-11-12
type: ADR
description: "The optional local Git restore baseline, the init collision set for the two manifests and manager state, the single-creation .gitignore, and the conditional development.md document."
tags: []
absorbs: ["023", "029", "030"]
---

# ADR-05 Generated Project Lifecycle and Reversibility

## Context

During visual iteration, the Designer needs a restore baseline. Without version control, the Designer cannot return to the last accepted state. The build artifacts of a generated project should not be under version control by default. The project also needs a clear rule for how `.gitignore` is created and managed. Most projects can be developed with just the four commands, and they do not need an empty development workflow document created by default.

## Options

**Have the Skill run the Git operations, have every command manage `.gitignore`, and always create development.md.** This makes the Skill carry the version-control work.
The Core commands would also check `.gitignore` on every run and create five documents for each new project.
When the Skill carries deterministic operations, it may also change the user's custom repository settings.
This approach also produces many empty documents.

**Replace Git with the checkpoint of the AI host, merge the existing `.gitignore`, and let a baseline trigger the creation of development.md.** This depends on the private snapshot feature of each AI platform.
It also requires parsing and merging the existing ignore list.
A browser or a11y baseline would trigger the creation of a fifth document.
The mechanisms of different hosts are not consistent, and they may not exist.
The merge rules are complex and easily break the user's existing settings.
The baseline settings only need a summary in architecture.md or design.md.

**Have `wda init` create a local Git repository, create `.gitignore` only once, and create development.md on a condition.** When `init` succeeds, it creates a local Git repository in the project root.
It also creates the first commit.
It does not set a remote, and `git` is optional.
`init` creates a `.gitignore` whose content is fixed to `/dist/` and `/node_modules/`.
Later commands do not read or write that file.
`init` does not create development.md.
Only when the project has a dedicated workflow beyond the four commands does the project create development.md itself.

## Decision

After `wda init` succeeds, the project directory becomes a local Git repository when `git` is available. The first commit covers the whole directory, its message is fixed to `wda init`, and no remote is set. Initialization no longer requires the directory to be completely empty. Instead, it checks a specified collision set. The set contains the write targets, which are the two manifests `anchor.json` and `package.json`, `.gitignore`, and the Minimal or Spectrum starter files. It also contains the required directories, `.git/`, `node_modules/`, a leftover `wda.json` from an older project, and the lock or state files of the supported and commonly used package managers, which are `package-lock.json`, `npm-shrinkwrap.json`, `bun.lock`, `bun.lockb`, and `yarn.lock`. An existing entry in the set is a refused collision, and `init` never adopts an existing install. Other files are neither read nor modified.

When `git` is not on the PATH, `wda init` still creates the project, still writes `.gitignore`, creates no `.git`, and issues a Warning.

The AI Skill asks consent to install `git`; the user may decline. After decline, before the first visual change, the Skill states once "without git you cannot restore" and continues after consent. The Skill still runs no version-control commands. If the user later installs `git`, the Skill only explains they can run `git init` and commit the whole directory as the first commit, which is the restore baseline.

`wda init` creates a `.gitignore` in the project root, and its content is fixed to `/dist/` and `/node_modules/`. This file is only a convenience and is not a system dependency. Other commands do not rewrite or manage it. A missing or unlaunchable package manager does not block `wda check` or `wda build`, because they read only installed content. A missing embedded TypeScript cache is reported by the build, with a hint to run `wda tools fetch`.

The development.md of a Generated Project is a conditional document. `wda init` does not create it on its own. Only when the four WDA commands and the four required documents do not cover a project-specific workflow does the project create and maintain that file itself.

## Consequences

- When `git` is available, the Designer can use a clear local restore baseline. When `git` is absent, the project operates without a restore baseline until the user initializes Git. The Skill does not need to run version-control commands, and it must not run them.
- The test environment calls the real `git`. If the environment has no user identity configured, `init` creates the commit with a temporary identity and raises a Warning.
- Core does not manage the repository, so it respects the user's later changes to `.gitignore`.
- Lock files that a package manager creates after the first external install belong to that manager. The collision set refuses an existing lock file at init time, and WDA does not manage those files afterward.
- The required project documents stay at four. `wda check` validates only these four required documents. If the project creates development.md itself, `wda check` validates it under the rule for optional documents.

## Status History

- 2026-09-02: The Project Owner decided that `deno` is the only PATH prerequisite tool.
- 2026-09-08: The Project Owner revised this to two tools by adding `git`. On the same day the Architect reversed "Core never hard-binds git" to "Core hard-binds git". The reason is that reversible iteration needs a real restore baseline, and a local Git repository is the only implementation that is reliable enough.
- 2026-09-09: The implementation landed.
- 2026-09-18: Rewritten in the fixed five-section format. The decision is unchanged.
- 2026-08-24: Decided. At that time `init` accepted only an empty directory.
- 2026-09-08: The collision rule replaced the empty-directory premise, and `.gitignore` joined the collision set.
- 2026-09-18: Backfilled as an ADR.
- 2026-08-24: Decided.
- 2026-09-18: Backfilled as an ADR.
- 2026-10-14: Added the current handling in each command when `deno` or `git` is missing.
- 2026-09-21: Consolidated from existing atomic decisions. The sources are listed in the front matter `absorbs`.
- 2026-10-23: Translated to English. The decision is unchanged.
- 2026-11-06: Removed Deno as a prerequisite tool, made Git optional at initialization with a warning and consent flow, and updated the collision set and restore-baseline rules.
- 2026-11-12: Updated the current decision to the two-manifest collision set, the manager state files, `.gitignore` content that also ignores `/node_modules/`, and the build behavior without a tool on the PATH.
- 2026-11-12: Removed the statements that `wda build` and `wda deps` fail when a tool is missing, and the statement that `.gitignore` contains only `/dist/`, from the current decision.
