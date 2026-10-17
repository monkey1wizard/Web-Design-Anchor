# Changelog

This file documents all notable changes to WDA. The public repository
receives a curated snapshot of the private source repository, and this file is
the authoritative release record for that snapshot.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

No unreleased changes.

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
