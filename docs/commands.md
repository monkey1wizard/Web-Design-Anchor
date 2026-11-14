---
title: "Commands"
status: stable
updated: 2026-11-12
type: Reference
description: "The tool requirements, behavior, diagnostics, and exit status of the commands."
tags: []
---

# Commands

Core provides four commands. The Designer and the AI edit project files directly, and Core does not handle general editing. The Designer workflow, Skill installation, and preview do not add other commands.

```text
wda init    Create a new project and, when Git is usable, a local Git repository
wda check   Validate without producing output
wda build   Validate, type-check, then build the output in dist/ if validation passes
wda tools fetch   Restore the embedded TypeScript tool cache for this platform
```

`wda --version` only prints version information, so the Skill can compare versions. It is not a fifth command.

`wda tools fetch` takes no arguments. It restores only the one TypeScript tool version that this WDA release embeds, for the current platform. It never switches versions and never changes `anchor.json` or `package.json`. A version, tag, range, URL, or any other argument is a usage error with exit status `2`.

WDA has no `wda deps` command. Website packages belong to the project's own package manager, for example `npm install <package>` or `bun add <package>`. `wda deps`, with or without a subcommand, is a usage error with exit status `2` and a hint that points to the external package manager. WDA keeps no alias such as `sync`, `refresh`, or `add`.

When you run `wda` with no arguments, the program writes the usage of the commands and `--version` to stdout, leaves stderr empty, and exits with status code `0`. This output is a usage description. It is not another command and not a command-line usage error.

## External Tools

### How git and Package Managers Are Used

`wda init` uses Git to create a local repository and first commit when Git is usable. If Git is missing or cannot start, initialization still creates the project with a Warning, without a local repository or first commit. `wda check` probes Git and reports the toolchain status.

`wda check` and `wda build` never launch a package manager, never install or update packages, and never use the network. A prepared project does not need its package manager on PATH. They read only the content that is already installed in `node_modules` and the native tool cache. `wda init` is the only command that calls an external package manager, and only when it installs the default packages. When no usable manager exists, or the machine cannot reach the registry, `wda init` creates a complete `WDA Minimal` project instead.

`wda check`, `wda build`, and `wda tools fetch` do not need an existing Git repository or a `.gitignore`.

### Which Command Needs Which Tool

| Command | `git` | Package manager | Native TypeScript tool cache |
| :--- | :--- | :--- | :--- |
| `init` | Optional. When it is not on PATH or cannot start, the project is still created without `.git` and the first commit, with a `wda.toolchain.git-missing` Warning | Optional. Without a usable manager, `init` creates `WDA Minimal` | Not required. `init` tries to fetch it and only warns on failure |
| `check` | Optional. When it is not on PATH, the command prints a `wda.toolchain.git-missing` Warning. When it is found but cannot start, the command reports a `wda.tool.fault` Warning. In both cases the command still succeeds | Never launched. A missing manager alone is not a problem | Looked up only when a page references a `scripts/**/*.ts` entry. A missing cache gives a `wda.build.tool-unavailable` Warning that points to `wda tools fetch` |
| `build` | Not required | Never launched | Required only when a page references a script entry. A pure-HTML project builds with no cache. A missing cache is a `wda.build.tool-unavailable` Error |
| `tools fetch` | Not required | Not required | Restores it. This is the only command besides `init` that uses the network for tools |

When `wda check` finds a package that `package.json` declares but `node_modules` does not contain, it reports a `wda.build.dependency-unlocked` Warning that names the package and points to the manager named in `packageManager`. When a script entry exists but the tool cache is missing, it reports the `wda.build.tool-unavailable` Warning. These Warnings do not change the success status and `check` writes nothing, so run the manager's install command or `wda tools fetch` yourself.

When `wda init` finds Git missing or unable to start, it still creates the project and `.gitignore`, skips `.git` and the first commit, and emits a `wda.toolchain.git-missing` Warning. A missing Deno is not a condition anywhere: WDA no longer uses Deno, and no command emits `wda.toolchain.deno-missing`.

## wda init

### What It Creates

`wda init [--package-manager <name>]` creates a new project in the current directory and uses the default values directly. Supported manager names are `npm` and `bun`. An unknown name is a usage error. When you name no manager, `init` probes only the supported ones, never downloads anything while probing, and picks the single usable one. When several are usable, an interactive terminal shows a menu, and a non-interactive run exits with a usage error that asks for `--package-manager`. Cancelling the menu ends the command before it writes anything.

The `WDA Minimal` path writes:

```text
.git/                   Local Git repository (skipped when Git is unavailable)
.gitignore              Content is fixed to /dist/ and /node_modules/
README.md
anchor.json
package.json
docs/architecture.md
docs/design.md
docs/glossary.md
pages/index.html
tokens/tokens.json
```

`anchor.json` holds the WDA settings and the Design System that is actually in use. `package.json` is the only place that declares website packages. `WDA Minimal` has no dependencies, no `packageManager` field, and no manager lockfile until the first external install. When a fallback happened, `anchor.json` also records the Design System that init originally intended and the fallback `reason` as text.

The Spectrum 2 path also writes `scripts/main.ts`. When exactly one manager is chosen and the registry answers, `init` resolves the exact latest stable Spectrum version, installs the six Spectrum 2 packages and `lit` with that manager in a separate stage directory, and records the manager name and exact version in the `packageManager` field of `package.json`. The manager owns its lockfile and `node_modules`.

When the machine is offline, when no usable manager exists, or when a named manager is unavailable, `init` creates a complete `WDA Minimal` project. A local package cache never switches `init` to Spectrum. `init` does not run a manager for `WDA Minimal`.

When Git is usable, the program runs `git init` and commits the whole directory as the initial commit. The message is fixed to `wda init`, and the program does not set a remote repository. Existing Designer files are also part of the commit. When Git is not on PATH or cannot start, `init` still creates the project and `.gitignore`, skips `.git` and the first commit, and emits a `wda.toolchain.git-missing` Warning.

After a successful run, `init` also tries to restore the embedded TypeScript tool cache, but only when the registry answered during this run. A failure there is a `wda.init.tool-unavailable` Warning that points to `wda tools fetch`. It never fails the project.

The `.gitignore` file is created only during initialization. After `wda init` finishes, WDA commands never read or write this file again.

### Collision Rule

The target directory can contain other files. If a file or directory that `wda init` needs to create already exists, the program aborts before writing. The check covers the items above, the required directories, `.git/`, an existing `package.json`, the legacy `wda.json`, `node_modules`, and the lock and state files of each supported manager. The program does not read, modify, or merge files outside this scope. `init` never adopts an existing install.

### Design System Resolution and Fallback

`wda init` first resolves the exact version of the latest stable Spectrum from the registry. When resolution succeeds, the program writes the Design System to `anchor.json` and installs the dependency packages with the chosen manager. If resolution or installation fails, the program removes what this run created for Spectrum, falls back to `WDA Minimal`, and explains the Design System it used and the reason on stdout. The reason is the one the manager reported, and the output suggests that the Skill can add Spectrum and Lit later. A registry that cannot be reached is described as "registry unreachable", not as the machine being offline. Falling back to Minimal is not an execution failure. For the conditions that count as a resolution failure, see `docs/design-system.md`.

### Rollback Cleanup on Failure

`wda init` completes the whole initialization in one pass. When any step fails to write, or the menu is cancelled, the program removes what it has written and returns the directory to the state before the run. The program removes files in the reverse of the order in which it created them and touches only content that this run created. If installing the Spectrum 2 packages fails and the program falls back to WDA Minimal, it first removes the Spectrum 2-specific files and then creates the Minimal project. A cleanup that fails is an explicit error, never a claimed success. The effects of third-party package scripts that the manager ran cannot be promised revertible.

### Refusal Conditions and Diagnostic Codes

| Diagnostic code | Severity | Condition |
| :--- | :--- | :--- |
| `wda.init.tool-unavailable` | Warning | The embedded TypeScript tool package could not be restored after the project was written. The project is complete and `wda tools fetch` can retry |
| `wda.init.path-conflict` | Error | A name or path collision occurs (Collision Rule). Nothing is written |
| `wda.init.git-identity-fallback` | Warning | The environment has no Git user identity configured, so init completes the commit with a temporary identity |

## Website Packages

WDA does not resolve, download, lock, or update website packages. `package.json` declares them, and the package manager that the project uses owns the lockfile and `node_modules`. Add, remove, and update packages with the manager's own commands, for example `npm install <package>` or `bun add <package>`, and then run `wda check` or `wda build`.

`wda check` and `wda build` never install or update packages. With the packages and the native tool cache in place, they run without the network. A missing package or missing tool cache gives an explicit diagnostic. The commands never hide a download. Package installation runs third-party code, so it stays a separate step that you start yourself.

The command `wda deps` is removed. Running it gives `wda.cli.usage` with exit status `2` and a hint that names the external package manager.

## wda check

### Sharing One Validation Engine with build

`wda check` examines the project in read-only mode. It writes no file and does not update `wdaVersion`. It uses the same project contract, documents, HTML, and Tokens validation rules as `wda build`. The correctness of a project must not depend on the user running `check` first.

`check` also runs a preflight that looks at what the project needs on this computer:

- If Git is not on PATH, the program reports the `wda.toolchain.git-missing` Warning. If Git exists but cannot start, the program reports a `wda.tool.fault` Warning.
- For each package that `package.json` declares but `node_modules` lacks, the program reports a `wda.build.dependency-unlocked` Warning that names the package and the manager from `packageManager`.
- If a page references a `scripts/**/*.ts` entry and the native TypeScript tool cache is missing, the program reports a `wda.build.tool-unavailable` Warning that points to `wda tools fetch`. A project without script entries needs no cache and gets no such Warning.

The preflight never launches a package manager, never installs, and never writes. It does not probe for the package manager on PATH, so a missing manager alone does not make an installed project look broken. These Warnings do not change the success status. Build does not run the PATH tool checks, and it reports the missing packages and the missing tool cache as Errors because it cannot continue without them.

Check and Build both run the four project validation rule modules. The module paths are:

- `src/validation/project_contract.rs`
- `src/validation/documents.rs`
- `src/validation/html.rs`
- `src/validation/tokens.rs`

Check also runs `src/validation/toolchain.rs`, which holds the preflight above. Build also runs the output rules in `src/validation/build_output.rs`.

### Rule Module: Project Contract

This module validates that `anchor.json` and `package.json` exist and are valid JSON. `anchor.json` must conform to `schemas/anchor.schema.json` and have no unknown fields. `package.json` must have an object at its root, an optional `dependencies` object of string versions, and an optional `packageManager` field that is a name, `@`, and an exact version. `wda.json` is not read or converted. A project that lacks either `anchor.json` or `package.json` gets a `wda.contract.missing` Error. For the field specification, see `docs/generated-project.md`.

| Diagnostic code | Severity |
| :--- | :--- |
| `wda.contract.missing` | Error |
| `wda.contract.malformed-json` | Error |
| `wda.contract.unknown-field` | Error |
| `wda.contract.schema-violation` | Error |

### Rule Module: Documents

The program checks that the following four required documents exist: `README.md`, `docs/architecture.md`, `docs/design.md`, and `docs/glossary.md`. The three documents in `docs/` must have YAML front matter, and `title`, `status`, and `updated` must not be empty. The format of `updated` is `YYYY-MM-DD`. The program checks other `docs/**/*.md` files only when they exist, and it reports a Warning when their front matter has a problem. If the front matter has `sourceRefs`, every item must map to a file or section inside the project.

The document consistency check looks only at structure and metadata. It does not compare the document content with the code, `anchor.json`, tokens, or the dependency state. Human review finds content differences.

| Diagnostic code | Severity |
| :--- | :--- |
| `wda.docs.required-document-missing` | Error |
| `wda.docs.front-matter-missing` | Error |
| `wda.docs.front-matter-malformed` | Error for a required document, Warning for an optional document |
| `wda.docs.front-matter-field-invalid` | Error for a required document, Warning for an optional document |
| `wda.docs.source-ref-unresolvable` | Warning |
| `wda.docs.source-ref-escapes-root` | Warning |

### Rule Module: HTML

The HTML module checks only three rules: `html[lang]` must not be empty, every `img` has an `alt` attribute, and no `id` repeats within a document. The `alt` check only confirms that the attribute exists. It does not assess the quality of the text.

Human review covers the other accessibility (a11y) features. These include whether the `alt` text is meaningful, the contrast over overlaid images, the visibility and order of focus, keyboard operation, zoom and reflow, and motion. Passing every automatic check also does not mean conformance to WCAG. `wda check` does not produce a "needs human review" Warning for this.

| Diagnostic code | Severity |
| :--- | :--- |
| `wda.html.lang-missing` | Error |
| `wda.html.img-alt-missing` | Error |
| `wda.html.duplicate-id` | Error |

### Rule Module: Tokens

The Tokens module checks JSON and the WDA CSS token profile described in [Design System](design-system.md). It checks values, types, aliases, and normalized names before build output.

| Diagnostic code | Severity |
| :--- | :--- |
| `wda.tokens.malformed-json` | Error |
| `wda.tokens.value-missing` | Error |
| `wda.tokens.type-unresolvable` | Error |
| `wda.tokens.alias-unresolvable` | Error |
| `wda.tokens.unsupported` | Error |

## wda build

### Required Validation and the Build Output Rule Module

`wda build` first runs the same four project validation rule modules as `check`. If there is any Error, Build aborts and does not change `dist/`. A Warning does not block the build. Only when there is no Error does Build enter the output stage and run the output rules in `src/validation/build_output.rs`.

### wda-include Expansion and No Template Language

The only custom build-time syntax of WDA is:

```text
<wda-include src="..."></wda-include>
```

It follows the Custom Element naming rule and expands into plain HTML at build time. It is not a runtime Web Component, and this tag must not remain in `dist/`. The `src` attribute can only point to a file inside the project. It must not go beyond the project root, and it must not form a circular reference.

The project provides no template language and does not support `<slot>`, `if`, `for`, variables, expressions, or data binding. For general layout and static content, prefer native semantic HTML and CSS. Create a component only when you need reusable behavior, product semantics, or an encapsulated component.

### Type Checking and Module Build

`wda build` type-checks every `scripts/**/*.ts` entry that a page references with the single TypeScript tool version that this WDA release embeds. The check always runs before bundling, and it fixes strict settings that the project `tsconfig` cannot weaken. A parseable type error is a project Error with exit status `1`. A tool that cannot start, crashes, or times out is a ToolFault with exit status `2`. When no page references a script entry, the build looks up no tool cache and launches no tool.

The tool comes from the embedded or cached platform package. When the cache is missing, the build reports a `wda.build.tool-unavailable` Error that points to `wda tools fetch` and leaves `dist/` unchanged. The build never downloads the tool.

WDA then builds the entries with a native Rust bundler (Rolldown) in browser mode with ES module output. The bundler resolves the packages that the project's own package manager already installed in `node_modules`. WDA does not resolve package versions and keeps no CDN mirror. Remote module imports, Node-only modules, and dynamic imports that WDA cannot prove fully emitted are reported as Errors. The build does not claim that every npm package works in a browser.

### Temporary Build Workspace

`wda build` writes the bundler output into a fresh staging directory first and commits only the listed entries, chunks, and static assets to `dist/`. It never copies `node_modules` as a whole. The files `package.json` and the manager lockfile do not change by any byte during the build. After the build ends, the program deletes the staging directory, whether the build succeeds or fails.

### Use Only Installed Packages

`wda build` reads only what is installed in `node_modules`. It does not install, pick, or upgrade any package, and it does not use the network. When a script imports a package that is not installed, it reports a `wda.build.dependency-unlocked` Error that tells you to install the project's packages with your package manager. The output in `dist/` leaves no remote runtime dependency, and it needs no Node.js, Bun, or WDA at website runtime.

### Secrets Stay Out of dist/

The build does not copy dotfiles or `.env*` files. If a page or a style references these paths, the build reports an Error and aborts. This exclusion rule looks only at file paths and does not inspect file content.

### Print the Absolute Path of dist/ on Success

When the build succeeds, the program updates the `wdaVersion` field of `anchor.json` and prints the absolute path of `dist/` to stdout. When the build fails, it does not update `wdaVersion`. If the program detects ES modules, `fetch`, or another known feature that does not support `file://`, it also reminds the user that the output needs an HTTP host.


### Refusal Conditions and Diagnostic Codes

| Diagnostic code | Severity | Condition |
| :--- | :--- | :--- |
| `wda.build.tool-unavailable` | Error in build, Warning in check | The native TypeScript tool cache is missing or cannot be located while a script entry needs it. Build fails and points to `wda tools fetch`. The diagnostic code and severity are stable, and the summary text is not part of the stable contract |
| `wda.build.tokens-css-in-source` | Error | `styles/tokens.css` exists on the source side |
| `wda.build.include-unresolvable` | Error | The `src` of `wda-include` points to a file that does not exist |
| `wda.build.include-escapes-root` | Error | The `src` of `wda-include` goes beyond the project root |
| `wda.build.include-cycle` | Error | `wda-include` forms a circular reference |
| `wda.build.type-error` | Error | The TypeScript type check fails |
| `wda.build.secret-path-referenced` | Error | A page or style references a dotfile or `.env*` |
| `wda.build.reference-unresolvable` | Error | A file that a page or style references does not exist, or a script imports a remote module |
| `wda.build.unreferenced-asset` | Warning | The `assets/` directory contains a file that no page or style references |
| `wda.build.output-path-collision` | Error | Two source paths map to the same `dist/` path |
| `wda.build.dependency-unlocked` | Error in build, Warning in check | A package that `package.json` declares or that a script imports is not installed in `node_modules`. The name is historical: the code now means "not installed", and no lock file is consulted |

## Diagnostic Output

### Severity Depends on Impact

Every diagnostic contains a severity and a diagnostic code. The program decides the severity by the actual impact. It does not map a category of problems directly to a level, for example by listing all a11y problems as Warnings or all script problems as Errors. The program emits a Warning only when the problem is specific and the user can act to improve it, and it never shows a fixed "needs human review" notice that the user cannot clear.

| Severity | Meaning |
| :--- | :--- |
| Error | The problem would make the build result or the project incorrect, and it is serious enough to block the build. If the program can determine that the artifact will be wrong, it can set the severity to Error |
| Warning | The problem does not block the build, but it signals a concern about correctness or quality. It must have enough visibility and keep appearing, so the Designer and the AI cannot easily ignore it |

### Externally Stable Semantics

The diagnostic output does not use a specific structured format. Exactly five items stay stable for external consumers:

| Item | Content |
| :--- | :--- |
| severity | The severity level |
| diagnostic code | The diagnostic code itself |
| location | The normalized project-relative path, with the line number and column (line:col) |
| output stream | stderr and stdout are separate |
| exit status | The exit status code |

The message and the next-step suggestion must exist and be clear and easy to understand, but their exact wording can change later. Punctuation, separators, and column width are not part of the contract. A consumer must not rely on splitting the text into fields to parse it.

### Exit Status

`src/main.rs` sets the exit status from the returned result. When the diagnostics that `init`, `check`, and `build` report through `ValidationReport` contain an Error, the status code is `1`. `wda.cli.usage`, tool errors, and any other ToolFault give status code `2`. `wda tools fetch` exits with `1` when it cannot restore the tool cache.

`wda.init.path-conflict` and `wda.build.tool-unavailable` (as an Error) are reported through `ValidationReport`, so their status code is `1`. `wda.init.tool-unavailable` and the `check` Warnings of the preflight never change the status code.

```text
0   Success; Warnings may exist
1   The `ValidationReport` contains an Error
2   CLI usage error (including the removed `wda deps`), or ToolFault
```

### Output Streams

In human mode, diagnostics go to stderr. Success results and other summary messages go to stdout, including the Design System explanation from `wda init` and the `dist/` path from `wda build`.

### Single-Line, Decidable, Readable by Humans and AI

Each diagnostic takes one line. With the same input and the same PATH state, the same binary prints the same set of diagnostics in the same order. The toolchain diagnostics of Check change with whether Git is on PATH and can start, which packages are installed in `node_modules`, and whether the tool cache exists. The ordering has an explicit tie-breaker, so the file system traversal order does not affect it. A consumer only needs to read the diagnostic code and the severity, and it does not need to split the text by fixed fields. Every diagnostic provides a message and a next-step suggestion, and the wording can change between versions.

### JSON Output

`wda check --json` and `wda build --json` emit one UTF-8 JSON document followed by a newline on stdout. Subprocess output is captured and stderr does not repeat the report. OS termination or a stdout write failure can prevent a complete document.

The document has `schema_version` 1, `command` (`check` or `build`), `status` (`success`, `project-error`, `usage-error`, or `tool-fault`), `diagnostics`, and `summary`. Each diagnostic has `code`, `severity` (`error` or `warning`), `path`, `line`, `column`, `message`, and `next_action`. Missing location fields are null. Paths use forward slashes. Messages retain control characters as JSON values.

The summary contains `errors`, `warnings`, `dist_path`, and `needs_http`. Only a successful build supplies the project-relative `dist_path` and boolean `needs_http`. They are null for check and every failure, even when an old dist exists. Warnings retain success status and exit 0. Project errors exit 1. Usage errors and tool faults exit 2. JSON and human output use the same sorted diagnostics.

JSON mode requires `check` or `build` as the first argument and an exact `--json` before any `--` terminator. Unknown or duplicate arguments in that mode produce JSON usage errors. Other invocations retain human usage behavior.

### Path Normalization and Control-Character Protection

The program prints paths relative to the project root, separated by forward slashes (`/`), and it does not leak the absolute path of the host. Even if a project file contains a line break or a control character, it cannot use that to insert a fake diagnostic.

### Cross-Command Diagnostic Codes

| Diagnostic code | Severity | Condition |
| :--- | :--- | :--- |
| `wda.cli.usage` | Error | The command-line usage is wrong. The exit status code is `2` |
| `wda.cli.json-not-supported` | Error | The user passes `--json` outside check or build. The exit status code is `2` |
| `wda.tool.fault` | Error or Warning | An external tool or the file system fails. The severity depends on the command and the impact. The exit status code of an Error is `2` |
| `wda.command.not-implemented` | Error | A reserved code. No command emits this diagnostic code now |

### All Diagnostic Codes

`src/codes.rs` is the only place that declares diagnostic codes. This document is a lookup table, and every code must appear in this document once.

| Diagnostic code | Severity | Command |
| :--- | :--- | :--- |
| `wda.command.not-implemented` | Error | Cross-command |
| `wda.cli.json-not-supported` | Error | Cross-command |
| `wda.cli.usage` | Error | Cross-command |
| `wda.tool.fault` | Error or Warning | Cross-command |
| `wda.toolchain.git-missing` | Warning | check / init |
| `wda.toolchain.deno-missing` | Retired | none. No command emits it; the declaration stays in the registry until a later cleanup removes it |
| `wda.contract.missing` | Error | check / build |
| `wda.contract.malformed-json` | Error | check / build |
| `wda.contract.unknown-field` | Error | check / build |
| `wda.contract.schema-violation` | Error | check / build |
| `wda.docs.required-document-missing` | Error | check / build |
| `wda.docs.front-matter-missing` | Error | check / build |
| `wda.docs.front-matter-malformed` | Error or Warning | check / build |
| `wda.docs.front-matter-field-invalid` | Error or Warning | check / build |
| `wda.docs.source-ref-unresolvable` | Warning | check / build |
| `wda.docs.source-ref-escapes-root` | Warning | check / build |
| `wda.html.lang-missing` | Error | check / build |
| `wda.html.img-alt-missing` | Error | check / build |
| `wda.html.duplicate-id` | Error | check / build |
| `wda.tokens.malformed-json` | Error | check / build |
| `wda.tokens.value-missing` | Error | check / build |
| `wda.tokens.type-unresolvable` | Error | check / build |
| `wda.tokens.alias-unresolvable` | Error | check / build |
| `wda.init.tool-unavailable` | Warning | init |
| `wda.init.path-conflict` | Error | init |
| `wda.init.git-identity-fallback` | Warning | init |
| `wda.build.tool-unavailable` | Error or Warning | check / build |
| `wda.build.tokens-css-in-source` | Error | build |
| `wda.build.include-unresolvable` | Error | build |
| `wda.build.include-escapes-root` | Error | build |
| `wda.build.include-cycle` | Error | build |
| `wda.build.type-error` | Error | build |
| `wda.build.secret-path-referenced` | Error | build |
| `wda.build.reference-unresolvable` | Error | build |
| `wda.build.unreferenced-asset` | Warning | build |
| `wda.build.output-path-collision` | Error | build |
| `wda.build.dependency-unlocked` | Error or Warning | check / build |
