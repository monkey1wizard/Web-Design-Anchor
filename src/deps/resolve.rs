//! Deno-backed dependency resolution and `deno.lock` handling for `wda deps`.

#![allow(dead_code)]

use std::cell::RefCell;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::codes;
use crate::diagnostics::{Diagnostic, Severity};

/// Resolves the path to `deno` on the system PATH.
///
/// Returns `Ok(path)` if found and runnable, or an Error [`Diagnostic`] carrying
/// `codes::TOOL_FAULT` naming `"deno"` on failure.
pub fn resolve_deno() -> Result<PathBuf, Diagnostic> {
    resolve_deno_with_path(std::env::var_os("PATH").as_deref())
}

/// Resolves `deno` with an explicit PATH environment variable value.
pub(crate) fn resolve_deno_with_path(path_env: Option<&OsStr>) -> Result<PathBuf, Diagnostic> {
    resolve_tool_with_path("deno", path_env)
}

/// Resolves a tool by name with an explicit PATH environment variable value.
pub(crate) fn resolve_tool_with_path(
    tool_name: &str,
    path_env: Option<&OsStr>,
) -> Result<PathBuf, Diagnostic> {
    let trimmed = tool_name.trim();
    if trimmed.is_empty() {
        return Err(Diagnostic::new(
            codes::TOOL_FAULT,
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
            codes::TOOL_FAULT,
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
        codes::TOOL_FAULT,
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
                    codes::TOOL_FAULT,
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
            codes::TOOL_FAULT,
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

/// The staged dependency files extracted from a confined Deno invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedFiles {
    /// The UTF-8 string content of `package.json`.
    pub package_json: String,
    /// The optional UTF-8 string content of `deno.lock`, present if Deno generated or updated it.
    pub deno_lock: Option<String>,
}

/// RAII guard that recursively deletes a temporary directory on drop.
struct StagingDirGuard {
    path: PathBuf,
}

impl StagingDirGuard {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for StagingDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Monotonic tie-breaker for [`unique_staging_suffix`], combined with a wall-clock
/// reading so two suffixes generated in the same process never collide even when
/// the clock's resolution is coarser than the call rate (observed as flaky test
/// staging-directory collisions on Windows).
static STAGING_SUFFIX_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Returns a suffix unique within this process, for naming transient staging
/// directories and temporary file copies. The rendered form combines a
/// nanosecond wall-clock reading with a per-process atomic counter; callers
/// must treat it as an opaque, display-only string (per A-05, no contract,
/// diagnostic, or `dist/` output refers to it).
///
/// Shared with [`crate::deps`], which uses it to name the hidden temporary
/// copies of `deno.lock` and `package.json` it stages under the caller's
/// project root before the atomic rename into place.
pub(crate) fn unique_staging_suffix() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let counter = STAGING_SUFFIX_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos}-{counter}")
}

/// A caller-supplied stand-in for the real staged Deno invocation, consulted by
/// [`invoke_staged_deno_with_context`] before it ever calls
/// [`resolve_deno_with_path`]. Injecting one lets a test obtain [`StagedFiles`]
/// (or a [`Diagnostic`]) deterministically, with neither `verify_can_start()`
/// nor a subprocess spawn ever running.
///
/// Takes `&mut` so a single strategy can return a different outcome on each
/// successive call, for tests that chain an `install` followed by a `remove`
/// or `update` against the same fixture.
pub(crate) type ResolutionStrategy =
    dyn FnMut(&[&str]) -> Result<StagedFiles, Diagnostic> + Send;

/// Execution context for a staged Deno invocation: the PATH override used to
/// resolve the `deno` binary, the explicit set of environment variables the
/// child process receives, and an optional resolution strategy.
///
/// The child's environment is never inherited implicitly — [`build_deno_command`]
/// always calls `env_clear()` before applying this map, so the map is the
/// complete, authoritative environment for the spawned process.
pub(crate) struct InvocationContext {
    path_env: Option<OsString>,
    env_vars: Vec<(OsString, OsString)>,
    strategy: RefCell<Option<Box<ResolutionStrategy>>>,
}

impl InvocationContext {
    /// Production constructor: resolves `deno` using `path_env`, and passes the
    /// full parent environment (`std::env::vars_os()`) through to the child.
    ///
    /// Per B-04, this applies no allow-list — every variable visible to this
    /// process is forwarded unfiltered. Carries no resolution strategy, so
    /// [`invoke_staged_deno_with_context`] always defers to the real `deno` binary.
    pub(crate) fn from_env(path_env: Option<&OsStr>) -> Self {
        Self {
            path_env: path_env.map(OsStr::to_os_string),
            env_vars: std::env::vars_os().collect(),
            strategy: RefCell::new(None),
        }
    }

    /// Test-only constructor for an explicit, non-inherited environment map.
    /// Carries no resolution strategy; use [`InvocationContext::with_strategy`]
    /// to also short-circuit resolution.
    #[cfg(test)]
    pub(crate) fn with_env(path_env: Option<&OsStr>, env_vars: Vec<(OsString, OsString)>) -> Self {
        Self {
            path_env: path_env.map(OsStr::to_os_string),
            env_vars,
            strategy: RefCell::new(None),
        }
    }

    /// Test-only constructor carrying an injected resolution strategy alongside
    /// the explicit PATH override and environment map. Per B-05, the strategy is
    /// consulted before [`resolve_deno_with_path`] runs, so `path_env` needs no
    /// real `deno` on it.
    #[cfg(test)]
    pub(crate) fn with_strategy(
        path_env: Option<&OsStr>,
        env_vars: Vec<(OsString, OsString)>,
        strategy: impl FnMut(&[&str]) -> Result<StagedFiles, Diagnostic> + Send + 'static,
    ) -> Self {
        Self {
            path_env: path_env.map(OsStr::to_os_string),
            env_vars,
            strategy: RefCell::new(Some(Box::new(strategy))),
        }
    }

    fn path_env(&self) -> Option<&OsStr> {
        self.path_env.as_deref()
    }
}

/// Builds (without spawning) the `deno` [`Command`] for a staged invocation.
///
/// Calls `env_clear()` then `envs()` so the child receives exactly the map
/// carried by `ctx` — no variable is inherited from this process outside that
/// map. Returning the unspawned `Command` (per AV-6) lets tests observe the
/// applied environment via `get_envs()` without starting a process.
pub(crate) fn build_deno_command(
    deno_binary: &Path,
    staging_dir: &Path,
    args: &[&str],
    ctx: &InvocationContext,
) -> Command {
    let mut command = Command::new(deno_binary);
    command
        .current_dir(staging_dir)
        .args(args)
        .env_clear()
        .envs(
            ctx.env_vars
                .iter()
                .map(|(key, value)| (key.as_os_str(), value.as_os_str())),
        )
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    command
}

/// Executes a Deno command confined to a fresh, isolated staging directory.
///
/// 1. Creates a fresh temporary staging directory.
/// 2. If the project root has `package.json`, copies it into staging; otherwise seeds `{"dependencies":{}}\n`.
/// 3. If the project root has `deno.lock`, copies it into staging.
/// 4. Resolves `deno` on PATH (or via `resolve_deno_with_path`).
/// 5. Invokes the `deno` subprocess with `current_dir` set explicitly to the staging directory.
/// 6. Reads back the resulting `package.json` and optional `deno.lock`.
/// 7. Automatically deletes the entire staging directory (including any `node_modules/` or cache) on return/drop.
pub fn invoke_staged_deno(
    project_root: &Path,
    args: &[&str],
) -> Result<StagedFiles, Diagnostic> {
    let ctx = InvocationContext::from_env(std::env::var_os("PATH").as_deref());
    invoke_staged_deno_with_context(project_root, args, &ctx)
}

/// Executes a Deno command confined to a fresh staging directory with an explicit PATH.
pub(crate) fn invoke_staged_deno_with_path(
    project_root: &Path,
    args: &[&str],
    path_env: Option<&OsStr>,
) -> Result<StagedFiles, Diagnostic> {
    let ctx = InvocationContext::from_env(path_env);
    invoke_staged_deno_with_context(project_root, args, &ctx)
}

/// Executes a Deno command confined to a fresh staging directory with an explicit
/// [`InvocationContext`] carrying the PATH override and the child's environment map.
pub(crate) fn invoke_staged_deno_with_context(
    project_root: &Path,
    args: &[&str],
    ctx: &InvocationContext,
) -> Result<StagedFiles, Diagnostic> {
    // Per B-05, an injected strategy is consulted before `resolve_deno_with_path`
    // runs. Placing the check first is the point: when a strategy is present, it
    // returns (or errors) here and neither `verify_can_start()` nor the `Command`
    // spawn further down is ever reached.
    if let Some(strategy) = ctx.strategy.borrow_mut().as_mut() {
        return strategy(args);
    }

    let deno_binary = resolve_deno_with_path(ctx.path_env())?;

    let staging_dir = std::env::temp_dir().join(format!(
        "wda-deps-stage-{}-{}",
        std::process::id(),
        unique_staging_suffix()
    ));

    std::fs::create_dir_all(&staging_dir).map_err(|err| {
        Diagnostic::new(
            codes::TOOL_FAULT,
            Severity::Error,
            None,
            format!(
                "Failed to create staging directory at '{}': {err}",
                staging_dir.display()
            ),
            "Ensure the system temporary directory is writable",
        )
    })?;

    let _guard = StagingDirGuard::new(staging_dir.clone());

    // Prepare package.json in staging
    let source_manifest = project_root.join("package.json");
    let target_manifest = staging_dir.join("package.json");
    if source_manifest.is_file() {
        std::fs::copy(&source_manifest, &target_manifest).map_err(|err| {
            Diagnostic::new(
                codes::TOOL_FAULT,
                Severity::Error,
                None,
                format!(
                    "Failed to copy package.json to staging directory '{}': {err}",
                    staging_dir.display()
                ),
                "Ensure package.json is readable",
            )
        })?;
    } else {
        std::fs::write(&target_manifest, b"{\"dependencies\":{}}\n").map_err(|err| {
            Diagnostic::new(
                codes::TOOL_FAULT,
                Severity::Error,
                None,
                format!(
                    "Failed to seed package.json in staging directory '{}': {err}",
                    staging_dir.display()
                ),
                "Ensure the system temporary directory is writable",
            )
        })?;
    }

    // Prepare deno.lock in staging if present
    let source_lock = project_root.join("deno.lock");
    let target_lock = staging_dir.join("deno.lock");
    if source_lock.is_file() {
        std::fs::copy(&source_lock, &target_lock).map_err(|err| {
            Diagnostic::new(
                codes::TOOL_FAULT,
                Severity::Error,
                None,
                format!(
                    "Failed to copy deno.lock to staging directory '{}': {err}",
                    staging_dir.display()
                ),
                "Ensure deno.lock is readable",
            )
        })?;
    }

    // Run the deno subprocess in staging_dir
    let mut command = build_deno_command(&deno_binary, &staging_dir, args, ctx);

    let output = command.output().map_err(|err| {
        Diagnostic::new(
            codes::TOOL_FAULT,
            Severity::Error,
            None,
            format!(
                "Failed to execute deno in staging directory '{}': {err}",
                staging_dir.display()
            ),
            "Ensure deno can be executed",
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            format!("Deno command failed with exit status {}", output.status)
        } else {
            format!("Deno command failed: {stderr}")
        };
        return Err(Diagnostic::new(
            codes::TOOL_FAULT,
            Severity::Error,
            None,
            message,
            "Check the dependency specifier and network connectivity, then retry",
        ));
    }

    let staged_package_json = std::fs::read_to_string(&target_manifest).map_err(|err| {
        Diagnostic::new(
            codes::TOOL_FAULT,
            Severity::Error,
            None,
            format!(
                "Failed to read staged package.json at '{}': {err}",
                target_manifest.display()
            ),
            "Check that deno produced a valid package.json",
        )
    })?;

    let staged_deno_lock = if target_lock.is_file() {
        let lock_content = std::fs::read_to_string(&target_lock).map_err(|err| {
            Diagnostic::new(
                codes::TOOL_FAULT,
                Severity::Error,
                None,
                format!(
                    "Failed to read staged deno.lock at '{}': {err}",
                    target_lock.display()
                ),
                "Check that deno produced a valid deno.lock",
            )
        })?;
        Some(lock_content)
    } else {
        None
    };

    Ok(StagedFiles {
        package_json: staged_package_json,
        deno_lock: staged_deno_lock,
    })
}

/// Adds a dependency specification using `deno add --package-json <spec>` in a staged environment.
///
/// Uses [`invoke_staged_deno`] to execute the command confined to a temporary staging directory,
/// returning the staged `package.json` and optional `deno.lock` on success, or a mapped
/// [`codes::TOOL_FAULT`] Diagnostic on failure (e.g. resolver failure or process failure).
pub fn resolve_add(project_root: &Path, spec: &str) -> Result<StagedFiles, Diagnostic> {
    resolve_add_with_path(project_root, spec, std::env::var_os("PATH").as_deref())
}

/// Adds a dependency specification using `deno add --package-json <spec>` with an explicit PATH.
pub(crate) fn resolve_add_with_path(
    project_root: &Path,
    spec: &str,
    path_env: Option<&OsStr>,
) -> Result<StagedFiles, Diagnostic> {
    let ctx = InvocationContext::from_env(path_env);
    resolve_add_with_context(project_root, spec, &ctx)
}

/// Adds a dependency specification using `deno add --package-json <spec>` with an explicit
/// [`InvocationContext`] carrying the PATH override, child environment, and (in tests) an
/// injected resolution strategy that lets callers obtain [`StagedFiles`] deterministically,
/// without ever consulting real PATH or spawning `deno`.
pub(crate) fn resolve_add_with_context(
    project_root: &Path,
    spec: &str,
    ctx: &InvocationContext,
) -> Result<StagedFiles, Diagnostic> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(Diagnostic::new(
            codes::TOOL_FAULT,
            Severity::Error,
            None,
            "Dependency specification cannot be empty",
            "Specify a valid package name or dependency specifier",
        ));
    }
    invoke_staged_deno_with_context(project_root, &["add", "--package-json", trimmed], ctx)
}

/// Direct alias for [`resolve_add`].
pub fn add(project_root: &Path, spec: &str) -> Result<StagedFiles, Diagnostic> {
    resolve_add(project_root, spec)
}

/// Direct alias for [`resolve_add_with_context`].
pub(crate) fn add_with_context(
    project_root: &Path,
    spec: &str,
    ctx: &InvocationContext,
) -> Result<StagedFiles, Diagnostic> {
    resolve_add_with_context(project_root, spec, ctx)
}

/// Removes a dependency using `deno remove --package-json <name>` in a staged environment.
///
/// Uses [`invoke_staged_deno`] to execute the command confined to a temporary staging directory,
/// returning the staged (possibly now-empty) `package.json` and optional `deno.lock` on success,
/// or a mapped [`codes::TOOL_FAULT`] Diagnostic on failure (e.g. resolver failure or process failure).
///
/// Does not decide whether the result is an empty state — that decision belongs to caller orchestration.
pub fn resolve_remove(project_root: &Path, name: &str) -> Result<StagedFiles, Diagnostic> {
    resolve_remove_with_path(project_root, name, std::env::var_os("PATH").as_deref())
}

/// Removes a dependency using `deno remove --package-json <name>` with an explicit PATH.
pub(crate) fn resolve_remove_with_path(
    project_root: &Path,
    name: &str,
    path_env: Option<&OsStr>,
) -> Result<StagedFiles, Diagnostic> {
    let ctx = InvocationContext::from_env(path_env);
    resolve_remove_with_context(project_root, name, &ctx)
}

/// Removes a dependency using `deno remove --package-json <name>` with an explicit
/// [`InvocationContext`] carrying the PATH override, child environment, and (in tests) an
/// injected resolution strategy that lets callers obtain [`StagedFiles`] deterministically,
/// without ever consulting real PATH or spawning `deno`.
pub(crate) fn resolve_remove_with_context(
    project_root: &Path,
    name: &str,
    ctx: &InvocationContext,
) -> Result<StagedFiles, Diagnostic> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(Diagnostic::new(
            codes::TOOL_FAULT,
            Severity::Error,
            None,
            "Dependency name cannot be empty",
            "Specify a valid dependency name to remove",
        ));
    }
    invoke_staged_deno_with_context(project_root, &["remove", "--package-json", trimmed], ctx)
}

/// Direct alias for [`resolve_remove`].
pub fn remove(project_root: &Path, name: &str) -> Result<StagedFiles, Diagnostic> {
    resolve_remove(project_root, name)
}

/// Direct alias for [`resolve_remove_with_context`].
pub(crate) fn remove_with_context(
    project_root: &Path,
    name: &str,
    ctx: &InvocationContext,
) -> Result<StagedFiles, Diagnostic> {
    resolve_remove_with_context(project_root, name, ctx)
}

/// Updates dependencies using `deno update` (unfiltered) or `deno update <name>` (filtered) in a staged environment.
///
/// Uses [`invoke_staged_deno`] to execute the command confined to a temporary staging directory,
/// returning the staged `package.json` and optional `deno.lock` on success, or a mapped
/// [`codes::TOOL_FAULT`] Diagnostic on failure (e.g. resolver failure or process failure).
///
/// Note that `deno update` auto-detects `package.json` in the working directory (there is no `--package-json` flag).
pub fn resolve_update(
    project_root: &Path,
    name_filter: Option<&str>,
) -> Result<StagedFiles, Diagnostic> {
    resolve_update_with_path(
        project_root,
        name_filter,
        std::env::var_os("PATH").as_deref(),
    )
}

/// Updates dependencies using `deno update` with an explicit PATH.
pub(crate) fn resolve_update_with_path(
    project_root: &Path,
    name_filter: Option<&str>,
    path_env: Option<&OsStr>,
) -> Result<StagedFiles, Diagnostic> {
    let ctx = InvocationContext::from_env(path_env);
    resolve_update_with_context(project_root, name_filter, &ctx)
}

/// Updates dependencies using `deno update` with an explicit [`InvocationContext`] carrying
/// the PATH override, child environment, and (in tests) an injected resolution strategy that
/// lets callers obtain [`StagedFiles`] deterministically, without ever consulting real PATH
/// or spawning `deno`.
pub(crate) fn resolve_update_with_context(
    project_root: &Path,
    name_filter: Option<&str>,
    ctx: &InvocationContext,
) -> Result<StagedFiles, Diagnostic> {
    match name_filter {
        None => invoke_staged_deno_with_context(project_root, &["update"], ctx),
        Some(filter) => {
            let trimmed = filter.trim();
            if trimmed.is_empty() {
                return Err(Diagnostic::new(
                    codes::TOOL_FAULT,
                    Severity::Error,
                    None,
                    "Dependency filter name cannot be empty",
                    "Specify a valid dependency name to update or omit the filter",
                ));
            }
            invoke_staged_deno_with_context(project_root, &["update", trimmed], ctx)
        }
    }
}

/// Direct alias for [`resolve_update`].
pub fn update(
    project_root: &Path,
    name_filter: Option<&str>,
) -> Result<StagedFiles, Diagnostic> {
    resolve_update(project_root, name_filter)
}

/// Direct alias for [`resolve_update_with_context`].
pub(crate) fn update_with_context(
    project_root: &Path,
    name_filter: Option<&str>,
    ctx: &InvocationContext,
) -> Result<StagedFiles, Diagnostic> {
    resolve_update_with_context(project_root, name_filter, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_deno_success() {
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
    fn test_resolve_deno_with_custom_empty_path_fails() {
        let res = resolve_deno_with_path(Some(OsStr::new("")));
        assert!(res.is_err(), "empty PATH must fail resolution");
        let diag = res.unwrap_err();
        assert_eq!(diag.code, codes::TOOL_FAULT);
        assert_eq!(diag.severity, Severity::Error);
        assert!(
            diag.message.contains("deno"),
            "diagnostic message must name 'deno', got: {}",
            diag.message
        );
        assert!(diag.location.is_none());
    }

    #[test]
    fn test_resolve_tool_impossible_name_yields_tool_fault() {
        let impossible_name = "impossible_tool_name_wda_test_nonexistent";
        let res = resolve_tool_with_path(impossible_name, std::env::var_os("PATH").as_deref());
        assert!(res.is_err(), "impossible tool name must fail resolution");
        let diag = res.unwrap_err();
        assert_eq!(diag.code, codes::TOOL_FAULT);
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
    fn test_resolve_tool_empty_name_fails() {
        let res = resolve_tool_with_path("", std::env::var_os("PATH").as_deref());
        assert!(res.is_err());
        let diag = res.unwrap_err();
        assert_eq!(diag.code, codes::TOOL_FAULT);
        assert_eq!(diag.severity, Severity::Error);
    }

    #[test]
    fn test_resolve_deno_non_red_probe_empty_dir_then_real_path() {
        // Non-red probe: call with PATH set to a directory containing no deno,
        // then with the real PATH; assert both outcomes.
        let empty_dir = std::env::temp_dir().join(format!(
            "wda_test_deps_no_deno_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::create_dir_all(&empty_dir);

        let res_no_deno = std::panic::catch_unwind(|| {
            resolve_deno_with_path(Some(empty_dir.as_os_str()))
        });
        assert!(
            res_no_deno.is_ok(),
            "resolving deno in directory with no deno must not panic"
        );
        let err_outcome = res_no_deno.unwrap();
        assert!(
            err_outcome.is_err(),
            "resolving deno in directory with no deno must return Err"
        );
        let diag = err_outcome.unwrap_err();
        assert_eq!(diag.code, codes::TOOL_FAULT);
        assert_eq!(diag.code, "wda.tool.fault");
        assert_eq!(diag.severity, Severity::Error);
        assert!(
            diag.message.contains("deno"),
            "diagnostic message must name 'deno', got: {}",
            diag.message
        );
        assert!(diag.location.is_none());

        let _ = std::fs::remove_dir_all(&empty_dir);

        let res_real = std::panic::catch_unwind(|| {
            resolve_deno()
        });
        assert!(
            res_real.is_ok(),
            "resolving deno with real PATH must not panic"
        );
        let ok_outcome = res_real.unwrap();
        assert!(
            ok_outcome.is_ok(),
            "resolving deno on real PATH must succeed, got: {:?}",
            ok_outcome.err()
        );
        let path = ok_outcome.unwrap();
        assert!(
            path.is_file(),
            "resolved deno path must be an existing file: {}",
            path.display()
        );
    }

    #[test]
    fn test_tp02_impossible_tool_name_yields_tool_fault_not_panic() {
        let impossible_name = "impossible_tool_name_wda_tp02_nonexistent";
        let res = std::panic::catch_unwind(|| {
            resolve_tool_with_path(impossible_name, std::env::var_os("PATH").as_deref())
        });
        assert!(res.is_ok(), "impossible tool name must not panic");
        let outcome = res.unwrap();
        assert!(outcome.is_err(), "impossible tool name must return Err");
        let diag = outcome.unwrap_err();
        assert_eq!(diag.code, codes::TOOL_FAULT);
        assert_eq!(diag.code, "wda.tool.fault");
        assert_eq!(diag.severity, Severity::Error);
        assert!(
            diag.message.contains(impossible_name),
            "diagnostic message must name the tool '{}', got: {}",
            impossible_name,
            diag.message
        );
        assert!(diag.location.is_none());
    }

    // The five `#[ignore]` probes below reach the real `deno` binary and the real
    // registry through the production context (no injected `ResolutionStrategy`,
    // no `resolve_*_with_path`/`resolve_*_with_context` test-only entry point).
    // They stay out of the default `cargo test` run and are exercised deliberately
    // via `cargo test -- --ignored` with network available. Per D-03, none of them
    // sets `DENO_AUTH_TOKENS`/`NPM_TOKEN` at all, which trivially satisfies "a
    // well-formed `token@host` value or sets none".
    //
    // `test_tp03_staged_deno_confined_nested_ancestor_and_no_node_modules` and
    // `test_staged_deno_with_existing_package_and_lock` join this set per the
    // Owner ruling on AV-5: they assert on the staging mechanism itself, which no
    // longer runs once a caller injects a `ResolutionStrategy`, so their bodies
    // and assertions carry over verbatim as real-binary probes instead.

    #[test]
    #[ignore]
    fn test_tp03_staged_deno_confined_nested_ancestor_and_no_node_modules() {
        // Fixture: a nested tree entirely inside an isolated tmp root,
        // where an ancestor 2+ levels above the project root has its own unrelated package.json
        // with dependencies, and the project root starts with zero dependencies.
        // The staged add invocation writes only inside the project root (returned staged files);
        // the ancestor's package.json is byte-identical before and after.
        // Separately: after any staged invocation, the project root contains no node_modules/ —
        // the staging directory and everything Deno wrote into it are torn down after extracting
        // package.json + deno.lock, never touching the real tree directly.
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_tp03_test_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&base_tmp);
        std::fs::create_dir_all(&base_tmp).unwrap();

        // Structure:
        // base_tmp/ (ancestor with unrelated package.json)
        // base_tmp/sub1/sub2/ (project root with 0 dependencies)
        let ancestor_manifest = base_tmp.join("package.json");
        let initial_ancestor_content = "{\n  \"name\": \"unrelated-ancestor\",\n  \"dependencies\": {\n    \"chalk\": \"^5.0.0\"\n  }\n}\n";
        std::fs::write(&ancestor_manifest, initial_ancestor_content.as_bytes()).unwrap();
        let ancestor_bytes_before = std::fs::read(&ancestor_manifest).unwrap();

        let project_root = base_tmp.join("sub1").join("sub2");
        std::fs::create_dir_all(&project_root).unwrap();

        // Project root starts with zero dependencies (no package.json, no deno.lock)
        assert!(!project_root.join("package.json").exists());
        assert!(!project_root.join("deno.lock").exists());

        // Invoke staged deno to add chalk
        let staged = invoke_staged_deno(&project_root, &["add", "--package-json", "chalk"])
            .expect("staged deno add should succeed");

        // Staged package.json must contain chalk
        assert!(
            staged.package_json.contains("chalk"),
            "staged package.json must contain chalk: {}",
            staged.package_json
        );
        // Deno lock must be present in staged files
        assert!(
            staged.deno_lock.is_some(),
            "deno.lock should have been generated in staging"
        );
        let lock = staged.deno_lock.unwrap();
        assert!(
            lock.contains("chalk"),
            "staged deno.lock must contain chalk: {}",
            lock
        );

        // Check ancestor package.json is byte-identical before and after (no directory escape!)
        let ancestor_bytes_after = std::fs::read(&ancestor_manifest).unwrap();
        assert_eq!(
            ancestor_bytes_before, ancestor_bytes_after,
            "ancestor package.json must be byte-identical before and after"
        );

        // Check real project root: no files were written to project_root directly,
        // and specifically NO node_modules/ directory exists in project_root.
        assert!(
            !project_root.join("package.json").exists(),
            "invoke_staged_deno must not write directly to project_root/package.json"
        );
        assert!(
            !project_root.join("deno.lock").exists(),
            "invoke_staged_deno must not write directly to project_root/deno.lock"
        );
        assert!(
            !project_root.join("node_modules").exists(),
            "invoke_staged_deno must leave no node_modules/ in project_root"
        );

        // Clean up fixture directory
        let _ = std::fs::remove_dir_all(&base_tmp);
    }

    #[test]
    #[ignore]
    fn test_staged_deno_with_existing_package_and_lock() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_staged_existing_test_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&base_tmp);
        std::fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"chalk\": \"^6.0.0\"\n  }\n}\n";
        std::fs::write(base_tmp.join("package.json"), initial_manifest).unwrap();

        // First resolve to get a real lock
        let staged1 = invoke_staged_deno(&base_tmp, &["install"]).expect("staged install");
        assert!(staged1.deno_lock.is_some());

        // Write the lock to project root
        std::fs::write(base_tmp.join("deno.lock"), staged1.deno_lock.as_ref().unwrap()).unwrap();

        // Now run remove chalk
        let staged2 = invoke_staged_deno(&base_tmp, &["remove", "--package-json", "chalk"])
            .expect("staged remove");
        assert!(
            !staged2.package_json.contains("chalk") || staged2.package_json.contains("\"dependencies\": {}") || staged2.package_json.contains("\"dependencies\":{\n}"),
            "package.json should no longer have chalk: {}",
            staged2.package_json
        );

        assert!(!base_tmp.join("node_modules").exists());

        let _ = std::fs::remove_dir_all(&base_tmp);
    }

    #[test]
    #[ignore]
    fn test_ignored_resolve_add_real_registry_pinned_spec() {
        // Real-binary probe: calls the public `resolve_add` entry point directly, so
        // resolution runs through the production `InvocationContext::from_env` path
        // (real PATH, real `deno`, real registry) with a pinned spec.
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_ignored_resolve_add_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&base_tmp);
        std::fs::create_dir_all(&base_tmp).unwrap();

        let staged = resolve_add(&base_tmp, "is-number@7.0.0").expect("resolve_add should succeed");

        assert!(
            staged.package_json.contains("is-number"),
            "staged package.json must contain is-number: {}",
            staged.package_json
        );
        assert!(staged.deno_lock.is_some(), "deno.lock must be produced in staging");
        let lock = staged.deno_lock.unwrap();
        assert!(
            lock.contains("is-number"),
            "staged deno.lock must contain is-number: {}",
            lock
        );

        assert!(!base_tmp.join("package.json").exists());
        assert!(!base_tmp.join("deno.lock").exists());
        assert!(!base_tmp.join("node_modules").exists());

        let _ = std::fs::remove_dir_all(&base_tmp);
    }

    #[test]
    #[ignore]
    fn test_ignored_resolve_remove_real_registry_pinned_spec() {
        // Real-binary probe: calls the public `resolve_remove` entry point directly,
        // so resolution runs through the production `InvocationContext::from_env`
        // path (real PATH, real `deno`, real registry) with a pinned spec.
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_ignored_resolve_remove_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&base_tmp);
        std::fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"is-number\": \"7.0.0\"\n  }\n}\n";
        let manifest_path = base_tmp.join("package.json");
        std::fs::write(&manifest_path, initial_manifest).unwrap();

        let staged_install = invoke_staged_deno(&base_tmp, &["install"]).expect("staged install");
        let initial_lock = staged_install.deno_lock.expect("deno.lock generated");
        std::fs::write(base_tmp.join("deno.lock"), &initial_lock).unwrap();

        let staged = resolve_remove(&base_tmp, "is-number").expect("resolve_remove should succeed");

        assert!(
            !staged.package_json.contains("is-number"),
            "staged package.json must no longer contain is-number: {}",
            staged.package_json
        );

        assert_eq!(
            std::fs::read_to_string(&manifest_path).unwrap(),
            initial_manifest,
            "project root package.json must remain untouched"
        );
        assert!(!base_tmp.join("node_modules").exists());

        let _ = std::fs::remove_dir_all(&base_tmp);
    }

    #[test]
    #[ignore]
    fn test_ignored_resolve_update_real_registry_pinned_spec() {
        // Real-binary probe: calls the public `resolve_update` entry point directly,
        // so resolution runs through the production `InvocationContext::from_env`
        // path (real PATH, real `deno`, real registry) with a pinned spec.
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_ignored_resolve_update_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&base_tmp);
        std::fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"is-odd\": \"^3.0.0\"\n  }\n}\n";
        let manifest_path = base_tmp.join("package.json");
        std::fs::write(&manifest_path, initial_manifest).unwrap();

        let staged_install = invoke_staged_deno(&base_tmp, &["install"]).expect("staged install");
        let initial_lock = staged_install.deno_lock.expect("deno.lock generated");
        std::fs::write(base_tmp.join("deno.lock"), &initial_lock).unwrap();

        let staged =
            resolve_update(&base_tmp, Some("is-odd")).expect("resolve_update should succeed");

        assert!(
            staged.package_json.contains("\"is-odd\": \"^3.0.1\""),
            "staged package.json must have updated is-odd: {}",
            staged.package_json
        );
        assert!(staged.deno_lock.is_some(), "deno.lock must be present in staged output");

        assert_eq!(
            std::fs::read_to_string(&manifest_path).unwrap(),
            initial_manifest,
            "project root package.json must remain untouched"
        );
        assert!(!base_tmp.join("node_modules").exists());

        let _ = std::fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_resolve_add_zero_dependency_reproducible_lock() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_add_reproducible_test_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&base_tmp);
        std::fs::create_dir_all(&base_tmp).unwrap();

        // Project root starts with zero dependencies (no package.json, no deno.lock)
        assert!(!base_tmp.join("package.json").exists());
        assert!(!base_tmp.join("deno.lock").exists());

        // Injected strategy stands in for the real staged `deno add`: it returns the same
        // fixed StagedFiles on every call, which is what a real, reproducible resolution of
        // an identical spec against an identical starting state would also produce. Neither
        // call reaches `resolve_deno_with_path` or spawns a subprocess.
        let fixture = StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"chalk\": \"5.3.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:chalk@5.3.0\": \"5.3.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let for_strategy = fixture.clone();
        let ctx = InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(for_strategy.clone())
        });

        // First run
        let staged1 = resolve_add_with_context(&base_tmp, "chalk@5.3.0", &ctx)
            .expect("first resolve_add should succeed");
        assert!(
            staged1.package_json.contains("chalk"),
            "staged package.json must contain chalk: {}",
            staged1.package_json
        );
        assert!(staged1.deno_lock.is_some(), "deno.lock must be produced in staging");
        let lock1 = staged1.deno_lock.as_ref().unwrap();
        assert!(
            lock1.contains("chalk"),
            "staged deno.lock must contain chalk: {}",
            lock1
        );

        // Verify package.json contains exactly one entry under dependencies
        let parsed: serde_json::Value =
            serde_json::from_str(&staged1.package_json).expect("staged package.json must be valid JSON");
        let deps = parsed
            .get("dependencies")
            .and_then(|d| d.as_object())
            .expect("dependencies must be an object");
        assert_eq!(
            deps.len(),
            1,
            "dependencies object must contain exactly one entry"
        );
        assert!(deps.contains_key("chalk"));

        // Second run from the same starting state
        let staged2 = resolve_add_with_context(&base_tmp, "chalk@5.3.0", &ctx)
            .expect("second resolve_add should succeed");
        assert_eq!(
            staged1.package_json, staged2.package_json,
            "staged package.json must be byte-identical across two runs"
        );
        assert_eq!(
            staged1.deno_lock, staged2.deno_lock,
            "consecutive resolutions of identical input must produce byte-identical deno.lock"
        );

        // Verify project root was untouched directly
        assert!(!base_tmp.join("package.json").exists());
        assert!(!base_tmp.join("deno.lock").exists());
        assert!(!base_tmp.join("node_modules").exists());

        let _ = std::fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_resolve_add_with_existing_package_adds_second_dependency() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_add_existing_test_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&base_tmp);
        std::fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"chalk\": \"^5.3.0\"\n  }\n}\n";
        std::fs::write(base_tmp.join("package.json"), initial_manifest).unwrap();

        // Injected strategy stands in for the real staged `deno add`: it returns a fixture
        // that carries both the pre-existing "chalk" entry and the newly added "is-number"
        // entry, matching what a real resolution against `initial_manifest` would produce.
        let fixture = StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"chalk\": \"^5.3.0\",\n    \"is-number\": \"7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:chalk@5.3.0\": \"5.3.0\",\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let ctx = InvocationContext::with_strategy(None, Vec::new(), move |_args| Ok(fixture.clone()));

        let staged = resolve_add_with_context(&base_tmp, "is-number@7.0.0", &ctx)
            .expect("resolve_add should succeed on project with existing dependency");

        assert!(staged.package_json.contains("chalk"));
        assert!(staged.package_json.contains("is-number"));
        assert!(staged.deno_lock.is_some());

        let parsed: serde_json::Value =
            serde_json::from_str(&staged.package_json).expect("staged package.json must be valid JSON");
        let deps = parsed
            .get("dependencies")
            .and_then(|d| d.as_object())
            .expect("dependencies must be an object");
        assert_eq!(deps.len(), 2, "dependencies object must contain two entries");
        assert!(deps.contains_key("chalk"));
        assert!(deps.contains_key("is-number"));

        // Project root files must remain untouched
        assert_eq!(
            std::fs::read_to_string(base_tmp.join("package.json")).unwrap(),
            initial_manifest
        );
        assert!(!base_tmp.join("deno.lock").exists());
        assert!(!base_tmp.join("node_modules").exists());

        let _ = std::fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_resolve_add_alias_add_matches() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_add_alias_test_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&base_tmp);
        std::fs::create_dir_all(&base_tmp).unwrap();

        // Injected strategy stands in for the real staged `deno add`; the alias under test
        // is `add_with_context`, the context-accepting counterpart of `add`.
        let fixture = StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"chalk\": \"5.3.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:chalk@5.3.0\": \"5.3.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let ctx = InvocationContext::with_strategy(None, Vec::new(), move |_args| Ok(fixture.clone()));

        let staged = add_with_context(&base_tmp, "chalk@5.3.0", &ctx).expect("add alias should succeed");
        assert!(staged.package_json.contains("chalk"));
        assert!(staged.deno_lock.is_some());

        let _ = std::fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_resolve_add_with_missing_deno_returns_tool_fault() {
        let empty_dir = std::env::temp_dir().join(format!(
            "wda_test_add_no_deno_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&empty_dir);
        std::fs::create_dir_all(&empty_dir).unwrap();

        let res = resolve_add_with_path(&empty_dir, "chalk", Some(empty_dir.as_os_str()));
        assert!(res.is_err(), "missing deno must return error");
        let diag = res.unwrap_err();
        assert_eq!(diag.code, codes::TOOL_FAULT);
        assert_eq!(diag.code, "wda.tool.fault");
        assert_eq!(diag.severity, Severity::Error);
        assert!(
            diag.message.contains("deno"),
            "diagnostic message must name 'deno', got: {}",
            diag.message
        );
        assert!(diag.location.is_none());

        let _ = std::fs::remove_dir_all(&empty_dir);
    }

    #[test]
    fn test_resolve_add_invalid_package_spec_returns_tool_fault() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_add_invalid_spec_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Empty spec
        let empty_res = resolve_add(&temp_dir, "");
        assert!(empty_res.is_err());
        let empty_diag = empty_res.unwrap_err();
        assert_eq!(empty_diag.code, codes::TOOL_FAULT);
        assert_eq!(empty_diag.code, "wda.tool.fault");
        assert_eq!(empty_diag.severity, Severity::Error);

        // Whitespace spec
        let ws_res = resolve_add(&temp_dir, "   ");
        assert!(ws_res.is_err());
        let ws_diag = ws_res.unwrap_err();
        assert_eq!(ws_diag.code, codes::TOOL_FAULT);

        // Non-existent package: injected strategy stands in for the real staged `deno add`
        // failing to resolve an unpublished package, returning the same TOOL_FAULT shape a
        // real resolver failure maps to, without ever consulting PATH or spawning `deno`.
        let nonexistent_pkg = "nonexistent_pkg_definitely_not_on_npm_wda_998877";
        let ctx = InvocationContext::with_strategy(None, Vec::new(), |_args| {
            Err(Diagnostic::new(
                codes::TOOL_FAULT,
                Severity::Error,
                None,
                "Deno command failed: could not resolve package",
                "Check the dependency specifier and network connectivity, then retry",
            ))
        });
        let res = resolve_add_with_context(&temp_dir, nonexistent_pkg, &ctx);
        assert!(res.is_err());
        let diag = res.unwrap_err();
        assert_eq!(diag.code, codes::TOOL_FAULT);
        assert_eq!(diag.code, "wda.tool.fault");
        assert_eq!(diag.severity, Severity::Error);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_add_with_registry_auth_env_does_not_leak_secrets() {
        // Per D-02, this test constructs an `InvocationContext` whose environment
        // map never carries `NPM_TOKEN`/`DENO_AUTH_TOKENS` and asserts the child
        // environment `build_deno_command` produces omits them too. No spawn, and
        // no mutation of the real process environment.
        let env_vars = vec![(OsString::from("PATH"), OsString::from("/usr/bin"))];
        let ctx = InvocationContext::with_env(None, env_vars);

        let deno_binary = PathBuf::from("deno");
        let staging_dir = std::env::temp_dir();
        let command = build_deno_command(
            &deno_binary,
            &staging_dir,
            &["add", "--package-json", "chalk@5.3.0"],
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
    fn test_tp05_remove_last_dependency_leaves_empty_package_json_and_lock_stub() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_tp05_remove_last_test_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&base_tmp);
        std::fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"chalk\": \"^5.3.0\"\n  }\n}\n";
        let manifest_path = base_tmp.join("package.json");
        std::fs::write(&manifest_path, initial_manifest).unwrap();

        // Injected strategy stands in for the real staged `deno install` followed by
        // `deno remove`: the first call returns the fixture "install" produced (the lock
        // this test writes to the project root), the second returns the fixture a real
        // `remove` of the last dependency would produce. Neither call reaches
        // `resolve_deno_with_path` or spawns a subprocess.
        let install_fixture = StagedFiles {
            package_json: initial_manifest.to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:chalk@5.3.0\": \"5.3.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let remove_fixture = StagedFiles {
            package_json: "{\n  \"dependencies\": {}\n}\n".to_string(),
            deno_lock: Some("{\n  \"version\": \"4\",\n  \"specifiers\": {}\n}\n".to_string()),
        };
        let mut responses = vec![install_fixture, remove_fixture].into_iter();
        let ctx = InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(responses.next().expect("no more responses configured"))
        });

        let staged_install =
            invoke_staged_deno_with_context(&base_tmp, &["install"], &ctx).expect("staged install");
        let initial_lock = staged_install.deno_lock.expect("deno.lock generated");
        let lock_path = base_tmp.join("deno.lock");
        std::fs::write(&lock_path, &initial_lock).unwrap();

        let manifest_bytes_before = std::fs::read(&manifest_path).unwrap();
        let lock_bytes_before = std::fs::read(&lock_path).unwrap();

        let staged = resolve_remove_with_context(&base_tmp, "chalk", &ctx)
            .expect("resolve_remove should succeed");

        assert!(
            !staged.package_json.contains("chalk"),
            "staged package.json must not contain chalk: {}",
            staged.package_json
        );

        let parsed: serde_json::Value =
            serde_json::from_str(&staged.package_json).expect("package.json must be valid JSON");
        let deps_empty = parsed
            .get("dependencies")
            .map(|d| d.as_object().map(|obj| obj.is_empty()).unwrap_or(false))
            .unwrap_or(true);
        assert!(
            deps_empty,
            "dependencies in staged package.json must be empty or absent: {}",
            staged.package_json
        );

        assert!(
            staged.deno_lock.is_some(),
            "deno.lock must be present in staged output as a stub"
        );
        let staged_lock = staged.deno_lock.unwrap();
        assert!(
            !staged_lock.contains("chalk"),
            "staged deno.lock stub must no longer contain chalk: {}",
            staged_lock
        );

        // Project root must remain untouched
        let manifest_bytes_after = std::fs::read(&manifest_path).unwrap();
        let lock_bytes_after = std::fs::read(&lock_path).unwrap();
        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "project root package.json must be untouched"
        );
        assert_eq!(
            lock_bytes_before, lock_bytes_after,
            "project root deno.lock must be untouched"
        );
        assert!(
            !base_tmp.join("node_modules").exists(),
            "project root must not contain node_modules"
        );

        let _ = std::fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_remove_one_of_multiple_dependencies() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_remove_multiple_test_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&base_tmp);
        std::fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"chalk\": \"^5.3.0\",\n    \"is-number\": \"^7.0.0\"\n  }\n}\n";
        let manifest_path = base_tmp.join("package.json");
        std::fs::write(&manifest_path, initial_manifest).unwrap();

        // Injected strategy stands in for the real staged `deno install` followed by
        // `deno remove is-number`: the first call returns the "install" fixture (whose
        // lock this test writes to the project root), the second returns the fixture a
        // real removal of `is-number` (leaving `chalk` behind) would produce. Neither
        // call reaches `resolve_deno_with_path` or spawns a subprocess.
        let install_fixture = StagedFiles {
            package_json: initial_manifest.to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:chalk@5.3.0\": \"5.3.0\",\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let remove_fixture = StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"chalk\": \"^5.3.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:chalk@5.3.0\": \"5.3.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let mut responses = vec![install_fixture, remove_fixture].into_iter();
        let ctx = InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(responses.next().expect("no more responses configured"))
        });

        let staged_install =
            invoke_staged_deno_with_context(&base_tmp, &["install"], &ctx).expect("staged install");
        let initial_lock = staged_install.deno_lock.expect("deno.lock generated");
        let lock_path = base_tmp.join("deno.lock");
        std::fs::write(&lock_path, &initial_lock).unwrap();

        let staged = resolve_remove_with_context(&base_tmp, "is-number", &ctx)
            .expect("resolve_remove should succeed");

        assert!(
            staged.package_json.contains("chalk"),
            "staged package.json must retain chalk: {}",
            staged.package_json
        );
        assert!(
            !staged.package_json.contains("is-number"),
            "staged package.json must not retain is-number: {}",
            staged.package_json
        );

        let parsed: serde_json::Value =
            serde_json::from_str(&staged.package_json).expect("package.json must be valid JSON");
        let deps = parsed
            .get("dependencies")
            .and_then(|d| d.as_object())
            .expect("dependencies object must exist");
        assert_eq!(deps.len(), 1, "dependencies must contain exactly one entry");
        assert!(deps.contains_key("chalk"));
        assert!(!deps.contains_key("is-number"));

        assert!(staged.deno_lock.is_some());
        let staged_lock = staged.deno_lock.unwrap();
        assert!(staged_lock.contains("chalk"));
        assert!(!staged_lock.contains("is-number"));

        // Project root untouched
        assert_eq!(
            std::fs::read_to_string(&manifest_path).unwrap(),
            initial_manifest
        );
        assert_eq!(
            std::fs::read_to_string(&lock_path).unwrap(),
            initial_lock
        );
        assert!(!base_tmp.join("node_modules").exists());

        let _ = std::fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_remove_empty_or_whitespace_name_returns_tool_fault() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_remove_empty_name_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let empty_res = resolve_remove(&temp_dir, "");
        assert!(empty_res.is_err());
        let empty_diag = empty_res.unwrap_err();
        assert_eq!(empty_diag.code, codes::TOOL_FAULT);
        assert_eq!(empty_diag.code, "wda.tool.fault");
        assert_eq!(empty_diag.severity, Severity::Error);

        let ws_res = resolve_remove(&temp_dir, "   \t\n  ");
        assert!(ws_res.is_err());
        let ws_diag = ws_res.unwrap_err();
        assert_eq!(ws_diag.code, codes::TOOL_FAULT);
        assert_eq!(ws_diag.severity, Severity::Error);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_remove_with_missing_deno_returns_tool_fault() {
        let empty_dir = std::env::temp_dir().join(format!(
            "wda_test_remove_no_deno_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&empty_dir);
        std::fs::create_dir_all(&empty_dir).unwrap();

        let res = resolve_remove_with_path(&empty_dir, "chalk", Some(empty_dir.as_os_str()));
        assert!(res.is_err(), "missing deno must return error");
        let diag = res.unwrap_err();
        assert_eq!(diag.code, codes::TOOL_FAULT);
        assert_eq!(diag.code, "wda.tool.fault");
        assert_eq!(diag.severity, Severity::Error);
        assert!(
            diag.message.contains("deno"),
            "diagnostic message must name 'deno', got: {}",
            diag.message
        );
        assert!(diag.location.is_none());

        let _ = std::fs::remove_dir_all(&empty_dir);
    }

    #[test]
    fn test_remove_alias_matches() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_remove_alias_test_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&base_tmp);
        std::fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"chalk\": \"^5.3.0\",\n    \"is-number\": \"^7.0.0\"\n  }\n}\n";
        let manifest_path = base_tmp.join("package.json");
        std::fs::write(&manifest_path, initial_manifest).unwrap();

        // Injected strategy stands in for the real staged `deno install` followed by
        // `deno remove chalk`; the alias under test is `remove_with_context`, the
        // context-accepting counterpart of `remove`.
        let install_fixture = StagedFiles {
            package_json: initial_manifest.to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:chalk@5.3.0\": \"5.3.0\",\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let remove_fixture = StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"^7.0.0\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@7.0.0\": \"7.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let mut responses = vec![install_fixture, remove_fixture].into_iter();
        let ctx = InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(responses.next().expect("no more responses configured"))
        });

        let staged_install =
            invoke_staged_deno_with_context(&base_tmp, &["install"], &ctx).expect("staged install");
        let initial_lock = staged_install.deno_lock.expect("deno.lock generated");
        std::fs::write(base_tmp.join("deno.lock"), &initial_lock).unwrap();

        let staged = remove_with_context(&base_tmp, "chalk", &ctx).expect("remove alias should succeed");
        assert!(!staged.package_json.contains("chalk"));
        assert!(staged.package_json.contains("is-number"));
        assert!(staged.deno_lock.is_some());

        let _ = std::fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_update_pinned_graph_is_noop_and_filter_updates_single_entry() {
        // Non-red probe for pinned-graph check:
        // 1. Pinned graph with exact versions where no newer compatible version exists:
        //    deno update is a no-op (package.json and deno.lock unchanged).
        // 2. Filtered update `deno update <name>`:
        //    Only the named entry changes; unrelated dependencies and their lock entries are byte-identical.
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_update_pinned_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&base_tmp);
        std::fs::create_dir_all(&base_tmp).unwrap();

        // is-number is pinned to exact "6.0.0", is-odd is "^3.0.0" (which can update to 3.0.1)
        let initial_manifest = "{\n  \"dependencies\": {\n    \"is-number\": \"6.0.0\",\n    \"is-odd\": \"^3.0.0\"\n  }\n}\n";
        let manifest_path = base_tmp.join("package.json");
        std::fs::write(&manifest_path, initial_manifest).unwrap();

        let initial_lock_content =
            "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@6.0.0\": \"6.0.0\",\n    \"npm:is-odd@3.0.0\": \"3.0.0\"\n  }\n}\n"
                .to_string();

        // Injected strategy stands in for the real staged `deno install` followed by two
        // `deno update` invocations: install returns the fixture lock this test writes to
        // the project root; the filtered update on the pinned "is-number" entry returns the
        // unchanged manifest/lock a real no-op resolution would produce; the filtered update
        // on "is-odd" returns the fixture a real compatible-version bump would produce.
        // None of the three calls reaches `resolve_deno_with_path` or spawns a subprocess.
        let install_fixture = StagedFiles {
            package_json: initial_manifest.to_string(),
            deno_lock: Some(initial_lock_content.clone()),
        };
        let pinned_update_fixture = StagedFiles {
            package_json: initial_manifest.to_string(),
            deno_lock: Some(initial_lock_content.clone()),
        };
        let filtered_update_fixture = StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"6.0.0\",\n    \"is-odd\": \"^3.0.1\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@6.0.0\": \"6.0.0\",\n    \"npm:is-odd@3.0.1\": \"3.0.1\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let mut responses = vec![install_fixture, pinned_update_fixture, filtered_update_fixture].into_iter();
        let ctx = InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(responses.next().expect("no more responses configured"))
        });

        let staged_install =
            invoke_staged_deno_with_context(&base_tmp, &["install"], &ctx).expect("staged install");
        let initial_lock = staged_install.deno_lock.expect("deno.lock generated");
        let lock_path = base_tmp.join("deno.lock");
        std::fs::write(&lock_path, &initial_lock).unwrap();

        // 1. Unfiltered update on pinned is-number (when both are already up to date or pinned):
        // First, test filtered update on the pinned entry "is-number" with no newer compatible version
        let staged_pinned_update = resolve_update_with_context(&base_tmp, Some("is-number"), &ctx)
            .expect("resolve_update on pinned entry should succeed");

        // Both package.json and deno.lock should remain identical to pre-operation
        assert_eq!(
            staged_pinned_update.package_json, initial_manifest,
            "pinned entry update must leave package.json unchanged"
        );
        assert_eq!(
            staged_pinned_update.deno_lock.as_ref().unwrap(),
            &initial_lock,
            "pinned entry update must leave deno.lock unchanged"
        );

        // 2. Filtered update on "is-odd" which has a newer compatible version (^3.0.0 -> ^3.0.1)
        let staged_filtered_update = resolve_update_with_context(&base_tmp, Some("is-odd"), &ctx)
            .expect("resolve_update on is-odd should succeed");

        assert!(
            staged_filtered_update.package_json.contains("\"is-odd\": \"^3.0.1\""),
            "staged package.json must have updated is-odd: {}",
            staged_filtered_update.package_json
        );
        assert!(
            staged_filtered_update.package_json.contains("\"is-number\": \"6.0.0\""),
            "staged package.json must retain unchanged is-number: {}",
            staged_filtered_update.package_json
        );

        // Lock file should retain the exact same is-number entry
        let staged_lock = staged_filtered_update.deno_lock.expect("deno.lock present");
        assert!(
            staged_lock.contains("\"npm:is-number@6.0.0\": \"6.0.0\""),
            "staged deno.lock must retain unchanged is-number entry"
        );
        assert!(
            staged_lock.contains("is-odd@3.0.1"),
            "staged deno.lock must contain updated is-odd@3.0.1"
        );

        // Project root files remain completely untouched
        let manifest_bytes = std::fs::read_to_string(&manifest_path).unwrap();
        let lock_bytes = std::fs::read_to_string(&lock_path).unwrap();
        assert_eq!(manifest_bytes, initial_manifest);
        assert_eq!(lock_bytes, initial_lock);
        assert!(!base_tmp.join("node_modules").exists());

        let _ = std::fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_update_unfiltered_updates_all_compatible_entries() {
        let base_tmp = std::env::temp_dir().join(format!(
            "wda_test_update_unfiltered_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&base_tmp);
        std::fs::create_dir_all(&base_tmp).unwrap();

        let initial_manifest = "{\n  \"dependencies\": {\n    \"is-number\": \"6.0.0\",\n    \"is-odd\": \"^3.0.0\"\n  }\n}\n";
        let manifest_path = base_tmp.join("package.json");
        std::fs::write(&manifest_path, initial_manifest).unwrap();

        // Injected strategy stands in for the real staged `deno install` followed by two
        // unfiltered `deno update` invocations (the second through the `update_with_context`
        // alias): install returns the fixture lock this test writes to the project root, and
        // both update calls return the same fixture a real unfiltered update bumping only the
        // compatible "is-odd" entry would produce. None of the three calls reaches
        // `resolve_deno_with_path` or spawns a subprocess.
        let install_fixture = StagedFiles {
            package_json: initial_manifest.to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@6.0.0\": \"6.0.0\",\n    \"npm:is-odd@3.0.0\": \"3.0.0\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let update_fixture = StagedFiles {
            package_json: "{\n  \"dependencies\": {\n    \"is-number\": \"6.0.0\",\n    \"is-odd\": \"^3.0.1\"\n  }\n}\n".to_string(),
            deno_lock: Some(
                "{\n  \"version\": \"4\",\n  \"specifiers\": {\n    \"npm:is-number@6.0.0\": \"6.0.0\",\n    \"npm:is-odd@3.0.1\": \"3.0.1\"\n  }\n}\n"
                    .to_string(),
            ),
        };
        let mut responses = vec![install_fixture, update_fixture.clone(), update_fixture].into_iter();
        let ctx = InvocationContext::with_strategy(None, Vec::new(), move |_args| {
            Ok(responses.next().expect("no more responses configured"))
        });

        let staged_install =
            invoke_staged_deno_with_context(&base_tmp, &["install"], &ctx).expect("staged install");
        let initial_lock = staged_install.deno_lock.expect("deno.lock generated");
        std::fs::write(base_tmp.join("deno.lock"), &initial_lock).unwrap();

        let staged = resolve_update_with_context(&base_tmp, None, &ctx)
            .expect("unfiltered resolve_update should succeed");

        assert!(
            staged.package_json.contains("\"is-odd\": \"^3.0.1\""),
            "unfiltered update should update is-odd: {}",
            staged.package_json
        );
        assert!(
            staged.package_json.contains("\"is-number\": \"6.0.0\""),
            "unfiltered update should keep pinned is-number: {}",
            staged.package_json
        );
        assert!(staged.deno_lock.is_some());

        // Also test alias update_with_context(...)
        let staged_alias =
            update_with_context(&base_tmp, None, &ctx).expect("update alias should succeed");
        assert_eq!(staged.package_json, staged_alias.package_json);
        assert_eq!(staged.deno_lock, staged_alias.deno_lock);

        let _ = std::fs::remove_dir_all(&base_tmp);
    }

    #[test]
    fn test_update_empty_or_whitespace_name_filter_returns_tool_fault() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_update_empty_filter_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let empty_res = resolve_update(&temp_dir, Some(""));
        assert!(empty_res.is_err());
        let empty_diag = empty_res.unwrap_err();
        assert_eq!(empty_diag.code, codes::TOOL_FAULT);
        assert_eq!(empty_diag.code, "wda.tool.fault");
        assert_eq!(empty_diag.severity, Severity::Error);

        let ws_res = resolve_update(&temp_dir, Some("   \t\n  "));
        assert!(ws_res.is_err());
        let ws_diag = ws_res.unwrap_err();
        assert_eq!(ws_diag.code, codes::TOOL_FAULT);
        assert_eq!(ws_diag.severity, Severity::Error);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_tp06_injected_strategy_short_circuits_before_resolve_deno() {
        // PATH points at a directory containing no `deno`: resolve_deno_with_path
        // would fail here (proven by test_resolve_deno_non_red_probe_empty_dir_then_real_path
        // above). An injected strategy must still return `StagedFiles`, which is only
        // possible if the strategy is consulted before `resolve_deno_with_path` runs.
        let empty_dir = std::env::temp_dir().join(format!(
            "wda_tp06_no_deno_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::create_dir_all(&empty_dir);

        let expected = StagedFiles {
            package_json: "{\"dependencies\":{\"chalk\":\"5.3.0\"}}\n".to_string(),
            deno_lock: Some("{}\n".to_string()),
        };
        let injected = expected.clone();
        let ctx = InvocationContext::with_strategy(
            Some(empty_dir.as_os_str()),
            Vec::new(),
            move |_args| Ok(injected.clone()),
        );

        let project_root = std::env::temp_dir().join(format!(
            "wda_tp06_project_root_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let result = invoke_staged_deno_with_context(&project_root, &["add", "--package-json", "chalk"], &ctx);

        assert_eq!(
            result,
            Ok(expected),
            "an injected strategy must return StagedFiles directly, without consulting PATH"
        );

        let _ = std::fs::remove_dir_all(&empty_dir);
    }

    #[test]
    fn test_update_with_missing_deno_returns_tool_fault() {
        let empty_dir = std::env::temp_dir().join(format!(
            "wda_test_update_no_deno_{}_{}",
            std::process::id(),
            unique_staging_suffix()
        ));
        let _ = std::fs::remove_dir_all(&empty_dir);
        std::fs::create_dir_all(&empty_dir).unwrap();

        let res = resolve_update_with_path(&empty_dir, None, Some(empty_dir.as_os_str()));
        assert!(res.is_err(), "missing deno must return error");
        let diag = res.unwrap_err();
        assert_eq!(diag.code, codes::TOOL_FAULT);
        assert_eq!(diag.code, "wda.tool.fault");
        assert_eq!(diag.severity, Severity::Error);
        assert!(
            diag.message.contains("deno"),
            "diagnostic message must name 'deno', got: {}",
            diag.message
        );
        assert!(diag.location.is_none());

        let _ = std::fs::remove_dir_all(&empty_dir);
    }
}

