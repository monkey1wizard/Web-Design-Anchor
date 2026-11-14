# Changelog

This file documents all notable changes to WDA. The public repository
receives a curated snapshot of the private source repository, and this file is
the authoritative release record for that snapshot.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-11-14

### Breaking

- WDA no longer uses Deno, and it no longer needs Node.js or Bun. WDA Minimal
  needs none of them. The default Spectrum and Lit packages use npm or Bun,
  which must already be installed on PATH.
- Projects created with `wda.json`, `deno.jsonc`, or `deno.lock` are not
  migrated automatically. Create a new project with `wda init`, or manually
  rewrite the manifests as `anchor.json` and `package.json` using the current
  project contract.
- `wda deps` is removed. Running it prints a usage error that names the
  external package manager. Add, remove, and update website packages with
  `npm install`, `bun add`, or the manager's own commands.

- An installed Skill may still use the old contract after the CLI is updated,
  even when version strings match. Replace the installed `skills/wda/` copy
  with the Skill from the same release or source revision as the CLI.
- The WDA token profile is validated more strictly. `wda check` and
  `wda build` now reject token values whose type the profile does not accept,
  and a rejected token stops the build without writing `dist/`. A project that
  relied on a previously accepted value may now fail validation. Fix the
  reported token source, then run `wda check` again.

### Added

- `anchor.json` holds the WDA settings, the Design System in use, and the
  fallback record. `package.json` is the only place that declares website
  packages.
- `wda init --package-manager <name>` selects `npm` or `bun`. Without the
  option, `init` picks the single usable manager, asks on an interactive
  terminal when several are usable, and creates WDA Minimal when none is usable
  or the registry cannot be reached.
- `wda check --json` and `wda build --json` print a stable JSON report on
  stdout. A tool fault, which is a failure of WDA itself rather than a project
  diagnostic, is also reported as JSON in this mode.
- The Skill reads the JSON report of `wda check` and `wda build`, and it adds a
  contextual review flow for the reported diagnostics.
- `wda check` shares one include analysis with the build and keeps the source
  location that the author wrote, so a diagnostic points at the authored file
  and line.
- `wda tools fetch` restores the one TypeScript tool version that this WDA
  release embeds, for the current platform. It takes no arguments.

### Changed

- `wda check` and `wda build` never install packages and never use the
  network. The build bundles the packages that the project's package manager
  has already installed, with a native Rust bundler.
- Git is optional. When Git is missing or cannot start, `wda init` still
  creates the project and reports a warning.
- The public install guidance, the README, SECURITY, and the translations
  describe these requirements.

- The embedded tool cache key is shorter, so the cache path stays under the
  Windows path length limit.

### Removed

- The Deno-based build path and the external esbuild dependency.

### Maintainer

- A permission-isolated compatibility watch workflow probes real npm and Bun
  installs. It does not gate releases.

## [0.1.4] - 2026-10-30

### Added

- Traditional Chinese and Japanese translations of the README, SECURITY, and
  contract documents in `docs/i18n/zh-TW/` and `docs/i18n/ja/`.
- English and Japanese pages for the official website. The English,
  Traditional Chinese, and Japanese routes are `index.html`, `zh-TW.html`,
  and `ja.html`.
- First-visit language detection for the official website, with manual
  language selections taking priority on later visits.

### Changed

- The README, SECURITY, the contract documents in `docs/`, and the WDA Skill
  are now written in English. English is the canonical version, and the
  translations name the English file they follow.
- The README and its translations open with the tagline "Build websites with
  AI. Keep design decisions in your project."
- The README states what you need to build a site without writing code: an AI
  coding tool that can run commands on your computer, such as Claude Code or
  Codex, and Git and Deno.
- The README no longer claims that the AI always follows the design rules,
  that a loaded Skill makes the AI follow the workflow automatically, that
  `wda check` reports every deviation, or that `dist/` has zero dependencies.

### Fixed

- The README Quick Start ran `wda init my-site`, which the CLI rejects. It now
  creates and enters a folder and runs `wda init`. The description of the
  created project now lists `pages/index.html` and `tokens/tokens.json`
  instead of a root `index.html` and a components directory.
- The official website lists the languages in the same order on every page.

The `wda` CLI behaves the same as in 0.1.3.

## [0.1.3] - 2026-10-21

### Changed

- The README opens with a short demo animation of the Designer and AI
  workflow. The conversation in the animation is an example.
- The WinGet package is still under review, so the README no longer lists
  `winget install Monkey1Wizard.WDA`. Windows users install with the
  PowerShell installer.
- The website marks the WinGet install route as in progress, with a muted
  command and no copy button.

The `wda` CLI behaves the same as in 0.1.2.

## [0.1.2] - 2026-10-17

### Breaking

- Projects must rename `docs/naming.md` to `docs/glossary.md`. WDA no longer
  accepts `docs/naming.md` and provides no compatibility handling. See
  `docs/generated-project.md` for the migration.

### Changed

- The generated glossary now lives at `docs/glossary.md`, and it renders its
  Official Name from the selected Design System.
- The documentation was refreshed for clearer wording and current behavior,
  including the README, the ADRs, and the generated-project templates.
- The website homepage and README lead with a plain-language summary for
  non-engineers.
- The website adds feedback motion and evidence replays. The arrival underline
  now matches the heading text width, and two-column tops align to the title
  cap line at the large breakpoint.

## [0.1.1] - 2026-10-14

### Fixed

- Running `wda` with no arguments prints usage to stdout and exits with
  status `0`. Earlier builds reported the executable path as an unknown
  command and exited with status `2`.

### Changed

- The `website` page now presents the four steps
  from one sentence to `dist/`, with a single top bar.

## [0.1.0] - 2026-10-05

The first WDA release provides a standalone CLI for creating and maintaining
framework-independent Standard Web projects.

### Added

- Four CLI commands: `wda check`, `wda init`, `wda build`, and `wda deps`.
  The `deps` command supports `add`, `remove`, and `update`.
- `wda --version`, which reports the installed WDA version.
- Two built-in Design System paths: WDA Minimal and Spectrum 2. The six
  Spectrum 2 direct dependencies are resolved and pinned together when that
  adapter is selected.
- Build targets for six architectures and platforms: x86_64 Windows,
  aarch64 Windows, x86_64 macOS, aarch64 macOS, x86_64 Linux musl, and
  aarch64 Linux musl.
- A build pipeline that requires `deno` on PATH and invokes
  `npm:esbuild@0.25.5` through Deno. The binary's build logic pins `esbuild`,
  which is not required as a separate PATH tool.
- Project initialization that requires `git` on PATH, creates a local Git
  repository, and records the initial project state.

### Changed

- `wda check` reports named toolchain warnings when Git or Deno is missing or
  cannot start. `wda init` keeps the WDA Minimal fallback when Deno is absent.
- The release pipeline publishes a curated public snapshot with checksums,
  cosign signatures, installer scripts, and six platform targets.

### Format

- `wdaVersion` `0.1.0` is the **Current format**. No legacy project format is
  implemented in this release.

### Known limitations

- When `wda` is not on PATH, `skills/wda/references/install.md` routes the AI
  to `docs/install.md`, but `docs/install.md` does not yet exist. The Skill
  must stop and report this gap rather than invent installation steps.
