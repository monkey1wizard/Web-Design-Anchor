//! Script toolchain resolution, type-checking, transformation, and dependency boundaries.
//!
//! Provides PATH resolution for external tools required during the build pipeline.
//! In WDA V1, `deno` is the sole external PATH dependency; esbuild is invoked via `deno run`
//! rather than looked up directly on PATH.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::codes;
use crate::diagnostics::{Diagnostic, Location, Severity, ToolFault, ValidationReport};

/// Browser library globals made available to Deno's static type-checker, shared with
/// [`BuildWorkspace`]'s `deno.json` so both configurations describe the same TypeScript
/// `lib` surface.
const DENO_CHECK_LIB: &[&str] = &["dom", "dom.iterable", "dom.asynciterable", "esnext"];

fn deno_check_lib_json_array() -> String {
    DENO_CHECK_LIB
        .iter()
        .map(|lib| format!("\"{lib}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Checks that external script imports are covered by the project's existing Deno lock state.
/// This preflight is read-only; graph changes belong to `wda deps`.
pub fn check_locked_dependencies(
    root: &Path,
    expanded_pages: &std::collections::BTreeMap<PathBuf, String>,
) -> ValidationReport {
    let mut report = ValidationReport::new();
    let entries = discover_script_entries(root, expanded_pages);
    if entries.is_empty() {
        return report;
    }

    let lock_state = read_lock_state(root);
    let reachable = reachable_script_paths(root, entries);
    let mut unlocked = BTreeSet::new();
    for source_rel in &reachable {
        let Ok(source) = fs::read_to_string(root.join(source_rel)) else {
            continue;
        };
        for specifier in extract_module_specifiers(&source) {
            if !is_local_module_specifier(&specifier) && !lock_state.covers(&specifier) {
                unlocked.insert((source_rel.clone(), specifier));
            }
        }
    }
    for (source, specifier) in unlocked {
        report.add(Diagnostic::new(
            codes::BUILD_DEPENDENCY_UNLOCKED,
            Severity::Error,
            Some(Location::path_only(source)),
            format!("Dependency specifier '{specifier}' is not covered by deno.lock"),
            "Run `wda deps` to update the dependency graph, then run the build again",
        ));
    }
    report.sort();
    report
}

#[derive(Debug, Default)]
struct LockState {
    keys: BTreeSet<String>,
}

impl LockState {
    fn covers(&self, specifier: &str) -> bool {
        if self.keys.contains(specifier) {
            return true;
        }
        if is_local_module_specifier(specifier) || specifier.contains("://") {
            return false;
        }
        let bare = specifier.strip_prefix("npm:").unwrap_or(specifier);
        if bare.is_empty() {
            return false;
        }
        let package = package_name_from_bare_specifier(bare);
        let prefix = format!("npm:{package}@");
        self.keys.iter().any(|key| key.starts_with(&prefix))
    }
}

/// Derives the package name a bare (non-`npm:`-prefixed, non-relative, non-URL) module
/// specifier resolves to, so a subpath import matches the same lock entry as the bare
/// package import. A scoped package keeps its first two `/`-separated segments
/// (`@scope/name`); any other specifier keeps only its first segment.
fn package_name_from_bare_specifier(bare: &str) -> String {
    let mut segments = bare.splitn(3, '/');
    let first = segments.next().unwrap_or("");
    if let Some(second) = first.starts_with('@').then(|| segments.next()).flatten() {
        format!("{first}/{second}")
    } else {
        first.to_string()
    }
}

fn read_lock_state(root: &Path) -> LockState {
    let Ok(bytes) = fs::read(root.join("deno.lock")) else {
        return LockState::default();
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return LockState::default();
    };
    let mut keys = BTreeSet::new();
    if let Some(serde_json::Value::Object(specifiers)) = value.get("specifiers") {
        keys.extend(specifiers.keys().cloned());
    }
    LockState { keys }
}

fn is_local_module_specifier(specifier: &str) -> bool {
    specifier.starts_with('.') || specifier.starts_with('/')
}

fn resolve_script_import(source: &Path, specifier: &str) -> PathBuf {
    let base = source.parent().unwrap_or_else(|| Path::new(""));
    let mut path = base.join(specifier);
    if path.extension().is_none() {
        path.set_extension("ts");
    }
    path
}

fn extract_module_specifiers(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        let candidate = if trimmed.starts_with("import") || trimmed.starts_with("export") {
            trimmed
                .split_once(" from ")
                .map(|(_, rest)| rest)
                .or_else(|| trimmed.strip_prefix("import"))
                .or_else(|| trimmed.strip_prefix("export"))
        } else {
            None
        };
        if let Some(rest) = candidate {
            if let Some(specifier) = quoted_specifier(rest) {
                found.push(specifier);
            }
        }
        let mut rest = trimmed;
        while let Some(start) = rest.find("import(") {
            rest = &rest[start + "import(".len()..];
            if let Some(specifier) = quoted_specifier(rest) {
                found.push(specifier);
            }
            if rest.is_empty() {
                break;
            }
            rest = &rest[1..];
        }
    }
    found
}

fn quoted_specifier(input: &str) -> Option<String> {
    let trimmed = input.trim_start();
    let quote = trimmed.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let rest = &trimmed[quote.len_utf8()..];
    Some(rest[..rest.find(quote)?].to_string())
}

/// Resolves a tool by name from the system PATH.
///
/// Searches PATH for an executable matching `tool_name`.
/// Returns `Ok(path)` if found and runnable, or an Error [`Diagnostic`] carrying
/// `wda.build.tool-unavailable` if not found or if the executable cannot start.
pub fn resolve_tool(tool_name: &str) -> Result<PathBuf, Diagnostic> {
    resolve_tool_with_path(tool_name, std::env::var_os("PATH").as_deref())
}

/// Resolves the path to `deno` on the system PATH.
///
/// Returns `Ok(path)` if found and runnable, or an Error [`Diagnostic`] carrying
/// `wda.build.tool-unavailable` if missing or unable to start.
pub fn resolve_deno() -> Result<PathBuf, Diagnostic> {
    resolve_tool("deno")
}

/// Runs Deno's static type-check against script entries referenced by expanded pages.
///
/// An entry is a project-relative `scripts/**/*.ts` file referenced by an HTML page.
/// The check runs inside a temporary [`BuildWorkspace`], never against the project directly,
/// so its `deno.json` (holding the browser library configuration) is never written into the
/// project. Tool resolution happens even when there are no entries so a missing Deno
/// installation cannot be mistaken for a successful skipped stage.
pub fn type_check_scripts(
    root: &Path,
    expanded_pages: &std::collections::BTreeMap<PathBuf, String>,
) -> ValidationReport {
    let mut report = ValidationReport::new();
    let result = type_check_entries(root, expanded_pages);
    if let Err(diagnostic) = result {
        report.add(diagnostic);
    }
    report.sort();
    report
}

/// Alias for the build pipeline's entry-oriented terminology.
///
/// Builds its own transient [`BuildWorkspace`] (populated with the reachable script set) so a
/// caller not yet wired to the pipeline's shared workspace still gets a working type-check.
pub fn type_check_entries(
    root: &Path,
    expanded_pages: &std::collections::BTreeMap<PathBuf, String>,
) -> Result<(), Diagnostic> {
    let path_env = std::env::var_os("PATH");
    resolve_tool_with_path("deno", path_env.as_deref())?;

    let entries = discover_script_entries(root, expanded_pages);
    if entries.is_empty() {
        return Ok(());
    }

    let workspace = BuildWorkspace::new(root).map_err(|fault| {
        Diagnostic::new(
            codes::BUILD_TYPE_ERROR,
            Severity::Error,
            None,
            format!("Unable to prepare the build workspace for type-checking: {fault}"),
            "Ensure the system temporary directory is writable",
        )
    })?;
    let reachable = reachable_script_paths(root, entries);
    workspace.copy_scripts(root, &reachable).map_err(|fault| {
        Diagnostic::new(
            codes::BUILD_TYPE_ERROR,
            Severity::Error,
            None,
            format!("Unable to populate the build workspace for type-checking: {fault}"),
            "Ensure the system temporary directory is writable",
        )
    })?;

    type_check_entries_with_path(root, expanded_pages, &workspace, path_env.as_deref())
}

/// Bundles each type-checked script entry into the staged `dist/` tree.
///
/// Deno is resolved through the shared PATH resolver. Esbuild is deliberately
/// specified as a pinned npm package in the Deno invocation, so it is never a
/// second PATH dependency. Entries must be project-relative `scripts/**/*.ts`
/// paths; their output paths follow the fixed source-to-dist mapping.
///
/// Builds its own transient [`BuildWorkspace`] (populated with the reachable script set, the
/// same way [`type_check_entries`] does) so a caller not yet wired to the pipeline's shared
/// workspace still gets a working transform. Esbuild runs with the workspace as its `current_dir`
/// so the emitted bundle never carries an absolute path or a segment from outside the workspace.
pub fn transform_script_entries(
    root: &Path,
    output_dir: &Path,
    entries: &[PathBuf],
) -> Result<(), Diagnostic> {
    let workspace = BuildWorkspace::new(root).map_err(|fault| {
        Diagnostic::new(
            codes::BUILD_TYPE_ERROR,
            Severity::Error,
            None,
            format!("Unable to prepare the build workspace for script transformation: {fault}"),
            "Ensure the system temporary directory is writable",
        )
    })?;
    let reachable = reachable_script_paths(root, entries.iter().cloned());
    workspace.copy_scripts(root, &reachable).map_err(|fault| {
        Diagnostic::new(
            codes::BUILD_TYPE_ERROR,
            Severity::Error,
            None,
            format!("Unable to populate the build workspace for script transformation: {fault}"),
            "Ensure the system temporary directory is writable",
        )
    })?;

    transform_script_entries_with_path(
        &workspace,
        output_dir,
        entries,
        std::env::var_os("PATH").as_deref(),
    )
}

/// Transforms the script entries discovered from expanded pages.
pub fn transform_scripts(
    root: &Path,
    output_dir: &Path,
    expanded_pages: &std::collections::BTreeMap<PathBuf, String>,
) -> Result<(), Diagnostic> {
    let entries = discover_script_entries(root, expanded_pages);
    transform_script_entries(root, output_dir, &entries)
}

/// Runs `deno run -A npm:esbuild@0.25.5 … --bundle --format=esm` against `entries` inside
/// `workspace`, so esbuild resolves imports the same way the workspace's `deno check` does:
/// with the workspace's copied `deno.lock` and (when present) its `node_modules/`. `--frozen`
/// is deliberately never passed here: unlike `deno install` and `deno check`, esbuild's `deno
/// run` invocation is not itself a lock-writing command, and probing shows `--frozen` makes it
/// exit 1 with "The lockfile is out of date" because esbuild and its per-platform binaries are
/// not themselves `deno.lock` entries. `entries` are workspace-relative, matching
/// `current_dir(workspace.path())`; `--outfile` stays the absolute, already-mapped stage path.
pub(crate) fn transform_script_entries_with_path(
    workspace: &BuildWorkspace,
    output_dir: &Path,
    entries: &[PathBuf],
    path_env: Option<&OsStr>,
) -> Result<(), Diagnostic> {
    let deno = resolve_tool_with_path("deno", path_env)?;
    let mut sorted_entries = entries.to_vec();
    sorted_entries.sort();

    for entry in sorted_entries {
        let Some(dist_path) = crate::build::layout::map_source_to_dist(&entry) else {
            return Err(Diagnostic::new(
                codes::BUILD_TYPE_ERROR,
                Severity::Error,
                Some(Location::path_only(&entry)),
                format!(
                    "Script entry '{}' is outside the supported scripts mapping",
                    entry.display()
                ),
                "Reference a TypeScript entry below scripts/",
            ));
        };

        let output_rel = dist_path.strip_prefix("dist").unwrap_or(&dist_path);
        let output_path = output_dir.join(output_rel);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                Diagnostic::new(
                    codes::BUILD_TYPE_ERROR,
                    Severity::Error,
                    Some(Location::path_only(&output_path)),
                    format!("Unable to prepare transformed script output: {err}"),
                    "Ensure the build output directory is writable",
                )
            })?;
        }

        let output = Command::new(&deno)
            .current_dir(workspace.path())
            .arg("run")
            .arg("-A")
            .arg("npm:esbuild@0.25.5")
            .arg(&entry)
            .arg("--bundle")
            .arg("--format=esm")
            .arg(format!("--outfile={}", output_path.display()))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
            .map_err(|err| {
                Diagnostic::new(
                    codes::BUILD_TYPE_ERROR,
                    Severity::Error,
                    Some(Location::path_only(&entry)),
                    format!("Deno esbuild transform could not be started: {err}"),
                    "Ensure the resolved deno executable can run",
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(Diagnostic::new(
                codes::BUILD_TYPE_ERROR,
                Severity::Error,
                Some(Location::path_only(&entry)),
                if stderr.is_empty() {
                    format!(
                        "Deno esbuild transform failed with status {}",
                        output.status
                    )
                } else {
                    format!("Deno esbuild transform failed: {stderr}")
                },
                "Fix the TypeScript source and run the build again",
            ));
        }
    }

    Ok(())
}

/// Runs `deno check --frozen` against `entries` inside `workspace`, so the checker resolves
/// imports the same way the eventual esbuild transform does: with the workspace's `deno.json`,
/// its copied `deno.lock`, and (when present) its `node_modules/`. `entries` are joined onto
/// `workspace.path()` and passed as absolute paths; `--config` points at the workspace's own
/// `deno.json`, never the project's.
pub(crate) fn type_check_entries_with_path(
    root: &Path,
    expanded_pages: &std::collections::BTreeMap<PathBuf, String>,
    workspace: &BuildWorkspace,
    path_env: Option<&OsStr>,
) -> Result<(), Diagnostic> {
    let deno = resolve_tool_with_path("deno", path_env)?;
    let entries = discover_script_entries(root, expanded_pages);
    if entries.is_empty() {
        return Ok(());
    }

    let mut command = Command::new(&deno);
    command
        .current_dir(workspace.path())
        .arg("check")
        .arg("--config")
        .arg(workspace.path().join("deno.json"))
        .arg("--frozen")
        .args(entries.iter().map(|entry| workspace.path().join(entry)))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    let output = command.output().map_err(|err| {
        Diagnostic::new(
            codes::BUILD_TYPE_ERROR,
            Severity::Error,
            None,
            format!("Deno type-check could not be started: {err}"),
            "Ensure the resolved deno executable can be run",
        )
    })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stderr = rewrite_workspace_paths(&stderr, workspace.path(), root);
    let location = entries.first().map(|entry| Location::path_only(entry));
    Err(Diagnostic::new(
        codes::BUILD_TYPE_ERROR,
        Severity::Error,
        location,
        if stderr.is_empty() {
            format!("Deno type-check failed with status {}", output.status)
        } else {
            format!("Deno type-check failed: {stderr}")
        },
        "Fix the TypeScript diagnostics and run the build again",
    ))
}

/// Rewrites every occurrence of `workspace_path` in `stderr` with `root`'s equivalent, so a
/// type-check diagnostic names project-relative files instead of the workspace's temporary
/// location. Both of `workspace_path`'s printed forms are covered: its `file:///` URL form and
/// its native OS-path form. A third form is covered as well: `workspace_path` run through
/// `fs::canonicalize` and stripped of the Windows `\\?\` verbatim prefix, because Deno can print
/// that resolved form instead of the raw one (observed when the system temp directory carries an
/// 8.3 short name). When the canonicalized form is identical to the raw one, it is replaced only
/// once.
fn rewrite_workspace_paths(stderr: &str, workspace_path: &Path, root: &Path) -> String {
    let native = workspace_path.to_string_lossy().into_owned();
    let root_native = root.to_string_lossy().into_owned();
    let root_url = file_url_form(root);

    let mut rewritten = stderr.replace(&file_url_form(workspace_path), &root_url);
    rewritten = rewritten.replace(&native, &root_native);

    if let Ok(canonical) = fs::canonicalize(workspace_path) {
        let canonical_native = canonical.to_string_lossy().into_owned();
        let stripped = canonical_native
            .strip_prefix(r"\\?\")
            .unwrap_or(&canonical_native)
            .to_string();
        if stripped != native {
            let stripped_path = PathBuf::from(&stripped);
            rewritten = rewritten.replace(&file_url_form(&stripped_path), &root_url);
            rewritten = rewritten.replace(&stripped, &root_native);
        }
    }

    rewritten
}

/// Renders `path` as a `file://` URL the way Deno prints paths in diagnostics: forward slashes
/// throughout, with the drive letter (on Windows) directly after the third slash.
fn file_url_form(path: &Path) -> String {
    let forward = path.to_string_lossy().replace('\\', "/");
    if forward.starts_with('/') {
        format!("file://{forward}")
    } else {
        format!("file:///{forward}")
    }
}

fn discover_script_entries(
    root: &Path,
    expanded_pages: &std::collections::BTreeMap<PathBuf, String>,
) -> Vec<PathBuf> {
    let mut entries = BTreeSet::new();
    for (page, content) in expanded_pages {
        for reference in crate::build::layout::extract_references_from_html(content, page) {
            if !matches!(
                crate::build::layout::classify_reference(&reference.raw_url),
                crate::build::layout::ReferenceKind::FilePath(_)
            ) {
                continue;
            }
            let candidate =
                crate::build::layout::resolve_reference_target(&reference.raw_url, page);
            let normalized = candidate.to_string_lossy().replace('\\', "/");
            if normalized.starts_with("scripts/")
                && normalized.ends_with(".ts")
                && root.join(&candidate).is_file()
            {
                entries.insert(candidate);
            }
        }
    }
    entries.into_iter().collect()
}

/// Walks `entries` and every local (`./`- or `/`-prefixed) import specifier reachable from
/// them, returning the ordered set of project-relative script paths visited. A local
/// specifier that does not resolve to an existing file under `root` is skipped rather than
/// treated as an error; that decision belongs to callers that need to distinguish it (a
/// missing local import fails type-checking and bundling on its own).
pub(crate) fn reachable_script_paths(
    root: &Path,
    entries: impl IntoIterator<Item = PathBuf>,
) -> BTreeSet<PathBuf> {
    let mut pending: Vec<PathBuf> = entries.into_iter().collect();
    let mut visited = BTreeSet::new();
    while let Some(source_rel) = pending.pop() {
        if !visited.insert(source_rel.clone()) {
            continue;
        }
        let Ok(source) = fs::read_to_string(root.join(&source_rel)) else {
            continue;
        };
        for specifier in extract_module_specifiers(&source) {
            if is_local_module_specifier(&specifier) {
                let local = resolve_script_import(&source_rel, &specifier);
                if root.join(&local).is_file() && !visited.contains(&local) {
                    pending.push(local);
                }
            }
        }
    }
    visited
}

/// A transient, drop-guarded workspace directory that runs `deno install` and Deno's
/// type-checker with npm's `node_modules` layout enabled, without ever writing that
/// configuration into the project itself.
///
/// Created fresh under the system temporary directory for each build, populated with a
/// WDA-owned `deno.json` plus a copy of the project's `package.json` and `deno.lock`
/// (when present), and removed on drop so a failed or interrupted build never leaves an
/// orphaned workspace behind.
pub(crate) struct BuildWorkspace {
    path: PathBuf,
}

impl BuildWorkspace {
    /// Creates a fresh workspace directory under the system temporary directory, writes
    /// its `deno.json` (npm-compatible `node_modules` layout, same `lib` list used for
    /// type-checking), and copies `package.json` and `deno.lock` from `root` into it when
    /// each file exists.
    ///
    /// Filesystem failures map to [`ToolFault`]. The workspace directory created before a
    /// later failure is still removed: once `path` is wrapped in `Self`, an early return
    /// via `?` drops it and its `Drop` impl reclaims the directory.
    pub(crate) fn new(root: &Path) -> Result<Self, ToolFault> {
        let path = std::env::temp_dir().join(format!(
            "wda-build-ws-{}-{}",
            std::process::id(),
            crate::deps::resolve::unique_staging_suffix()
        ));
        // `create_dir` (not `create_dir_all`) so an existing directory at this
        // path surfaces as an error instead of being silently reused: two
        // workspaces sharing one directory would let whichever drops first
        // delete it out from under the other.
        fs::create_dir(&path).map_err(|error| {
            ToolFault::new(
                format!("Failed to create build workspace directory: {error}"),
                Some(path.clone()),
            )
        })?;

        let workspace = Self { path };

        let config_path = workspace.path.join("deno.json");
        fs::write(&config_path, workspace.deno_json()).map_err(|error| {
            ToolFault::new(
                format!("Failed to write build workspace configuration: {error}"),
                Some(config_path),
            )
        })?;

        for name in ["package.json", "deno.lock"] {
            let source = root.join(name);
            if !source.is_file() {
                continue;
            }
            let destination = workspace.path.join(name);
            fs::copy(&source, &destination).map_err(|error| {
                ToolFault::new(
                    format!("Failed to copy '{name}' into the build workspace: {error}"),
                    Some(source),
                )
            })?;
        }

        Ok(workspace)
    }

    fn deno_json(&self) -> String {
        format!(
            "{{\n  \"nodeModulesDir\": \"manual\",\n  \"compilerOptions\": {{\n    \"lib\": [{}]\n  }}\n}}\n",
            deno_check_lib_json_array()
        )
    }

    /// The absolute path to the workspace directory.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Copies each project-relative script path in `paths` from `root` into the workspace
    /// at the same relative path, creating parent directories as needed.
    pub(crate) fn copy_scripts(
        &self,
        root: &Path,
        paths: &BTreeSet<PathBuf>,
    ) -> Result<(), ToolFault> {
        for relative in paths {
            let source = root.join(relative);
            let destination = self.path.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    ToolFault::new(
                        format!(
                            "Failed to prepare build workspace directory for '{}': {error}",
                            relative.display()
                        ),
                        Some(destination.clone()),
                    )
                })?;
            }
            fs::copy(&source, &destination).map_err(|error| {
                ToolFault::new(
                    format!(
                        "Failed to copy '{}' into the build workspace: {error}",
                        relative.display()
                    ),
                    Some(source),
                )
            })?;
        }
        Ok(())
    }

    /// Runs `deno install --frozen` inside the workspace when it holds a copied
    /// `package.json`, populating `<workspace>/node_modules/` from the already-resolved
    /// lock. When the workspace has no `package.json`, returns `Ok(())` without spawning a
    /// process, so a dependency-free project never touches the network.
    ///
    /// `--frozen` is passed so a lock that does not satisfy `package.json` is reported as a
    /// failure instead of being silently rewritten; `--allow-scripts` is never passed, so no
    /// npm lifecycle script runs. On a non-zero exit, the returned diagnostic carries Deno's
    /// stderr verbatim: it may name a stale lock ("The lockfile is out of date") or a fetch
    /// error on a cold, offline cache, and this method does not assert which.
    pub(crate) fn install_dependencies(&self, path_env: Option<&OsStr>) -> Result<(), Diagnostic> {
        if !self.path.join("package.json").is_file() {
            return Ok(());
        }

        let deno = resolve_tool_with_path("deno", path_env)?;
        let output = Command::new(&deno)
            .current_dir(&self.path)
            .arg("install")
            .arg("--frozen")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
            .map_err(|err| {
                Diagnostic::new(
                    codes::BUILD_DEPENDENCY_UNLOCKED,
                    Severity::Error,
                    None,
                    format!("`deno install --frozen` could not be started: {err}"),
                    "Run `wda deps` to update the dependency graph, then run the build again",
                )
            })?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(Diagnostic::new(
            codes::BUILD_DEPENDENCY_UNLOCKED,
            Severity::Error,
            None,
            if stderr.is_empty() {
                format!(
                    "`deno install --frozen` failed in the build workspace with status {}",
                    output.status
                )
            } else {
                format!("`deno install --frozen` failed in the build workspace: {stderr}")
            },
            "Run `wda deps` to update the dependency graph, then run the build again",
        ))
    }
}

impl Drop for BuildWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Resolves a tool with an explicit PATH environment variable value.
pub(crate) fn resolve_tool_with_path(
    tool_name: &str,
    path_env: Option<&OsStr>,
) -> Result<PathBuf, Diagnostic> {
    let trimmed = tool_name.trim();
    if trimmed.is_empty() {
        return Err(Diagnostic::new(
            codes::BUILD_TOOL_UNAVAILABLE,
            Severity::Error,
            None,
            "Required tool name cannot be empty",
            "Specify a valid tool name",
        ));
    }

    let has_separator = trimmed.contains('/') || trimmed.contains('\\');
    if has_separator {
        let p = Path::new(trimmed);
        for name in candidate_names(p) {
            let candidate = PathBuf::from(name);
            if is_executable(&candidate) {
                return verify_can_start(trimmed, &candidate);
            }
        }
        return Err(Diagnostic::new(
            codes::BUILD_TOOL_UNAVAILABLE,
            Severity::Error,
            None,
            format!("Required tool '{trimmed}' was not found at specified path"),
            format!("Ensure '{trimmed}' exists and is executable"),
        ));
    }

    let search_dirs: Vec<PathBuf> = match path_env {
        Some(val) => std::env::split_paths(val).collect(),
        None => Vec::new(),
    };

    let candidates = candidate_names(Path::new(trimmed));

    for dir in search_dirs {
        for cand in &candidates {
            let candidate_path = dir.join(cand);
            if is_executable(&candidate_path) {
                return verify_can_start(trimmed, &candidate_path);
            }
        }
    }

    Err(Diagnostic::new(
        codes::BUILD_TOOL_UNAVAILABLE,
        Severity::Error,
        None,
        format!("Required tool '{trimmed}' was not found on PATH"),
        format!("Ensure '{trimmed}' is installed and available on PATH"),
    ))
}

#[cfg(windows)]
fn candidate_names(p: &Path) -> Vec<String> {
    let base = p.to_string_lossy().into_owned();
    if p.extension().is_some() {
        return vec![base];
    }

    let mut names = vec![base.clone()];
    let default_exts = [".exe", ".cmd", ".bat", ".com"];
    if let Some(pathext) = std::env::var_os("PATHEXT") {
        if let Some(s) = pathext.to_str() {
            for ext in s.split(';') {
                let trimmed = ext.trim();
                if !trimmed.is_empty() {
                    let with_dot = if trimmed.starts_with('.') {
                        trimmed.to_string()
                    } else {
                        format!(".{trimmed}")
                    };
                    let upper = format!("{base}{with_dot}");
                    let lower = format!("{base}{}", with_dot.to_lowercase());
                    if !names.contains(&upper) {
                        names.push(upper);
                    }
                    if !names.contains(&lower) {
                        names.push(lower);
                    }
                }
            }
        }
    } else {
        for ext in &default_exts {
            let cand = format!("{base}{ext}");
            if !names.contains(&cand) {
                names.push(cand);
            }
        }
    }
    names
}

#[cfg(not(windows))]
fn candidate_names(p: &Path) -> Vec<String> {
    vec![p.to_string_lossy().into_owned()]
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = path.metadata() {
            return meta.permissions().mode() & 0o111 != 0;
        }
        false
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn verify_can_start(tool_name: &str, path: &Path) -> Result<PathBuf, Diagnostic> {
    let mut cmd = Command::new(path);
    cmd.arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    match cmd.output() {
        Ok(output) => {
            let is_deno = tool_name == "deno" || tool_name == "deno.exe";
            if is_deno && !output.status.success() {
                Err(Diagnostic::new(
                    codes::BUILD_TOOL_UNAVAILABLE,
                    Severity::Error,
                    None,
                    format!(
                        "Required tool '{tool_name}' at '{}' failed to start (exit status: {})",
                        path.display(),
                        output.status
                    ),
                    format!("Ensure '{tool_name}' is properly installed and functional"),
                ))
            } else {
                Ok(path.to_path_buf())
            }
        }
        Err(err) => Err(Diagnostic::new(
            codes::BUILD_TOOL_UNAVAILABLE,
            Severity::Error,
            None,
            format!(
                "Required tool '{tool_name}' at '{}' could not be executed: {err}",
                path.display()
            ),
            format!("Ensure '{tool_name}' has execute permissions and can run"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root(label: &str) -> PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "wda_type_check_{label}_{}_{}",
            std::process::id(),
            suffix
        ));
        std::fs::create_dir_all(root.join("scripts")).unwrap();
        root
    }

    fn page_with_script(script: &str) -> std::collections::BTreeMap<PathBuf, String> {
        std::collections::BTreeMap::from([(
            PathBuf::from("pages/index.html"),
            format!("<script type=\"module\" src=\"scripts/{script}\"></script>"),
        )])
    }

    fn transform_fixture_root(label: &str) -> PathBuf {
        let root = fixture_root(&format!("transform_{label}"));
        std::fs::write(
            root.join("scripts/dep.ts"),
            "export const answer: number = 42;\n",
        )
        .unwrap();
        std::fs::write(
            root.join("scripts/main.ts"),
            "import { answer } from './dep.ts';\nexport const result = answer;\n",
        )
        .unwrap();
        root
    }

    fn ts_module_specifier_count(source: &str) -> usize {
        source
            .lines()
            .filter(|line| {
                let line = line.trim();
                let specifier = line
                    .split_once(" from ")
                    .map(|(_, rest)| rest)
                    .or_else(|| line.strip_prefix("import "));
                specifier.is_some_and(|specifier| {
                    specifier
                        .trim()
                        .trim_matches(|character| matches!(character, '\"' | '\'' | ';'))
                        .ends_with(".ts")
                })
            })
            .count()
    }

    #[test]
    fn transform_maps_entries_and_removes_ts_module_specifiers_deterministically() {
        let root = transform_fixture_root("mapped");
        let first_output =
            std::env::temp_dir().join(format!("wda_transform_output_{}_first", std::process::id()));
        let second_output = std::env::temp_dir().join(format!(
            "wda_transform_output_{}_second",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&first_output);
        let _ = std::fs::remove_dir_all(&second_output);

        assert_eq!(
            ts_module_specifier_count("// scripts/main.ts\nconst value = 'dep.ts';"),
            0
        );
        assert_eq!(ts_module_specifier_count("import './dep.ts';"), 1);
        let entries = [PathBuf::from("scripts/main.ts")];
        transform_script_entries(&root, &first_output, &entries).unwrap();
        transform_script_entries(&root, &second_output, &entries).unwrap();

        let first_path = first_output.join("scripts/main.js");
        let second_path = second_output.join("scripts/main.js");
        assert!(first_path.is_file(), "mapped output must be emitted");
        let first_bytes = std::fs::read(&first_path).unwrap();
        let second_bytes = std::fs::read(&second_path).unwrap();
        let first_text = String::from_utf8(first_bytes.clone()).unwrap();
        assert_eq!(ts_module_specifier_count(&first_text), 0);
        assert_eq!(
            first_bytes, second_bytes,
            "unchanged input must be deterministic"
        );

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&first_output);
        let _ = std::fs::remove_dir_all(&second_output);
    }

    #[test]
    fn transform_reports_unresolvable_deno() {
        let root = transform_fixture_root("missing_deno");
        let output =
            std::env::temp_dir().join(format!("wda_transform_missing_deno_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&output);

        let workspace = BuildWorkspace::new(&root).expect("workspace creation must succeed");
        let diagnostic = transform_script_entries_with_path(
            &workspace,
            &output,
            &[PathBuf::from("scripts/main.ts")],
            Some(OsStr::new("")),
        )
        .expect_err("an empty PATH must make Deno unavailable");
        assert_eq!(diagnostic.code, codes::BUILD_TOOL_UNAVAILABLE);
        assert_eq!(diagnostic.severity, Severity::Error);

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&output);
    }

    #[test]
    fn transform_resolves_node_modules_from_workspace_not_project_ancestor() {
        // `ancestor` plants a `node_modules/` above `root` holding a package of the same
        // name as the one the workspace itself provides, with content that lets the test
        // tell which copy esbuild actually resolved. `workspace` (created by
        // `BuildWorkspace::new`) is an unrelated directory elsewhere under the system temp
        // directory, so this only proves isolation when `current_dir` is the workspace, not
        // `root`: with `current_dir(root)`, esbuild's Node-style resolution would walk up
        // from `root/scripts/main.ts` and find the ancestor copy.
        let ancestor = std::env::temp_dir().join(format!(
            "wda_transform_ancestor_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = ancestor.join("project");
        std::fs::create_dir_all(root.join("scripts")).unwrap();

        let fake_pkg = ancestor.join("node_modules/mini-pkg");
        std::fs::create_dir_all(&fake_pkg).unwrap();
        std::fs::write(
            fake_pkg.join("package.json"),
            "{\"name\": \"mini-pkg\", \"main\": \"index.js\"}\n",
        )
        .unwrap();
        std::fs::write(
            fake_pkg.join("index.js"),
            "export const marker = 'FAKE_ANCESTOR_MARKER';\n",
        )
        .unwrap();

        std::fs::write(
            root.join("scripts/main.ts"),
            "import { marker } from 'mini-pkg';\nexport const result = marker;\n",
        )
        .unwrap();

        let entries = [PathBuf::from("scripts/main.ts")];
        let workspace = BuildWorkspace::new(&root).expect("workspace creation must succeed");
        let reachable = reachable_script_paths(&root, entries.iter().cloned());
        workspace.copy_scripts(&root, &reachable).unwrap();

        let real_pkg = workspace.path().join("node_modules/mini-pkg");
        std::fs::create_dir_all(&real_pkg).unwrap();
        std::fs::write(
            real_pkg.join("package.json"),
            "{\"name\": \"mini-pkg\", \"main\": \"index.js\"}\n",
        )
        .unwrap();
        std::fs::write(
            real_pkg.join("index.js"),
            "export const marker = 'REAL_WORKSPACE_MARKER';\n",
        )
        .unwrap();

        let output = std::env::temp_dir().join(format!(
            "wda_transform_workspace_cwd_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&output);

        let workspace_abs = workspace.path().to_string_lossy().into_owned();
        let workspace_dir_name = workspace
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        transform_script_entries_with_path(
            &workspace,
            &output,
            &entries,
            std::env::var_os("PATH").as_deref(),
        )
        .expect("transform must succeed when the package resolves from the workspace");

        let bundle = std::fs::read_to_string(output.join("scripts/main.js")).unwrap();
        let _ = std::fs::remove_dir_all(&ancestor);
        let _ = std::fs::remove_dir_all(&output);

        assert!(
            bundle.contains("REAL_WORKSPACE_MARKER"),
            "bundle must contain the package content resolved from the workspace's own \
             node_modules, got: {bundle}"
        );
        assert!(
            !bundle.contains("FAKE_ANCESTOR_MARKER"),
            "bundle must not resolve node_modules from an ancestor of the project root, got: \
             {bundle}"
        );
        assert!(
            !bundle.contains(&workspace_abs) && !bundle.contains(&workspace_dir_name),
            "bundle must carry no absolute path or path segment naming the workspace \
             directory, got: {bundle}"
        );
        if let Ok(profile) = std::env::var("USERPROFILE") {
            assert!(
                !bundle.contains(&profile),
                "bundle must not leak the user profile path, got: {bundle}"
            );
        }
    }

    #[test]
    fn test_resolve_tool_impossible_name_yields_tool_unavailable_error() {
        let impossible_name = "impossible_tool_name_wda_test_nonexistent";
        let res = resolve_tool(impossible_name);
        assert!(res.is_err(), "impossible tool name must fail resolution");
        let diag = res.unwrap_err();
        assert_eq!(diag.code, codes::BUILD_TOOL_UNAVAILABLE);
        assert_eq!(diag.severity, Severity::Error);
        assert!(
            diag.message.contains(impossible_name),
            "diagnostic message must name the tool '{}', got: {}",
            impossible_name,
            diag.message
        );
        assert!(diag.location.is_none());
    }

    #[test]
    fn test_resolve_deno_returns_path_and_can_start() {
        let res = resolve_deno();
        assert!(
            res.is_ok(),
            "deno must be resolvable on PATH in CI/dev environment, got: {:?}",
            res.err()
        );
        let path = res.unwrap();
        assert!(
            path.is_file(),
            "resolved path must be an existing file: {}",
            path.display()
        );
    }

    #[test]
    fn test_resolve_tool_empty_name_fails() {
        let res = resolve_tool("");
        assert!(res.is_err());
        let diag = res.unwrap_err();
        assert_eq!(diag.code, codes::BUILD_TOOL_UNAVAILABLE);
        assert_eq!(diag.severity, Severity::Error);
    }

    #[test]
    fn test_resolve_tool_with_custom_empty_path() {
        let res = resolve_tool_with_path("deno", None);
        assert!(res.is_err());
        let diag = res.unwrap_err();
        assert_eq!(diag.code, codes::BUILD_TOOL_UNAVAILABLE);
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("deno"));
    }

    #[test]
    fn test_resolve_tool_cannot_start_yields_tool_unavailable_error() {
        let temp_dir =
            std::env::temp_dir().join(format!("wda_test_cannot_start_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create an invalid/corrupted binary file that cannot be executed by the OS
        #[cfg(windows)]
        let tool_file = temp_dir.join("corrupted_tool.exe");
        #[cfg(not(windows))]
        let tool_file = temp_dir.join("corrupted_tool");

        std::fs::write(&tool_file, "not an executable binary").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&tool_file).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&tool_file, perms).unwrap();
        }

        let res = resolve_tool_with_path("corrupted_tool", Some(temp_dir.as_os_str()));
        assert!(res.is_err(), "non-startable tool must yield Error");
        let diag = res.unwrap_err();
        assert_eq!(diag.code, codes::BUILD_TOOL_UNAVAILABLE);
        assert_eq!(diag.severity, Severity::Error);
        assert!(
            diag.message.contains("corrupted_tool"),
            "error message must name the tool"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn type_check_accepts_browser_dom_globals() {
        let root = fixture_root("dom");
        std::fs::write(
            root.join("scripts/main.ts"),
            "const element: HTMLElement = document.body;\ndocument.title = element.tagName;",
        )
        .unwrap();
        let pages = page_with_script("main.ts");

        let result = type_check_entries(&root, &pages);
        let _ = std::fs::remove_dir_all(&root);
        assert!(
            result.is_ok(),
            "browser DOM source must type-check: {result:?}"
        );
    }

    #[test]
    fn type_check_reports_real_type_errors() {
        let root = fixture_root("error");
        std::fs::write(
            root.join("scripts/main.ts"),
            "const count: number = 'not a number';",
        )
        .unwrap();
        let pages = page_with_script("main.ts");

        let result = type_check_entries(&root, &pages);
        let _ = std::fs::remove_dir_all(&root);
        let diagnostic = result.expect_err("a real TypeScript error must fail the check");
        assert_eq!(diagnostic.code, codes::BUILD_TYPE_ERROR);
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(
            diagnostic.message.contains("scripts/main.ts") || diagnostic.message.contains("scripts\\main.ts"),
            "type-check failure must name the project-relative script, got: {}",
            diagnostic.message
        );
        assert!(
            !diagnostic.message.contains("wda-build-ws"),
            "type-check failure must not leak the workspace directory name, got: {}",
            diagnostic.message
        );
    }

    #[test]
    fn type_check_succeeds_when_page_has_no_script_entries() {
        let root = fixture_root("empty");
        let pages = std::collections::BTreeMap::from([(
            PathBuf::from("pages/index.html"),
            "<main>No scripts</main>".to_string(),
        )]);

        let result = type_check_entries(&root, &pages);
        let _ = std::fs::remove_dir_all(&root);
        assert!(
            result.is_ok(),
            "an entry-free project has no type diagnostics"
        );
    }

    #[test]
    fn type_check_reports_unresolvable_deno() {
        let root = fixture_root("missing-deno");
        let pages = std::collections::BTreeMap::new();
        let workspace = BuildWorkspace::new(&root).expect("workspace creation must succeed");

        let diagnostic =
            type_check_entries_with_path(&root, &pages, &workspace, Some(OsStr::new("")))
                .expect_err("an empty PATH must not skip the type-check stage");
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(diagnostic.code, codes::BUILD_TOOL_UNAVAILABLE);
        assert_eq!(diagnostic.severity, Severity::Error);
    }

    #[test]
    fn rewrite_workspace_paths_replaces_native_and_url_forms_with_the_project_root() {
        let workspace_path = PathBuf::from(if cfg!(windows) {
            r"C:\Users\tester\AppData\Local\Temp\wda-build-ws-1-2"
        } else {
            "/tmp/wda-build-ws-1-2"
        });
        let root = PathBuf::from(if cfg!(windows) {
            r"C:\Code\myproject"
        } else {
            "/home/tester/myproject"
        });
        let entry = workspace_path.join("scripts").join("main.ts");
        let native_message = format!("TS2322 [ERROR]: at {}:1:7", entry.display());
        let url_message = format!("TS2322 [ERROR]: at {}:1:7", file_url_form(&entry));

        let rewritten_native = rewrite_workspace_paths(&native_message, &workspace_path, &root);
        let rewritten_url = rewrite_workspace_paths(&url_message, &workspace_path, &root);

        assert!(
            rewritten_native.contains(&root.to_string_lossy().into_owned())
                && !rewritten_native.contains("wda-build-ws"),
            "native-form stderr must be rewritten to the project root, got: {rewritten_native}"
        );
        assert!(
            rewritten_url.contains(&file_url_form(&root)) && !rewritten_url.contains("wda-build-ws"),
            "URL-form stderr must be rewritten to the project root, got: {rewritten_url}"
        );
    }

    #[test]
    fn locked_dependency_check_reports_unlocked_specifier_without_dependency_state() {
        let root = fixture_root("locked-dependencies-no-state");
        std::fs::write(
            root.join("scripts/main.ts"),
            "import value from 'npm:unlocked-package@1';\nexport default value;\n",
        )
        .unwrap();
        let pages = page_with_script("main.ts");

        let report = check_locked_dependencies(&root, &pages);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(report.diagnostics.len(), 1);
        let diagnostic = &report.diagnostics[0];
        assert_eq!(diagnostic.code, codes::BUILD_DEPENDENCY_UNLOCKED);
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(diagnostic.message.contains("npm:unlocked-package@1"));
    }

    #[test]
    fn locked_dependency_check_accepts_project_without_script_entries() {
        let root = fixture_root("locked-dependencies-no-entries");
        let pages = std::collections::BTreeMap::from([(
            PathBuf::from("pages/index.html"),
            "<main>No scripts</main>".to_string(),
        )]);

        let report = check_locked_dependencies(&root, &pages);
        let _ = std::fs::remove_dir_all(&root);
        assert!(
            report.diagnostics.is_empty(),
            "an entry-free project produces no diagnostics: {report:?}"
        );
    }

    #[test]
    fn locked_dependency_check_reports_unlocked_specifier() {
        let root = fixture_root("locked-dependencies-unlocked");
        std::fs::write(
            root.join("scripts/main.ts"),
            "import value from 'npm:unlocked-package@1';\nexport default value;\n",
        )
        .unwrap();
        std::fs::write(root.join("deno.lock"), "{}\n").unwrap();
        let pages = page_with_script("main.ts");

        let report = check_locked_dependencies(&root, &pages);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(report.diagnostics.len(), 1);
        let diagnostic = &report.diagnostics[0];
        assert_eq!(diagnostic.code, codes::BUILD_DEPENDENCY_UNLOCKED);
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(diagnostic.message.contains("npm:unlocked-package@1"));
    }

    #[test]
    fn locked_dependency_check_does_not_modify_dependency_files() {
        let root = fixture_root("locked-dependencies-read-only");
        std::fs::write(
            root.join("scripts/main.ts"),
            "import value from 'npm:unlocked-package@1';\nexport default value;\n",
        )
        .unwrap();
        std::fs::write(root.join("deno.lock"), "{\"version\": \"4\"}\n").unwrap();
        std::fs::write(root.join("package.json"), "{\"name\": \"fixture\"}\n").unwrap();
        let lock_before = std::fs::read(root.join("deno.lock")).unwrap();
        let package_before = std::fs::read(root.join("package.json")).unwrap();
        let pages = page_with_script("main.ts");

        let _ = check_locked_dependencies(&root, &pages);

        assert_eq!(std::fs::read(root.join("deno.lock")).unwrap(), lock_before);
        assert_eq!(
            std::fs::read(root.join("package.json")).unwrap(),
            package_before
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn covers_matches_bare_specifier_and_subpath_against_locked_scoped_package() {
        let lock = LockState {
            keys: BTreeSet::from(["npm:@scope/name@1.0.0".to_string()]),
        };
        assert!(lock.covers("@scope/name"));
        assert!(lock.covers("@scope/name/sub/file.js"));
    }

    #[test]
    fn covers_matches_bare_specifier_and_subpath_against_locked_unscoped_package() {
        let lock = LockState {
            keys: BTreeSet::from(["npm:name@1.0.0".to_string()]),
        };
        assert!(lock.covers("name"));
        assert!(lock.covers("name/sub.js"));
    }

    #[test]
    fn covers_rejects_bare_specifier_and_subpath_when_lock_lacks_package() {
        let lock = LockState {
            keys: BTreeSet::from(["npm:other@1.0.0".to_string()]),
        };
        assert!(!lock.covers("@scope/name"));
        assert!(!lock.covers("@scope/name/sub/file.js"));
        assert!(!lock.covers("name"));
        assert!(!lock.covers("name/sub.js"));
    }

    #[test]
    fn covers_leaves_relative_absolute_and_url_specifiers_uncovered() {
        let lock = LockState {
            keys: BTreeSet::from(["npm:name@1.0.0".to_string()]),
        };
        assert!(!lock.covers("./x.ts"));
        assert!(!lock.covers("/x.ts"));
        assert!(!lock.covers("https://example.com/x.ts"));
    }

    #[test]
    fn covers_matches_versionless_npm_specifier_against_locked_package() {
        let lock = LockState {
            keys: BTreeSet::from(["npm:name@1.0.0".to_string()]),
        };
        assert!(lock.covers("npm:name"));
    }

    #[test]
    fn covers_excludes_transitive_dependency_present_only_in_npm_section() {
        let root = fixture_root("lock-state-transitive");
        std::fs::write(
            root.join("deno.lock"),
            r#"{
  "version": "5",
  "specifiers": {
    "npm:once@1.4.0": "1.4.0"
  },
  "npm": {
    "once@1.4.0": {
      "integrity": "sha512-once",
      "dependencies": ["wrappy@1.0.2"]
    },
    "wrappy@1.0.2": {
      "integrity": "sha512-wrappy"
    }
  }
}
"#,
        )
        .unwrap();

        let lock_state = read_lock_state(&root);
        let _ = std::fs::remove_dir_all(&root);
        assert!(
            !lock_state.covers("wrappy"),
            "a package present only as a transitive dependency under `npm` must not count as locked"
        );
        assert!(lock_state.covers("once"));
    }

    #[test]
    fn reachable_script_paths_walks_local_imports_and_skips_missing_files() {
        let root = fixture_root("reachable-walk");
        std::fs::write(
            root.join("scripts/main.ts"),
            "import './dep.ts';\nimport './missing.ts';\n",
        )
        .unwrap();
        std::fs::write(root.join("scripts/dep.ts"), "export const dep = 1;\n").unwrap();

        let entries = [PathBuf::from("scripts/main.ts")];
        let reachable = reachable_script_paths(&root, entries);
        assert_eq!(
            reachable,
            BTreeSet::from([
                PathBuf::from("scripts/main.ts"),
                PathBuf::from("scripts/dep.ts"),
            ]),
            "the missing local import must not appear in the reachable set"
        );

        let workspace = BuildWorkspace::new(&root).expect("workspace creation must succeed");
        workspace
            .copy_scripts(&root, &reachable)
            .expect("copying the reachable set must succeed");

        assert!(
            workspace.path().join("scripts/main.ts").is_file(),
            "the entry must be copied into the workspace"
        );
        assert!(
            workspace.path().join("scripts/dep.ts").is_file(),
            "the existing local import must be copied into the workspace"
        );
        assert!(
            !workspace.path().join("scripts/missing.ts").exists(),
            "the missing local import must not be copied"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn build_workspace_creates_directory_with_deno_json_and_no_project_files() {
        let root = fixture_root("build-workspace-empty");

        let workspace = BuildWorkspace::new(&root).expect("workspace creation must succeed");
        assert!(workspace.path().is_dir(), "workspace directory must exist");
        assert!(
            workspace.path().starts_with(std::env::temp_dir()),
            "workspace must live under the system temp directory"
        );
        assert!(
            !workspace.path().starts_with(&root),
            "workspace must never be created under the project root"
        );

        let deno_json = std::fs::read_to_string(workspace.path().join("deno.json"))
            .expect("deno.json must be written");
        assert!(deno_json.contains("\"nodeModulesDir\": \"manual\""));
        for lib in DENO_CHECK_LIB {
            assert!(
                deno_json.contains(&format!("\"{lib}\"")),
                "deno.json must list lib entry '{lib}'"
            );
        }

        assert!(
            !workspace.path().join("package.json").exists(),
            "package.json must not be copied when the project has none"
        );
        assert!(
            !workspace.path().join("deno.lock").exists(),
            "deno.lock must not be copied when the project has none"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn build_workspace_copies_existing_package_json_and_deno_lock() {
        let root = fixture_root("build-workspace-populated");
        std::fs::write(root.join("package.json"), "{\"name\": \"fixture\"}\n").unwrap();
        std::fs::write(root.join("deno.lock"), "{\"version\": \"5\"}\n").unwrap();

        let workspace = BuildWorkspace::new(&root).expect("workspace creation must succeed");
        assert_eq!(
            std::fs::read_to_string(workspace.path().join("package.json")).unwrap(),
            "{\"name\": \"fixture\"}\n"
        );
        assert_eq!(
            std::fs::read_to_string(workspace.path().join("deno.lock")).unwrap(),
            "{\"version\": \"5\"}\n"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn install_dependencies_skips_spawning_without_package_json() {
        let root = fixture_root("install-dependencies-no-package-json");

        let workspace = BuildWorkspace::new(&root).expect("workspace creation must succeed");
        let result = workspace.install_dependencies(std::env::var_os("PATH").as_deref());

        let _ = std::fs::remove_dir_all(&root);
        assert!(
            result.is_ok(),
            "a workspace without package.json must not spawn deno install: {result:?}"
        );
        assert!(
            !workspace.path().join("node_modules").exists(),
            "no node_modules must appear when there is no package.json"
        );
    }

    #[test]
    fn install_dependencies_reports_tool_unavailable_when_deno_is_missing() {
        let root = fixture_root("install-dependencies-missing-deno");
        std::fs::write(root.join("package.json"), "{\"name\": \"fixture\"}\n").unwrap();

        let workspace = BuildWorkspace::new(&root).expect("workspace creation must succeed");
        let result = workspace.install_dependencies(Some(OsStr::new("")));

        let _ = std::fs::remove_dir_all(&root);
        let diagnostic = result.expect_err("an empty PATH must make deno unavailable");
        assert_eq!(diagnostic.code, codes::BUILD_TOOL_UNAVAILABLE);
        assert_eq!(diagnostic.severity, Severity::Error);
    }

    #[test]
    fn install_dependencies_reports_dependency_unlocked_on_command_failure() {
        let root = fixture_root("install-dependencies-invalid-package-json");
        // Malformed JSON fails `deno install` locally, before any network access, so this
        // test exercises the non-zero-exit diagnostic path deterministically and offline.
        std::fs::write(root.join("package.json"), "{ this is not valid json").unwrap();

        let workspace = BuildWorkspace::new(&root).expect("workspace creation must succeed");
        let result = workspace.install_dependencies(std::env::var_os("PATH").as_deref());

        let workspace_path = workspace.path().to_path_buf();
        let _ = std::fs::remove_dir_all(&root);
        let diagnostic = result.expect_err("a malformed package.json must fail deno install");
        assert_eq!(diagnostic.code, codes::BUILD_DEPENDENCY_UNLOCKED);
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.location, None);
        assert!(
            diagnostic
                .message
                .starts_with("`deno install --frozen` failed in the build workspace"),
            "diagnostic message must name the failing command, got: {}",
            diagnostic.message
        );
        assert_eq!(
            diagnostic.next_action,
            "Run `wda deps` to update the dependency graph, then run the build again"
        );
        assert!(
            !workspace_path.join("node_modules").exists(),
            "a failed install must not leave node_modules behind"
        );
    }

    #[test]
    fn build_workspace_removes_directory_on_drop() {
        let root = fixture_root("build-workspace-drop");

        let workspace = BuildWorkspace::new(&root).expect("workspace creation must succeed");
        let workspace_path = workspace.path().to_path_buf();
        assert!(workspace_path.is_dir(), "workspace directory must exist before drop");

        drop(workspace);
        assert!(
            !workspace_path.exists(),
            "workspace directory must be removed after drop"
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}
