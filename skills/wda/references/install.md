# install (Install and First-Use Routing)

## When to Load

Load this file for an explicit WDA install, setup, or new project request when no `anchor.json` exists yet. Also load it when `wda` is not on PATH. Also load it when Core reports a diagnostic that starts with `wda.toolchain.`. Also load its supplement section when `anchor.json` records `initialization.fallback`, or when the Designer asks to add Spectrum or Lit to an existing project. A general website request does not apply.

## Source and Probing

First read the "Installation Guide" section of the accessible `README.md`. If that source cannot be found, ask the user for a readable official WDA document. Stop until it is provided and do not guess commands. Authority: `docs/skill-spec.md`.

Before any WDA command, handle `wda --version` according to the version comparison rule of this Skill. Probe Git as needed. Probe npm or Bun only for a supplement. Missing Git permits project creation without a local baseline. For every process that starts, record stdout, stderr, and the exit status. When a process cannot start, record only the start error. When a process cannot start, do not invent an exit status or infer from it that an install is needed. Authority: `docs/skill-spec.md`.

## Install and Follow-Up Actions

Use one summary to state separately which tools are available, which are not found on PATH, and which are found but cannot start. Offer install options only for tools confirmed as not found on PATH, using the commands that README shows. Before each command, state the text, the publisher, the install target, the network access, and any possible system permission. Obtain a separate yes/no consent for each command before running it. Do not ask for a password. Authority: `docs/skill-spec.md`.

After each consented install finishes, rerun `--version` for that tool. An installer exit status of `0` does not mean the tool is ready. If the tool is still not found when `--version` is rerun, ask the user to open a new terminal or reload the environment, then recheck `--version` for that tool. Report the actual observed result and stop. Until the tool passes the version check, do not claim that it is ready. When the user asked only for an install, stop after verification. Project creation needs separate consent. Authority: `docs/skill-spec.md`.

## Supplement After a Minimal Fallback

Load this section when `anchor.json` records `initialization.fallback`, or when the Designer asks to add Spectrum or Lit to an existing WDA project. The fallback record has two fields. `requestedDesignSystem` names the system that init tried first, and `reason` gives the cause that init reported. Authority: `docs/skill-spec.md`, "Skill Discovery and Installation Guidance".

Work through the steps below in order. Each step is a separate stop point for the Designer where a consent is needed. Authority: `docs/skill-spec.md`.

### 1. Read the Current State

Read `anchor.json` and record `designSystem` and `initialization.fallback`. Read the `dependencies` and `packageManager` fields of `package.json`. Run `wda check` and read stdout, stderr, and the exit status. Report these observed facts before you propose any change. Do not reuse the reason from the earlier init run as a current fact.

### 2. Recheck the Conditions

Check each condition again at the time of the request.

- Run the version command of each manager that may be used, and report the printed version.
- Check whether the registry can be reached for the default packages, and report the observed result.
- List the existing pages, stylesheets, token files, and each use of the current design system.

When no manager starts, explain that the missing manager is the condition, and stay on the Minimal project. When the registry is unreachable, name the cause as "registry unreachable". Do not claim that the machine is offline. When several managers start, ask the Designer which one to use. The Skill does not install Node.js or Bun, and it does not force a manager. A reconnect does not start the supplement by itself. Authority: `docs/skill-spec.md`.

### 3. Propose the Scope and the Revert Method

Before any change, state the following in one message.

- The packages to add, the manager to use, and the exact command for each install step.
- Each project file that will change, including `anchor.json`, `package.json`, the manager lock file, and each page or stylesheet that integration touches. Include the record update and its restoration if the final check fails.
- The revert method from `references/reversibility.md`, including the no-git limit when Git is absent.

When the project has no `packageManager` field, the proposal names the manager name and exact version to record there. Ask for consent to the integration scope. Integration adds imports and component use. It does not replace existing page content or the existing design. Authority: `docs/skill-spec.md`.

### 4. Install with the Manager's Native Commands

Each install command needs its own yes/no consent, as the section above requires. After consent, run only the consented command with the manager's native add syntax, and read stdout, stderr, and the exit status. Rerun the manager's version command after the install. An installer exit status of `0` does not mean the packages are usable. Do not rerun `wda init`. Do not edit the manager lock file by hand. Authority: `docs/skill-spec.md`.

### 5. Integrate Within the Consented Scope

Change only the files named in the consented scope. Keep existing pages, stylesheets, and tokens. Add the component imports and their use where the Designer approved them. Authority: `docs/skill-spec.md`.

### 6. Verify, Then Update the Record

Run `wda check`, then run `wda build`. Read stdout, stderr, and the exit status of each run. Report each result.

Only when both runs finish without an Error and Spectrum integration is complete, update `anchor.json`. Set `designSystem` to the name and exact installed version, then remove the `initialization` wrapper. Run `wda check` once more and report the result. If that final check fails, restore only the prior `designSystem` and `initialization` within the consented scope, retain the successful build's `wdaVersion`, report the failure, and run `wda check` again. A Lit-only supplement retains both fields. Authority: `docs/skill-spec.md`.

When an earlier step fails, or the Designer refuses integration, leave the prior `designSystem` and `initialization` unchanged. Follow `references/troubleshooting.md` for the failure. Apply the manager side-effect limits in `references/reversibility.md`. Authority: `docs/skill-spec.md`.
