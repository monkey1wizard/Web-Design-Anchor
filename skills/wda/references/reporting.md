# reporting (Reporting Duties)

## When to Load

Load this file when reporting a `wda` result or a design review, or when judging inline preview ability. Authority: `docs/skill-spec.md`.

## This File Does One Thing

Report CLI diagnostics, review findings, and preview results separately. Include what was observed, inferred, and not run. Authority: `docs/skill-spec.md` and the "Web Preview" section of `docs/architecture.md`.

## Actions (Run in Order)

1. **Capture stdout, stderr, and exit status for every `wda` call.** In human mode preserve the original diagnostic channels and read stderr even on exit 0. In JSON mode parse the single stdout document as schema version 1 and combine exit status with `status`, `diagnostics`, and `summary`. Report every Warning, including on exit 0. Missing, malformed, or unsupported JSON is not success. Report CLI errors and ToolFault truthfully and route them to troubleshooting. Authority: `docs/skill-spec.md`.
2. **Report an artifact only for a successful build with this run's `dist_path`.** Resolve that path against the project root and report it as an absolute path. Never infer success or an artifact from an old `dist/`. Authority: `docs/skill-spec.md`.
3. **Separate CLI diagnostics from AI analysis.** Identify files and the basis for suggestions, observed results, and unverified or unrun checks. CLI success does not complete a design review. Authority: `docs/skill-spec.md`.
4. **Passing technical checks does not mean the design is complete.** Every report of a visual change attaches the preview location and states clearly "not yet seen by the Designer, so not counted as accepted". A change the user has not confirmed seeing does not count as accepted state.
5. **After `wda build` succeeds with this run's `dist_path`, run the preview according to host ability.** Report the absolute path of `dist/` and point out that `index.html` inside it is the entry. The preview must serve the project root directory and never uses `dist/` as the document root. It binds loopback only and uses HTTP/1.1 keep-alive or newer. When the host has preview ability, open the page with that ability and confirm at least the first viewport. If screenshots are unreliable, verify by DOM measurement instead and say that the Designer still has to confirm the rendered view. The AI does not start a long-running server on its own.
6. **When the host has no preview ability, announce the downgrade.** Deliver the absolute path of `dist/` together with the four rules above. Do not declare a visual change complete when the user has not seen it. This is an explicit downgrade and not a silent skip.
7. **After the state is accepted, ask the Designer to record the current state as a new restore point.** For the mechanism, see `references/reversibility.md`. This file does not restate it. Authority: the "Web Preview" section of `docs/architecture.md`.

## Boundary

This file does not judge whether the host has preview ability. The caller (the main flow of the Skill) chooses between the "open and verify" branch and the "explicit downgrade" branch according to the actual host environment. This file sets what each of the two branches reports. This file also provides no `file://` fallback and no version control advice. Those belong to a fixed limit of Core and to `references/reversibility.md` respectively. The diagnostic procedure for when `check` or `build` reports an Error is not in this file. Route it to `references/troubleshooting.md`. Authority: the "Web Preview" section of `docs/architecture.md`.
