//! Project initialization (`wda init`) implementation.
//!
//! Provides the entry seam holding the empty-directory gate, ordered write plan execution,
//! intermediate directory tracking, reverse cleanup on failure, and residue reporting.

pub(crate) mod documents;
pub(crate) mod manifest;
pub(crate) mod resolve;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::codes;
use crate::diagnostics::{Diagnostic, Location, Severity, ToolFault, ValidationReport};

/// An entry in the ordered write plan representing a file to create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WritePlanEntry {
    pub(crate) relative_path: PathBuf,
    pub(crate) content: String,
}

impl WritePlanEntry {
    pub(crate) fn new(relative_path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Self {
            relative_path: relative_path.into(),
            content: content.into(),
        }
    }
}

/// An ordered sequence of file entries to write into a target project root.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WritePlan {
    pub(crate) entries: Vec<WritePlanEntry>,
}

impl WritePlan {
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub(crate) fn add(&mut self, relative_path: impl Into<PathBuf>, content: impl Into<String>) {
        self.entries
            .push(WritePlanEntry::new(relative_path, content));
    }
}

/// Counts the number of filesystem entries in `dir`.
#[cfg(test)]
pub(crate) fn count_entries(dir: &Path) -> Result<usize, ToolFault> {
    let entries = fs::read_dir(dir).map_err(|e| {
        ToolFault::new(
            format!("Failed to read directory: {e}"),
            Some(dir.to_path_buf()),
        )
    })?;

    let mut count = 0usize;
    for entry in entries {
        let _ = entry.map_err(|e| {
            ToolFault::new(
                format!("Failed to read directory entry: {e}"),
                Some(dir.to_path_buf()),
            )
        })?;
        count += 1;
    }
    Ok(count)
}

/// Default real writer that creates intermediate directories as needed and writes the file,
/// recording every newly created filesystem entry (intermediate directories and file)
/// in `created` in order of creation.
pub(crate) fn real_writer(
    root: &Path,
    entry: &WritePlanEntry,
    created: &mut Vec<PathBuf>,
) -> Result<(), ToolFault> {
    let rel_path = &entry.relative_path;
    let full_path = root.join(rel_path);

    if let Some(parent) = rel_path.parent() {
        let mut current_rel = PathBuf::new();
        for component in parent.components() {
            current_rel.push(component.as_os_str());
            let dir_full = root.join(&current_rel);
            if !dir_full.exists() {
                fs::create_dir(&dir_full).map_err(|e| {
                    ToolFault::new(
                        format!("Failed to create directory {}: {e}", current_rel.display()),
                        Some(root.to_path_buf()),
                    )
                })?;
                created.push(current_rel.clone());
            }
        }
    }

    fs::write(&full_path, &entry.content).map_err(|e| {
        ToolFault::new(
            format!("Failed to write file {}: {e}", rel_path.display()),
            Some(root.to_path_buf()),
        )
    })?;
    created.push(rel_path.clone());

    Ok(())
}

/// Default real remover that removes a single file, symlink, or empty directory at `rel_path` under `root`.
pub(crate) fn real_remover(root: &Path, rel_path: &Path) -> Result<(), ToolFault> {
    let full_path = root.join(rel_path);
    // `symlink_metadata` is `lstat`: it describes the entry itself and never the
    // target a symlink points at. `Path::is_dir()` follows symlinks, so it would
    // route a symlink-to-directory into the `remove_dir` branch, where `rmdir`
    // rejects it and the entry is left behind. Callers classify entries with
    // `DirEntry::file_type()`, which is equally symlink-aware, so this keeps both
    // sides agreeing on what counts as a directory.
    let metadata = match fs::symlink_metadata(&full_path) {
        Ok(metadata) => metadata,
        // `NotADirectory` means an ancestor component is a plain file, so `rel_path`
        // cannot exist as such either way; treat it the same as `NotFound`.
        Err(e)
            if e.kind() == io::ErrorKind::NotFound
                || e.kind() == io::ErrorKind::NotADirectory =>
        {
            return Ok(())
        }
        Err(e) => {
            return Err(ToolFault::new(
                format!("Failed to inspect {}: {e}", rel_path.display()),
                Some(root.to_path_buf()),
            ));
        }
    };

    if metadata.file_type().is_dir() {
        fs::remove_dir(&full_path).map_err(|e| {
            ToolFault::new(
                format!("Failed to remove directory {}: {e}", rel_path.display()),
                Some(root.to_path_buf()),
            )
        })?;
    } else {
        // A symlink is unlinked here, never followed. Unix unlinks every symlink
        // with `remove_file`; Windows requires `remove_dir` for a directory
        // symlink, so fall back to it while still reporting the first error when
        // both calls refuse the entry.
        fs::remove_file(&full_path)
            .or_else(|first| fs::remove_dir(&full_path).map_err(|_| first))
            .map_err(|e| {
                ToolFault::new(
                    format!("Failed to remove file {}: {e}", rel_path.display()),
                    Some(root.to_path_buf()),
                )
            })?;
    }
    Ok(())
}

/// Rolls back recorded entries in reverse order using `remover`.
/// If any removal fails, returns a `ToolFault` naming all residual project-relative paths.
pub(crate) fn rollback<R>(
    root: &Path,
    created: &[PathBuf],
    mut remover: R,
) -> Result<(), ToolFault>
where
    R: FnMut(&Path, &Path) -> Result<(), ToolFault>,
{
    let mut failed_removals = Vec::new();
    for rel_path in created.iter().rev() {
        if rel_path == Path::new(".git") || rel_path == Path::new(".git/") {
            let git_dir = root.join(rel_path);
            let mut git_remover = |dir: &Path, rel: &Path| -> Result<(), ToolFault> {
                let full_path = if rel.as_os_str().is_empty() {
                    dir.to_path_buf()
                } else {
                    dir.join(rel)
                };
                if let Ok(metadata) = fs::symlink_metadata(&full_path) {
                    let mut perms = metadata.permissions();
                    if perms.readonly() {
                        perms.set_readonly(false);
                        let _ = fs::set_permissions(&full_path, perms);
                    }
                }
                let project_rel = if rel.as_os_str().is_empty() {
                    rel_path.clone()
                } else {
                    rel_path.join(rel)
                };
                remover(root, &project_rel)
            };

            if let Err(_) = crate::build::remove_tree(
                root,
                &git_dir,
                rel_path,
                "Failed to clean up git repository",
                &mut git_remover,
            ) {
                failed_removals.push(rel_path.clone());
            }
        } else if let Err(_) = remover(root, rel_path) {
            failed_removals.push(rel_path.clone());
        }
    }

    if !failed_removals.is_empty() {
        let normalized_residuals: Vec<String> = failed_removals
            .iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();
        return Err(ToolFault::new(
            format!(
                "Failed to clean up project directory after error; residual paths remain: {}",
                normalized_residuals.join(", ")
            ),
            Some(root.to_path_buf()),
        ));
    }

    Ok(())
}

/// Executes an ordered write plan against `root` using `writer` and `remover`.
///
/// On success, returns every project-relative path created, in creation order (the
/// same shape `real_writer` tracks: intermediate directories before the file they
/// contain), so a caller performing further work after the plan can extend the same
/// list for its own rollback.
///
/// On failure during any write step, initiates reverse cleanup of all entries created so far.
/// If cleanup fails, returns a `ToolFault` naming residual project-relative paths.
#[allow(dead_code)]
pub(crate) fn execute_write_plan<W, R>(
    root: &Path,
    plan: &WritePlan,
    writer: W,
    remover: R,
) -> Result<Vec<PathBuf>, ToolFault>
where
    W: FnMut(&Path, &WritePlanEntry, &mut Vec<PathBuf>) -> Result<(), ToolFault>,
    R: FnMut(&Path, &Path) -> Result<(), ToolFault>,
{
    let mut created = Vec::new();
    execute_write_plan_with_created(root, plan, &mut created, writer, remover)?;
    Ok(created)
}

/// Executes an ordered write plan against `root`, recording created entries into `created`.
///
/// If any write step fails, initiates reverse cleanup of all entries in `created`
/// (including any pre-existing entries recorded before the write plan, such as `.git`).
pub(crate) fn execute_write_plan_with_created<W, R>(
    root: &Path,
    plan: &WritePlan,
    created: &mut Vec<PathBuf>,
    mut writer: W,
    remover: R,
) -> Result<(), ToolFault>
where
    W: FnMut(&Path, &WritePlanEntry, &mut Vec<PathBuf>) -> Result<(), ToolFault>,
    R: FnMut(&Path, &Path) -> Result<(), ToolFault>,
{
    for entry in &plan.entries {
        if let Err(write_err) = writer(root, entry, created) {
            let cleanup_res = rollback(root, created, remover);
            if let Err(cleanup_err) = cleanup_res {
                return Err(cleanup_err);
            }
            return Err(write_err);
        }
    }
    Ok(())
}

/// The npm packages the Spectrum 2 starter script (`scripts/main.ts`) imports directly,
/// per the corrected paper adapter's direct-dependency set
/// (docs/design-systems/spectrum-paper-adapter.md, Facet 5). Each is resolved to a
/// full `npm:<package>@<version>` spec pinned to the same version recorded in
/// `wda.json`, so `wda.json` and the lockfile never take their versions from two
/// independent decisions. This list must be kept in sync with the starter script's
/// imports; it deliberately does not enumerate transitive dependencies.
const SPECTRUM_TWO_DIRECT_DEPENDENCY_PACKAGES: [&str; 6] = [
    "@spectrum-web-components/theme",
    "@spectrum-web-components/button",
    "@spectrum-web-components/textfield",
    "@spectrum-web-components/picker",
    "@spectrum-web-components/card",
    "@spectrum-web-components/dialog",
];

/// Builds the pinned `npm:<package>@<version>` dependency specs for every package in
/// [`SPECTRUM_TWO_DIRECT_DEPENDENCY_PACKAGES`], each pinned to `version`.
fn spectrum_two_dependency_specs(version: &str) -> Vec<String> {
    SPECTRUM_TWO_DIRECT_DEPENDENCY_PACKAGES
        .iter()
        .map(|pkg| format!("npm:{pkg}@{version}"))
        .collect()
}

/// Assembles the ordered write plan for project initialization.
///
/// Selects the tokens, starter page, and (for Spectrum 2) starter script assets by
/// the resolved design system's name: the WDA Minimal path emits the eight paths it
/// always has; the Spectrum 2 path additionally emits `scripts/main.ts`.
pub(crate) fn build_write_plan<Tz: chrono::TimeZone>(
    dt: &chrono::DateTime<Tz>,
    resolved_ds: &resolve::ResolvedDesignSystem,
) -> WritePlan
where
    Tz::Offset: std::fmt::Display,
{
    let mut plan = WritePlan::new();
    let doc_date = documents::format_document_date(dt);
    let is_spectrum_two = resolved_ds.name == resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME;

    plan.add("wda.json", manifest::generate_manifest(dt, resolved_ds));
    plan.add("README.md", documents::render_readme());
    plan.add(
        "docs/architecture.md",
        documents::render_architecture_doc(&doc_date),
    );
    plan.add(
        "docs/design.md",
        documents::render_design_doc(&doc_date, &resolved_ds.name),
    );
    plan.add("docs/naming.md", documents::render_naming_doc(&doc_date));

    if is_spectrum_two {
        plan.add(
            "tokens/tokens.json",
            crate::builtins::spectrum_two::tokens_json(),
        );
        plan.add(
            "pages/index.html",
            crate::builtins::spectrum_two::starter_page().replace(
                "<title>Spectrum 2 Starter</title>",
                &format!("<title>{}</title>", manifest::PROJECT_NAME),
            ),
        );
        plan.add(
            "scripts/main.ts",
            crate::builtins::spectrum_two::starter_script(),
        );
    } else {
        plan.add("tokens/tokens.json", documents::tokens_json_content());
        plan.add(
            "pages/index.html",
            documents::render_starter_page(manifest::PROJECT_NAME),
        );
    }

    plan.add(".gitignore", documents::gitignore_content());

    plan
}

/// Builds the ordered list of collision candidates for a project initialization run.
///
/// Returns write-plan files in plan order, then the intermediate directories those
/// files need, then `.git/`, plus `package.json` and `deno.lock` on Spectrum 2.
pub(crate) fn build_collision_candidates(
    plan: &WritePlan,
    is_spectrum_two: bool,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // 1. Write-plan files in order
    for entry in &plan.entries {
        candidates.push(entry.relative_path.clone());
    }

    // 2. The directories those files need, in order of appearance
    let mut directories = Vec::new();
    for entry in &plan.entries {
        if let Some(parent) = entry.relative_path.parent() {
            let mut current = PathBuf::new();
            for component in parent.components() {
                current.push(component.as_os_str());
                if !directories.contains(&current) {
                    directories.push(current.clone());
                }
            }
        }
    }
    candidates.extend(directories);

    // 3. The local git repository directory
    candidates.push(PathBuf::from(".git"));

    // 4. On Spectrum 2, the dependency state files created by deps::add
    if is_spectrum_two {
        candidates.push(PathBuf::from("package.json"));
        candidates.push(PathBuf::from("deno.lock"));
    }

    candidates
}

/// Scans the target project root for the presence of collision candidates.
///
/// Uses `fs::symlink_metadata` so a symlink counts as present without being followed,
/// matching `real_remover`'s classification. Returns which candidates exist in the
/// order they were passed.
pub(crate) fn scan_collision_candidates(
    root: &Path,
    candidates: &[PathBuf],
) -> Result<Vec<PathBuf>, ToolFault> {
    let mut collisions = Vec::new();
    for candidate in candidates {
        let full_path = root.join(candidate);
        match fs::symlink_metadata(&full_path) {
            Ok(_) => {
                collisions.push(candidate.clone());
            }
            // `NotADirectory` means an ancestor component is a plain file, so
            // `candidate` cannot exist as such either way; treat it the same as
            // `NotFound` rather than the real collision it descends from, which
            // `symlink_metadata` on that ancestor path already reports.
            Err(e)
                if e.kind() == io::ErrorKind::NotFound
                    || e.kind() == io::ErrorKind::NotADirectory => {}
            Err(e) => {
                return Err(ToolFault::new(
                    format!("Failed to inspect {}: {e}", candidate.display()),
                    Some(root.to_path_buf()),
                ));
            }
        }
    }
    Ok(collisions)
}

/// Seam to initialize a project at `root` with a specific date-time.
///
/// Checks that `root` has no colliding paths with the write plan or initialized repository.
/// If any candidate path already exists, returns a `ValidationReport` with Errors
/// carrying [`codes::INIT_PATH_CONFLICT`] for each colliding path in candidate order.
///
/// If no candidates collide, executes the ordered write plan and returns a clean `ValidationReport`.
/// Returns the resolution result alongside the report.
pub fn init_project_with_time<Tz: chrono::TimeZone>(
    root: &Path,
    dt: &chrono::DateTime<Tz>,
) -> Result<(ValidationReport, resolve::ResolvedDesignSystem), ToolFault>
where
    Tz::Offset: std::fmt::Display,
{
    init_project_with_time_and_resolution(root, dt, None)
}

/// Variant of [`init_project_with_time`] accepting a pre-resolved design system,
/// bypassing [`resolve::resolve_design_system`]'s online resolution.
///
/// Production code always passes `None` here, which resolves online exactly as
/// [`init_project_with_time`] always did. Tests pass `Some(resolved_ds)` so
/// initialization stays offline and deterministic under the bare `cargo test`,
/// mirroring the injected-PATH/injected-result seam `fix-deps-test-determinism`
/// established for `src/deps/`.
pub(crate) fn init_project_with_time_and_resolution<Tz: chrono::TimeZone>(
    root: &Path,
    dt: &chrono::DateTime<Tz>,
    resolved_ds: Option<resolve::ResolvedDesignSystem>,
) -> Result<(ValidationReport, resolve::ResolvedDesignSystem), ToolFault>
where
    Tz::Offset: std::fmt::Display,
{
    init_project_with_time_resolution_and_path(
        root,
        dt,
        resolved_ds,
        std::env::var_os("PATH").as_deref(),
    )
}

/// Variant of [`init_project_with_time_and_resolution`] accepting an explicit PATH environment value.
pub(crate) fn init_project_with_time_resolution_and_path<Tz: chrono::TimeZone>(
    root: &Path,
    dt: &chrono::DateTime<Tz>,
    resolved_ds: Option<resolve::ResolvedDesignSystem>,
    path_env: Option<&std::ffi::OsStr>,
) -> Result<(ValidationReport, resolve::ResolvedDesignSystem), ToolFault>
where
    Tz::Offset: std::fmt::Display,
{
    init_project_with_time_path_and_git_env(root, dt, resolved_ds, path_env, None, None)
}

/// Variant of [`init_project_with_time_path_and_git_env`] additionally accepting an
/// explicit [`crate::deps::resolve::InvocationContext`] for the Spectrum 2 dependency
/// pinning loop, so tests can drive its failure branch offline and deterministically.
/// Production code passes `None`, which builds the context from `path_env` exactly as
/// the loop always did.
pub(crate) fn init_project_with_time_path_git_env_and_deps_context<Tz: chrono::TimeZone>(
    root: &Path,
    dt: &chrono::DateTime<Tz>,
    resolved_ds: Option<resolve::ResolvedDesignSystem>,
    path_env: Option<&std::ffi::OsStr>,
    home_env: Option<&std::ffi::OsStr>,
    git_config_global_env: Option<&std::ffi::OsStr>,
    deps_ctx: Option<&crate::deps::resolve::InvocationContext>,
) -> Result<(ValidationReport, resolve::ResolvedDesignSystem), ToolFault>
where
    Tz::Offset: std::fmt::Display,
{
    init_inner(
        root,
        dt,
        resolved_ds,
        path_env,
        home_env,
        git_config_global_env,
        deps_ctx,
    )
}

/// Variant of [`init_project_with_time_resolution_and_path`] additionally accepting
/// explicit `HOME` and `GIT_CONFIG_GLOBAL` overrides for the git child processes,
/// so tests can isolate git identity resolution without mutating the process
/// environment (racy under parallel `cargo test`).
pub(crate) fn init_project_with_time_path_and_git_env<Tz: chrono::TimeZone>(
    root: &Path,
    dt: &chrono::DateTime<Tz>,
    resolved_ds: Option<resolve::ResolvedDesignSystem>,
    path_env: Option<&std::ffi::OsStr>,
    home_env: Option<&std::ffi::OsStr>,
    git_config_global_env: Option<&std::ffi::OsStr>,
) -> Result<(ValidationReport, resolve::ResolvedDesignSystem), ToolFault>
where
    Tz::Offset: std::fmt::Display,
{
    init_inner(
        root,
        dt,
        resolved_ds,
        path_env,
        home_env,
        git_config_global_env,
        None,
    )
}

fn init_inner<Tz: chrono::TimeZone>(
    root: &Path,
    dt: &chrono::DateTime<Tz>,
    resolved_ds: Option<resolve::ResolvedDesignSystem>,
    path_env: Option<&std::ffi::OsStr>,
    home_env: Option<&std::ffi::OsStr>,
    git_config_global_env: Option<&std::ffi::OsStr>,
    deps_ctx: Option<&crate::deps::resolve::InvocationContext>,
) -> Result<(ValidationReport, resolve::ResolvedDesignSystem), ToolFault>
where
    Tz::Offset: std::fmt::Display,
{
    /// Applies the caller's `PATH`/`HOME`/`GIT_CONFIG_GLOBAL` overrides to a git child
    /// `Command`, leaving every other inherited environment variable untouched.
    fn apply_git_env(
        cmd: &mut std::process::Command,
        path_env: Option<&std::ffi::OsStr>,
        home_env: Option<&std::ffi::OsStr>,
        git_config_global_env: Option<&std::ffi::OsStr>,
    ) {
        if let Some(path) = path_env {
            cmd.env("PATH", path);
        }
        if let Some(home) = home_env {
            cmd.env("HOME", home);
        }
        if let Some(global) = git_config_global_env {
            cmd.env("GIT_CONFIG_GLOBAL", global);
        }
    }
    if !root.is_dir() {
        return Err(ToolFault::new(
            "Project root directory is invalid or unreadable",
            Some(root.to_path_buf()),
        ));
    }

    let git_binary = match crate::build::scripts::resolve_tool_with_path("git", path_env) {
        Ok(path) => path,
        Err(_) => {
            let mut report = ValidationReport::new();
            report.add(Diagnostic::new(
                codes::INIT_TOOL_UNAVAILABLE,
                Severity::Error,
                None,
                "Required tool 'git' was not found on PATH",
                "Ensure 'git' is installed and available on PATH",
            ));
            let resolved_ds = resolved_ds.unwrap_or_else(|| resolve::ResolvedDesignSystem {
                name: resolve::BUILTIN_DESIGN_SYSTEM_NAME.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                fallback_reason: None,
            });
            return Ok((report, resolved_ds));
        }
    };

    let mut resolved_ds = resolved_ds.unwrap_or_else(resolve::resolve_design_system);
    let plan = build_write_plan(dt, &resolved_ds);
    let is_spectrum_two = resolved_ds.name == resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME;
    let candidates = build_collision_candidates(&plan, is_spectrum_two);
    let collisions = scan_collision_candidates(root, &candidates)?;

    if !collisions.is_empty() {
        let mut report = ValidationReport::new();
        for path in collisions {
            let normalized = path.to_string_lossy().replace('\\', "/");
            report.add(Diagnostic::new(
                codes::INIT_PATH_CONFLICT,
                Severity::Error,
                Some(Location::path_only(path)),
                format!("Target path '{normalized}' already exists"),
                format!("Remove or rename '{normalized}' before initializing"),
            ));
        }
        return Ok((report, resolved_ds));
    }

    let mut git_cmd = std::process::Command::new(&git_binary);
    git_cmd.arg("init").current_dir(root);
    apply_git_env(&mut git_cmd, path_env, home_env, git_config_global_env);
    let output = git_cmd.output().map_err(|e| {
        ToolFault::new(
            format!("Failed to execute 'git init': {e}"),
            Some(root.to_path_buf()),
        )
    })?;

    if !output.status.success() {
        if root.join(".git").exists() {
            let _ = rollback(root, &[PathBuf::from(".git")], real_remover);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        let trimmed = stderr.trim();
        let msg = if trimmed.is_empty() {
            format!("'git init' failed with status {}", output.status)
        } else {
            format!("'git init' failed: {trimmed}")
        };
        return Err(ToolFault::new(msg, Some(root.to_path_buf())));
    }

    let mut created = vec![PathBuf::from(".git")];
    execute_write_plan_with_created(root, &plan, &mut created, real_writer, real_remover)?;

    // Spectrum 2, resolved online, pins every direct npm dependency the corrected
    // paper adapter's starter script imports (docs/design-systems/spectrum-paper-adapter.md,
    // Facet 5: theme, button, textfield, picker, card, dialog) at the exact version
    // just resolved, one `crate::deps::add` call per package. A failure on any call
    // discards the Spectrum 2 write plan and writes the WDA Minimal plan in its place,
    // matching what the seven resolution-time branches in `resolve.rs` already do when
    // Spectrum 2 never resolves at all: init leaves a fully initialized WDA Minimal
    // project, never a partial one and never an empty directory. The local git
    // repository survives the swap, so the cleanup set is every write-plan entry
    // except `.git`, plus `package.json`/`deno.lock` (idempotent via `real_remover`'s
    // NotFound-is-Ok handling even when `deps::add` never got far enough to create them).
    if resolved_ds.name == resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME {
        let specs: Vec<String> = spectrum_two_dependency_specs(&resolved_ds.version);
        let owned_ctx;
        let ctx = match deps_ctx {
            Some(ctx) => ctx,
            None => {
                owned_ctx = crate::deps::resolve::InvocationContext::from_env(path_env);
                &owned_ctx
            }
        };

        for spec in &specs {
            if crate::deps::add_with_context(root, spec, ctx).is_err() {
                let mut cleanup: Vec<PathBuf> = created
                    .iter()
                    .filter(|p| p.as_path() != Path::new(".git"))
                    .cloned()
                    .collect();
                cleanup.push(PathBuf::from("package.json"));
                cleanup.push(PathBuf::from("deno.lock"));
                rollback(root, &cleanup, real_remover)?;

                resolved_ds =
                    resolve::builtin_fallback(resolve::FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED);
                let minimal_plan = build_write_plan(dt, &resolved_ds);
                created = vec![PathBuf::from(".git")];
                execute_write_plan_with_created(
                    root,
                    &minimal_plan,
                    &mut created,
                    real_writer,
                    real_remover,
                )?;
                break;
            }
        }
    }

    // The first commit covers the whole directory, including any pre-existing
    // entries the collision scan let through: a commit holding only init's own
    // files cannot restore what the Designer cares about. Failure at either
    // step rolls back through the same `created` set as every other mutation.
    let mut add_cmd = std::process::Command::new(&git_binary);
    add_cmd.args(["add", "-A"]).current_dir(root);
    apply_git_env(&mut add_cmd, path_env, home_env, git_config_global_env);
    let add_output = add_cmd.output().map_err(|e| {
        ToolFault::new(
            format!("Failed to execute 'git add -A': {e}"),
            Some(root.to_path_buf()),
        )
    })?;
    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        let trimmed = stderr.trim();
        let msg = if trimmed.is_empty() {
            format!("'git add -A' failed with status {}", add_output.status)
        } else {
            format!("'git add -A' failed: {trimmed}")
        };
        if let Err(cleanup_err) = rollback(root, &created, real_remover) {
            return Err(cleanup_err);
        }
        return Err(ToolFault::new(msg, Some(root.to_path_buf())));
    }

    let mut report = ValidationReport::new();

    let mut commit_cmd = std::process::Command::new(&git_binary);
    commit_cmd
        .args(["commit", "-m", "wda init"])
        .current_dir(root);
    apply_git_env(&mut commit_cmd, path_env, home_env, git_config_global_env);
    let commit_output = commit_cmd.output().map_err(|e| {
        ToolFault::new(
            format!("Failed to execute 'git commit': {e}"),
            Some(root.to_path_buf()),
        )
    })?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        // Global git configuration is never written: the fallback identity is
        // passed through `-c` on this one invocation only, never `git config
        // --global`. A retry is attempted only for the specific "no identity
        // configured" failure git reports on a machine with no `user.name`/
        // `user.email` set anywhere; any other refusal (a `commit.gpgsign`
        // requirement, a `core.hooksPath` pre-commit hook, etc.) is reported
        // as-is with git's own stderr, not silently retried.
        let looks_like_missing_identity = stderr.contains("Please tell me who you are")
            || stderr.contains("user.email")
            || stderr.contains("user.name");

        if looks_like_missing_identity {
            let mut fallback_cmd = std::process::Command::new(&git_binary);
            fallback_cmd
                .args([
                    "-c",
                    "user.name=WDA",
                    "-c",
                    "user.email=wda@localhost",
                    "commit",
                    "-m",
                    "wda init",
                ])
                .current_dir(root);
            apply_git_env(&mut fallback_cmd, path_env, home_env, git_config_global_env);
            let fallback_output = fallback_cmd.output().map_err(|e| {
                ToolFault::new(
                    format!("Failed to execute 'git commit' with fallback identity: {e}"),
                    Some(root.to_path_buf()),
                )
            })?;

            if fallback_output.status.success() {
                report.add(Diagnostic::new(
                    codes::INIT_GIT_IDENTITY_FALLBACK,
                    Severity::Warning,
                    None,
                    "No git identity was configured, so the initial commit was authored as WDA <wda@localhost>",
                    "Configure 'git config user.name' and 'git config user.email' to author future commits under your own identity",
                ));
                return Ok((report, resolved_ds));
            }

            let fallback_stderr = String::from_utf8_lossy(&fallback_output.stderr);
            let trimmed = fallback_stderr.trim();
            let msg = if trimmed.is_empty() {
                format!(
                    "'git commit' with fallback identity failed with status {}",
                    fallback_output.status
                )
            } else {
                format!("'git commit' with fallback identity failed: {trimmed}")
            };
            if let Err(cleanup_err) = rollback(root, &created, real_remover) {
                return Err(cleanup_err);
            }
            return Err(ToolFault::new(msg, Some(root.to_path_buf())));
        }

        let trimmed = stderr.trim();
        let msg = if trimmed.is_empty() {
            format!("'git commit' failed with status {}", commit_output.status)
        } else {
            format!("'git commit' failed: {trimmed}")
        };
        if let Err(cleanup_err) = rollback(root, &created, real_remover) {
            return Err(cleanup_err);
        }
        return Err(ToolFault::new(msg, Some(root.to_path_buf())));
    }

    Ok((report, resolved_ds))
}

/// Seam to initialize a project at `root`.
///
/// Checks that `root` has no path collisions with the write plan or initialized repository.
/// If any candidate path already exists, returns a `ValidationReport` with Errors
/// carrying [`codes::INIT_PATH_CONFLICT`] for each colliding path in candidate order.
///
/// If no candidates collide, executes the ordered write plan using the current local time
/// and returns a clean `ValidationReport`.
/// Returns the resolution result alongside the report.
pub fn init_project(root: &Path) -> Result<(ValidationReport, resolve::ResolvedDesignSystem), ToolFault> {
    let now = chrono::Local::now();
    init_project_with_time(root, &now)
}

/// Variant of [`init_project`] accepting a pre-resolved design system, for
/// deterministic, offline-only initialization in tests. See
/// [`init_project_with_time_and_resolution`].
#[cfg(test)]
pub(crate) fn init_project_with_resolution(
    root: &Path,
    resolved_ds: resolve::ResolvedDesignSystem,
) -> Result<(ValidationReport, resolve::ResolvedDesignSystem), ToolFault> {
    let now = chrono::Local::now();
    init_project_with_time_and_resolution(root, &now, Some(resolved_ds))
}

/// Variant of [`init_project_with_resolution`] accepting an explicit PATH environment value.
#[cfg(test)]
pub(crate) fn init_project_with_resolution_and_path(
    root: &Path,
    resolved_ds: resolve::ResolvedDesignSystem,
    path_env: Option<&std::ffi::OsStr>,
) -> Result<(ValidationReport, resolve::ResolvedDesignSystem), ToolFault> {
    let now = chrono::Local::now();
    init_project_with_time_resolution_and_path(root, &now, Some(resolved_ds), path_env)
}

/// Variant of [`init_project_with_resolution_and_path`] additionally accepting explicit
/// `HOME` and `GIT_CONFIG_GLOBAL` overrides for the git child processes, isolating identity
/// resolution for tests without mutating the process environment.
#[cfg(test)]
pub(crate) fn init_project_with_resolution_path_and_git_env(
    root: &Path,
    resolved_ds: resolve::ResolvedDesignSystem,
    path_env: Option<&std::ffi::OsStr>,
    home_env: Option<&std::ffi::OsStr>,
    git_config_global_env: Option<&std::ffi::OsStr>,
) -> Result<(ValidationReport, resolve::ResolvedDesignSystem), ToolFault> {
    let now = chrono::Local::now();
    init_project_with_time_path_and_git_env(
        root,
        &now,
        Some(resolved_ds),
        path_env,
        home_env,
        git_config_global_env,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// Fixture standing in for offline, injected design-system resolution so init
    /// tests never reach the real online resolver's network call.
    fn offline_wda_minimal() -> resolve::ResolvedDesignSystem {
        resolve::ResolvedDesignSystem {
            name: resolve::BUILTIN_DESIGN_SYSTEM_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            fallback_reason: Some(resolve::FALLBACK_REASON_DENO_ABSENT.to_string()),
        }
    }

    #[test]
    fn test_spectrum_two_dependency_specs_pins_every_direct_package_to_version() {
        let specs = spectrum_two_dependency_specs("1.2.3");
        let expected: Vec<String> = vec![
            "npm:@spectrum-web-components/theme@1.2.3".to_string(),
            "npm:@spectrum-web-components/button@1.2.3".to_string(),
            "npm:@spectrum-web-components/textfield@1.2.3".to_string(),
            "npm:@spectrum-web-components/picker@1.2.3".to_string(),
            "npm:@spectrum-web-components/card@1.2.3".to_string(),
            "npm:@spectrum-web-components/dialog@1.2.3".to_string(),
        ];
        assert_eq!(specs, expected);
    }

    #[test]
    fn test_gate_accepts_directory_with_hidden_file() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_hidden_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        let hidden_file = temp_dir.join(".hidden");
        fs::write(&hidden_file, "content").unwrap();

        let result = init_project_with_resolution(&temp_dir, offline_wda_minimal());
        let (report, _) = result.expect("directory with hidden file succeeds under collision gate");
        assert!(
            report.diagnostics.is_empty(),
            "init report must contain zero diagnostics, got: {:?}",
            report.diagnostics
        );

        assert!(temp_dir.join(".hidden").exists(), ".hidden must be preserved");
        assert_eq!(
            fs::read_to_string(temp_dir.join(".hidden")).unwrap(),
            "content",
            "content of pre-existing hidden file must remain unchanged"
        );
        assert!(temp_dir.join("wda.json").exists(), "wda.json must be written");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_gate_refuses_directory_with_git_dir() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_git_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        let git_dir = temp_dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();

        let before_count = count_entries(&temp_dir).unwrap();
        let result = init_project_with_resolution(&temp_dir, offline_wda_minimal());
        let (report, _) = result.expect("directory with .git returns Ok((ValidationReport, ResolvedDesignSystem))");
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].code, codes::INIT_PATH_CONFLICT);
        assert_eq!(report.diagnostics[0].severity, Severity::Error);
        let loc = report.diagnostics[0].location.as_ref().expect("location must be present");
        assert_eq!(loc.path, Path::new(".git"));
        assert!(report.diagnostics[0].message.contains(".git"));
        let after_count = count_entries(&temp_dir).unwrap();
        assert_eq!(before_count, after_count, "entry count must be unchanged after refusal");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_gate_refuses_directory_with_gitignore() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_gitignore_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        let gitignore_file = temp_dir.join(".gitignore");
        fs::write(&gitignore_file, "/dist/\n").unwrap();

        let before_count = count_entries(&temp_dir).unwrap();
        let result = init_project_with_resolution(&temp_dir, offline_wda_minimal());
        let (report, _) = result.expect("directory with .gitignore returns Ok((ValidationReport, ResolvedDesignSystem))");
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].code, codes::INIT_PATH_CONFLICT);
        assert_eq!(report.diagnostics[0].severity, Severity::Error);
        let loc = report.diagnostics[0].location.as_ref().expect("location must be present");
        assert_eq!(loc.path, Path::new(".gitignore"));
        assert!(report.diagnostics[0].message.contains(".gitignore"));
        let after_count = count_entries(&temp_dir).unwrap();
        assert_eq!(before_count, after_count, "entry count must be unchanged after refusal");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_gate_refuses_when_git_unavailable_on_path() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_git_unavailable_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        let notes_file = temp_dir.join("notes.txt");
        fs::write(&notes_file, "designer notes").unwrap();

        let before_count = count_entries(&temp_dir).unwrap();
        let empty_path = std::ffi::OsStr::new("");
        let result = init_project_with_resolution_and_path(
            &temp_dir,
            offline_wda_minimal(),
            Some(empty_path),
        );
        let (report, _) = result.expect("missing git returns Ok((ValidationReport, ResolvedDesignSystem))");
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].code, codes::INIT_TOOL_UNAVAILABLE);
        assert_eq!(report.diagnostics[0].severity, Severity::Error);
        assert!(
            report.diagnostics[0].message.contains("git"),
            "diagnostic message must name 'git', got: {}",
            report.diagnostics[0].message
        );
        let after_count = count_entries(&temp_dir).unwrap();
        assert_eq!(
            before_count, after_count,
            "entry count must be unchanged after refusal"
        );
        assert_eq!(
            fs::read_to_string(&notes_file).unwrap(),
            "designer notes",
            "pre-existing file content must be untouched"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_gate_refuses_when_git_path_is_none() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_git_none_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);

        let before_count = count_entries(&temp_dir).unwrap();
        let result = init_project_with_resolution_and_path(
            &temp_dir,
            offline_wda_minimal(),
            None,
        );
        let (report, _) = result.expect("missing git returns Ok((ValidationReport, ResolvedDesignSystem))");
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].code, codes::INIT_TOOL_UNAVAILABLE);
        assert_eq!(report.diagnostics[0].severity, Severity::Error);
        assert!(
            report.diagnostics[0].message.contains("git"),
            "diagnostic message must name 'git', got: {}",
            report.diagnostics[0].message
        );
        let after_count = count_entries(&temp_dir).unwrap();
        assert_eq!(
            before_count, after_count,
            "entry count must be unchanged after refusal"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_init_commits_whole_directory_with_wda_init_message_and_no_remote() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_commit_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("notes.txt"), "designer notes").unwrap();

        let (report, _) = init_project_with_resolution(&temp_dir, offline_wda_minimal())
            .expect("init with a pre-existing identity must succeed");
        assert!(
            report.diagnostics.is_empty(),
            "init report must contain zero diagnostics when identity is already configured, got: {:?}",
            report.diagnostics
        );

        let git_binary =
            crate::build::scripts::resolve_tool("git").expect("git must be available on PATH");

        let log_output = std::process::Command::new(&git_binary)
            .args(["log", "--oneline"])
            .current_dir(&temp_dir)
            .output()
            .expect("git log must run");
        assert!(log_output.status.success(), "git log must exit 0");
        let log_stdout = String::from_utf8_lossy(&log_output.stdout).into_owned();
        let log_lines: Vec<&str> = log_stdout.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(
            log_lines.len(),
            1,
            "repository must have exactly one commit, got: {log_lines:?}"
        );

        let show_output = std::process::Command::new(&git_binary)
            .args(["log", "-1", "--format=%s"])
            .current_dir(&temp_dir)
            .output()
            .expect("git log -1 must run");
        assert_eq!(
            String::from_utf8_lossy(&show_output.stdout).trim(),
            "wda init",
            "commit message must be 'wda init'"
        );

        let remote_output = std::process::Command::new(&git_binary)
            .args(["remote", "-v"])
            .current_dir(&temp_dir)
            .output()
            .expect("git remote -v must run");
        assert!(
            String::from_utf8_lossy(&remote_output.stdout).trim().is_empty(),
            "git remote -v must be empty"
        );

        let tree_output = std::process::Command::new(&git_binary)
            .args(["ls-tree", "-r", "--name-only", "HEAD"])
            .current_dir(&temp_dir)
            .output()
            .expect("git ls-tree must run");
        let tree_stdout = String::from_utf8_lossy(&tree_output.stdout);
        assert!(
            tree_stdout.lines().any(|l| l.trim() == "notes.txt"),
            "the pre-existing file must be committed alongside init's own files, git ls-tree:\n{tree_stdout}"
        );
        assert!(
            !tree_stdout.lines().any(|l| l.starts_with("dist/")),
            "the commit must not contain 'dist/', git ls-tree:\n{tree_stdout}"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_init_falls_back_to_wda_identity_when_none_configured() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_identity_fallback_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Isolated HOME with no `.gitconfig`, and GIT_CONFIG_GLOBAL pointed at a
        // non-existent file, so git resolves no identity from any global source.
        // Passed on the child `Command` only (see `init_project_with_resolution_path_and_git_env`
        // / `apply_git_env`), never through the process environment, which would be
        // racy under parallel `cargo test`.
        let isolated_home = std::env::temp_dir().join(format!(
            "wda_test_init_identity_home_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&isolated_home);
        fs::create_dir_all(&isolated_home).unwrap();
        let isolated_global_config = isolated_home.join("nonexistent-gitconfig");

        let (report, _) = init_project_with_resolution_path_and_git_env(
            &temp_dir,
            offline_wda_minimal(),
            Some(std::env::var_os("PATH").unwrap_or_default().as_os_str()),
            Some(isolated_home.as_os_str()),
            Some(isolated_global_config.as_os_str()),
        )
        .expect("init must succeed via the fallback identity, not fail");

        assert_eq!(
            report.diagnostics.len(),
            1,
            "report must carry exactly one identity-fallback diagnostic, got: {:?}",
            report.diagnostics
        );
        assert_eq!(report.diagnostics[0].code, codes::INIT_GIT_IDENTITY_FALLBACK);
        assert_eq!(report.diagnostics[0].severity, Severity::Warning);
        assert!(
            report.diagnostics[0].message.contains("WDA")
                && report.diagnostics[0].message.contains("wda@localhost"),
            "fallback report line must name the WDA <wda@localhost> identity, got: {}",
            report.diagnostics[0].message
        );

        let git_binary =
            crate::build::scripts::resolve_tool("git").expect("git must be available on PATH");
        let show_output = std::process::Command::new(&git_binary)
            .args(["log", "-1", "--format=%an <%ae>"])
            .current_dir(&temp_dir)
            .output()
            .expect("git log -1 must run");
        assert_eq!(
            String::from_utf8_lossy(&show_output.stdout).trim(),
            "WDA <wda@localhost>",
            "commit author must be the fallback identity"
        );

        // Global git configuration is never written: the real global config file
        // (as far as this isolated HOME/GIT_CONFIG_GLOBAL know it) must not exist.
        assert!(
            !isolated_global_config.exists(),
            "the fallback commit must never write global git configuration"
        );

        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::remove_dir_all(&isolated_home);
    }

    #[test]
    fn test_init_project_empty_directory_produces_eight_paths_and_validates_clean() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_success_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
        let naive_date = chrono::NaiveDate::from_ymd_opt(2026, 9, 2).unwrap();
        let naive_time = chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        let dt = offset
            .from_local_datetime(&naive_date.and_time(naive_time))
            .unwrap();

        let result =
            init_project_with_time_and_resolution(&temp_dir, &dt, Some(offline_wda_minimal()));
        let (report, resolved_ds) = result.expect("init on empty directory must succeed");
        assert_eq!(resolved_ds.name, "WDA Minimal");
        assert_eq!(
            resolved_ds.fallback_reason.as_deref(),
            Some(resolve::FALLBACK_REASON_DENO_ABSENT)
        );
        assert!(
            report.diagnostics.is_empty(),
            "init report must contain zero diagnostics: {:?}",
            report.diagnostics
        );

        let expected_paths = [
            "wda.json",
            "README.md",
            "docs/architecture.md",
            "docs/design.md",
            "docs/naming.md",
            "tokens/tokens.json",
            "pages/index.html",
            ".gitignore",
        ];

        for rel_path in &expected_paths {
            let p = temp_dir.join(rel_path);
            assert!(
                p.exists(),
                "expected path '{}' must exist after initialization",
                rel_path
            );
            assert!(
                p.is_file(),
                "expected path '{}' must be a regular file",
                rel_path
            );
        }

        // Verify docs/ directory contains exactly 3 Markdown files
        let docs_dir = temp_dir.join("docs");
        assert!(docs_dir.is_dir(), "docs directory must exist");
        let docs_entries: Vec<PathBuf> = fs::read_dir(&docs_dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(
            docs_entries.len(),
            3,
            "docs/ directory must contain exactly 3 entries, got: {:?}",
            docs_entries
        );

        for doc_entry in &docs_entries {
            assert_eq!(
                doc_entry.extension().and_then(|ext| ext.to_str()),
                Some("md"),
                "every file in docs/ must have .md extension: {}",
                doc_entry.display()
            );
        }

        // Validate the generated project with the real validation engine
        let validation_report = crate::validate_project(&temp_dir)
            .expect("real validate_project on initialized project must succeed");
        assert!(
            validation_report.diagnostics.is_empty(),
            "generated project must pass validate_project with zero diagnostics, got: {:?}",
            validation_report.diagnostics
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_init_project_deterministic_byte_identical_runs() {
        let temp_dir1 = std::env::temp_dir()
            .join(format!("wda_test_init_determ_1_{}", std::process::id()));
        let temp_dir2 = std::env::temp_dir()
            .join(format!("wda_test_init_determ_2_{}", std::process::id()));

        let _ = fs::remove_dir_all(&temp_dir1);
        let _ = fs::remove_dir_all(&temp_dir2);
        fs::create_dir_all(&temp_dir1).unwrap();
        fs::create_dir_all(&temp_dir2).unwrap();

        let offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
        let naive_date = chrono::NaiveDate::from_ymd_opt(2026, 9, 2).unwrap();
        let naive_time = chrono::NaiveTime::from_hms_opt(15, 30, 45).unwrap();
        let dt = offset
            .from_local_datetime(&naive_date.and_time(naive_time))
            .unwrap();

        let res1 =
            init_project_with_time_and_resolution(&temp_dir1, &dt, Some(offline_wda_minimal()));
        let res2 =
            init_project_with_time_and_resolution(&temp_dir2, &dt, Some(offline_wda_minimal()));

        assert!(res1.is_ok());
        assert!(res2.is_ok());

        let expected_paths = [
            "wda.json",
            "README.md",
            "docs/architecture.md",
            "docs/design.md",
            "docs/naming.md",
            "tokens/tokens.json",
            "pages/index.html",
            ".gitignore",
        ];

        for rel_path in &expected_paths {
            let bytes1 = fs::read(temp_dir1.join(rel_path)).unwrap();
            let bytes2 = fs::read(temp_dir2.join(rel_path)).unwrap();
            assert_eq!(
                bytes1, bytes2,
                "file '{}' must be byte-identical across two runs at the same instant",
                rel_path
            );
        }

        let _ = fs::remove_dir_all(&temp_dir1);
        let _ = fs::remove_dir_all(&temp_dir2);
    }

    #[test]
    fn test_build_write_plan_entries_and_paths() {
        let offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
        let naive_date = chrono::NaiveDate::from_ymd_opt(2026, 9, 2).unwrap();
        let naive_time = chrono::NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        let dt = offset
            .from_local_datetime(&naive_date.and_time(naive_time))
            .unwrap();

        let resolved_ds = offline_wda_minimal();
        let plan = build_write_plan(&dt, &resolved_ds);

        assert_eq!(plan.entries.len(), 8);
        assert_eq!(plan.entries[0].relative_path, Path::new("wda.json"));
        assert_eq!(plan.entries[1].relative_path, Path::new("README.md"));
        assert_eq!(
            plan.entries[2].relative_path,
            Path::new("docs/architecture.md")
        );
        assert_eq!(plan.entries[3].relative_path, Path::new("docs/design.md"));
        assert_eq!(plan.entries[4].relative_path, Path::new("docs/naming.md"));
        assert_eq!(
            plan.entries[5].relative_path,
            Path::new("tokens/tokens.json")
        );
        assert_eq!(plan.entries[6].relative_path, Path::new("pages/index.html"));
        assert_eq!(plan.entries[7].relative_path, Path::new(".gitignore"));

        assert_eq!(
            plan.entries[5].content,
            documents::tokens_json_content(),
            "Minimal resolution must source tokens/tokens.json from the Minimal module"
        );
        assert_eq!(
            plan.entries[6].content,
            documents::render_starter_page(manifest::PROJECT_NAME),
            "Minimal resolution must source pages/index.html from the Minimal module"
        );
    }

    #[test]
    fn test_build_write_plan_selects_spectrum_two_assets_and_adds_scripts_main_ts() {
        let offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
        let naive_date = chrono::NaiveDate::from_ymd_opt(2026, 9, 2).unwrap();
        let naive_time = chrono::NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        let dt = offset
            .from_local_datetime(&naive_date.and_time(naive_time))
            .unwrap();

        let resolved_ds = resolve::ResolvedDesignSystem {
            name: resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME.to_string(),
            version: "1.2.3".to_string(),
            fallback_reason: None,
        };
        let plan = build_write_plan(&dt, &resolved_ds);

        assert_eq!(
            plan.entries.len(),
            9,
            "Spectrum 2 path adds scripts/main.ts on top of the base eight paths"
        );
        let paths: Vec<&Path> = plan
            .entries
            .iter()
            .map(|entry| entry.relative_path.as_path())
            .collect();
        assert!(paths.contains(&Path::new("scripts/main.ts")));

        let tokens_entry = plan
            .entries
            .iter()
            .find(|entry| entry.relative_path == Path::new("tokens/tokens.json"))
            .expect("tokens/tokens.json must be present");
        assert_eq!(
            tokens_entry.content,
            crate::builtins::spectrum_two::tokens_json()
        );

        let page_entry = plan
            .entries
            .iter()
            .find(|entry| entry.relative_path == Path::new("pages/index.html"))
            .expect("pages/index.html must be present");
        assert!(page_entry
            .content
            .contains(&format!("<title>{}</title>", manifest::PROJECT_NAME)));
        assert!(!page_entry
            .content
            .contains("<title>Spectrum 2 Starter</title>"));

        let script_entry = plan
            .entries
            .iter()
            .find(|entry| entry.relative_path == Path::new("scripts/main.ts"))
            .expect("scripts/main.ts must be present");
        assert_eq!(
            script_entry.content,
            crate::builtins::spectrum_two::starter_script(),
            "Spectrum 2 resolution must source scripts/main.ts from the Spectrum 2 module"
        );
    }

    #[test]
    fn test_spectrum_two_dependency_failure_falls_back_to_a_valid_minimal_project() {
        // Offline counterpart to the real-registry probe below: an injected
        // resolution strategy fails every `deps::add` call deterministically, so the
        // fallback path runs under the bare `cargo test` gate with no network and no
        // real `deno`. Guards the contract that a post-resolution dependency failure
        // leaves a complete WDA Minimal project, not the empty directory an earlier
        // implementation left behind.
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_spectrum_two_dep_fallback_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
        let naive_date = chrono::NaiveDate::from_ymd_opt(2026, 9, 17).unwrap();
        let naive_time = chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let dt = offset
            .from_local_datetime(&naive_date.and_time(naive_time))
            .unwrap();

        let spectrum_ds = resolve::ResolvedDesignSystem {
            name: resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME.to_string(),
            version: "1.2.3".to_string(),
            fallback_reason: None,
        };

        let ctx = crate::deps::resolve::InvocationContext::with_strategy(None, Vec::new(), |_| {
            Err(Diagnostic::new(
                codes::INIT_TOOL_UNAVAILABLE,
                Severity::Error,
                None,
                "injected dependency resolution failure",
                "no action; this is a test injection",
            ))
        });

        let (report, resolved_ds) = init_project_with_time_path_git_env_and_deps_context(
            &temp_dir,
            &dt,
            Some(spectrum_ds),
            std::env::var_os("PATH").as_deref(),
            None,
            None,
            Some(&ctx),
        )
        .expect("a dependency failure must fall back, not fault");

        assert_eq!(
            resolved_ds.name,
            resolve::BUILTIN_DESIGN_SYSTEM_NAME,
            "a dependency failure must fall back to WDA Minimal"
        );
        assert_eq!(
            resolved_ds.fallback_reason.as_deref(),
            Some(resolve::FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED),
            "the fallback reason must name the dependency install failure, not a resolution branch"
        );
        assert!(
            report.diagnostics.is_empty(),
            "the fallback project must report zero diagnostics, got: {:?}",
            report.diagnostics
        );

        // Spectrum 2-only artifacts are gone; the Minimal document set is present.
        for absent in ["package.json", "deno.lock", "scripts/main.ts"] {
            assert!(
                !temp_dir.join(absent).exists(),
                "{absent} must not survive the fallback"
            );
        }
        for present in [
            "wda.json",
            "README.md",
            "docs/architecture.md",
            "docs/design.md",
            "docs/naming.md",
            "tokens/tokens.json",
            "pages/index.html",
            ".gitignore",
        ] {
            assert!(
                temp_dir.join(present).exists(),
                "{present} must exist after the fallback"
            );
        }

        let validation_report = crate::validate_project(&temp_dir)
            .expect("validate_project on the fallback directory must succeed");
        assert!(
            validation_report
                .diagnostics
                .iter()
                .all(|d| d.severity != Severity::Error),
            "the fallback directory must validate with zero Errors, got: {:?}",
            validation_report.diagnostics
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[ignore]
    fn test_spectrum_two_dependency_resolution_failure_rolls_back_to_minimal() {
        // Real-registry probe: injects a Spectrum 2 resolution (bypassing the
        // online resolver via `Some(resolved_ds)`, per this seam's own doc
        // comment) pinned to a version that cannot exist on the npm registry,
        // so the real `crate::deps::add` call (reached via the real `deno`
        // binary on PATH) deterministically fails while every other write
        // stays real. Matches the real-registry probe tier in
        // `src/deps/mod.rs`, `src/deps/resolve.rs`, and `tests/build.rs`; run
        // separately via `cargo test -- --ignored`.
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_spectrum_two_dep_failure_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
        let naive_date = chrono::NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        let naive_time = chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let dt = offset
            .from_local_datetime(&naive_date.and_time(naive_time))
            .unwrap();

        let unresolvable_ds = resolve::ResolvedDesignSystem {
            name: resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME.to_string(),
            version: "0.0.0-wda-t16-nonexistent-probe".to_string(),
            fallback_reason: None,
        };

        let result =
            init_project_with_time_and_resolution(&temp_dir, &dt, Some(unresolvable_ds));

        // On injected resolution failure, neither dependency file exists.
        assert!(
            !temp_dir.join("package.json").exists(),
            "package.json must not exist after a dependency-resolution failure"
        );
        assert!(
            !temp_dir.join("deno.lock").exists(),
            "deno.lock must not exist after a dependency-resolution failure"
        );
        assert!(
            !temp_dir.join("scripts/main.ts").exists(),
            "scripts/main.ts (Spectrum 2-only) must not exist after a dependency-resolution failure"
        );

        // The directory validates as a Minimal project.
        let validation_report = crate::validate_project(&temp_dir)
            .expect("validate_project on the rolled-back directory must succeed");
        assert!(
            validation_report
                .diagnostics
                .iter()
                .all(|d| d.severity != Severity::Error),
            "rolled-back directory must validate with zero Errors, got: {:?}",
            validation_report.diagnostics
        );

        let wda_json: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(temp_dir.join("wda.json"))
                .expect("wda.json must exist on the completed Minimal path"),
        )
        .expect("wda.json must be valid JSON");
        assert_eq!(
            wda_json["designSystem"]["name"].as_str(),
            Some("WDA Minimal"),
            "completed project must record the Minimal path, got: {}",
            wda_json
        );

        // The fallback reason travels on the resolved design system, the same
        // carrier the seven resolution-time branches use, and reaches the Designer
        // through the G11 report. Its wording is this failure point's own, so the
        // report can tell an unresolvable Spectrum 2 apart from one that resolved
        // and then could not install its pinned dependencies.
        let (_, resolved) = result.expect("a dependency failure must fall back, not fault");
        assert_eq!(
            resolved.fallback_reason.as_deref(),
            Some(resolve::FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED),
            "the fallback reason must name the dependency install failure"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_non_directory_root_yields_tool_fault() {
        let temp_file = std::env::temp_dir()
            .join(format!("wda_test_init_non_dir_{}", std::process::id()));
        fs::write(&temp_file, "not a dir").unwrap();

        let result = init_project(&temp_file);
        assert!(result.is_err());
        let fault = result.unwrap_err();
        assert_eq!(fault.message, "Project root directory is invalid or unreadable");

        let _ = fs::remove_file(&temp_file);
    }

    #[test]
    fn test_injected_write_failure_leaves_target_empty_at_each_position() {
        let mut plan = WritePlan::new();
        plan.add("wda.json", "{}");
        plan.add("README.md", "# Test Project");
        plan.add("docs/architecture.md", "# Architecture");
        plan.add("docs/naming.md", "# Naming");
        plan.add("tokens/tokens.json", "{}");

        assert!(
            plan.entries.len() >= 4,
            "test plan must have at least 4 entries"
        );

        for fail_pos in 0..plan.entries.len() {
            let temp_dir = std::env::temp_dir().join(format!(
                "wda_test_init_fail_pos_{}_{}",
                fail_pos,
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&temp_dir);
            fs::create_dir_all(&temp_dir).unwrap();

            let mut current_pos = 0usize;
            let mut pre_cleanup_entry_count = 0usize;

            let injected_writer =
                |r: &Path, entry: &WritePlanEntry, created: &mut Vec<PathBuf>| {
                    if current_pos == fail_pos {
                        pre_cleanup_entry_count = count_entries(r).unwrap();
                        current_pos += 1;
                        return Err(ToolFault::new(
                            format!("Injected write failure at position {fail_pos}"),
                            Some(r.to_path_buf()),
                        ));
                    }
                    current_pos += 1;
                    real_writer(r, entry, created)
                };

            let res = execute_write_plan(&temp_dir, &plan, injected_writer, real_remover);
            assert!(
                res.is_err(),
                "execute_write_plan must fail when writer fails at position {fail_pos}"
            );

            if fail_pos > 0 {
                assert!(
                    pre_cleanup_entry_count > 0,
                    "for fail_pos {fail_pos} > 0, pre-cleanup entry count must be non-zero"
                );
            }

            let post_cleanup_entry_count = count_entries(&temp_dir).unwrap();
            assert_eq!(
                post_cleanup_entry_count, 0,
                "for fail_pos {fail_pos}, target directory entry count must be 0 after cleanup"
            );

            let _ = fs::remove_dir_all(&temp_dir);
        }
    }

    #[test]
    fn test_injected_write_failure_preserves_pre_existing_notes_txt_at_each_position() {
        let mut plan = WritePlan::new();
        plan.add("wda.json", "{}");
        plan.add("README.md", "# Test Project");
        plan.add("docs/architecture.md", "# Architecture");
        plan.add("docs/naming.md", "# Naming");
        plan.add("tokens/tokens.json", "{}");
        plan.add("pages/index.html", "<!DOCTYPE html><html></html>");

        assert!(
            plan.entries.len() >= 4,
            "test plan must have at least 4 entries"
        );

        let notes_filename = "notes.txt";
        let notes_bytes = b"Pre-existing designer notes\nKeep this intact!\n";

        for fail_pos in 0..plan.entries.len() {
            let temp_dir = std::env::temp_dir().join(format!(
                "wda_test_init_notes_preserve_fail_pos_{}_{}",
                fail_pos,
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&temp_dir);
            fs::create_dir_all(&temp_dir).unwrap();

            let notes_path = temp_dir.join(notes_filename);
            fs::write(&notes_path, notes_bytes).unwrap();

            let mut current_pos = 0usize;
            let mut pre_cleanup_entry_count = 0usize;

            let injected_writer =
                |r: &Path, entry: &WritePlanEntry, created: &mut Vec<PathBuf>| {
                    if current_pos == fail_pos {
                        pre_cleanup_entry_count = count_entries(r).unwrap();
                        current_pos += 1;
                        return Err(ToolFault::new(
                            format!("Injected write failure at position {fail_pos}"),
                            Some(r.to_path_buf()),
                        ));
                    }
                    current_pos += 1;
                    real_writer(r, entry, created)
                };

            let res = execute_write_plan(&temp_dir, &plan, injected_writer, real_remover);
            assert!(
                res.is_err(),
                "execute_write_plan must fail when writer fails at position {fail_pos}"
            );

            if fail_pos > 0 {
                assert!(
                    pre_cleanup_entry_count > 1,
                    "for fail_pos {fail_pos} > 0, pre-cleanup entry count must include created entries alongside notes.txt"
                );
            }

            // Assert notes.txt survives byte for byte
            assert!(
                notes_path.exists(),
                "for fail_pos {fail_pos}, notes.txt must exist after rollback"
            );
            let read_back = fs::read(&notes_path).unwrap();
            assert_eq!(
                read_back.as_slice(),
                notes_bytes,
                "for fail_pos {fail_pos}, notes.txt must survive byte for byte"
            );

            // Assert every created entry is gone and only notes.txt remains
            let post_cleanup_entry_count = count_entries(&temp_dir).unwrap();
            assert_eq!(
                post_cleanup_entry_count, 1,
                "for fail_pos {fail_pos}, target directory entry count must be 1 (notes.txt only) after cleanup"
            );

            let remaining_entries: Vec<PathBuf> = fs::read_dir(&temp_dir)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            assert_eq!(
                remaining_entries.len(),
                1,
                "for fail_pos {fail_pos}, target directory must hold exactly 1 entry after cleanup"
            );
            assert_eq!(
                remaining_entries[0], notes_path,
                "for fail_pos {fail_pos}, the sole remaining entry must be notes.txt"
            );

            let _ = fs::remove_dir_all(&temp_dir);
        }
    }

    #[test]
    fn test_removal_sequence_is_reverse_of_creation_sequence() {
        let mut plan = WritePlan::new();
        plan.add("wda.json", "{}");
        plan.add("README.md", "# Test Project");
        plan.add("docs/architecture.md", "# Architecture");
        plan.add("docs/naming.md", "# Naming");
        plan.add("tokens/tokens.json", "{}");
        plan.add("pages/index.html", "<!DOCTYPE html><html></html>");

        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_reverse_seq_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let fail_at_position = 5usize;
        let mut current_pos = 0usize;
        let mut created_snapshot = Vec::new();
        let mut recorded_removals = Vec::new();

        let injected_writer =
            |r: &Path, entry: &WritePlanEntry, created: &mut Vec<PathBuf>| {
                if current_pos == fail_at_position {
                    created_snapshot = created.clone();
                    current_pos += 1;
                    return Err(ToolFault::new(
                        "Injected write failure for reversal test",
                        Some(r.to_path_buf()),
                    ));
                }
                current_pos += 1;
                real_writer(r, entry, created)
            };

        let injected_remover = |r: &Path, rel_path: &Path| {
            recorded_removals.push(rel_path.to_path_buf());
            real_remover(r, rel_path)
        };

        let res = execute_write_plan(&temp_dir, &plan, injected_writer, injected_remover);
        assert!(res.is_err());

        let mut expected_removal_order = created_snapshot.clone();
        expected_removal_order.reverse();

        assert_eq!(
            recorded_removals, expected_removal_order,
            "recorded removal sequence must be the exact reverse of the creation sequence"
        );

        let final_count = count_entries(&temp_dir).unwrap();
        assert_eq!(final_count, 0, "directory must be empty after cleanup");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_real_filesystem_conflict_fails_and_cleans_up_to_zero() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_real_conflict_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Order entry 1 writing regular file ahead of entry 2 writing underneath it
        let mut plan = WritePlan::new();
        plan.add("conflict_file", "file content");
        plan.add("conflict_file/underneath.txt", "child content");

        let res = execute_write_plan(&temp_dir, &plan, real_writer, real_remover);
        assert!(
            res.is_err(),
            "writing underneath a regular file on real filesystem must fail"
        );

        let entry_count = count_entries(&temp_dir).unwrap();
        assert_eq!(
            entry_count, 0,
            "real-filesystem failure must return target directory entry count to 0"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_injected_remover_failure_produces_naming_fault() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_remover_fail_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let mut plan = WritePlan::new();
        plan.add("file1.txt", "content 1");
        plan.add("docs/architecture.md", "# Architecture");
        plan.add("tokens/tokens.json", "{}");

        let mut current_pos = 0usize;
        let injected_writer =
            |r: &Path, entry: &WritePlanEntry, created: &mut Vec<PathBuf>| {
                if current_pos == 2 {
                    return Err(ToolFault::new(
                        "Injected write failure to trigger cleanup",
                        Some(r.to_path_buf()),
                    ));
                }
                current_pos += 1;
                real_writer(r, entry, created)
            };

        // Injected remover fails when asked to remove docs/architecture.md
        let injected_remover = |r: &Path, rel_path: &Path| {
            if rel_path == Path::new("docs/architecture.md") {
                return Err(ToolFault::new(
                    "Injected remover failure",
                    Some(r.to_path_buf()),
                ));
            }
            real_remover(r, rel_path)
        };

        let res = execute_write_plan(&temp_dir, &plan, injected_writer, injected_remover);
        assert!(res.is_err());
        let fault = res.unwrap_err();

        assert!(
            fault.message.contains("residual paths remain"),
            "fault message must mention residual paths: {}",
            fault.message
        );
        assert!(
            fault.message.contains("docs/architecture.md"),
            "fault message must name residual path docs/architecture.md: {}",
            fault.message
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_rollback_removes_git_repository_with_read_only_loose_objects() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_rollback_git_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let git_binary = crate::build::scripts::resolve_tool("git")
            .expect("git must be available on PATH for this test");
        let init_status = std::process::Command::new(&git_binary)
            .arg("init")
            .current_dir(&temp_dir)
            .status()
            .expect("git init must succeed");
        assert!(init_status.success(), "git init must exit with success");

        let sample_file = temp_dir.join("sample.txt");
        fs::write(&sample_file, "loose object payload for test").unwrap();
        let add_status = std::process::Command::new(&git_binary)
            .args(["add", "-A"])
            .current_dir(&temp_dir)
            .status()
            .expect("git add must succeed");
        assert!(add_status.success(), "git add must exit with success");

        let git_objects = temp_dir.join(".git").join("objects");
        assert!(git_objects.is_dir(), ".git/objects must exist");
        let mut object_count = 0usize;
        for entry in fs::read_dir(&git_objects).unwrap().flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                for obj_file in fs::read_dir(entry.path()).unwrap().flatten() {
                    let mut perms = obj_file.metadata().unwrap().permissions();
                    perms.set_readonly(true);
                    let _ = fs::set_permissions(obj_file.path(), perms);
                    object_count += 1;
                }
            }
        }
        assert!(object_count > 0, "at least one loose object must have been created");

        let created = vec![PathBuf::from(".git"), PathBuf::from("sample.txt")];
        let rollback_res = rollback(&temp_dir, &created, real_remover);
        assert!(rollback_res.is_ok(), "rollback of git repo must succeed: {:?}", rollback_res);

        assert!(!temp_dir.join(".git").exists(), ".git directory must not exist after rollback");
        assert!(!temp_dir.join("sample.txt").exists(), "sample.txt must not exist after rollback");
        let remaining = count_entries(&temp_dir).unwrap();
        assert_eq!(remaining, 0, "directory must have 0 entries after rollback, found: {remaining}");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_rollback_git_repository_preserves_preexisting_file() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_git_preserve_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let notes_file = temp_dir.join("notes.txt");
        let expected_content = "Designer notes that must survive rollback";
        fs::write(&notes_file, expected_content).unwrap();

        let git_binary = crate::build::scripts::resolve_tool("git")
            .expect("git must be available on PATH for this test");
        let init_status = std::process::Command::new(&git_binary)
            .arg("init")
            .current_dir(&temp_dir)
            .status()
            .expect("git init must succeed");
        assert!(init_status.success());

        let created_file = temp_dir.join("wda.json");
        fs::write(&created_file, "{}").unwrap();
        let add_status = std::process::Command::new(&git_binary)
            .args(["add", "-A"])
            .current_dir(&temp_dir)
            .status()
            .expect("git add must succeed");
        assert!(add_status.success());

        let created = vec![PathBuf::from(".git"), PathBuf::from("wda.json")];
        let rollback_res = rollback(&temp_dir, &created, real_remover);
        assert!(rollback_res.is_ok(), "rollback must succeed: {:?}", rollback_res);

        assert!(!temp_dir.join(".git").exists(), ".git must be removed");
        assert!(!temp_dir.join("wda.json").exists(), "wda.json must be removed");
        assert!(notes_file.exists(), "notes.txt must survive rollback");
        assert_eq!(
            fs::read_to_string(&notes_file).unwrap(),
            expected_content,
            "notes.txt content must remain unchanged"
        );
        assert_eq!(count_entries(&temp_dir).unwrap(), 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_execute_write_plan_with_created_git_rollback_on_injected_failure() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_git_plan_fail_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let git_binary = crate::build::scripts::resolve_tool("git")
            .expect("git must be available on PATH for this test");
        let init_status = std::process::Command::new(&git_binary)
            .arg("init")
            .current_dir(&temp_dir)
            .status()
            .expect("git init must succeed");
        assert!(init_status.success());

        let mut plan = WritePlan::new();
        plan.add("wda.json", "{}");
        plan.add("README.md", "# Test");
        plan.add("docs/architecture.md", "# Architecture");

        let mut current_pos = 0usize;
        let injected_writer =
            |r: &Path, entry: &WritePlanEntry, created: &mut Vec<PathBuf>| {
                if current_pos == 1 {
                    return Err(ToolFault::new(
                        "Injected write failure after git init",
                        Some(r.to_path_buf()),
                    ));
                }
                current_pos += 1;
                real_writer(r, entry, created)
            };

        let mut created = vec![PathBuf::from(".git")];
        let res = execute_write_plan_with_created(&temp_dir, &plan, &mut created, injected_writer, real_remover);
        assert!(res.is_err(), "write plan must fail on injected writer failure");

        assert!(!temp_dir.join(".git").exists(), ".git must be rolled back on plan failure");
        assert!(!temp_dir.join("wda.json").exists(), "wda.json must be rolled back");
        assert_eq!(count_entries(&temp_dir).unwrap(), 0, "directory must be clean after rollback");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_build_collision_candidates_order_minimal() {
        let dt = chrono::Utc.with_ymd_and_hms(2026, 9, 8, 12, 0, 0).unwrap();
        let plan = build_write_plan(&dt, &offline_wda_minimal());
        let candidates = build_collision_candidates(&plan, false);

        let expected = vec![
            PathBuf::from("wda.json"),
            PathBuf::from("README.md"),
            PathBuf::from("docs/architecture.md"),
            PathBuf::from("docs/design.md"),
            PathBuf::from("docs/naming.md"),
            PathBuf::from("tokens/tokens.json"),
            PathBuf::from("pages/index.html"),
            PathBuf::from(".gitignore"),
            PathBuf::from("docs"),
            PathBuf::from("tokens"),
            PathBuf::from("pages"),
            PathBuf::from(".git"),
        ];

        assert_eq!(candidates, expected);
    }

    #[test]
    fn test_build_collision_candidates_order_spectrum_two() {
        let dt = chrono::Utc.with_ymd_and_hms(2026, 9, 8, 12, 0, 0).unwrap();
        let spectrum_ds = resolve::ResolvedDesignSystem {
            name: resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME.to_string(),
            version: "2.0.0".to_string(),
            fallback_reason: None,
        };
        let plan = build_write_plan(&dt, &spectrum_ds);
        let candidates = build_collision_candidates(&plan, true);

        let expected = vec![
            PathBuf::from("wda.json"),
            PathBuf::from("README.md"),
            PathBuf::from("docs/architecture.md"),
            PathBuf::from("docs/design.md"),
            PathBuf::from("docs/naming.md"),
            PathBuf::from("tokens/tokens.json"),
            PathBuf::from("pages/index.html"),
            PathBuf::from("scripts/main.ts"),
            PathBuf::from(".gitignore"),
            PathBuf::from("docs"),
            PathBuf::from("tokens"),
            PathBuf::from("pages"),
            PathBuf::from("scripts"),
            PathBuf::from(".git"),
            PathBuf::from("package.json"),
            PathBuf::from("deno.lock"),
        ];

        assert_eq!(candidates, expected);
    }

    #[cfg(unix)]
    fn create_dangling_symlink(link: &Path, non_existent_target: &Path) {
        std::os::unix::fs::symlink(non_existent_target, link).expect("create unix symlink");
    }

    #[cfg(windows)]
    fn create_dangling_symlink(link: &Path, non_existent_target: &Path) {
        if std::os::windows::fs::symlink_file(non_existent_target, link).is_ok() {
            return;
        }
        let dummy_target = link.parent().unwrap().join("wda_dummy_target_for_dangling_link");
        let _ = fs::create_dir_all(&dummy_target);
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(&dummy_target)
            .status()
            .expect("mklink /J execution");
        assert!(status.success(), "mklink /J must succeed");
        fs::remove_dir(&dummy_target).expect("remove dummy target for junction");
    }

    #[test]
    fn test_scan_collision_candidates_reports_symlink_hit_without_following_it() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_scan_symlink_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let link_rel = PathBuf::from("docs");
        let link_full = temp_dir.join(&link_rel);
        let non_existent_target = temp_dir.join("non_existent_target_path");

        create_dangling_symlink(&link_full, &non_existent_target);

        // Following the symlink reports target does not exist
        assert!(
            !link_full.exists(),
            "target does not exist so link_full.exists() must be false when followed"
        );

        // Scanner using symlink_metadata detects the entry without following the symlink
        let candidates = vec![
            PathBuf::from("wda.json"),
            link_rel.clone(),
            PathBuf::from(".git"),
        ];
        let collisions = scan_collision_candidates(&temp_dir, &candidates)
            .expect("scanning candidates should succeed");

        assert_eq!(
            collisions,
            vec![link_rel.clone()],
            "scanner must detect the dangling symlink without following it"
        );

        let _ = real_remover(&temp_dir, &link_rel);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_scan_collision_candidates_matches_existing_entries_in_candidate_order() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_scan_order_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create pre-existing entries in arbitrary creation order
        fs::create_dir_all(temp_dir.join(".git")).unwrap();
        fs::write(temp_dir.join("README.md"), "hello").unwrap();
        fs::create_dir_all(temp_dir.join("docs")).unwrap();
        // Also create a non-candidate file that must not appear in collisions
        fs::write(temp_dir.join("notes.txt"), "notes").unwrap();

        let candidates = vec![
            PathBuf::from("wda.json"),
            PathBuf::from("README.md"),
            PathBuf::from("docs"),
            PathBuf::from(".git"),
            PathBuf::from("package.json"),
        ];

        let collisions = scan_collision_candidates(&temp_dir, &candidates)
            .expect("scanning candidates should succeed");

        assert_eq!(
            collisions,
            vec![
                PathBuf::from("README.md"),
                PathBuf::from("docs"),
                PathBuf::from(".git"),
            ],
            "collisions must preserve candidate order and omit non-candidates"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_init_succeeds_with_unrelated_existing_entries_and_preserves_content() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_init_unrelated_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let notes_path = temp_dir.join("notes.txt");
        let logo_path = temp_dir.join("logo.png");
        let hero_dir = temp_dir.join("assets");
        let hero_path = hero_dir.join("hero.png");
        let ds_store_path = temp_dir.join(".DS_Store");

        fs::create_dir_all(&hero_dir).unwrap();
        fs::write(&notes_path, b"designer notes").unwrap();
        fs::write(&logo_path, b"fake png logo").unwrap();
        fs::write(&hero_path, b"fake png hero").unwrap();
        fs::write(&ds_store_path, b"\x00\x00\x00\x01Mac OS X").unwrap();

        let result = init_project_with_resolution(&temp_dir, offline_wda_minimal());
        let (report, _) = result.expect("init must succeed when only unrelated files exist");
        assert!(
            report.diagnostics.is_empty(),
            "init report must contain zero diagnostics, got: {:?}",
            report.diagnostics
        );

        assert_eq!(fs::read(&notes_path).unwrap(), b"designer notes");
        assert_eq!(fs::read(&logo_path).unwrap(), b"fake png logo");
        assert_eq!(fs::read(&hero_path).unwrap(), b"fake png hero");
        assert_eq!(fs::read(&ds_store_path).unwrap(), b"\x00\x00\x00\x01Mac OS X");

        assert!(temp_dir.join("wda.json").exists());
        assert!(temp_dir.join("README.md").exists());
        assert!(temp_dir.join("pages/index.html").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_init_refuses_multiple_conflicts_in_candidate_order_and_leaves_entries_unchanged() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_init_multi_conflict_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let readme_path = temp_dir.join("README.md");
        let wda_json_path = temp_dir.join("wda.json");
        let notes_path = temp_dir.join("notes.txt");

        fs::write(&readme_path, "original readme").unwrap();
        fs::write(&wda_json_path, "original wda.json").unwrap();
        fs::write(&notes_path, "original notes").unwrap();

        let before_count = count_entries(&temp_dir).unwrap();
        assert_eq!(before_count, 3);

        let result = init_project_with_resolution(&temp_dir, offline_wda_minimal());
        let (report, _) = result.expect("refusal returns Ok((report, resolved_ds))");

        assert_eq!(report.diagnostics.len(), 2);

        // First collision is wda.json (index 0 in candidate plan)
        assert_eq!(report.diagnostics[0].code, codes::INIT_PATH_CONFLICT);
        assert_eq!(report.diagnostics[0].severity, Severity::Error);
        assert_eq!(
            report.diagnostics[0].location.as_ref().unwrap().path,
            Path::new("wda.json")
        );
        assert!(report.diagnostics[0].message.contains("wda.json"));

        // Second collision is README.md (index 1 in candidate plan)
        assert_eq!(report.diagnostics[1].code, codes::INIT_PATH_CONFLICT);
        assert_eq!(report.diagnostics[1].severity, Severity::Error);
        assert_eq!(
            report.diagnostics[1].location.as_ref().unwrap().path,
            Path::new("README.md")
        );
        assert!(report.diagnostics[1].message.contains("README.md"));

        assert_eq!(fs::read_to_string(&readme_path).unwrap(), "original readme");
        assert_eq!(fs::read_to_string(&wda_json_path).unwrap(), "original wda.json");
        assert_eq!(fs::read_to_string(&notes_path).unwrap(), "original notes");
        assert_eq!(count_entries(&temp_dir).unwrap(), 3);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_init_refuses_directory_conflict_docs() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_init_docs_dir_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let docs_dir = temp_dir.join("docs");
        fs::create_dir_all(&docs_dir).unwrap();
        let brief_path = docs_dir.join("brief.md");
        fs::write(&brief_path, "client brief").unwrap();

        let result = init_project_with_resolution(&temp_dir, offline_wda_minimal());
        let (report, _) = result.expect("refusal on docs/ collision");

        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].code, codes::INIT_PATH_CONFLICT);
        assert_eq!(report.diagnostics[0].severity, Severity::Error);
        assert_eq!(
            report.diagnostics[0].location.as_ref().unwrap().path,
            Path::new("docs")
        );
        assert!(report.diagnostics[0].message.contains("docs"));

        assert_eq!(fs::read_to_string(&brief_path).unwrap(), "client brief");
        assert!(!temp_dir.join("wda.json").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_init_refuses_file_named_docs() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_init_docs_file_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let docs_file = temp_dir.join("docs");
        fs::write(&docs_file, "plain file named docs").unwrap();

        let result = init_project_with_resolution(&temp_dir, offline_wda_minimal());
        let (report, _) = result.expect("refusal on docs file collision");

        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].code, codes::INIT_PATH_CONFLICT);
        assert_eq!(report.diagnostics[0].severity, Severity::Error);
        assert_eq!(
            report.diagnostics[0].location.as_ref().unwrap().path,
            Path::new("docs")
        );
        assert!(report.diagnostics[0].message.contains("docs"));

        assert_eq!(fs::read_to_string(&docs_file).unwrap(), "plain file named docs");
        assert!(!temp_dir.join("wda.json").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_init_refuses_pages_directory_conflict() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_init_pages_dir_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let pages_dir = temp_dir.join("pages");
        fs::create_dir_all(&pages_dir).unwrap();
        let about_path = pages_dir.join("about.html");
        fs::write(&about_path, "<h1>About</h1>").unwrap();

        let result = init_project_with_resolution(&temp_dir, offline_wda_minimal());
        let (report, _) = result.expect("refusal on pages/ collision");

        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].code, codes::INIT_PATH_CONFLICT);
        assert_eq!(report.diagnostics[0].severity, Severity::Error);
        assert_eq!(
            report.diagnostics[0].location.as_ref().unwrap().path,
            Path::new("pages")
        );
        assert!(report.diagnostics[0].message.contains("pages"));

        assert_eq!(fs::read_to_string(&about_path).unwrap(), "<h1>About</h1>");
        assert!(!temp_dir.join("wda.json").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_init_refuses_package_json_on_spectrum_two() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_init_pkg_json_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let pkg_json_path = temp_dir.join("package.json");
        fs::write(&pkg_json_path, r#"{"name": "pre-existing"}"#).unwrap();

        let spectrum_ds = resolve::ResolvedDesignSystem {
            name: resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME.to_string(),
            version: "2.0.0".to_string(),
            fallback_reason: None,
        };

        let result = init_project_with_resolution(&temp_dir, spectrum_ds);
        let (report, _) = result.expect("refusal on package.json collision under spectrum 2");

        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].code, codes::INIT_PATH_CONFLICT);
        assert_eq!(report.diagnostics[0].severity, Severity::Error);
        assert_eq!(
            report.diagnostics[0].location.as_ref().unwrap().path,
            Path::new("package.json")
        );
        assert!(report.diagnostics[0].message.contains("package.json"));

        assert_eq!(
            fs::read_to_string(&pkg_json_path).unwrap(),
            r#"{"name": "pre-existing"}"#
        );
        assert!(!temp_dir.join("wda.json").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_init_succeeds_with_components_directory() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_init_components_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let components_dir = temp_dir.join("components");
        fs::create_dir_all(&components_dir).unwrap();
        let old_component_path = components_dir.join("old.html");
        fs::write(&old_component_path, "<section>component</section>").unwrap();

        let result = init_project_with_resolution(&temp_dir, offline_wda_minimal());
        let (report, _) = result.expect("init must succeed when components/ exists");
        assert!(
            report.diagnostics.is_empty(),
            "init report must contain zero diagnostics, got: {:?}",
            report.diagnostics
        );

        assert_eq!(
            fs::read_to_string(&old_component_path).unwrap(),
            "<section>component</section>"
        );
        assert!(temp_dir.join("wda.json").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_init_accepts_directory_with_ds_store_and_refuses_gitignore_with_unchanged_count() {
        // Part 1: root holding .DS_Store initializes successfully
        let temp_dir_ok = std::env::temp_dir().join(format!(
            "wda_test_init_tp04_ok_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&temp_dir_ok);
        fs::create_dir_all(&temp_dir_ok).unwrap();
        fs::write(temp_dir_ok.join(".DS_Store"), b"metadata").unwrap();

        let (report_ok, _) = init_project_with_resolution(&temp_dir_ok, offline_wda_minimal())
            .expect("init on directory with .DS_Store must succeed");
        assert!(report_ok.diagnostics.is_empty());
        assert!(temp_dir_ok.join(".DS_Store").exists());
        assert_eq!(fs::read(temp_dir_ok.join(".DS_Store")).unwrap(), b"metadata");
        assert!(temp_dir_ok.join("wda.json").exists());
        let _ = fs::remove_dir_all(&temp_dir_ok);

        // Part 2: root holding .gitignore returns one Error with wda.init.path-conflict naming .gitignore, count unchanged
        let temp_dir_err = std::env::temp_dir().join(format!(
            "wda_test_init_tp04_err_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&temp_dir_err);
        fs::create_dir_all(&temp_dir_err).unwrap();
        fs::write(temp_dir_err.join(".gitignore"), b"/dist/\n").unwrap();

        let before_count = count_entries(&temp_dir_err).unwrap();
        let (report_err, _) = init_project_with_resolution(&temp_dir_err, offline_wda_minimal())
            .expect("refusal on .gitignore returns Ok((report, _))");

        assert_eq!(report_err.diagnostics.len(), 1);
        assert_eq!(report_err.diagnostics[0].code, codes::INIT_PATH_CONFLICT);
        assert_eq!(report_err.diagnostics[0].severity, Severity::Error);
        let loc = report_err.diagnostics[0].location.as_ref().expect("location must be present");
        assert_eq!(loc.path, Path::new(".gitignore"));
        assert!(report_err.diagnostics[0].message.contains(".gitignore"));

        let after_count = count_entries(&temp_dir_err).unwrap();
        assert_eq!(
            before_count, after_count,
            "entry count must be unchanged after refusal"
        );
        let _ = fs::remove_dir_all(&temp_dir_err);
    }
}


