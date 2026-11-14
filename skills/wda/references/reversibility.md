# reversibility (Reversible Changes and Restore)

## When to Load

Load this file before one of two situations: the first visual change is about to be made, or the user asks for a restore. Do not load it otherwise.

## This File Does One Thing

This file sets two things. Before a visual change, confirm a usable baseline created by Core at `wda init` or later by the Designer. When restoring, say which version to restore to. A temporary directory is never a baseline. If no baseline exists, pause writes and explain the restore limit until the Designer gives explicit consent. Authority: the "Web Preview" section of `docs/architecture.md` and ADR-05.

## Actions (Run in Order)

1. **Before acting, confirm there is a restore point.** Before any visual change, confirm a usable baseline from `wda init` or one the Designer created later. If no baseline exists, pause writes and explain the restore limit until the Designer gives explicit consent. A temporary directory is not a baseline. Do not skip this step without the Designer's explicit confirmation. If a usable restore point already exists, for example one recorded in the previous round with no unrecorded change since, there is no need to record again.
2. **Make the visual change.**
3. **When restoring, restore to "the last version the Designer has seen and accepted", and not to any earlier or newer version.** A change the user has not confirmed seeing is not accepted state. A restore must not bring the project back to a version the Designer has never seen. The basis for judging "seen and accepted by the Designer" is the state at the moment the restore point was recorded in step 1, and not the latest state of the project at the moment of the restore.
4. **Verify after the restore.** Run `wda check` and confirm that the restored state has no Error. When an Error is reported, route to `references/troubleshooting.md`. This file does not restate diagnostic content. Authority: the "Web Preview" section of `docs/architecture.md`.
5. **Definition of accepted state.** `SEEN` means only that the Designer has seen the current state. It becomes `ACCEPTED` only after the Designer clearly confirms acceptance. The request that asks the Designer to record a new restore point after acceptance is issued at reporting time. See `references/reporting.md`.

## Boundary

A usable baseline may be created by Core at `wda init` or later by the Designer. The Skill runs no Git commands. Without a baseline it pauses writes and explains the limit until explicit consent. Authority: the "Web Preview" section of `docs/architecture.md` and ADR-05. This file also does not judge whether a given change counts as a visual change. That is the judgment of the caller (the main flow of the Skill and the conversation with the user).

## Supplement Revert Method

This section applies before the first edit of a supplement described in `references/install.md`. Authority: `docs/skill-spec.md`, "Skill Discovery and Installation Guidance".

When a Git baseline exists, the revert point is the baseline commit, or the last version the Designer accepted. The Designer restores with their own Git tools, and the Skill runs no version control command. State the revert point in the proposal.

When no usable baseline exists, including when Git was absent or refused at init and the Designer has not created one later, no baseline exists. Say so once, before the first edit, and state the limit: the Skill keeps the original text of each file it will change in this conversation, and a later session cannot revert from those copies. Pause writes until the Designer gives explicit consent to that limit. Do not ask about it again in the same supplement. Authority: `docs/skill-spec.md`.

Packages added by a manager are not restored by either method. The Designer removes them with the manager's native remove command. Lifecycle scripts of third-party packages can cause side effects that neither method reverts. State this in the proposal. Authority: `docs/skill-spec.md`.
