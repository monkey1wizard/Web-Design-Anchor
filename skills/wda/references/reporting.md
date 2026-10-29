# reporting (Reporting Duties)

## When to Load

Load this file when a reporting duty is triggered. Triggers include: `wda build` has finished, a Warning appears in stderr of any `wda` call, or there is a need to judge whether the host has inline preview ability. Do not load it otherwise.

## This File Does One Thing

This file sets the three kinds of information to report when a reporting duty is triggered: the absolute path of `dist/` after `wda build` succeeds, any Warning that appears in stderr, and the preview result, which covers the preview location, what was verified, or the explicit downgrade when the host has no preview ability. Authority: the "Web Preview" section of `docs/architecture.md`.

## Actions (Run in Order)

1. **Every `wda` call must capture stdout, stderr, and exit status.** Even when the command ends with exit 0, read stderr, because a Warning appears only in stderr. The exit code by itself does not mean success. Judge by neither the exit code alone nor stdout alone.
2. **After `wda build` succeeds, report the absolute path of `dist/`.** This is the minimum guarantee. Any host that can run `wda` can receive this line of output, so do not omit it.
3. **When a Warning appears in stderr, report that Warning.** If exit 0 comes with a Warning in stderr, the Warning itself is still reported. Do not report "everything is fine".
4. **Passing technical checks does not mean the design is complete.** Every report of a visual change attaches the preview location and states clearly "not yet seen by the Designer, so not counted as accepted". A change the user has not confirmed seeing does not count as accepted state.
5. **After `wda build` succeeds, run the preview according to host ability.** Report the absolute path of `dist/` and point out that `index.html` inside it is the entry. The preview must serve the project root directory and never uses `dist/` as the document root. It binds loopback only and uses HTTP/1.1 keep-alive or newer. When the host has preview ability, open the page with that ability and confirm at least the first viewport. If screenshots are unreliable, verify by DOM measurement instead and say that the Designer still has to confirm the rendered view. The AI does not start a long-running server on its own.
6. **When the host has no preview ability, announce the downgrade.** Deliver the absolute path of `dist/` together with the four rules above. Do not declare a visual change complete when the user has not seen it. This is an explicit downgrade and not a silent skip.
7. **After the state is accepted, ask the Designer to record the current state as a new restore point.** For the mechanism, see `references/reversibility.md`. This file does not restate it. Authority: the "Web Preview" section of `docs/architecture.md`.

## Boundary

This file does not judge whether the host has preview ability. The caller (the main flow of the Skill) chooses between the "open and verify" branch and the "explicit downgrade" branch according to the actual host environment. This file sets what each of the two branches reports. This file also provides no `file://` fallback and no version control advice. Those belong to a fixed limit of Core and to `references/reversibility.md` respectively. The diagnostic procedure for when `check` or `build` reports an Error is not in this file. Route it to `references/troubleshooting.md`. Authority: the "Web Preview" section of `docs/architecture.md`.
