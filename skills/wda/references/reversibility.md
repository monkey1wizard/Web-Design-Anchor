# reversibility (Reversible Changes and Restore)

## When to Load

Load this file before one of two situations: the first visual change is about to be made, or the user asks for a restore. Do not load it otherwise.

## This File Does One Thing

This file sets two things. Before a visual change, confirm that Core created the baseline of the local repository at `wda init`, because that is what guarantees a later restore. When restoring, say which version to restore to. A temporary directory is never a baseline. If the baseline does not exist, stop work and hand the choice of creating a baseline first to the Designer. Authority: the "Web Preview" section of `docs/architecture.md` and ADR-05.

## Actions (Run in Order)

1. **Before acting, confirm there is a restore point.** Before any visual change, confirm that `wda init` created the baseline of the local repository. If the baseline does not exist, stop work immediately and hand the choice of creating a baseline first to the Designer. A temporary directory is not a baseline. Do not skip this step without the Designer's explicit confirmation. If a usable restore point already exists, for example one recorded in the previous round with no unrecorded change since, there is no need to record again.
2. **Make the visual change.**
3. **When restoring, restore to "the last version the Designer has seen and accepted", and not to any earlier or newer version.** A change the user has not confirmed seeing is not accepted state. A restore must not bring the project back to a version the Designer has never seen. The basis for judging "seen and accepted by the Designer" is the state at the moment the restore point was recorded in step 1, and not the latest state of the project at the moment of the restore.
4. **Verify after the restore.** Run `wda check` and confirm that the restored state has no Error. When an Error is reported, route to `references/troubleshooting.md`. This file does not restate diagnostic content. Authority: the "Web Preview" section of `docs/architecture.md`.
5. **Definition of accepted state.** `SEEN` means only that the Designer has seen the current state. It becomes `ACCEPTED` only after the Designer clearly confirms acceptance. The request that asks the Designer to record a new restore point after acceptance is issued at reporting time. See `references/reporting.md`.

## Boundary

The baseline is created by Core at `wda init`. The Skill runs no version control command, and when the baseline does not exist it stops work. Authority: the "Web Preview" section of `docs/architecture.md` and ADR-05. This file also does not judge whether a given change counts as a visual change. That is the judgment of the caller (the main flow of the Skill and the conversation with the user).
