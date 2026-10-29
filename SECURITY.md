This document covers the vulnerability reporting process for the WDA tool and its public source code. It does not cover individual websites produced with WDA or their deployment environments.

## Supported Versions

Public GitHub Releases provide the stable version `v0.1.3` and prereleases. A prerelease is a test version and may change. See [GitHub Releases](https://github.com/monkey1wizard/Web-Design-Anchor/releases) for the versions and assets.

`gal-last-good` is a development tag, not a release.

Every security fix goes into `main` of the private source repository. The four-level format compatibility ladder in the "Version Compatibility" section of `docs/architecture.md` describes only project format compatibility. It does not represent any security support level. `Supported legacy format` does not equal a security support tier.

## Reporting a Vulnerability

The public repository does not currently enable GitHub private vulnerability reporting, so no private reporting channel is available. Do not disclose vulnerability details in a public issue.

We handle reports with best-effort coordinated disclosure. We provide no fixed-day SLA and no separate email reporting channel.

## Security Model

This section lists only the security boundaries that have landed.

The rules themselves have a single source in [`docs/architecture.md`](docs/architecture.md) and are not restated here.

### Assets

- The project contract, the source tree, and the `dist/` deployment output. Their boundaries and integrity are protected according to `docs/generated-project.md`, its "`wda.json`" section, and `docs/design-system.md`.
- The dependency declaration, the lock state, and build-time composition. Their sources and resolution results are protected according to the "wda build" section of `docs/commands.md` and the "wda deps" section of `docs/commands.md`.
- The Skill package and its host process contract. Their responsibility boundary is protected according to `docs/skill-spec.md`.

### Trust Roles

- The Designer owns the project source and the design decisions. The AI and Skill are collaborators that operate by routing through the authoritative documents. The Core CLI is the tool that performs the validation, init, dependency, and build duties. For the division of roles, see the "Core Principles" section of `docs/architecture.md`, `docs/commands.md`, and `docs/skill-spec.md`.
- The AI host is the external environment that runs the Skill. WDA does not treat it as a trusted runtime. Its assumed capability is limited to running a native binary and capturing stdout, stderr, and the exit status. See `docs/skill-spec.md`.

### Trust Boundaries

- The Designer, AI, and Skill are separated from the WDA Core CLI by the CLI responsibilities and the output contract. See the "Diagnostic Output" section of `docs/commands.md`.
- The source, the generated `dist/`, and the deployment environment are separate trees and responsibility boundaries. See `docs/generated-project.md`.
- Between the Skill and the AI host lies the boundary of the installed copy and the process contract. `scripts/` and `assets/` are not part of the Skill product. See `docs/skill-spec.md`.
- The private GitHub source repository and the public GitHub release repository are two different trust boundaries. The public repository receives a parentless curated snapshot. The export program excludes private content and scans for leftovers. This snapshot is not a mirror of the private repository.
- The private controller publishes a snapshot only after the required Linux and Windows checks pass. Public `main` and the matching version tag must point to the same snapshot commit in one atomic Git push. An existing version tag must not be overwritten.
- Cross-repository credentials go only to the temporary askpass helper of the publishing step. Credentials must not be written to a remote URL, the curated snapshot, or the repository settings.
- The public release workflow accepts only version tags of the public repository. It builds six targets, produces checksums and cosign signatures, and then creates the GitHub Release. Only stable releases run the Homebrew and WinGet publishing stages.

### Invariants

- Only the long-term contract defined in the "Stability Layers" section of `docs/architecture.md` is accepted. `dist/` is the only deployment output and follows `docs/generated-project.md`.
- `wda.json` is a strict project contract. The dependency graph is declared by `deno.jsonc` and locked by `deno.lock`. See the "`wda.json`" section of `docs/generated-project.md` and the "wda deps" section of `docs/commands.md`.
- The Skill does not become a second source of specification, and `scripts/` is not created. Its host capability is limited to the native binary process contract. See `docs/skill-spec.md`.

### Attack Surface

- The untrusted-input parsers are at `Cargo.toml` 16–21: `jsonschema`, `json-sourcemap`, `jsonc-parser`, `marked-yaml`, `html5gum`, and `serde_json`. They cover schema and JSON, YAML front matter, and HTML input respectively. They correspond to the "Diagnostic Output" section of `docs/commands.md`, the "`wda.json`" section of `docs/generated-project.md`, and the document and source boundary.
- `serde_json` parses `wda.json`, the generated `tokens.json`, and `deno.lock` directly. These inputs fall under the "`wda.json`" section of `docs/generated-project.md`, `docs/design-system.md`, and the "wda deps" section of `docs/commands.md` respectively.
- `wda init` writes `.git/`, `.gitignore`, `README.md`, `wda.json`, `docs/architecture.md`, `docs/design.md`, `docs/glossary.md`, `pages/index.html`, and `tokens/tokens.json`. The Spectrum 2 path also writes `scripts/main.ts`, `deno.jsonc`, and `deno.lock`. The WDA Minimal path writes `deno.jsonc` and `deno.lock` only when the Lit installation succeeds. Their collision, rollback, and local Git behavior follow `docs/generated-project.md` and `docs/commands.md`.
- `deno run -A npm:esbuild@…` downloads a tool and starts an all-permissions subprocess. This is a build-time boundary that has landed in P5 and P6. Its responsibilities follow the "Stability Layers" section of `docs/architecture.md` and the "wda deps" section of `docs/commands.md`.
- Spectrum online resolution resolves an exact version from the npm registry through Deno. For its Design System source and offline fallback, see `docs/design-system.md`.
- The Skill host capability is only running a native binary and capturing stdout, stderr, and the exit status. `scripts/` is empty and must not be created. See `docs/skill-spec.md`.

## Security Verification Limits

The build, signing, and verification results of each GitHub Release are determined by the Actions run records and assets linked from that Release. A prerelease does not run Homebrew publishing or WinGet submission. Those package manager paths belong to the stable release process.

The private source repository uses GitHub Free and a single-maintainer trust model. A maintainer who can modify a private workflow and its secrets can also authorize publishing. The maintenance policy for the GitLab backup is managed by a private maintenance procedure.
