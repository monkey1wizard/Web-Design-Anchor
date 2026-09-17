//! Dependency graph management for Web Design Anchor (WDA) `wda deps`.

#![allow(dead_code)]

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::codes;
use crate::diagnostics::{Diagnostic, Severity, ToolFault};

pub mod manifest;
pub mod resolve;

/// Execution steps during atomic dependency commit where simulated interruptions can be injected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommitStep {
    /// Interruption point after `deno.lock` has been renamed into place, but before `package.json` is renamed.
    AfterLockRenamed,
    /// Interruption point after `deno.lock` has been deleted, but before `package.json` is deleted.
    AfterLockDeleted,
}

/// The outcome of dependency resolution or state modification to be committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitOutcome {
    /// Non-empty dependency state with staged `package.json` and optional/present `deno.lock`.
    NonEmpty {
        package_json: String,
        deno_lock: Option<String>,
    },
    /// Explicit empty / no-dependency state where both files must be deleted.
    Empty,
}

impl CommitOutcome {
    /// Creates a non-empty commit outcome.
    pub fn non_empty(package_json: impl Into<String>, deno_lock: impl Into<String>) -> Self {
        Self::NonEmpty {
            package_json: package_json.into(),
            deno_lock: Some(deno_lock.into()),
        }
    }

    /// Creates an empty-state commit outcome.
    pub fn empty() -> Self {
        Self::Empty
    }

    /// Returns `true` if this outcome represents the empty state (both files deleted).
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns the staged files if this outcome is non-empty.
    pub fn staged_files(&self) -> Option<resolve::StagedFiles> {
        match self {
            Self::NonEmpty {
                package_json,
                deno_lock,
            } => Some(resolve::StagedFiles {
                package_json: package_json.clone(),
                deno_lock: deno_lock.clone(),
            }),
            Self::Empty => None,
        }
    }
}

impl From<CommitOutcome> for Option<resolve::StagedFiles> {
    fn from(outcome: CommitOutcome) -> Self {
        outcome.staged_files()
    }
}

impl From<resolve::StagedFiles> for CommitOutcome {
    fn from(files: resolve::StagedFiles) -> Self {
        Self::NonEmpty {
            package_json: files.package_json,
            deno_lock: files.deno_lock,
        }
    }
}

impl From<&resolve::StagedFiles> for CommitOutcome {
    fn from(files: &resolve::StagedFiles) -> Self {
        Self::NonEmpty {
            package_json: files.package_json.clone(),
            deno_lock: files.deno_lock.clone(),
        }
    }
}

/// Errors that can occur during dependency management operations (`wda deps`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepsError {
    /// A diagnostic finding (e.g. tool fault or usage error).
    Diagnostic(Diagnostic),
    /// An internal tool fault (e.g. unreadable file or directory fault).
    ToolFault(ToolFault),
}

impl From<Diagnostic> for DepsError {
    fn from(diag: Diagnostic) -> Self {
        Self::Diagnostic(diag)
    }
}

impl From<ToolFault> for DepsError {
    fn from(fault: ToolFault) -> Self {
        Self::ToolFault(fault)
    }
}

impl std::fmt::Display for DepsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Diagnostic(diag) => write!(f, "{}", crate::diagnostics::render(diag)),
            Self::ToolFault(fault) => write!(f, "{}", crate::diagnostics::render_fault(fault)),
        }
    }
}

impl std::error::Error for DepsError {}

impl DepsError {
    /// Returns the inner [`Diagnostic`] if this is a diagnostic variant.
    pub fn diagnostic(&self) -> Option<&Diagnostic> {
        match self {
            Self::Diagnostic(diag) => Some(diag),
            Self::ToolFault(_) => None,
        }
    }

    /// Returns the inner [`ToolFault`] if this is a tool fault variant.
    pub fn tool_fault(&self) -> Option<&ToolFault> {
        match self {
            Self::ToolFault(fault) => Some(fault),
            Self::Diagnostic(_) => None,
        }
    }
}

/// RAII guard that cleans up a temporary file on drop unless defused.
struct TempFileGuard {
    path: PathBuf,
    active: bool,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, active: true }
    }

    fn defuse(&mut self) {
        self.active = false;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Commits dependency state changes to the project root with fixed ordering.
///
/// For non-empty state: writes temp files into project root, renames `deno.lock` first,
/// then `package.json` second, so the canonical declaration changes last.
/// For empty state: deletes `deno.lock` first, then `package.json` second, for the same reason.
/// If any fault occurs prior to commit, real project files are never touched.
pub fn commit_dependencies(root: &Path, outcome: &CommitOutcome) -> Result<(), ToolFault> {
    commit_dependencies_with_hook(root, outcome, |_| Ok(()))
}

/// Direct alias for [`commit_dependencies`].
pub fn commit(root: &Path, outcome: &CommitOutcome) -> Result<(), ToolFault> {
    commit_dependencies(root, outcome)
}

/// Commits staged dependency files or deletes them if `None` (representing empty state).
pub fn commit_staged(root: &Path, files: Option<&resolve::StagedFiles>) -> Result<(), ToolFault> {
    match files {
        Some(staged) => commit_dependencies(root, &CommitOutcome::from(staged)),
        None => commit_dependencies(root, &CommitOutcome::Empty),
    }
}

/// Adds a dependency specification to the project at `root`.
///
/// 1. Reads the current manifest (`package.json`) via [`manifest::read_manifest`].
/// 2. Invokes [`resolve::resolve_add`] in a staged, confined environment.
/// 3. Commits the staged `package.json` and `deno.lock` atomically via [`commit_dependencies`].
///
/// If any fault occurs prior to commit, real project files are never touched.
pub fn add(root: &Path, spec: &str) -> Result<resolve::StagedFiles, DepsError> {
    add_with_path(root, spec, std::env::var_os("PATH").as_deref())
}

/// Direct alias for [`add`].
pub fn add_dependency(root: &Path, spec: &str) -> Result<resolve::StagedFiles, DepsError> {
    add(root, spec)
}

/// Adds a dependency specification to the project at `root` with an explicit PATH.
pub(crate) fn add_with_path(
    root: &Path,
    spec: &str,
    path_env: Option<&OsStr>,
) -> Result<resolve::StagedFiles, DepsError> {
    let ctx = resolve::InvocationContext::from_env(path_env);
    add_with_context(root, spec, &ctx)
}

/// Adds a dependency specification to the project at `root` with an explicit
/// [`resolve::InvocationContext`] carrying the PATH override, child environment,
/// and optional resolution strategy.
pub(crate) fn add_with_context(
    root: &Path,
    spec: &str,
    ctx: &resolve::InvocationContext,
) -> Result<resolve::StagedFiles, DepsError> {
    add_with_context_and_hook(root, spec, ctx, |_| Ok(()))
}

/// Internal implementation of dependency addition with interruption hook for tests.
pub(crate) fn add_with_path_and_hook<H>(
    root: &Path,
    spec: &str,
    path_env: Option<&OsStr>,
    hook: H,
) -> Result<resolve::StagedFiles, DepsError>
where
    H: FnMut(CommitStep) -> Result<(), ToolFault>,
{
    let ctx = resolve::InvocationContext::from_env(path_env);
    add_with_context_and_hook(root, spec, &ctx, hook)
}

/// Internal implementation of dependency addition, taking an explicit
/// [`resolve::InvocationContext`] and interruption hook for tests.
pub(crate) fn add_with_context_and_hook<H>(
    root: &Path,
    spec: &str,
    ctx: &resolve::InvocationContext,
    hook: H,
) -> Result<resolve::StagedFiles, DepsError>
where
    H: FnMut(CommitStep) -> Result<(), ToolFault>,
{
    if !root.is_dir() {
        return Err(ToolFault::new(
            format!("Project root directory not found: {}", root.display()),
            Some(root.to_path_buf()),
        )
        .into());
    }

    // 1. Read current manifest (fails early if package.json is corrupt or unreadable).
    let _ = manifest::read_manifest(root)?;

    // 2. Resolve and stage addition via Deno, consulting the context's resolution
    // strategy (if any) before any real invocation, per B-05.
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(Diagnostic::new(
            codes::TOOL_FAULT,
            Severity::Error,
            None,
            "Dependency specification cannot be empty",
            "Specify a valid package name or dependency specifier",
        )
        .into());
    }
    let staged =
        resolve::invoke_staged_deno_with_context(root, &["add", "--package-json", trimmed], ctx)?;

    // 3. Atomically commit via fixed rename ordering.
    let outcome = CommitOutcome::from(&staged);
    commit_dependencies_with_hook(root, &outcome, hook)?;

    Ok(staged)
}

fn is_staged_dependencies_empty(package_json: &str) -> Result<bool, ToolFault> {
    let value: serde_json::Value = serde_json::from_str(package_json).map_err(|err| {
        ToolFault::new(
            format!("Failed to parse staged package.json: {err}"),
            Option::<PathBuf>::None,
        )
    })?;

    let root_obj = value.as_object().ok_or_else(|| {
        ToolFault::new(
            "Invalid staged package.json: root must be an object",
            Option::<PathBuf>::None,
        )
    })?;

    let is_empty = root_obj
        .get("dependencies")
        .map(|d| d.as_object().map(|obj| obj.is_empty()).unwrap_or(false))
        .unwrap_or(true);

    Ok(is_empty)
}

/// Removes a dependency from the project at `root`.
///
/// 1. Reads the current manifest (`package.json`) via [`manifest::read_manifest`].
///    If `<name>` is absent from `package.json`, returns a [`Diagnostic`] carrying
///    [`codes::CLI_USAGE`] without invoking Deno.
/// 2. Invokes [`resolve::resolve_remove`] in a staged, confined environment.
/// 3. Inspects the staged result: if the resulting `dependencies` map is empty,
///    commits as the empty-state delete-both case ([`CommitOutcome::Empty`]);
///    otherwise commits the updated `deno.lock` and `package.json` ([`CommitOutcome::NonEmpty`]).
///
/// If any fault occurs prior to commit, real project files are never touched.
pub fn remove(root: &Path, name: &str) -> Result<CommitOutcome, DepsError> {
    remove_with_path(root, name, std::env::var_os("PATH").as_deref())
}

/// Direct alias for [`remove`].
pub fn remove_dependency(root: &Path, name: &str) -> Result<CommitOutcome, DepsError> {
    remove(root, name)
}

/// Removes a dependency from the project at `root` with an explicit PATH.
pub(crate) fn remove_with_path(
    root: &Path,
    name: &str,
    path_env: Option<&OsStr>,
) -> Result<CommitOutcome, DepsError> {
    let ctx = resolve::InvocationContext::from_env(path_env);
    remove_with_context(root, name, &ctx)
}

/// Removes a dependency from the project at `root` with an explicit
/// [`resolve::InvocationContext`] carrying the PATH override, child environment,
/// and optional resolution strategy.
pub(crate) fn remove_with_context(
    root: &Path,
    name: &str,
    ctx: &resolve::InvocationContext,
) -> Result<CommitOutcome, DepsError> {
    remove_with_context_and_hook(root, name, ctx, |_| Ok(()))
}

/// Internal implementation of dependency removal with interruption hook for tests.
pub(crate) fn remove_with_path_and_hook<H>(
    root: &Path,
    name: &str,
    path_env: Option<&OsStr>,
    hook: H,
) -> Result<CommitOutcome, DepsError>
where
    H: FnMut(CommitStep) -> Result<(), ToolFault>,
{
    let ctx = resolve::InvocationContext::from_env(path_env);
    remove_with_context_and_hook(root, name, &ctx, hook)
}

/// Internal implementation of dependency removal, taking an explicit
/// [`resolve::InvocationContext`] and interruption hook for tests.
pub(crate) fn remove_with_context_and_hook<H>(
    root: &Path,
    name: &str,
    ctx: &resolve::InvocationContext,
    hook: H,
) -> Result<CommitOutcome, DepsError>
where
    H: FnMut(CommitStep) -> Result<(), ToolFault>,
{
    if !root.is_dir() {
        return Err(ToolFault::new(
            format!("Project root directory not found: {}", root.display()),
            Some(root.to_path_buf()),
        )
        .into());
    }

    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(Diagnostic::new(
            codes::CLI_USAGE,
            Severity::Error,
            None,
            "Dependency name cannot be empty",
            "Run 'wda deps' to inspect the current dependency graph",
        )
        .into());
    }

    // 1. Read current manifest (fails early if package.json is corrupt or unreadable).
    let manifest = manifest::read_manifest(root)?;

    // If <name> is absent, return CLI_USAGE without invoking Deno.
    if !manifest.contains_key(trimmed) {
        return Err(Diagnostic::new(
            codes::CLI_USAGE,
            Severity::Error,
            None,
            format!("Dependency '{trimmed}' is not declared in package.json"),
            "Run 'wda deps' to inspect the current dependency graph",
        )
        .into());
    }

    // 2. Resolve and stage removal via Deno, consulting the context's resolution
    // strategy (if any) before any real invocation, per B-05.
    let staged =
        resolve::invoke_staged_deno_with_context(root, &["remove", "--package-json", trimmed], ctx)?;

    // 3. Inspect staged result for empty-state detection.
    let is_empty = is_staged_dependencies_empty(&staged.package_json)?;
    let outcome = if is_empty {
        CommitOutcome::Empty
    } else {
        CommitOutcome::from(&staged)
    };

    // 4. Atomically commit via fixed ordering.
    commit_dependencies_with_hook(root, &outcome, hook)?;

    Ok(outcome)
}

/// Updates dependencies in the project at `root`.
///
/// 1. Reads the current manifest (`package.json`) via [`manifest::read_manifest`].
///    If a `<name>` filter is given and absent from `package.json`, returns a [`Diagnostic`]
///    carrying [`codes::CLI_USAGE`] (exit `2`) without invoking Deno.
/// 2. If the project has zero dependencies (and no filter is given), performs a no-op
///    without invoking Deno or writing anything, returning `Ok(None)`.
/// 3. Otherwise invokes [`resolve::resolve_update`] in a staged, confined environment.
/// 4. Atomically commits the staged `deno.lock` and `package.json` via [`commit_dependencies`].
///
/// If any fault occurs prior to commit, real project files are never touched.
pub fn update(
    root: &Path,
    name_filter: Option<&str>,
) -> Result<Option<resolve::StagedFiles>, DepsError> {
    update_with_path(root, name_filter, std::env::var_os("PATH").as_deref())
}

/// Direct alias for [`update`].
pub fn update_dependency(
    root: &Path,
    name_filter: Option<&str>,
) -> Result<Option<resolve::StagedFiles>, DepsError> {
    update(root, name_filter)
}

/// Direct alias for [`update`].
pub fn update_dependencies(
    root: &Path,
    name_filter: Option<&str>,
) -> Result<Option<resolve::StagedFiles>, DepsError> {
    update(root, name_filter)
}

/// Updates dependencies in the project at `root` with an explicit PATH.
pub(crate) fn update_with_path(
    root: &Path,
    name_filter: Option<&str>,
    path_env: Option<&OsStr>,
) -> Result<Option<resolve::StagedFiles>, DepsError> {
    let ctx = resolve::InvocationContext::from_env(path_env);
    update_with_context(root, name_filter, &ctx)
}

/// Updates dependencies in the project at `root` with an explicit
/// [`resolve::InvocationContext`] carrying the PATH override, child environment,
/// and optional resolution strategy.
pub(crate) fn update_with_context(
    root: &Path,
    name_filter: Option<&str>,
    ctx: &resolve::InvocationContext,
) -> Result<Option<resolve::StagedFiles>, DepsError> {
    update_with_context_and_hook(root, name_filter, ctx, |_| Ok(()))
}

/// Internal implementation of dependency update with interruption hook for tests.
pub(crate) fn update_with_path_and_hook<H>(
    root: &Path,
    name_filter: Option<&str>,
    path_env: Option<&OsStr>,
    hook: H,
) -> Result<Option<resolve::StagedFiles>, DepsError>
where
    H: FnMut(CommitStep) -> Result<(), ToolFault>,
{
    let ctx = resolve::InvocationContext::from_env(path_env);
    update_with_context_and_hook(root, name_filter, &ctx, hook)
}

/// Internal implementation of dependency update, taking an explicit
/// [`resolve::InvocationContext`] and interruption hook for tests.
pub(crate) fn update_with_context_and_hook<H>(
    root: &Path,
    name_filter: Option<&str>,
    ctx: &resolve::InvocationContext,
    hook: H,
) -> Result<Option<resolve::StagedFiles>, DepsError>
where
    H: FnMut(CommitStep) -> Result<(), ToolFault>,
{
    if !root.is_dir() {
        return Err(ToolFault::new(
            format!("Project root directory not found: {}", root.display()),
            Some(root.to_path_buf()),
        )
        .into());
    }

    // 1. Read current manifest (fails early if package.json is corrupt or unreadable).
    let manifest = manifest::read_manifest(root)?;

    // 2. Validate name filter if provided.
    let resolved_filter = match name_filter {
        Some(filter) => {
            let trimmed = filter.trim();
            if trimmed.is_empty() {
                return Err(Diagnostic::new(
                    codes::CLI_USAGE,
                    Severity::Error,
                    None,
                    "Dependency name cannot be empty",
                    "Run 'wda deps' to inspect the current dependency graph",
                )
                .into());
            }

            // If <name> filter is absent from package.json, return CLI_USAGE without invoking Deno.
            if !manifest.contains_key(trimmed) {
                return Err(Diagnostic::new(
                    codes::CLI_USAGE,
                    Severity::Error,
                    None,
                    format!("Dependency '{trimmed}' is not declared in package.json"),
                    "Run 'wda deps' to inspect the current dependency graph",
                )
                .into());
            }

            Some(trimmed)
        }
        None => {
            // With zero dependencies, no-op without invoking Deno or writing anything.
            if manifest.is_empty() {
                return Ok(None);
            }
            None
        }
    };

    // 3. Resolve and stage update via Deno, consulting the context's resolution
    // strategy (if any) before any real invocation, per B-05.
    let staged = match resolved_filter {
        None => resolve::invoke_staged_deno_with_context(root, &["update"], ctx)?,
        Some(filter) => resolve::invoke_staged_deno_with_context(root, &["update", filter], ctx)?,
    };

    // 4. Atomically commit via fixed ordering.
    let outcome = CommitOutcome::from(&staged);
    commit_dependencies_with_hook(root, &outcome, hook)?;

    Ok(Some(staged))
}

/// Internal commit implementation with injection hook for test interruption simulation.
pub(crate) fn commit_dependencies_with_hook<H>(
    root: &Path,
    outcome: &CommitOutcome,
    mut hook: H,
) -> Result<(), ToolFault>
where
    H: FnMut(CommitStep) -> Result<(), ToolFault>,
{
    if !root.is_dir() {
        return Err(ToolFault::new(
            format!("Project root directory not found: {}", root.display()),
            Some(root.to_path_buf()),
        ));
    }

    match outcome {
        CommitOutcome::NonEmpty {
            package_json,
            deno_lock,
        } => {
            let lock_content = match deno_lock {
                Some(content) => content,
                None => {
                    return Err(ToolFault::new(
                        "Missing deno.lock content for non-empty dependency commit",
                        Some(root.join("deno.lock")),
                    ));
                }
            };

            let pid = std::process::id();
            let suffix = resolve::unique_staging_suffix();
            let temp_lock_path = root.join(format!(".deno.lock.tmp-{pid}-{suffix}"));
            let temp_manifest_path = root.join(format!(".package.json.tmp-{pid}-{suffix}"));

            let mut temp_lock_guard = TempFileGuard::new(temp_lock_path.clone());
            if let Err(err) = fs::write(&temp_lock_path, lock_content.as_bytes()) {
                return Err(ToolFault::new(
                    format!(
                        "Failed to write temporary lock file at '{}': {err}",
                        temp_lock_path.display()
                    ),
                    Some(temp_lock_path),
                ));
            }

            let mut temp_manifest_guard = TempFileGuard::new(temp_manifest_path.clone());
            if let Err(err) = fs::write(&temp_manifest_path, package_json.as_bytes()) {
                return Err(ToolFault::new(
                    format!(
                        "Failed to write temporary manifest file at '{}': {err}",
                        temp_manifest_path.display()
                    ),
                    Some(temp_manifest_path),
                ));
            }

            // Fixed order: deno.lock first
            let target_lock_path = root.join("deno.lock");
            if let Err(err) = fs::rename(&temp_lock_path, &target_lock_path) {
                return Err(ToolFault::new(
                    format!(
                        "Failed to rename temporary lock file to '{}': {err}",
                        target_lock_path.display()
                    ),
                    Some(target_lock_path),
                ));
            }
            temp_lock_guard.defuse();

            // Interruption point between the two renames
            hook(CommitStep::AfterLockRenamed)?;

            // Fixed order: package.json second (canonical declaration changes last)
            let target_manifest_path = root.join("package.json");
            if let Err(err) = fs::rename(&temp_manifest_path, &target_manifest_path) {
                return Err(ToolFault::new(
                    format!(
                        "Failed to rename temporary manifest file to '{}': {err}",
                        target_manifest_path.display()
                    ),
                    Some(target_manifest_path),
                ));
            }
            temp_manifest_guard.defuse();

            Ok(())
        }
        CommitOutcome::Empty => {
            let target_lock_path = root.join("deno.lock");
            let target_manifest_path = root.join("package.json");

            // Fixed order: deno.lock first
            if target_lock_path.is_file() || target_lock_path.exists() {
                if let Err(err) = fs::remove_file(&target_lock_path) {
                    if err.kind() != std::io::ErrorKind::NotFound {
                        return Err(ToolFault::new(
                            format!(
                                "Failed to remove lock file '{}' during empty-state commit: {err}",
                                target_lock_path.display()
                            ),
                            Some(target_lock_path),
                        ));
                    }
                }
            }

            // Interruption point between the two deletions
            hook(CommitStep::AfterLockDeleted)?;

            // Fixed order: package.json second (canonical declaration deleted last)
            if target_manifest_path.is_file() || target_manifest_path.exists() {
                if let Err(err) = fs::remove_file(&target_manifest_path) {
                    if err.kind() != std::io::ErrorKind::NotFound {
                        return Err(ToolFault::new(
                            format!(
                                "Failed to remove manifest file '{}' during empty-state commit: {err}",
                                target_manifest_path.display()
                            ),
                            Some(target_manifest_path),
                        ));
                    }
                }
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deps_submodules_declared() {
        let _ = manifest::read_manifest;
        let _ = resolve::resolve_deno;
    }

    #[test]
    fn test_commit_non_empty_success_renames_both_files_in_fixed_order() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_commit_non_empty_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"is-number\": \"^7.0.0\"\n  }\n}\n";
        let initial_lock = "{\n  \"version\": \"3\",\n  \"packages\": {}\n}\n";
        fs::write(base_tmp.join("package.json"), initial_manifest).unwrap();
        fs::write(base_tmp.join("deno.lock"), initial_lock).unwrap();

        let new_manifest = "{\n  \"dependencies\": {\n    \"chalk\": \"^5.3.0\",\n    \"is-number\": \"^7.0.0\"\n  }\n}\n";
        let new_lock = "{\n  \"version\": \"3\",\n  \"packages\": {\"specifiers\": {\"chalk\": \"5.3.0\"}}\n}\n";

        let outcome = CommitOutcome::non_empty(new_manifest, new_lock);
        let commit_res = commit_dependencies(&base_tmp, &outcome);
        assert!(commit_res.is_ok(), "commit_dependencies must succeed");

        assert_eq!(
            fs::read_to_string(base_tmp.join("package.json")).unwrap(),
            new_manifest
        );
        assert_eq!(
            fs::read_to_string(base_tmp.join("deno.lock")).unwrap(),
            new_lock
        );

        // Verify no leftover temporary files in project root
        let entries = fs::read_dir(&base_tmp).unwrap();
        for entry in entries {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().into_owned();
            assert!(
                name == "package.json" || name == "deno.lock",
                "unexpected file left in project root: {name}"
            );
        }

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_commit_non_empty_interrupted_between_renames_leaves_named_state() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_commit_non_empty_interrupted_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"initial\": \"1.0.0\"\n  }\n}\n";
        let initial_lock = "{\n  \"version\": \"3\",\n  \"packages\": {\"specifiers\": {\"initial\": \"1.0.0\"}}\n}\n";
        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");

        fs::write(&manifest_path, initial_manifest).unwrap();
        fs::write(&lock_path, initial_lock).unwrap();

        let manifest_bytes_before = fs::read(&manifest_path).unwrap();

        let staged_manifest = "{\n  \"dependencies\": {\n    \"initial\": \"1.0.0\",\n    \"added\": \"2.0.0\"\n  }\n}\n";
        let staged_lock = "{\n  \"version\": \"3\",\n  \"packages\": {\"specifiers\": {\"initial\": \"1.0.0\", \"added\": \"2.0.0\"}}\n}\n";
        let outcome = CommitOutcome::non_empty(staged_manifest, staged_lock);

        // Inject interruption immediately after lock is renamed, before package.json is renamed
        let res = commit_dependencies_with_hook(&base_tmp, &outcome, |step| {
            if step == CommitStep::AfterLockRenamed {
                Err(ToolFault::new(
                    "Simulated interruption between renames",
                    None::<PathBuf>,
                ))
            } else {
                Ok(())
            }
        });

        assert!(res.is_err(), "interrupted commit must return Err");

        // Named state: lock updated, package.json byte-identical to pre-operation
        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "package.json must remain byte-identical to pre-operation"
        );

        let lock_content_after = fs::read_to_string(&lock_path).unwrap();
        assert_eq!(
            lock_content_after, staged_lock,
            "deno.lock must have been updated to staged lock"
        );

        // Verify temp manifest file was cleaned up on error
        let entries = fs::read_dir(&base_tmp).unwrap();
        for entry in entries {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().into_owned();
            assert!(
                name == "package.json" || name == "deno.lock",
                "temporary file was not cleaned up: {name}"
            );
        }

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_commit_empty_state_success_deletes_both_files() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_commit_empty_success_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");

        fs::write(&manifest_path, "{\n  \"dependencies\": {\"chalk\": \"5.0.0\"}\n}\n").unwrap();
        fs::write(&lock_path, "{\n  \"version\": \"3\"\n}\n").unwrap();

        assert!(manifest_path.exists());
        assert!(lock_path.exists());

        let res = commit_dependencies(&base_tmp, &CommitOutcome::Empty);
        assert!(res.is_ok(), "empty-state commit must succeed");

        assert!(
            !lock_path.exists(),
            "deno.lock must be deleted in empty state"
        );
        assert!(
            !manifest_path.exists(),
            "package.json must be deleted in empty state"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_commit_empty_state_interrupted_between_deletions_leaves_named_state() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_commit_empty_interrupted_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\"chalk\": \"5.0.0\"}\n}\n";
        let initial_lock = "{\n  \"version\": \"3\"\n}\n";
        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");

        fs::write(&manifest_path, initial_manifest).unwrap();
        fs::write(&lock_path, initial_lock).unwrap();

        let manifest_bytes_before = fs::read(&manifest_path).unwrap();

        // Inject interruption immediately after lock is deleted, before package.json is deleted
        let res = commit_dependencies_with_hook(&base_tmp, &CommitOutcome::Empty, |step| {
            if step == CommitStep::AfterLockDeleted {
                Err(ToolFault::new(
                    "Simulated interruption between deletions",
                    None::<PathBuf>,
                ))
            } else {
                Ok(())
            }
        });

        assert!(res.is_err(), "interrupted empty commit must return Err");

        // Named state: lock deleted, package.json still present
        assert!(
            !lock_path.exists(),
            "deno.lock must have been deleted"
        );
        assert!(
            manifest_path.exists(),
            "package.json must still be present"
        );
        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "package.json must remain byte-identical to pre-operation"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_pre_commit_fault_leaves_both_files_byte_identical() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_pre_commit_fault_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\"is-number\": \"^7.0.0\"}\n}\n";
        let initial_lock = "{\n  \"version\": \"3\"\n}\n";
        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");

        fs::write(&manifest_path, initial_manifest).unwrap();
        fs::write(&lock_path, initial_lock).unwrap();

        let manifest_bytes_before = fs::read(&manifest_path).unwrap();
        let lock_bytes_before = fs::read(&lock_path).unwrap();

        // Simulate a staged operation failing before commit (e.g. resolver failure or invalid spec)
        let simulated_fault: Result<resolve::StagedFiles, ToolFault> = Err(ToolFault::new(
            "Simulated staging error before commit",
            None::<PathBuf>,
        ));

        // When a fault surfaces before commit, commit is never called
        assert!(simulated_fault.is_err());

        // Real project files must remain completely untouched and byte-identical
        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        let lock_bytes_after = fs::read(&lock_path).unwrap();

        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "manifest must be byte-identical to pre-operation"
        );
        assert_eq!(
            lock_bytes_before, lock_bytes_after,
            "lock must be byte-identical to pre-operation"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_commit_non_empty_missing_lock_returns_tool_fault() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_commit_missing_lock_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let outcome = CommitOutcome::NonEmpty {
            package_json: "{}".to_string(),
            deno_lock: None,
        };

        let res = commit_dependencies(&base_tmp, &outcome);
        assert!(res.is_err(), "missing lock in non-empty state must return Err");
        let fault = res.unwrap_err();
        assert!(fault.message.contains("Missing deno.lock"));

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_commit_invalid_root_directory_returns_tool_fault() {
        let non_existent = std::env::temp_dir().join(format!(
            "wda_test_commit_non_existent_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&non_existent);

        let outcome = CommitOutcome::Empty;
        let res = commit_dependencies(&non_existent, &outcome);
        assert!(res.is_err());
        let fault = res.unwrap_err();
        assert!(fault.message.contains("Project root directory not found"));
    }

    #[test]
    fn test_commit_aliases_match() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_commit_alias_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let staged = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {}\n}\n".to_string(),
            deno_lock: Some("{\n  \"version\": \"3\"\n}\n".to_string()),
        };

        let res = commit_staged(&base_tmp, Some(&staged));
        assert!(res.is_ok());
        assert!(base_tmp.join("package.json").is_file());
        assert!(base_tmp.join("deno.lock").is_file());

        let res_empty = commit_staged(&base_tmp, None);
        assert!(res_empty.is_ok());
        assert!(!base_tmp.join("package.json").exists());
        assert!(!base_tmp.join("deno.lock").exists());

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_add_zero_dependency_creates_manifest_and_lock() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_add_zero_dep_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // Injected strategy stands in for the real staged `deno add`, matching what a
        // real resolution of "is-number@7.0.0" against an empty project would produce.
        let fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(fixture.clone())
        });

        let res = add_with_context(&base_tmp, "is-number@7.0.0", &ctx);
        assert!(res.is_ok(), "add on zero-dependency project must succeed: {:?}", res.err());

        let staged = res.unwrap();
        assert!(staged.package_json.contains("\"is-number\""));

        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");

        assert!(manifest_path.is_file(), "package.json must be created");
        assert!(lock_path.is_file(), "deno.lock must be created");

        let deps_map = manifest::read_manifest(&base_tmp).unwrap();
        assert_eq!(deps_map.len(), 1, "manifest must contain exactly one dependency");
        assert!(deps_map.contains_key("is-number"));

        let lock_content = fs::read_to_string(&lock_path).unwrap();
        assert!(lock_content.contains("is-number"));

        // Verify no extra files left in project root
        let entries = fs::read_dir(&base_tmp).unwrap();
        for entry in entries {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().into_owned();
            assert!(
                name == "package.json" || name == "deno.lock",
                "unexpected file left in project root: {name}"
            );
        }

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_add_twice_produces_byte_identical_lock() {
        let base_a = std::env::temp_dir().join(format!(
            "wda_test_add_repro_a_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let base_b = std::env::temp_dir().join(format!(
            "wda_test_add_repro_b_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_a);
        let _ = fs::remove_dir_all(&base_b);
        fs::create_dir_all(&base_a).unwrap();
        fs::create_dir_all(&base_b).unwrap();

        // Injected strategy returns the same fixed StagedFiles on every call, which is
        // what a real, reproducible resolution of an identical spec against an identical
        // starting state would also produce.
        let fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let for_a = fixture.clone();
        let ctx_a = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(for_a.clone())
        });
        let ctx_b = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(fixture.clone())
        });

        let res_a = add_with_context(&base_a, "is-number@7.0.0", &ctx_a);
        assert!(res_a.is_ok(), "first add must succeed");

        let res_b = add_with_context(&base_b, "is-number@7.0.0", &ctx_b);
        assert!(res_b.is_ok(), "second add must succeed");

        let lock_a = fs::read(&base_a.join("deno.lock")).unwrap();
        let lock_b = fs::read(&base_b.join("deno.lock")).unwrap();
        assert_eq!(lock_a, lock_b, "repeat add must produce byte-identical deno.lock");

        let manifest_a = fs::read(&base_a.join("package.json")).unwrap();
        let manifest_b = fs::read(&base_b.join("package.json")).unwrap();
        assert_eq!(manifest_a, manifest_b, "repeat add must produce byte-identical package.json");

        let _ = fs::remove_dir_all(&base_a);
        let _ = fs::remove_dir_all(&base_b);
    }

    #[test]
    fn test_add_preserves_unrelated_project_tree_files() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_add_preserve_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let src_dir = base_tmp.join("src");
        let docs_dir = base_tmp.join("docs");
        let dist_dir = base_tmp.join("dist");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&docs_dir).unwrap();
        fs::create_dir_all(&dist_dir).unwrap();

        let wda_json_path = base_tmp.join("wda.json");
        let src_file_path = src_dir.join("main.ts");
        let doc_file_path = docs_dir.join("architecture.md");
        let dist_file_path = dist_dir.join("output.html");

        let wda_json_content = "{\n  \"wdaVersion\": \"0.1.0\"\n}\n";
        let src_content = "console.log('web-design-anchor');\n";
        let doc_content = "# Documentation\nArchitecture details\n";
        let dist_content = "<html><body>Built site</body></html>\n";

        fs::write(&wda_json_path, wda_json_content).unwrap();
        fs::write(&src_file_path, src_content).unwrap();
        fs::write(&doc_file_path, doc_content).unwrap();
        fs::write(&dist_file_path, dist_content).unwrap();

        // Injected strategy stands in for the real staged `deno add`; this test cares
        // only about the surrounding project-tree files, not the staged content shape.
        let fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(fixture.clone())
        });

        let res = add_with_context(&base_tmp, "is-number@7.0.0", &ctx);
        assert!(res.is_ok(), "add must succeed");

        assert_eq!(
            fs::read_to_string(&wda_json_path).unwrap(),
            wda_json_content,
            "wda.json must remain byte-identical"
        );
        assert_eq!(
            fs::read_to_string(&src_file_path).unwrap(),
            src_content,
            "src/ file must remain byte-identical"
        );
        assert_eq!(
            fs::read_to_string(&doc_file_path).unwrap(),
            doc_content,
            "docs/ file must remain byte-identical"
        );
        assert_eq!(
            fs::read_to_string(&dist_file_path).unwrap(),
            dist_content,
            "dist/ file must remain byte-identical"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_add_with_registry_auth_env_does_not_leak_secrets() {
        // Per D-02, this test constructs an `InvocationContext` whose environment
        // map never carries `NPM_TOKEN`/`DENO_AUTH_TOKENS` and asserts the child
        // environment `resolve::build_deno_command` produces omits them too. No
        // spawn, and no mutation of the real process environment.
        let env_vars = vec![(
            std::ffi::OsString::from("PATH"),
            std::ffi::OsString::from("/usr/bin"),
        )];
        let ctx = resolve::InvocationContext::with_env(None, env_vars);

        let deno_binary = PathBuf::from("deno");
        let staging_dir = std::env::temp_dir();
        let command = resolve::build_deno_command(
            &deno_binary,
            &staging_dir,
            &["add", "--package-json", "is-number@7.0.0"],
            &ctx,
        );

        let envs: Vec<_> = command.get_envs().collect();
        assert!(
            !envs.iter().any(|(key, _)| *key == OsStr::new("NPM_TOKEN")),
            "child environment must omit NPM_TOKEN when the context's map omits it"
        );
        assert!(
            !envs
                .iter()
                .any(|(key, _)| *key == OsStr::new("DENO_AUTH_TOKENS")),
            "child environment must omit DENO_AUTH_TOKENS when the context's map omits it"
        );
    }

    #[test]
    fn test_add_corrupted_manifest_faults_before_commit() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_add_corrupt_manifest_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let corrupt_json = "{\n  \"dependencies\": { invalid json\n";
        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");
        fs::write(&manifest_path, corrupt_json).unwrap();

        let manifest_bytes_before = fs::read(&manifest_path).unwrap();

        let res = add(&base_tmp, "is-number@7.0.0");
        assert!(res.is_err(), "corrupt manifest must cause add to fail");

        match res.unwrap_err() {
            DepsError::ToolFault(fault) => {
                assert!(fault.message.contains("Failed to parse package.json"));
            }
            other => panic!("expected ToolFault, got: {other:?}"),
        }

        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "corrupt package.json must remain byte-identical"
        );
        assert!(
            !lock_path.exists(),
            "deno.lock must not be created when manifest read faults"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_add_invalid_package_spec_leaves_project_byte_identical() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_add_invalid_spec_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n";
        let initial_lock = "{\n  \"version\": \"3\",\n  \"packages\": {}\n}\n";
        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");

        fs::write(&manifest_path, initial_manifest).unwrap();
        fs::write(&lock_path, initial_lock).unwrap();

        let manifest_bytes_before = fs::read(&manifest_path).unwrap();
        let lock_bytes_before = fs::read(&lock_path).unwrap();

        // Injected strategy stands in for the real staged `deno add` failing to resolve
        // an unpublished package, returning the same TOOL_FAULT shape a real resolver
        // failure maps to, without ever consulting PATH or spawning `deno`.
        let ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), |_args| {
            Err(Diagnostic::new(
                crate::codes::TOOL_FAULT,
                Severity::Error,
                None,
                "Deno command failed: could not resolve package",
                "Check the dependency specifier and network connectivity, then retry",
            ))
        });

        let res = add_with_context(
            &base_tmp,
            "this-pkg-definitely-does-not-exist-xyz-987654321",
            &ctx,
        );
        assert!(res.is_err(), "invalid package spec must fail");

        match res.unwrap_err() {
            DepsError::Diagnostic(diag) => {
                assert_eq!(diag.code, crate::codes::TOOL_FAULT);
            }
            other => panic!("expected Diagnostic(TOOL_FAULT), got: {other:?}"),
        }

        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        let lock_bytes_after = fs::read(&lock_path).unwrap();
        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "package.json must remain byte-identical on resolve fault"
        );
        assert_eq!(
            lock_bytes_before, lock_bytes_after,
            "deno.lock must remain byte-identical on resolve fault"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_add_missing_deno_leaves_project_byte_identical() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_add_missing_deno_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n";
        let manifest_path = base_tmp.join("package.json");
        fs::write(&manifest_path, initial_manifest).unwrap();

        let manifest_bytes_before = fs::read(&manifest_path).unwrap();

        let res = add_with_path(&base_tmp, "chalk@5.0.0", Some(std::ffi::OsStr::new("")));
        assert!(res.is_err(), "missing deno on PATH must fail");

        match res.unwrap_err() {
            DepsError::Diagnostic(diag) => {
                assert_eq!(diag.code, crate::codes::TOOL_FAULT);
                assert!(diag.message.contains("deno"));
            }
            other => panic!("expected Diagnostic(TOOL_FAULT), got: {other:?}"),
        }

        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "package.json must remain byte-identical when deno resolution fails"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_add_non_existent_root_returns_tool_fault() {
        let non_existent = std::env::temp_dir().join(format!(
            "wda_test_add_non_existent_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&non_existent);

        let res = add(&non_existent, "is-number@7.0.0");
        assert!(res.is_err(), "non-existent root directory must fail");

        match res.unwrap_err() {
            DepsError::ToolFault(fault) => {
                assert!(fault.message.contains("Project root directory not found"));
            }
            other => panic!("expected ToolFault, got: {other:?}"),
        }
    }

    #[test]
    fn test_add_interrupted_between_renames_leaves_named_state() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_add_interrupted_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n";
        let initial_lock = "{\n  \"version\": \"3\",\n  \"packages\": {}\n}\n";
        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");

        fs::write(&manifest_path, initial_manifest).unwrap();
        fs::write(&lock_path, initial_lock).unwrap();

        let manifest_bytes_before = fs::read(&manifest_path).unwrap();

        // Injected strategy stands in for the real staged `deno add`, matching what a
        // real resolution of "is-odd@3.0.1" against `initial_manifest` would produce.
        let fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\",\n    \"is-odd\": \"3.0.1\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\",\n    \"npm:is-odd@3.0.1\": \"3.0.1\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(fixture.clone())
        });

        let res = add_with_context_and_hook(&base_tmp, "is-odd@3.0.1", &ctx, |step| {
            if step == CommitStep::AfterLockRenamed {
                Err(ToolFault::new(
                    "Simulated interruption between renames",
                    None::<PathBuf>,
                ))
            } else {
                Ok(())
            }
        });

        assert!(res.is_err(), "interrupted commit must return Err");

        // Named state: lock updated with new dependency, package.json byte-identical to pre-operation
        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "package.json must remain byte-identical to pre-operation when interrupted"
        );

        let lock_content_after = fs::read_to_string(&lock_path).unwrap();
        assert!(
            lock_content_after.contains("is-odd"),
            "deno.lock must have been updated to staged lock before interruption"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_add_aliases_match() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_add_alias_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // Injected strategy stands in for the real staged `deno add`; the alias under
        // test is `add_with_context`, the context-accepting counterpart of `add_dependency`.
        let fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(fixture.clone())
        });

        let res = add_with_context(&base_tmp, "is-number@7.0.0", &ctx);
        assert!(res.is_ok(), "add_dependency alias must succeed");

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_remove_last_dependency_deletes_both_files_and_restores_empty_state() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_remove_last_dep_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // Injected strategy stands in for the real staged `deno add`, matching what a
        // real resolution of "is-number@7.0.0" against an empty project would produce.
        let add_fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let add_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(add_fixture.clone())
        });
        let add_res = add_with_context(&base_tmp, "is-number@7.0.0", &add_ctx);
        assert!(add_res.is_ok(), "initial add must succeed");

        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");
        assert!(manifest_path.is_file());
        assert!(lock_path.is_file());

        // Injected strategy stands in for the real staged `deno remove` dropping the
        // project's last dependency, which is what a real removal of "is-number" from
        // this single-dependency project would produce.
        let remove_fixture = resolve::StagedFiles {
            package_json: "{}\n".to_string(),
            deno_lock: None,
        };
        let remove_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(remove_fixture.clone())
        });
        let remove_res = remove_with_context(&base_tmp, "is-number", &remove_ctx);
        assert!(remove_res.is_ok(), "remove must succeed: {:?}", remove_res.err());

        let outcome = remove_res.unwrap();
        assert_eq!(outcome, CommitOutcome::Empty);
        assert!(outcome.is_empty());
        assert!(outcome.staged_files().is_none());

        assert!(
            !manifest_path.exists(),
            "package.json must be deleted when removing last dependency"
        );
        assert!(
            !lock_path.exists(),
            "deno.lock must be deleted when removing last dependency"
        );

        let entries = fs::read_dir(&base_tmp).unwrap();
        let remaining_files: Vec<_> = entries.map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert!(
            remaining_files.is_empty(),
            "no files should remain in project root after removing last dependency, found: {remaining_files:?}"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_remove_unknown_name_returns_cli_usage_and_leaves_files_byte_identical() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_remove_unknown_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // Injected strategy stands in for the real staged `deno add`, matching what a
        // real resolution of "is-number@7.0.0" against an empty project would produce.
        let fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(fixture.clone())
        });
        let add_res = add_with_context(&base_tmp, "is-number@7.0.0", &ctx);
        assert!(add_res.is_ok(), "initial add must succeed");

        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");

        let manifest_bytes_before = fs::read(&manifest_path).unwrap();
        let lock_bytes_before = fs::read(&lock_path).unwrap();

        // "chalk" is absent from package.json, so `remove` returns CLI_USAGE before ever
        // consulting a resolution strategy; no context needed here.
        let remove_res = remove(&base_tmp, "chalk");
        assert!(remove_res.is_err(), "removing undeclared package must return Err");

        match remove_res.unwrap_err() {
            DepsError::Diagnostic(diag) => {
                assert_eq!(diag.code, codes::CLI_USAGE);
                assert_eq!(diag.severity, Severity::Error);
                assert!(diag.message.contains("chalk"));
                assert!(diag.message.contains("not declared in package.json"));
                assert_eq!(
                    diag.next_action,
                    "Run 'wda deps' to inspect the current dependency graph"
                );
            }
            other => panic!("expected Diagnostic(CLI_USAGE), got: {other:?}"),
        }

        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        let lock_bytes_after = fs::read(&lock_path).unwrap();

        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "package.json must remain byte-identical when remove fails for unknown name"
        );
        assert_eq!(
            lock_bytes_before, lock_bytes_after,
            "deno.lock must remain byte-identical when remove fails for unknown name"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_remove_zero_dependency_project_returns_cli_usage_without_creating_files() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_remove_zero_dep_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let remove_res = remove(&base_tmp, "is-number");
        assert!(remove_res.is_err(), "remove on zero-dep project must return Err");

        match remove_res.unwrap_err() {
            DepsError::Diagnostic(diag) => {
                assert_eq!(diag.code, codes::CLI_USAGE);
                assert_eq!(diag.severity, Severity::Error);
                assert!(diag.message.contains("is-number"));
                assert_eq!(
                    diag.next_action,
                    "Run 'wda deps' to inspect the current dependency graph"
                );
            }
            other => panic!("expected Diagnostic(CLI_USAGE), got: {other:?}"),
        }

        assert!(!base_tmp.join("package.json").exists());
        assert!(!base_tmp.join("deno.lock").exists());

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_remove_one_of_multiple_dependencies_keeps_non_empty_state() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_remove_multi_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // Injected strategy stands in for the real staged `deno add`, matching what real
        // resolutions of "is-number@7.0.0" then "is-odd@3.0.1" against this project would
        // each produce.
        let fixture1 = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let ctx1 = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(fixture1.clone())
        });
        let add1 = add_with_context(&base_tmp, "is-number@7.0.0", &ctx1);
        assert!(add1.is_ok());

        let fixture2 = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\",\n    \"is-odd\": \"3.0.1\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\",\n    \"npm:is-odd@3.0.1\": \"3.0.1\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let ctx2 = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(fixture2.clone())
        });
        let add2 = add_with_context(&base_tmp, "is-odd@3.0.1", &ctx2);
        assert!(add2.is_ok());

        let deps_before = manifest::read_manifest(&base_tmp).unwrap();
        assert_eq!(deps_before.len(), 2);
        assert!(deps_before.contains_key("is-number"));
        assert!(deps_before.contains_key("is-odd"));

        // Injected strategy stands in for the real staged `deno remove` dropping
        // "is-odd" and keeping "is-number", matching what a real removal against
        // `deps_before` would produce.
        let remove_fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let remove_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(remove_fixture.clone())
        });
        let remove_res = remove_with_context(&base_tmp, "is-odd", &remove_ctx);
        assert!(remove_res.is_ok(), "remove is-odd must succeed: {:?}", remove_res.err());

        let outcome = remove_res.unwrap();
        assert!(!outcome.is_empty());
        match &outcome {
            CommitOutcome::NonEmpty { package_json, deno_lock } => {
                assert!(!package_json.contains("\"is-odd\""));
                assert!(package_json.contains("\"is-number\""));
                assert!(deno_lock.is_some());
            }
            CommitOutcome::Empty => panic!("expected non-empty outcome"),
        }

        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");
        assert!(manifest_path.is_file());
        assert!(lock_path.is_file());

        let deps_after = manifest::read_manifest(&base_tmp).unwrap();
        assert_eq!(deps_after.len(), 1);
        assert!(deps_after.contains_key("is-number"));
        assert!(!deps_after.contains_key("is-odd"));

        let lock_content = fs::read_to_string(&lock_path).unwrap();
        assert!(lock_content.contains("is-number"));

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_remove_preserves_unrelated_project_tree_files() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_remove_preserve_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // Injected strategy stands in for the real staged `deno add`, matching what a
        // real resolution of "is-number@7.0.0" against an empty project would produce.
        let add_fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let add_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(add_fixture.clone())
        });
        let add_res = add_with_context(&base_tmp, "is-number@7.0.0", &add_ctx);
        assert!(add_res.is_ok());

        let src_dir = base_tmp.join("src");
        let docs_dir = base_tmp.join("docs");
        let dist_dir = base_tmp.join("dist");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&docs_dir).unwrap();
        fs::create_dir_all(&dist_dir).unwrap();

        let wda_json_path = base_tmp.join("wda.json");
        let src_file_path = src_dir.join("main.ts");
        let doc_file_path = docs_dir.join("architecture.md");
        let dist_file_path = dist_dir.join("output.html");

        let wda_json_content = "{\n  \"wdaVersion\": \"0.1.0\"\n}\n";
        let src_content = "console.log('web-design-anchor');\n";
        let doc_content = "# Documentation\nArchitecture details\n";
        let dist_content = "<html><body>Built site</body></html>\n";

        fs::write(&wda_json_path, wda_json_content).unwrap();
        fs::write(&src_file_path, src_content).unwrap();
        fs::write(&doc_file_path, doc_content).unwrap();
        fs::write(&dist_file_path, dist_content).unwrap();

        // Injected strategy stands in for the real staged `deno remove` dropping the
        // project's last dependency; this test cares only about the surrounding
        // project-tree files, not the staged content shape.
        let remove_fixture = resolve::StagedFiles {
            package_json: "{}\n".to_string(),
            deno_lock: None,
        };
        let remove_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(remove_fixture.clone())
        });
        let res = remove_with_context(&base_tmp, "is-number", &remove_ctx);
        assert!(res.is_ok(), "remove must succeed");

        assert_eq!(
            fs::read_to_string(&wda_json_path).unwrap(),
            wda_json_content,
            "wda.json must remain byte-identical after remove"
        );
        assert_eq!(
            fs::read_to_string(&src_file_path).unwrap(),
            src_content,
            "src/ file must remain byte-identical after remove"
        );
        assert_eq!(
            fs::read_to_string(&doc_file_path).unwrap(),
            doc_content,
            "docs/ file must remain byte-identical after remove"
        );
        assert_eq!(
            fs::read_to_string(&dist_file_path).unwrap(),
            dist_content,
            "dist/ file must remain byte-identical after remove"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_remove_corrupted_manifest_faults_before_commit() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_remove_corrupt_manifest_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let corrupt_json = "{\n  \"dependencies\": { invalid json\n";
        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");
        fs::write(&manifest_path, corrupt_json).unwrap();
        fs::write(&lock_path, "{\n  \"version\": \"3\"\n}\n").unwrap();

        let manifest_bytes_before = fs::read(&manifest_path).unwrap();
        let lock_bytes_before = fs::read(&lock_path).unwrap();

        let res = remove(&base_tmp, "is-number");
        assert!(res.is_err(), "corrupt manifest must cause remove to fail");

        match res.unwrap_err() {
            DepsError::ToolFault(fault) => {
                assert!(fault.message.contains("Failed to parse package.json"));
            }
            other => panic!("expected ToolFault, got: {other:?}"),
        }

        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        let lock_bytes_after = fs::read(&lock_path).unwrap();
        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "corrupt package.json must remain byte-identical"
        );
        assert_eq!(
            lock_bytes_before, lock_bytes_after,
            "deno.lock must remain byte-identical"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_remove_missing_deno_leaves_project_byte_identical() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_remove_missing_deno_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n";
        let initial_lock = "{\n  \"version\": \"3\",\n  \"packages\": {}\n}\n";
        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");
        fs::write(&manifest_path, initial_manifest).unwrap();
        fs::write(&lock_path, initial_lock).unwrap();

        let manifest_bytes_before = fs::read(&manifest_path).unwrap();
        let lock_bytes_before = fs::read(&lock_path).unwrap();

        let res = remove_with_path(&base_tmp, "is-number", Some(std::ffi::OsStr::new("")));
        assert!(res.is_err(), "missing deno on PATH must fail");

        match res.unwrap_err() {
            DepsError::Diagnostic(diag) => {
                assert_eq!(diag.code, crate::codes::TOOL_FAULT);
                assert!(diag.message.contains("deno"));
            }
            other => panic!("expected Diagnostic(TOOL_FAULT), got: {other:?}"),
        }

        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        let lock_bytes_after = fs::read(&lock_path).unwrap();
        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "package.json must remain byte-identical when deno resolution fails"
        );
        assert_eq!(
            lock_bytes_before, lock_bytes_after,
            "deno.lock must remain byte-identical when deno resolution fails"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_remove_empty_or_whitespace_name_returns_cli_usage() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_remove_empty_name_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n";
        let manifest_path = base_tmp.join("package.json");
        fs::write(&manifest_path, initial_manifest).unwrap();
        let manifest_bytes_before = fs::read(&manifest_path).unwrap();

        let res = remove(&base_tmp, "   ");
        assert!(res.is_err());

        match res.unwrap_err() {
            DepsError::Diagnostic(diag) => {
                assert_eq!(diag.code, codes::CLI_USAGE);
                assert_eq!(diag.severity, Severity::Error);
                assert!(diag.message.contains("empty"));
            }
            other => panic!("expected Diagnostic(CLI_USAGE), got: {other:?}"),
        }

        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        assert_eq!(manifest_bytes_before, manifest_bytes_after);

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_remove_interrupted_between_deletions_leaves_named_state() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_remove_interrupted_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // Injected strategy stands in for the real staged `deno add`, matching what a
        // real resolution of "is-number@7.0.0" against an empty project would produce.
        let add_fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let add_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(add_fixture.clone())
        });
        let add_res = add_with_context(&base_tmp, "is-number@7.0.0", &add_ctx);
        assert!(add_res.is_ok());

        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");
        let manifest_bytes_before = fs::read(&manifest_path).unwrap();

        // Injected strategy stands in for the real staged `deno remove` dropping the
        // project's last dependency, matching what a real removal of "is-number" from
        // this single-dependency project would produce.
        let remove_fixture = resolve::StagedFiles {
            package_json: "{}\n".to_string(),
            deno_lock: None,
        };
        let remove_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(remove_fixture.clone())
        });

        let res = remove_with_context_and_hook(&base_tmp, "is-number", &remove_ctx, |step| {
            if step == CommitStep::AfterLockDeleted {
                Err(ToolFault::new(
                    "Simulated interruption between deletions",
                    None::<PathBuf>,
                ))
            } else {
                Ok(())
            }
        });

        assert!(res.is_err(), "interrupted commit must return Err");

        assert!(
            !lock_path.exists(),
            "deno.lock must have been deleted before interruption"
        );
        assert!(
            manifest_path.exists(),
            "package.json must still exist when interrupted before deletion"
        );
        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "package.json must remain byte-identical to pre-operation when interrupted"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_remove_aliases_match() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_remove_alias_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // Injected strategy stands in for the real staged `deno add`, matching what a
        // real resolution of "is-number@7.0.0" against an empty project would produce.
        let add_fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let add_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(add_fixture.clone())
        });
        let add_res = add_with_context(&base_tmp, "is-number@7.0.0", &add_ctx);
        assert!(add_res.is_ok());

        // Injected strategy stands in for the real staged `deno remove`; the alias under
        // test is `remove_with_context`, the context-accepting counterpart of
        // `remove_dependency`.
        let remove_fixture = resolve::StagedFiles {
            package_json: "{}\n".to_string(),
            deno_lock: None,
        };
        let remove_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(remove_fixture.clone())
        });
        let res = remove_with_context(&base_tmp, "is-number", &remove_ctx);
        assert!(res.is_ok(), "remove_dependency alias must succeed");

        assert!(!base_tmp.join("package.json").exists());
        assert!(!base_tmp.join("deno.lock").exists());

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_remove_non_existent_root_returns_tool_fault() {
        let non_existent = std::env::temp_dir().join(format!(
            "wda_test_remove_non_existent_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&non_existent);

        let res = remove(&non_existent, "is-number");
        assert!(res.is_err(), "non-existent root directory must fail");

        match res.unwrap_err() {
            DepsError::ToolFault(fault) => {
                assert!(fault.message.contains("Project root directory not found"));
            }
            other => panic!("expected ToolFault, got: {other:?}"),
        }
    }

    #[test]
    fn test_update_zero_dependency_project_is_noop_without_invoking_deno_or_writing_anything() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_update_zero_dep_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // Provide an invalid PATH where deno does not exist; if deno were invoked, this would fail with TOOL_FAULT.
        let res = update_with_path(&base_tmp, None, Some(std::ffi::OsStr::new("invalid-nonexistent-path")));
        assert!(res.is_ok(), "update on zero-dependency project must succeed as a no-op: {:?}", res.err());
        assert_eq!(res.unwrap(), None, "zero-dependency update must return None (no staged files)");

        assert!(!base_tmp.join("package.json").exists(), "package.json must not be created");
        assert!(!base_tmp.join("deno.lock").exists(), "deno.lock must not be created");

        let entries: Vec<_> = fs::read_dir(&base_tmp).unwrap().collect();
        assert!(entries.is_empty(), "project directory must remain completely empty");

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_update_zero_dependency_with_empty_manifest_is_noop() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_update_zero_dep_empty_manifest_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {}\n}\n";
        let manifest_path = base_tmp.join("package.json");
        fs::write(&manifest_path, initial_manifest).unwrap();

        let res = update_with_path(&base_tmp, None, Some(std::ffi::OsStr::new("invalid-nonexistent-path")));
        assert!(res.is_ok(), "update on empty dependencies manifest must succeed as a no-op: {:?}", res.err());
        assert_eq!(res.unwrap(), None);

        assert_eq!(
            fs::read_to_string(&manifest_path).unwrap(),
            initial_manifest,
            "package.json must remain untouched"
        );
        assert!(!base_tmp.join("deno.lock").exists(), "deno.lock must not be created");

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_update_zero_dependency_with_unknown_name_filter_returns_cli_usage_without_creating_files() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_update_zero_dep_unknown_filter_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // With zero dependencies and a name filter, CLI_USAGE must be returned without invoking Deno.
        let res = update_with_path(
            &base_tmp,
            Some("chalk"),
            Some(std::ffi::OsStr::new("invalid-nonexistent-path")),
        );
        assert!(res.is_err(), "update with filter on zero-dep project must return Err");

        match res.unwrap_err() {
            DepsError::Diagnostic(diag) => {
                assert_eq!(diag.code, codes::CLI_USAGE);
                assert_eq!(diag.severity, Severity::Error);
                assert!(diag.message.contains("chalk"));
                assert!(diag.message.contains("not declared in package.json"));
                assert_eq!(
                    diag.next_action,
                    "Run 'wda deps' to inspect the current dependency graph"
                );
            }
            other => panic!("expected Diagnostic(CLI_USAGE), got: {other:?}"),
        }

        assert!(!base_tmp.join("package.json").exists(), "package.json must not be created");
        assert!(!base_tmp.join("deno.lock").exists(), "deno.lock must not be created");

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_update_unknown_name_filter_on_existing_project_returns_cli_usage_and_leaves_files_byte_identical() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_update_unknown_filter_existing_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // Injected strategy stands in for the real staged `deno add`, matching what a
        // real resolution of "is-number@7.0.0" against an empty project would produce.
        let fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let add_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(fixture.clone())
        });
        let add_res = add_with_context(&base_tmp, "is-number@7.0.0", &add_ctx);
        assert!(add_res.is_ok(), "initial add must succeed");

        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");

        let manifest_bytes_before = fs::read(&manifest_path).unwrap();
        let lock_bytes_before = fs::read(&lock_path).unwrap();

        // "chalk" is absent from package.json, so `update` returns CLI_USAGE before ever
        // consulting a resolution strategy; no context needed here.
        let update_res = update(&base_tmp, Some("chalk"));
        assert!(update_res.is_err(), "updating undeclared package must return Err");

        match update_res.unwrap_err() {
            DepsError::Diagnostic(diag) => {
                assert_eq!(diag.code, codes::CLI_USAGE);
                assert_eq!(diag.severity, Severity::Error);
                assert!(diag.message.contains("chalk"));
                assert!(diag.message.contains("not declared in package.json"));
                assert_eq!(
                    diag.next_action,
                    "Run 'wda deps' to inspect the current dependency graph"
                );
            }
            other => panic!("expected Diagnostic(CLI_USAGE), got: {other:?}"),
        }

        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        let lock_bytes_after = fs::read(&lock_path).unwrap();

        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "package.json must remain byte-identical when update fails for unknown name"
        );
        assert_eq!(
            lock_bytes_before, lock_bytes_after,
            "deno.lock must remain byte-identical when update fails for unknown name"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_update_unfiltered_updates_dependencies_and_commits() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_update_unfiltered_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // Injected strategy stands in for the real staged `deno add`, matching what a
        // real resolution of "is-number@7.0.0" against an empty project would produce.
        let add_fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let add_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(add_fixture.clone())
        });
        let add_res = add_with_context(&base_tmp, "is-number@7.0.0", &add_ctx);
        assert!(add_res.is_ok(), "initial add must succeed");

        // Injected strategy stands in for the real staged `deno update`, matching what a
        // real unfiltered update of this single-dependency project would produce.
        let update_fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let update_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(update_fixture.clone())
        });
        let update_res = update_with_context(&base_tmp, None, &update_ctx);
        assert!(update_res.is_ok(), "unfiltered update must succeed: {:?}", update_res.err());

        let staged_opt = update_res.unwrap();
        assert!(staged_opt.is_some(), "staged files must be returned for non-empty update");

        let staged = staged_opt.unwrap();
        assert!(staged.package_json.contains("is-number"));

        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");

        assert!(manifest_path.is_file(), "package.json must exist after update");
        assert!(lock_path.is_file(), "deno.lock must exist after update");

        let deps_map = manifest::read_manifest(&base_tmp).unwrap();
        assert_eq!(deps_map.len(), 1);
        assert!(deps_map.contains_key("is-number"));

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_update_filtered_name_updates_only_specified_dependency() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_update_filtered_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // Injected strategy stands in for the real staged `deno add`, matching what a
        // real resolution of "is-number@7.0.0" against an empty project would produce.
        let fixture1 = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let ctx1 = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(fixture1.clone())
        });
        let add1 = add_with_context(&base_tmp, "is-number@7.0.0", &ctx1);
        assert!(add1.is_ok());

        // Injected strategy stands in for the real staged `deno add`, matching what a
        // real resolution of "is-odd@3.0.0" against `deps_before` would produce.
        let fixture2 = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\",\n    \"is-odd\": \"3.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\",\n    \"npm:is-odd@3.0.0\": \"3.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let ctx2 = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(fixture2.clone())
        });
        let add2 = add_with_context(&base_tmp, "is-odd@3.0.0", &ctx2);
        assert!(add2.is_ok());

        let deps_before = manifest::read_manifest(&base_tmp).unwrap();
        assert_eq!(deps_before.len(), 2);
        assert_eq!(deps_before.get("is-number").unwrap(), "7.0.0");

        // Injected strategy stands in for the real staged `deno update` filtered to
        // "is-odd", matching what a real update of that dependency against
        // `deps_before` would produce: "is-odd" bumped, "is-number" left untouched.
        let update_fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\",\n    \"is-odd\": \"3.0.1\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\",\n    \"npm:is-odd@3.0.1\": \"3.0.1\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let update_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(update_fixture.clone())
        });
        let update_res = update_with_context(&base_tmp, Some("is-odd"), &update_ctx);
        assert!(update_res.is_ok(), "filtered update must succeed: {:?}", update_res.err());

        let staged = update_res.unwrap().unwrap();
        assert!(staged.package_json.contains("is-odd"));
        assert!(staged.package_json.contains("is-number"));

        let deps_after = manifest::read_manifest(&base_tmp).unwrap();
        assert_eq!(deps_after.len(), 2);
        assert_eq!(
            deps_after.get("is-number").unwrap(),
            "7.0.0",
            "unrelated pinned dependency version must remain untouched"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_update_preserves_unrelated_project_tree_files() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_update_preserve_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // Injected strategy stands in for the real staged `deno add`, matching what a
        // real resolution of "is-number@7.0.0" against an empty project would produce.
        let add_fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let add_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(add_fixture.clone())
        });
        let add_res = add_with_context(&base_tmp, "is-number@7.0.0", &add_ctx);
        assert!(add_res.is_ok());

        let src_dir = base_tmp.join("src");
        let docs_dir = base_tmp.join("docs");
        let dist_dir = base_tmp.join("dist");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&docs_dir).unwrap();
        fs::create_dir_all(&dist_dir).unwrap();

        let wda_json_path = base_tmp.join("wda.json");
        let src_file_path = src_dir.join("main.ts");
        let doc_file_path = docs_dir.join("architecture.md");
        let dist_file_path = dist_dir.join("output.html");

        let wda_json_content = "{\n  \"wdaVersion\": \"0.1.0\"\n}\n";
        let src_content = "console.log('web-design-anchor');\n";
        let doc_content = "# Documentation\nArchitecture details\n";
        let dist_content = "<html><body>Built site</body></html>\n";

        fs::write(&wda_json_path, wda_json_content).unwrap();
        fs::write(&src_file_path, src_content).unwrap();
        fs::write(&doc_file_path, doc_content).unwrap();
        fs::write(&dist_file_path, dist_content).unwrap();

        // Injected strategy stands in for the real staged `deno update`; this test
        // cares only about the surrounding project-tree files, not the staged content.
        let update_fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let update_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(update_fixture.clone())
        });
        let res = update_with_context(&base_tmp, None, &update_ctx);
        assert!(res.is_ok(), "update must succeed: {:?}", res.err());

        assert_eq!(
            fs::read_to_string(&wda_json_path).unwrap(),
            wda_json_content,
            "wda.json must remain byte-identical after update"
        );
        assert_eq!(
            fs::read_to_string(&src_file_path).unwrap(),
            src_content,
            "src/ file must remain byte-identical after update"
        );
        assert_eq!(
            fs::read_to_string(&doc_file_path).unwrap(),
            doc_content,
            "docs/ file must remain byte-identical after update"
        );
        assert_eq!(
            fs::read_to_string(&dist_file_path).unwrap(),
            dist_content,
            "dist/ file must remain byte-identical after update"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_update_corrupted_manifest_faults_before_commit() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_update_corrupt_manifest_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let corrupt_json = "{\n  \"dependencies\": { invalid json\n";
        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");
        fs::write(&manifest_path, corrupt_json).unwrap();
        fs::write(&lock_path, "{\n  \"version\": \"3\"\n}\n").unwrap();

        let manifest_bytes_before = fs::read(&manifest_path).unwrap();
        let lock_bytes_before = fs::read(&lock_path).unwrap();

        let res = update(&base_tmp, None);
        assert!(res.is_err(), "corrupt manifest must cause update to fail");

        match res.unwrap_err() {
            DepsError::ToolFault(fault) => {
                assert!(fault.message.contains("Failed to parse package.json"));
            }
            other => panic!("expected ToolFault, got: {other:?}"),
        }

        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        let lock_bytes_after = fs::read(&lock_path).unwrap();
        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "corrupt package.json must remain byte-identical"
        );
        assert_eq!(
            lock_bytes_before, lock_bytes_after,
            "deno.lock must remain byte-identical"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_update_missing_deno_leaves_project_byte_identical() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_update_missing_deno_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n";
        let initial_lock = "{\n  \"version\": \"3\",\n  \"packages\": {}\n}\n";
        let manifest_path = base_tmp.join("package.json");
        let lock_path = base_tmp.join("deno.lock");
        fs::write(&manifest_path, initial_manifest).unwrap();
        fs::write(&lock_path, initial_lock).unwrap();

        let manifest_bytes_before = fs::read(&manifest_path).unwrap();
        let lock_bytes_before = fs::read(&lock_path).unwrap();

        let res = update_with_path(&base_tmp, None, Some(std::ffi::OsStr::new("")));
        assert!(res.is_err(), "missing deno on PATH must fail");

        match res.unwrap_err() {
            DepsError::Diagnostic(diag) => {
                assert_eq!(diag.code, crate::codes::TOOL_FAULT);
                assert!(diag.message.contains("deno"));
            }
            other => panic!("expected Diagnostic(TOOL_FAULT), got: {other:?}"),
        }

        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        let lock_bytes_after = fs::read(&lock_path).unwrap();
        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "package.json must remain byte-identical when deno resolution fails"
        );
        assert_eq!(
            lock_bytes_before, lock_bytes_after,
            "deno.lock must remain byte-identical when deno resolution fails"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_update_empty_or_whitespace_name_filter_returns_cli_usage() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_update_empty_name_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n";
        let manifest_path = base_tmp.join("package.json");
        fs::write(&manifest_path, initial_manifest).unwrap();
        let manifest_bytes_before = fs::read(&manifest_path).unwrap();

        let res = update(&base_tmp, Some("   "));
        assert!(res.is_err());

        match res.unwrap_err() {
            DepsError::Diagnostic(diag) => {
                assert_eq!(diag.code, codes::CLI_USAGE);
                assert_eq!(diag.severity, Severity::Error);
                assert!(diag.message.contains("empty"));
            }
            other => panic!("expected Diagnostic(CLI_USAGE), got: {other:?}"),
        }

        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        assert_eq!(manifest_bytes_before, manifest_bytes_after);

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_update_interrupted_between_renames_leaves_named_state() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_update_interrupted_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // Injected strategy stands in for the real staged `deno add`, matching what a
        // real resolution of "is-number@7.0.0" against an empty project would produce.
        let add_fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let add_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(add_fixture.clone())
        });
        let add_res = add_with_context(&base_tmp, "is-number@7.0.0", &add_ctx);
        assert!(add_res.is_ok());

        let manifest_path = base_tmp.join("package.json");
        let manifest_bytes_before = fs::read(&manifest_path).unwrap();

        // Injected strategy stands in for the real staged `deno update`, matching what
        // a real unfiltered update of this single-dependency project would produce.
        let update_fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let update_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(update_fixture.clone())
        });

        let res = update_with_context_and_hook(&base_tmp, None, &update_ctx, |step| {
            if step == CommitStep::AfterLockRenamed {
                Err(ToolFault::new(
                    "Simulated interruption between renames",
                    None::<PathBuf>,
                ))
            } else {
                Ok(())
            }
        });

        assert!(res.is_err(), "interrupted commit must return Err");

        let manifest_bytes_after = fs::read(&manifest_path).unwrap();
        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "package.json must remain byte-identical to pre-operation when interrupted"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_update_aliases_match() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_update_alias_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        // Injected strategy stands in for the real staged `deno add`, matching what a
        // real resolution of "is-number@7.0.0" against an empty project would produce.
        let add_fixture = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let add_ctx = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(add_fixture.clone())
        });
        let add_res = add_with_context(&base_tmp, "is-number@7.0.0", &add_ctx);
        assert!(add_res.is_ok());

        // Injected strategy stands in for the real staged `deno update`; the alias
        // under test is `update_with_context`, the context-accepting counterpart of
        // `update_dependency`.
        let update_fixture1 = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let update_ctx1 = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(update_fixture1.clone())
        });
        let res1 = update_with_context(&base_tmp, Some("is-number"), &update_ctx1);
        assert!(res1.is_ok(), "update_dependency alias must succeed");

        // Injected strategy stands in for the real staged `deno update`; the alias
        // under test is `update_with_context`, the context-accepting counterpart of
        // `update_dependencies`.
        let update_fixture2 = resolve::StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let update_ctx2 = resolve::InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(update_fixture2.clone())
        });
        let res2 = update_with_context(&base_tmp, None, &update_ctx2);
        assert!(res2.is_ok(), "update_dependencies alias must succeed");

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_update_non_existent_root_returns_tool_fault() {
        let non_existent = std::env::temp_dir().join(format!(
            "wda_test_update_non_existent_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&non_existent);

        let res = update(&non_existent, None);
        assert!(res.is_err(), "non-existent root directory must fail");

        match res.unwrap_err() {
            DepsError::ToolFault(fault) => {
                assert!(fault.message.contains("Project root directory not found"));
            }
            other => panic!("expected ToolFault, got: {other:?}"),
        }
    }

    // The three `#[ignore]` probes below reach the real `deno` binary and the real
    // registry through the production entry points `add`/`remove`/`update` (real PATH
    // via `std::env::var_os("PATH")`, real `deno`, real registry, no injected
    // `ResolutionStrategy`). They stay out of the default `cargo test` run and are
    // exercised deliberately via `cargo test -- --ignored` with network available.
    // Per D-03, none of them sets `DENO_AUTH_TOKENS`/`NPM_TOKEN` at all, which
    // trivially satisfies "a well-formed `token@host` value or sets none".

    #[test]
    #[ignore]
    fn test_ignored_add_real_registry_pinned_spec() {
        // Real-binary probe: calls the public `add` entry point directly, so
        // resolution runs through the production `InvocationContext::from_env`
        // path (real PATH, real `deno`, real registry) with a pinned spec.
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_ignored_add_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let staged = add(&base_tmp, "is-number@7.0.0").expect("add should succeed");

        assert!(
            staged.package_json.contains("is-number"),
            "committed package.json must contain is-number: {}",
            staged.package_json
        );
        assert_eq!(
            fs::read_to_string(base_tmp.join("package.json")).unwrap(),
            staged.package_json,
            "add must commit the staged package.json to the project root"
        );
        assert!(!base_tmp.join("node_modules").exists());

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    #[ignore]
    fn test_ignored_remove_real_registry_pinned_spec() {
        // Real-binary probe: calls the public `remove` entry point directly, so
        // resolution runs through the production `InvocationContext::from_env`
        // path (real PATH, real `deno`, real registry) with a pinned spec.
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_ignored_remove_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let staged = add(&base_tmp, "is-number@7.0.0").expect("add should succeed");
        assert!(staged.package_json.contains("is-number"));

        // "is-number" is the project's only dependency, so removing it drives the
        // project to the empty state: `remove` deletes `package.json` and `deno.lock`
        // rather than committing an emptied manifest (see
        // `test_remove_last_dependency_deletes_both_files_and_restores_empty_state`).
        let outcome = remove(&base_tmp, "is-number").expect("remove should succeed");
        assert_eq!(outcome, CommitOutcome::Empty, "removing the last dependency must yield Empty");
        assert!(
            !base_tmp.join("package.json").exists(),
            "package.json must be deleted when removing the last dependency"
        );
        assert!(!base_tmp.join("deno.lock").exists());
        assert!(!base_tmp.join("node_modules").exists());

        let _ = fs::remove_dir_all(&base_tmp);
    }

    #[test]
    #[ignore]
    fn test_ignored_update_real_registry_pinned_spec() {
        // Real-binary probe: calls the public `update` entry point directly, so
        // resolution runs through the production `InvocationContext::from_env`
        // path (real PATH, real `deno`, real registry) with a pinned spec.
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_ignored_update_{}_{}",
            std::process::id(),
            resolve::unique_staging_suffix()
        ));
        let _ = fs::remove_dir_all(&base_tmp);
        fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"is-odd\": \"^3.0.0\"\n  }\n}\n";
        fs::write(base_tmp.join("package.json"), initial_manifest).unwrap();

        let staged = update(&base_tmp, Some("is-odd"))
            .expect("update should succeed")
            .expect("update should stage new files, not a no-op");

        assert!(
            staged.package_json.contains("\"is-odd\": \"^3.0.1\""),
            "committed package.json must have updated is-odd: {}",
            staged.package_json
        );
        assert_eq!(
            fs::read_to_string(base_tmp.join("package.json")).unwrap(),
            staged.package_json,
            "update must commit the staged package.json to the project root"
        );
        assert!(!base_tmp.join("node_modules").exists());

        let _ = fs::remove_dir_all(&base_tmp);
    }
}

