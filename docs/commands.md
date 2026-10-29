---
title: "Commands"
status: stable
updated: 2026-10-15
type: Reference
description: "The tool requirements, behavior, diagnostics, and exit status of the four commands."
tags: []
---

# Commands

Core provides four commands. The Designer and the AI edit project files directly, and Core does not handle general editing. The Designer workflow, Skill installation, and preview do not add other commands.

```text
wda init    Create a new project and a local Git repository
wda deps    Manage the dependency graph
wda check   Validate without producing output
wda build   Validate, then build the output in dist/ if validation passes
```

`wda --version` only prints version information, so the Skill can compare versions. It is not a fifth command.

When you run `wda` with no arguments, the program writes the usage of the four commands and `--version` to stdout, leaves stderr empty, and exits with status code `0`. This output is a usage description. It is not a fifth command and not a command-line usage error.

## External Tools

### How deno and git Are Used on PATH

`deno` resolves dependency packages and calls the transpiler. `wda init` needs `git` to create the local repository, and it first checks that Git can start before it writes any file. If Git is not on PATH or cannot start, initialization aborts. `wda check` probes Git and reports the toolchain status. You can still run a Minimal initialization without `deno`. If initialization needs Deno to resolve or install dependency packages, but Deno is not on PATH or cannot start, the program falls back to `WDA Minimal`.

`wda check`, `wda build`, and `wda deps` do not need an existing Git repository or a `.gitignore`.

### esbuild Is Version-Pinned by WDA and Called Through deno

WDA calls the TypeScript transpiler with `deno run -A npm:esbuild@<pinned version>`. Because WDA pins the esbuild version, PATH does not need `esbuild` or Node.js.

### Which Command Needs Which Tool

| Command | `deno` | `git` |
| :--- | :--- | :--- |
| `init` | Optional. When Deno is not on PATH or cannot start, WDA runs a limited initialization with WDA Minimal, without online Design System resolution or dependency installation | Required. When it is not on PATH or cannot start, the command aborts with an Error and writes nothing |
| `deps` | Required | Not required |
| `check` | Optional. When it is not on PATH, the command prints a `wda.toolchain.deno-missing` Warning. When it is found but cannot start, the command reports a `wda.tool.fault` Warning. In both cases the command still succeeds | Optional. When it is not on PATH, the command prints a `wda.toolchain.git-missing` Warning. When it is found but cannot start, the command reports a `wda.tool.fault` Warning. In both cases the command still succeeds |
| `build` | Required | Not required |

When Git or Deno is not on PATH, or is found but cannot start, `wda check` only emits a Warning. That alone never makes the command fail.
When `wda init` finds Git missing or unable to start, it emits a `wda.init.tool-unavailable` Error and aborts before writing. When Deno is not on PATH or cannot start, `wda init` runs a limited initialization with WDA Minimal. This path does not perform online Design System resolution or dependency installation. When Deno is not on PATH, it also emits a `wda.toolchain.deno-missing` Warning.

## wda init

### What It Creates

`wda init` creates a new project in the current directory and uses the default values directly.

The `WDA Minimal` path writes:

```text
.git/                   Local Git repository
.gitignore              Content is fixed to /dist/
README.md
wda.json
docs/architecture.md
docs/design.md
docs/glossary.md
pages/index.html
tokens/tokens.json
```

When a network is available, `WDA Minimal` tries to install `lit`. After a successful installation, it produces `deno.jsonc` and `deno.lock`. When the machine is offline or the installation fails, initialization still completes but produces no dependency files. stdout explains the reason and suggests running `wda deps add lit`. `WDA Minimal` includes no starter script by default.

The Spectrum 2 path also writes `scripts/main.ts`. When online resolution succeeds, initialization installs the six Spectrum 2 packages and `lit`, produces `deno.jsonc` and `deno.lock`, and locks the packages that the starter script actually imports at the same versions.

After the files are written, the program runs `git init` and commits the whole directory as the initial commit. The message is fixed to `wda init`, and the program does not set a remote repository. Existing Designer files are also part of the commit.

The `.gitignore` file is created only during initialization. After `wda init` finishes, WDA commands never read or write this file again.

### Collision Rule

The target directory can contain other files. If a file or directory that `wda init` needs to create already exists, the program aborts before writing. The check covers the items above, the required directories, `.git/`, `deno.jsonc`, the project-side `deno.json`, and `deno.lock`. The program does not read, modify, or merge files outside this scope.

### Design System Resolution and Fallback

`wda init` first resolves the exact version of the latest stable Spectrum online. When resolution succeeds, the program writes the version to `wda.json` and installs the dependency packages. If resolution or installation fails, the program falls back to `WDA Minimal` and explains the Design System it used and the reason on stdout. When installation fails, it also gives recovery steps. Falling back to Minimal is not an execution failure. For the conditions that count as a resolution failure, see `docs/design-system.md`.

### Rollback Cleanup on Failure

`wda init` completes the whole initialization in one pass. When any step fails to write, the program removes what it has written and returns the directory to the state before the run. The cleanup scope includes the `deno.jsonc` and `deno.lock` that Spectrum 2 and WDA Minimal create. The program removes files in the reverse of the order in which it created them. If Spectrum 2 fails to install dependency packages and the program falls back to WDA Minimal, it first removes the Spectrum 2-specific files and then creates the Minimal project.

### Refusal Conditions and Diagnostic Codes

| Diagnostic code | Severity | Condition |
| :--- | :--- | :--- |
| `wda.init.tool-unavailable` | Error | `git` is not on PATH, or was found but cannot start. The message states the actual reason. Nothing is written |
| `wda.init.path-conflict` | Error | A name or path collision occurs (Collision Rule). Nothing is written |
| `wda.init.git-identity-fallback` | Warning | The environment has no Git user identity configured, so init completes the commit with a temporary identity |

## wda deps

### Declaration and Locking

A current project declares npm packages in the `imports` field of `deno.jsonc`, uses the pinned `npm:<package>@<version>` specifier, and locks the versions in `deno.lock`. WDA also supports existing `package.json` projects. `wda deps` first determines the project format: when a `package.json` exists in the root, it uses the legacy branch, and otherwise it uses the current `deno.jsonc` branch.

`wda deps` calls `deno` to resolve and update `deno.lock`. The current format uses `--config deno.jsonc`, and the legacy format uses `--package-json`. Deno writes the files and keeps the comments, trailing commas, and other custom fields of `deno.jsonc`. Only this command changes the dependency graph. `wda build` only uses locked versions and never picks a new version on its own.

### The Separate Duties of add, remove, and update

```text
wda deps add <package>      Add one dependency package and lock it
wda deps remove <package>   Remove one dependency package and update the lock
wda deps update [package]   Re-resolve and lock within the existing declaration
```

### Staged Validation and Atomic Commit

`wda deps` first runs the Deno command in an isolated temporary directory and validates the dependency graph with `deno install --frozen` (staged validation). Only then does it modify project files. The run does not pass `--allow-scripts`, so it does not run npm lifecycle scripts.

After validation succeeds, WDA commits the changes in one pass in a fixed order. It first writes, renames, or deletes `deno.lock`, and then it updates the dependency declaration file (`deno.jsonc` or `package.json`). If an I/O error occurs during this process, the program restores the original bytes of the files, so it leaves no damaged or inconsistent state.

### Semantic Empty-State Rule

When a project has no actual dependency packages, it does not keep an empty declaration file or lock file:

- For a current `deno.jsonc` project: when you remove the last dependency package, WDA checks the root-level semantic structure. If the file has no semantic field other than an empty `imports`, WDA deletes `deno.jsonc` (a comment alone is not a reason to keep the file). If the file still contains other custom root fields, WDA keeps the file and only clears `imports`. When `deno.lock` has only the version header (`{"version": ...}`), WDA also deletes the empty `deno.lock`.
- For a legacy `package.json` project: after you remove the last dependency package, WDA deletes both `package.json` and `deno.lock`.
- A `WDA Minimal` project that was created offline does not contain these two files by default.

### Refusal Conditions, Recovery, and Diagnostic Codes

`wda deps` reports errors with cross-command diagnostic codes:

- If the project side has a `deno.json`, or has both `deno.json` and `deno.jsonc`, `wda deps` refuses and aborts with `wda.tool.fault`. The program asks the user to rename the configuration file or to switch to `deno.jsonc`. WDA never asks the user to create or edit a project-side `deno.json`.
- If the command usage is wrong, it refuses with `wda.cli.usage` (exit status code `2`).
- If the environment lacks `deno`, resolution fails, or staged validation fails, it refuses with `wda.tool.fault` (exit status code `2`). When execution fails, the command writes nothing and does not change project files.
- Recovery: if the project did not install Lit during initialization because it was offline, the user can run `wda deps add lit` after the network connection returns.

## wda check

### Sharing One Validation Engine with build

`wda check` examines the project in read-only mode. It writes no file and does not update `wdaVersion`. It uses the same project contract, documents, HTML, and Tokens validation rules as `wda build`. `check` also probes the optional tools on PATH. If Git or Deno is not on PATH, the program reports the matching `wda.toolchain.*-missing` Warning. If a tool exists but cannot start, the program reports a `wda.tool.fault` Warning. The correctness of a project must not depend on the user running `check` first.
These Warnings do not change the success status. Build does not run the PATH tool checks.

Check and Build both run the four project validation rule modules. The module paths are:

- `src/validation/project_contract.rs`
- `src/validation/documents.rs`
- `src/validation/html.rs`
- `src/validation/tokens.rs`

Check also runs `src/validation/toolchain.rs`, which reports toolchain diagnostics according to the PATH state. Build does not probe the optional tools on PATH. When Build needs Deno but cannot use it, it reports a `wda.build.tool-unavailable` Error. The summary text of this diagnostic can change between versions. Build also runs the output rules in `src/validation/build_output.rs`.

### Rule Module: Project Contract

This module validates that `wda.json` exists, is valid JSON, conforms to `schemas/wda.schema.json`, and has no unknown fields. For the field specification, see the `wda.json` field table in `docs/generated-project.md`.

| Diagnostic code | Severity |
| :--- | :--- |
| `wda.contract.missing` | Error |
| `wda.contract.malformed-json` | Error |
| `wda.contract.unknown-field` | Error |
| `wda.contract.schema-violation` | Error |

### Rule Module: Documents

The program checks that the following four required documents exist: `README.md`, `docs/architecture.md`, `docs/design.md`, and `docs/glossary.md`. The three documents in `docs/` must have YAML front matter, and `title`, `status`, and `updated` must not be empty. The format of `updated` is `YYYY-MM-DD`. The program checks other `docs/**/*.md` files only when they exist, and it reports a Warning when their front matter has a problem. If the front matter has `sourceRefs`, every item must map to a file or section inside the project.

The document consistency check looks only at structure and metadata. It does not compare the document content with the code, `wda.json`, tokens, or the dependency state. Human review finds content differences.

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

The Tokens module checks that `tokens/tokens.json` is valid JSON and conforms to DTCG. It also checks that every token has a value, that its type can be determined, and that its alias can be resolved.

| Diagnostic code | Severity |
| :--- | :--- |
| `wda.tokens.malformed-json` | Error |
| `wda.tokens.value-missing` | Error |
| `wda.tokens.type-unresolvable` | Error |
| `wda.tokens.alias-unresolvable` | Error |

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

### Temporary Build Workspace

`wda build` creates a temporary workspace (build workspace) outside the project directory. WDA writes a `deno.json` in a dedicated JSON format into the workspace. It contains the compile and module settings, for example `nodeModulesDir: "manual"` and the standard `lib` list.

For a current-format project, the program merges the `imports` of the project-side `deno.jsonc` into the workspace `deno.json`, and it does not bring in other custom fields. If the project side has a `deno.json`, the build reports a ToolFault and aborts. For an existing `package.json` project, the program copies that file into the workspace unchanged.

The program runs `deno install --frozen` in the workspace only when the dependency declaration of the selected format is not empty. That means the current format has at least one item in `imports`, or the legacy format has at least one item in `dependencies`. When the declaration is empty, the program skips the installation and does not require a `deno.lock`. The run does not pass `--allow-scripts`, so it does not run npm lifecycle scripts. After the build ends, the program deletes the temporary workspace, whether the build succeeds or fails. The `deno.lock` in the project root does not change by any byte during the build.

### Use Only Locked Dependencies

`wda build` only uses the versions that `deno.lock` locks. It does not pick new versions, change the dependency graph, or upgrade silently. At build time it bundles all dependency modules into `dist/` and leaves no remote runtime dependency.

### Secrets Stay Out of dist/

The build does not copy dotfiles or `.env*` files. If a page or a style references these paths, the build reports an Error and aborts. This exclusion rule looks only at file paths and does not inspect file content.

### Print the Absolute Path of dist/ on Success

When the build succeeds, the program updates the `wdaVersion` field of `wda.json` and prints the absolute path of `dist/` to stdout. When the build fails, it does not update `wdaVersion`. If the program detects ES modules, `fetch`, or another known feature that does not support `file://`, it also reminds the user that the output needs an HTTP host.

### Refusal Conditions and Diagnostic Codes

| Diagnostic code | Severity | Condition |
| :--- | :--- | :--- |
| `wda.build.tool-unavailable` | Error | The build needs Deno, but Deno is not available. The build fails. The diagnostic code and severity are stable, and the summary text is not part of the stable contract |
| `wda.build.tokens-css-in-source` | Error | `styles/tokens.css` exists on the source side |
| `wda.build.include-unresolvable` | Error | The `src` of `wda-include` points to a file that does not exist |
| `wda.build.include-escapes-root` | Error | The `src` of `wda-include` goes beyond the project root |
| `wda.build.include-cycle` | Error | `wda-include` forms a circular reference |
| `wda.build.type-error` | Error | The TypeScript type check fails |
| `wda.build.secret-path-referenced` | Error | A page or style references a dotfile or `.env*` |
| `wda.build.reference-unresolvable` | Error | A file that a page or style references does not exist |
| `wda.build.unreferenced-asset` | Warning | The `assets/` directory contains a file that no page or style references |
| `wda.build.output-path-collision` | Error | Two source paths map to the same `dist/` path |
| `wda.build.dependency-unlocked` | Error | The dependency declaration (deno.jsonc or package.json) or the source code references a dependency, but deno.lock has no matching lock entry, or the lock file is out of date |

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

`src/main.rs` sets the exit status from the returned result. When the diagnostics that `init`, `check`, and `build` report through `ValidationReport` contain an Error, the status code is `1`. `wda.cli.usage`, `deps`-level diagnostics or tool errors, and any other ToolFault give status code `2`.

`wda.init.tool-unavailable` and `wda.init.path-conflict` are reported through `ValidationReport`, so their status code is `1`. `wda.build.tool-unavailable` is returned as a ToolFault, so its status code is `2`.

```text
0   Success; Warnings may exist
1   The `ValidationReport` contains an Error
2   CLI usage error, `deps` error, or ToolFault
```

### Output Streams

Diagnostics go to stderr. Success results and other summary messages go to stdout, including the Design System explanation from `wda init` and the `dist/` path from `wda build`.

### Single-Line, Decidable, Readable by Humans and AI

Each diagnostic takes one line. With the same input and the same PATH state, the same binary prints the same set of diagnostics in the same order. The toolchain diagnostics of Check change with whether Git and Deno are on PATH and can start. The ordering has an explicit tie-breaker, so the file system traversal order does not affect it. A consumer only needs to read the diagnostic code and the severity, and it does not need to split the text by fixed fields. Every diagnostic provides a message and a next-step suggestion, and the wording can change between versions.

### No `--json` Option

There is currently no `--json` option, and there is no structured diagnostic format. If the user passes `--json`, the program refuses on stderr, prints no JSON on stdout, and exits with status code `2` and the diagnostic code `wda.cli.json-not-supported`.

A proposal for a versioned structured output must name the machine consumer and attach a reproducible failure case that shows the plain-text interface lacks necessary information. When the proposal meets these conditions, you can submit it separately as an ADR for review. A statement that "the consumer does not want to handle plain text" is not enough to support a proposal. The first stage considers only `check` and `build`.

### Path Normalization and Control-Character Protection

The program prints paths relative to the project root, separated by forward slashes (`/`), and it does not leak the absolute path of the host. Even if a project file contains a line break or a control character, it cannot use that to insert a fake diagnostic.

### Cross-Command Diagnostic Codes

| Diagnostic code | Severity | Condition |
| :--- | :--- | :--- |
| `wda.cli.usage` | Error | The command-line usage is wrong. The exit status code is `2` |
| `wda.cli.json-not-supported` | Error | The user passes the `--json` option. The exit status code is `2` |
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
| `wda.toolchain.git-missing` | Warning | check |
| `wda.toolchain.deno-missing` | Warning | check / init |
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
| `wda.init.tool-unavailable` | Error | init |
| `wda.init.path-conflict` | Error | init |
| `wda.init.git-identity-fallback` | Warning | init |
| `wda.build.tool-unavailable` | Error | build |
| `wda.build.tokens-css-in-source` | Error | build |
| `wda.build.include-unresolvable` | Error | build |
| `wda.build.include-escapes-root` | Error | build |
| `wda.build.include-cycle` | Error | build |
| `wda.build.type-error` | Error | build |
| `wda.build.secret-path-referenced` | Error | build |
| `wda.build.reference-unresolvable` | Error | build |
| `wda.build.unreferenced-asset` | Warning | build |
| `wda.build.output-path-collision` | Error | build |
| `wda.build.dependency-unlocked` | Error | build |
