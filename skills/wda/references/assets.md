# assets (Receiving Design Assets)

## When to Load

Load this file when the Designer provides assets such as a logo, fonts, or image files. Do not load it otherwise.

## This File Does One Thing

This file says where an asset goes in the project and how to verify the result after placement. The asset directory structure and the naming rules are not restated here. They belong to the project structure that `wda init` generates.

## Actions (Run in Order)

1. **Confirm the purpose of the asset.** First state whether the asset is a logo, a font, an icon, a content image, a background, a favicon, or a share preview image, and on which page and region it appears. If the purpose is unclear, ask the Designer first. Do not substitute a guess for the purpose. Authority: `docs/design-system.md`.
2. **Confirm the asset directory.** The location for the asset is decided by the project structure that `wda init` generates. This file sets no path or naming convention of its own. If unsure where to put it, read the existing project structure for the answer. Do not invent a new asset directory convention. Authority: `docs/skill-spec.md`.
3. **Choose a semantic file name.** Choose a name that a person can read and that stays maintainable over time, based on the purpose and content. Do not use names such as `final`, `new`, `image1`, or a name that is only a size. If the same asset has several versions, explain the difference between versions before choosing a name. Authority: `docs/skill-spec.md`.
4. **Place and integrate the asset.** Put the file in the location confirmed in step 2. Connect the file to the page, style, or token that actually uses it. During integration, confirm that the reference path matches the purpose. Do not just drop the file into a directory. Authority: `docs/design-system.md`.
5. **Add accessibility context.** Provide the alternative text, visible text, decorative marking, text contrast, or focus context that the purpose requires. Background and decorative assets must not carry the sole piece of information. If the information the asset should convey cannot be determined, ask the Designer first and do not invent meaning. Authority: the "wda build" section of `docs/commands.md`.
6. **Check the common pitfalls.** Check and report at least these five items: a PNG logo with a white background used on a dark bar, a portrait photo placed in a banner, a face cropped at the edge of the frame, a missing favicon, and a missing share preview image. When any item is found, first explain the impact and the available fixes. Do not quietly change the purpose of the asset for the Designer. Authority: `docs/design-system.md`.
7. **Disclose size risk.** If the file size or pixel size of an asset clearly exceeds the actual use, report the file size, the pixel size, and the affected pages. List these options: keep the original file, have the Designer provide a smaller version, or leave it to a later tooling flow. Core does not convert assets. The Skill discloses the risk and the options and does not pretend that optimization is done. Authority: `docs/skill-spec.md`.
8. **Verify the result.** Run `wda check`. When it reports an Error, stop and route to `references/troubleshooting.md`. Separately verify that the source asset exists and that each reference points to the intended file. When existing authorization permits a build, you can run `wda build` to check the references the builder resolves. This file does not restate diagnostic content. Authority: the "Diagnostic Output" section of `docs/commands.md`.
9. **Restore precondition.** If this asset change involves the first visual change, or a restore may be needed later, read `references/reversibility.md` before acting. Authority: `docs/skill-spec.md`.

## Boundary

This file must not contain any template content that could be copied directly into a project file, such as an `anchor.json` fragment, a front matter template, or a directory skeleton example. Generating project files is entirely the duty of `wda init`. This file also does not judge whether an asset fits the style of the project. That belongs to the visual review criteria in `docs/design.md`. This file covers where the asset goes and how to verify it. Authority: `docs/skill-spec.md`.
