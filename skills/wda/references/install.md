# install (Install and First-Use Routing)

## When to Load

Load this file for an explicit WDA install, setup, or new project request when no `wda.json` exists yet. Also load it when `wda` is not on PATH. Also load it when Core reports a diagnostic that starts with `wda.toolchain.`. A general website request does not apply.

## Source and Probing

First read the "Installation Guide" section of the accessible `README.md`. If that source cannot be found, ask the user for a readable official WDA document. Stop until it is provided and do not guess commands. Authority: `docs/skill-spec.md`.

Before any WDA command, handle `wda --version` according to the version comparison rule of this Skill. Then probe Git, Deno, and WDA in a read-only way. For every process that starts, record stdout, stderr, and the exit status. When a process cannot start, record only the start error. When a process cannot start, do not invent an exit status or infer from it that an install is needed. Authority: `docs/skill-spec.md`.

## Install and Follow-Up Actions

Use one summary to state separately which tools are available, which are not found on PATH, and which are found but cannot start. Offer install options only for tools confirmed as not found on PATH, using the commands that README shows. Before each command, state the text, the publisher, the install target, the network access, and any possible system permission. Obtain a separate yes/no consent for each command before running it. Do not ask for a password. Authority: `docs/skill-spec.md`.

After each consented install finishes, rerun `--version` for that tool. An installer exit status of `0` does not mean the tool is ready. If the tool is still not found when `--version` is rerun, ask the user to open a new terminal or reload the environment, then recheck `--version` for that tool. Report the actual observed result and stop. Until the tool passes the version check, do not claim that it is ready. When the user asked only for an install, stop after verification. Creating a project, the limited initialization when Deno is absent, and `wda deps add lit` each need a new consent. After creation, check whether `deno.jsonc` declares `lit` and whether `deno.lock` exists. The project can be called declared and locked only when `deno.jsonc` declares `lit` and `deno.lock` exists. Authority: `docs/skill-spec.md`.
