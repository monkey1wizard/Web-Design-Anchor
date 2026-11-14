# troubleshooting (Reporting and Handling Errors)

## When to Load

Load this file when `wda check` or `wda build` reports an Error. Do not load it otherwise.

## This File Does One Thing

This file sets how to report and handle an Error when one arrives, so that key information is not lost and the state is not misjudged. **The diagnostic codes and message text are not in this file.** Their sole declaration source is `src/codes.rs`. This file does not repeat, rewrite, or give an example of any diagnostic code or message content. Authority: the "Diagnostic Output" section of `docs/commands.md`.

## Actions (Run in Order)

1. **Read all of stderr and do not look at the exit code alone.** An Error always appears in stderr. The exit code is used only to judge whether there is an Error and cannot replace reading the content.
2. **Report the received diagnostic content verbatim. Do not summarize, rewrite, or omit fields.** Every diagnostic carries a diagnostic code, a message, and, where applicable, a project-relative path and a line and column position. Pass these fields to the user as they are. Do not say only "there is an error" without the actual content, and do not invent statements beyond the content.
3. **After the verbatim diagnostic, add a translation. The translation does not replace the original.** State clearly "what happened", "what it affects", "what will be done next", and "whether the user needs to decide". Translate only what was received this time and do not fill in diagnostic codes or messages on your own. Authority: the "Diagnostic Output" section of `docs/commands.md`.
4. **Split the handling path into three: I can fix it, you need to decide, an engineer is needed.** "I can fix it" means it can be handled directly from the diagnostic content and the next action. "You need to decide" means there are several reasonable directions, or a scope or an accepted risk needs the user's ruling. "An engineer is needed" means it goes beyond the boundary of this handling, so hand it to a plan or issue of the WDA repo and attach the full diagnostic, the translation, and the handling already tried. Authority: the "Diagnostic Output" section of `docs/commands.md`.
5. **Act on the next action built into the message. Do not skip the message and decide on your own how to fix it.** The message and the next action are guaranteed to exist and be understandable, and the verbatim content may change across versions. Act on the content received this time and do not apply an old version or a remembered wording.
6. **When there is an Error, do not treat it as safe to continue to build or deploy.** An Error means that build or project correctness is affected. This is a blocking condition and not a hint that can be ignored.
7. **After handling, rerun the same command and confirm the Error is cleared.** Do not judge "it should be fixed" from the edit alone. Have `wda` run the same command once more to verify.
8. **If the same Error is reported again after the rerun, or the edit seems to have no effect, give the full diagnostic content together with the handling already tried to the user, and do not retry on your own until it converges.** This is a case beyond the boundary of judgment. Route it to the user for a ruling. Authority: the "Diagnostic Output" section of `docs/commands.md`.

## Boundary

This file does not interpret the meaning of any specific diagnostic code or message text. It also lists no classification, example, or fix lookup table. Once such content is written into this file, it would duplicate the declaration in `src/codes.rs`. This file also does not judge whether a situation counts as an Error. That is the result `wda` itself decides by its severity rules. This file covers what to do after an Error has been received and nothing else. The reporting duty for a Warning is not in this file. Route it to `references/reporting.md`.

## Supplement Failures

Load this section when a step of the supplement in `references/install.md` fails, or when a manager or the registry reports a failure. Authority: `docs/skill-spec.md`.

1. **Report each failed step with its observed output.** Give the manager's stdout, stderr, and exit status for the failed step. For a registry failure, give the classification the manager reports, such as unreachable, authorization refused, rate limited, server error, TLS failure, or a malformed response. Do not merge these classes into one message.
2. **Do not claim success for any part that did not finish.** Say which packages were added, which files were changed, and which checks ran. Say which of these did not finish.
3. **Keep the project usable.** When integration failed, restore the files changed in this supplement to the revert point. Use `references/reversibility.md`. Leave `initialization.fallback` in `anchor.json` as it was. Authority: `docs/skill-spec.md`.
4. **Run `wda check` after any restore and report the result.** When that run reports an Error, handle it under the rules above.
5. **Ask the Designer how to continue.** The options are a retry of the same step, keeping the Minimal project, or removing the added packages with the manager's native remove command. Do not choose one of them on your own. Authority: `docs/skill-spec.md`.
