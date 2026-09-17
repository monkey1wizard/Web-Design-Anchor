# Changelog

All notable changes to WDA are documented in this file. The public repository
receives a curated snapshot of the private source repository, and this file is
the authoritative release record for that snapshot.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

No unreleased changes.

## [0.1.0] - 2026-09-16

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
  `npm:esbuild@0.25.5` through Deno. `esbuild` is pinned inside the binary's
  build logic rather than required as a separate PATH tool.
- Project initialization that requires `git` on PATH, creates a local Git
  repository, and records the initial project state.

### Format

- `wdaVersion` `0.1.0` is the **Current format**. No legacy project format is
  implemented in this release.

### Known limitations

- When `wda` is not on PATH, `skills/wda/references/install.md` routes the AI
  to `docs/install.md`, but `docs/install.md` does not yet exist. The Skill
  must stop and report this gap rather than invent installation steps.
