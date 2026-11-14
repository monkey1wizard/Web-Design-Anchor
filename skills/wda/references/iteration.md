# iteration (Visual Change Iteration)

## When to Load

Load this file before every visual change and for an explicit design review request. A review request alone does not authorize edits. Authority: `docs/skill-spec.md`.

## This File Does One Thing

This file sets the choice ladder for a single visual change, the disclosure of impact scope, the consistency judgment, and the decision marking. The visual review criteria still belong to `docs/design.md` of the Generated Project, and this file does not restate them. Authority: `docs/skill-spec.md`.

## Visual Change Actions (Run in Order)

For review-only requests, use Contextual Review below. Do not perform these change actions without edit authorization. Authority: `docs/skill-spec.md`.

1. **Choose the change ladder.** Try these in order: token, component style, page CSS, inline style. Before acting, say which rung this change uses. If the choice is not a token, also explain why it dropped down. Authority: `docs/skill-spec.md`.
2. **When the request names a reference site.** Before acting, ask the Designer back to confirm what is to be referenced this time. If the wording of the request already makes clear what it points to, state your interpretation and ask the Designer to confirm. If the wording does not make it clear, ask openly. However certain the interpretation is, do not change the project based on it before the confirmation arrives. After confirmation, write the reference source and the confirmed scope into `docs/design.md` of the Generated Project. If no matching section exists, add one. For the full intake question order and the split between content and style sources, see `references/intake.md`. Authority: `docs/skill-spec.md`.
3. **Handle the hidden state.** An author-level `display` rule overrides the user-agent `[hidden]` rule, so the hidden state needs an author-level rule to hold reliably. Authority: `docs/skill-spec.md`.
4. **Disclose the impact scope.** If the implementation scope goes beyond the visual scope the Designer specified, ask first, or immediately list the affected pages and regions in the Designer's language. Declare the side effects caused by the Designer's own edits. Do not fix them on your own without a request. Authority: `docs/skill-spec.md`.
5. **Keep consistency.** New content uses existing tokens and patterns and nothing else. A new token may be added only when the same step can be derived from the design system the project has declared. Explain the derivation that makes the light step and the dark step equivalent. In every other case, ask the Designer explicitly first. Authority: `docs/design-system.md`.
6. **Mark substituted judgment.** Mark every visual decision that the AI makes in place of the Designer with "I decided this; it can be changed". If a decision trades an accessibility criterion for a visual requirement, write it back to `docs/design.md` of the Generated Project. Do not leave it only in a code comment. Authority: `docs/design-system.md`.
7. **Write back to documents.** After the change is complete, write the decisions that need to keep applying back to `docs/`. Do not require the Designer to read the documents before continuing. Authority: `docs/skill-spec.md`.

## Contextual Review

For a design iteration or explicit review, review accessibility, fixed CSS values, and Token relationships. Keep CLI diagnostics separate from AI suggestions and identify each finding's file, basis, and unverified parts. A successful CLI check does not complete these reviews. Authority: `docs/skill-spec.md`.

For accessibility, distinguish source inspection from observed screen and keyboard interaction. Use a usable host browser for actual screen and keyboard results. If it is absent or fails, list unrun checks and manual guidance. Do not claim interaction success or full WCAG conformance from source inspection, and do not force a browser installation. Authority: `docs/skill-spec.md`.

Judge fixed values against first-party CSS, existing Tokens, purpose, theme, and cascade. Equal values alone do not establish semantic equivalence. Do not scan packages or `dist/` for suggestions or initiate rewrites. Review primitive, semantic, and component reference direction only when the project explicitly adopts that layering or the user asks, using the project's recorded rules. Authority: `docs/skill-spec.md`.

A review alone does not authorize edits. Preserve the requested scope, new-Token consent, and accessibility tradeoff disclosure rules above. Authority: `docs/skill-spec.md`.

## Boundary

This file does not judge whether the visuals of a project meet the criteria, and it does not create new visual criteria on its own. The criteria are carried only by `docs/design.md` of the Generated Project. Authority: `docs/design-system.md`.

This file does not decide, for the Designer, side-effect changes beyond the specified scope. It does not hide accessibility trade-offs in code comments. It does not add tokens quietly in the name of consistency. Authority: `docs/design-system.md`.
