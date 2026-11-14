# Web Design Anchor Project

> Project documentation entry point and getting started instructions.

## About this website

This project is the official Web Design Anchor (WDA) website and a reference implementation. It provides Traditional Chinese, English, and Japanese pages and is built with WDA and the Spectrum 2 design system.

- **Audience:** designers who work with an AI coding agent and do not write code, and engineers who arrive from GitHub or a shared link. Both matter equally.
- **Desired action:** the visitor installs WDA and starts a first project.
- **Positioning:** the output is clean and does not lock you in. `wda build` emits a plain static `dist/` with no framework, and the deployed site does not depend on WDA at runtime. The page proves this with real `wda build` output instead of claims.

## Getting Started

This Web Design Anchor (WDA) project uses standards-first HTML5, Modern CSS, and TypeScript.

### Project Initialization and Version Control

This project was initialized by `wda init`. The project is already a local git repository. Its first commit covers the whole directory (including any files that were in place), and the project has no configured remote. There is no separate step to set up version control.

## Development Commands

The website packages are managed by npm. `package.json` declares the dependencies, and `package-lock.json` is the lockfile that npm owns. Run `npm ci --ignore-scripts` in `website/` to install exactly the locked packages before building. Use `npm install <package>` to add or update a package, which changes `package.json` and the lockfile.

- `wda tools fetch` — Restore the WDA TypeScript tool cache once, which type checking needs.
- `wda check` — Validate project structure, required documents, DTCG tokens, and HTML compliance. It reads installed packages and does not install them.
- `wda build` — Compile static deployment artifacts into `dist/`. It does not install or update packages, and it does not rewrite `package.json` or `package-lock.json`.

## Preview

After `wda build` succeeds, ask the AI host to serve `dist/` over local HTTP. Inspect all three locale routes and their first viewports. A successful build is not visual acceptance.

- Open `index.html`, `zh-TW.html`, and `ja.html` through the local HTTP server and verify the module-based interactions on each page.

## Languages and routes

The website provides English at `index.html`, Traditional Chinese at `zh-TW.html`, and Japanese at `ja.html`. Each route is owned by its matching source page in `pages/`: `pages/index.html`, `pages/zh-TW.html`, and `pages/ja.html`. Keep the three pages aligned whenever shared meaning or site structure changes. A change to wording in one language may be localized, but it must preserve the same information and intent in the other two pages in the same change.

Every page's language menu links directly to all three routes: English, 繁體中文, and 日本語. The links work as ordinary page navigation, including when JavaScript is disabled. For English terminology, use `docs/glossary.md` from the WDA repository root. For Japanese terminology, use `docs/i18n/ja/glossary.ja.md` from the WDA repository root.

On the first visit to the English default route, JavaScript reads `navigator.languages`. It redirects Japanese preferences to `ja.html` and Traditional Chinese preferences to `zh-TW.html`. Other preferences remain on the English page. A manual language-menu selection is stored in `localStorage` and takes priority on later visits. If JavaScript or storage is unavailable, the English route remains the default and all language links continue to work.

Before publishing a content or layout change, visually review all three routes at narrow and wide widths in light and dark modes. Check translated text wrapping, clipping, navigation, and section alignment. `wda build` success does not replace this review. See `docs/design.md` for the review criteria.

## Project Structure

- `anchor.json` — WDA project settings, the actual Design System, and the initialization fallback record.
- `package.json` — Website dependencies. The npm lockfile is `package-lock.json`.
- `pages/` — Semantic HTML entry points for MPA routes.
- `tokens/` — Design token definitions in W3C DTCG format (`tokens/tokens.json`).
- `docs/` — Core project documentation (`architecture.md`, `design.md`, `glossary.md`).
- `dist/` — `wda build` produces isolated static output (untracked in Git).
