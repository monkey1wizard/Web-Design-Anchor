//! Project initialization (`wda init`) implementation.
//!
//! Provides the one init transaction shared by every entry: collision scan, package
//! manager decision, stage install, ordered write plan execution, intermediate
//! directory tracking, reverse cleanup on failure, and residue reporting.

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
        if is_tree_entry(rel_path) {
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
/// (docs/design-systems/spectrum-adapter.md, Facet 7). Each is pinned to the exact
/// version the registry reported for the default design system, so `package.json` never
/// takes its versions from two independent decisions. This list must be kept in sync
/// with the starter script's imports; it deliberately does not enumerate transitive
/// dependencies.
const SPECTRUM_TWO_DIRECT_DEPENDENCY_PACKAGES: [&str; 6] = [
    "@spectrum-web-components/theme",
    "@spectrum-web-components/button",
    "@spectrum-web-components/textfield",
    "@spectrum-web-components/picker",
    "@spectrum-web-components/card",
    "@spectrum-web-components/dialog",
];

/// Directory inside the project root where the dedicated stage install runs.
const STAGE_DIR: &str = ".wda-init-stage";

/// Lockfiles a supported manager writes after a first install.
const LOCKFILES: [&str; 3] = ["package-lock.json", "bun.lock", "bun.lockb"];

/// Dependency state reserved on every path: install output and every supported or
/// commonly used manager's lock and state files. An existing entry is a collision, so
/// init never adopts an existing install.
const RESERVED_DEPENDENCY_STATE: [&str; 8] = [
    "wda.json",
    "node_modules",
    STAGE_DIR,
    "package-lock.json",
    "npm-shrinkwrap.json",
    "bun.lock",
    "bun.lockb",
    "yarn.lock",
];

/// Reserved entries that are removed as whole directory trees on rollback.
fn is_tree_entry(rel_path: &Path) -> bool {
    rel_path == Path::new(".git")
        || rel_path == Path::new(".git/")
        || rel_path == Path::new("node_modules")
        || rel_path == Path::new(STAGE_DIR)
}

/// Options for `wda init`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InitOptions {
    /// Package manager named with `--package-manager`; `None` lets init detect one.
    pub package_manager: Option<String>,
}

/// Result of presenting the manager menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MenuChoice {
    Selected(String),
    /// The user cancelled, or input ended (EOF), before choosing.
    Cancelled,
    /// There is no interactive terminal, so a choice cannot be asked for.
    NonInteractive,
}

/// Prints the manager menu to `out` and reads the choice from `input`.
///
/// An empty line, `q`, or end of input cancels. A number or a listed name selects.
/// Anything else asks again.
pub(crate) fn run_menu(
    names: &[String],
    input: &mut dyn io::BufRead,
    out: &mut dyn io::Write,
) -> MenuChoice {
    let _ = writeln!(out, "Several package managers are available:");
    for (index, name) in names.iter().enumerate() {
        let _ = writeln!(out, "  {}) {name}", index + 1);
    }
    loop {
        let _ = write!(out, "Choose one (number or name, empty line cancels): ");
        let _ = out.flush();
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) | Err(_) => return MenuChoice::Cancelled,
            Ok(_) => {}
        }
        let answer = line.trim();
        if answer.is_empty() || answer.eq_ignore_ascii_case("q") {
            return MenuChoice::Cancelled;
        }
        if let Ok(number) = answer.parse::<usize>() {
            if (1..=names.len()).contains(&number) {
                return MenuChoice::Selected(names[number - 1].clone());
            }
        } else if let Some(name) = names.iter().find(|name| name.as_str() == answer) {
            return MenuChoice::Selected(name.clone());
        }
        let _ = writeln!(out, "Not a listed choice.");
    }
}

/// Every external effect init performs, so tests need no real terminal, network,
/// package registry or package manager.
pub(crate) trait InitPorts {
    fn path_env(&self) -> Option<std::ffi::OsString>;
    fn home_env(&self) -> Option<std::ffi::OsString> {
        None
    }
    fn git_config_global_env(&self) -> Option<std::ffi::OsString> {
        None
    }
    /// Reads registry metadata for `package`. Metadata only, never a download.
    fn registry_metadata(&self, package: &str) -> Result<serde_json::Value, resolve::ManagerError>;
    /// Finds `name` on PATH and probes its exact version. Never downloads.
    fn probe_manager(
        &self,
        name: &str,
        cwd: &Path,
    ) -> Result<resolve::PackageManager, resolve::ManagerError>;
    /// Runs the manager's install in the stage directory `cwd`.
    fn install(
        &self,
        manager: &resolve::PackageManager,
        cwd: &Path,
    ) -> Result<(), resolve::ManagerError>;
    /// Restores the embedded tool cache when it is missing.
    fn ensure_tools(&self) -> Result<(), String>;
    /// Asks which of several available managers to use.
    fn choose_manager(&self, names: &[String]) -> MenuChoice;
    fn write(
        &self,
        root: &Path,
        entry: &WritePlanEntry,
        created: &mut Vec<PathBuf>,
    ) -> Result<(), ToolFault> {
        real_writer(root, entry, created)
    }
    fn remove(&self, root: &Path, rel_path: &Path) -> Result<(), ToolFault> {
        real_remover(root, rel_path)
    }
}

/// Production ports: real PATH, registry, managers, tool cache and terminal.
struct SystemPorts;

impl InitPorts for SystemPorts {
    fn path_env(&self) -> Option<std::ffi::OsString> {
        std::env::var_os("PATH")
    }

    fn registry_metadata(&self, package: &str) -> Result<serde_json::Value, resolve::ManagerError> {
        let registry = std::env::var("npm_config_registry")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "https://registry.npmjs.org/".to_string());
        resolve::registry_metadata(&registry, package)
    }

    fn probe_manager(
        &self,
        name: &str,
        cwd: &Path,
    ) -> Result<resolve::PackageManager, resolve::ManagerError> {
        use crate::tool_probe::{resolve_tool_candidate_with_path, ToolCandidateResolution};
        match resolve_tool_candidate_with_path(name, self.path_env().as_deref()) {
            ToolCandidateResolution::Startable(path) => {
                resolve::PackageManager::probe(name, &path, cwd)
            }
            ToolCandidateResolution::NotFound | ToolCandidateResolution::InvalidName(_) => {
                Err(resolve::ManagerError::NotFound)
            }
            ToolCandidateResolution::FoundButUnstartable(diagnostic) => {
                Err(resolve::ManagerError::CannotStart(diagnostic.message))
            }
        }
    }

    fn install(
        &self,
        manager: &resolve::PackageManager,
        cwd: &Path,
    ) -> Result<(), resolve::ManagerError> {
        manager.install(cwd)
    }

    fn ensure_tools(&self) -> Result<(), String> {
        if crate::tools::EMBEDDED_ARCHIVE.is_some() {
            return Ok(());
        }
        match crate::tools::cache_status() {
            Ok(Some(_)) => Ok(()),
            _ => crate::tools::fetch().map(|_| ()),
        }
    }

    fn choose_manager(&self, names: &[String]) -> MenuChoice {
        use io::IsTerminal;
        if !(io::stdin().is_terminal() && io::stderr().is_terminal()) {
            return MenuChoice::NonInteractive;
        }
        let stdin = io::stdin();
        let mut input = stdin.lock();
        let mut out = io::stderr();
        run_menu(names, &mut input, &mut out)
    }
}

/// Ports for callers that pass an already-decided design system: no registry, no
/// manager, no tool fetch. Only PATH (for git) and the git identity variables are real.
#[cfg(test)]
struct DecidedPorts {
    path_env: Option<std::ffi::OsString>,
    home_env: Option<std::ffi::OsString>,
    git_config_global_env: Option<std::ffi::OsString>,
}

#[cfg(test)]
impl InitPorts for DecidedPorts {
    fn path_env(&self) -> Option<std::ffi::OsString> {
        self.path_env.clone()
    }
    fn home_env(&self) -> Option<std::ffi::OsString> {
        self.home_env.clone()
    }
    fn git_config_global_env(&self) -> Option<std::ffi::OsString> {
        self.git_config_global_env.clone()
    }
    fn registry_metadata(&self, _package: &str) -> Result<serde_json::Value, resolve::ManagerError> {
        Err(resolve::ManagerError::RegistryUnreachable(
            "no registry access in this configuration".to_string(),
        ))
    }
    fn probe_manager(
        &self,
        _name: &str,
        _cwd: &Path,
    ) -> Result<resolve::PackageManager, resolve::ManagerError> {
        Err(resolve::ManagerError::NotFound)
    }
    fn install(
        &self,
        _manager: &resolve::PackageManager,
        _cwd: &Path,
    ) -> Result<(), resolve::ManagerError> {
        Err(resolve::ManagerError::NotFound)
    }
    fn ensure_tools(&self) -> Result<(), String> {
        Ok(())
    }
    fn choose_manager(&self, _names: &[String]) -> MenuChoice {
        MenuChoice::NonInteractive
    }
}

/// Builds the exact-version dependency list for the default Spectrum 2 and Lit project.
fn spectrum_two_dependencies(version: &str, lit_version: &str) -> Vec<(String, String)> {
    let mut dependencies: Vec<(String, String)> = SPECTRUM_TWO_DIRECT_DEPENDENCY_PACKAGES
        .iter()
        .map(|package| ((*package).to_string(), version.to_string()))
        .collect();
    dependencies.push(("lit".to_string(), lit_version.to_string()));
    dependencies
}

/// Assembles the ordered write plan for project initialization.
///
/// `.gitignore` is first, so it exists before any `git add`. `dependencies` and
/// `package_manager` (`name@exact-version`) go into `package.json`; the Minimal plan
/// passes an empty list and `None`. The resolved design system's name selects the
/// tokens, starter page and (for Spectrum 2) starter script assets.
pub(crate) fn build_write_plan_with<Tz: chrono::TimeZone>(
    dt: &chrono::DateTime<Tz>,
    resolved_ds: &resolve::ResolvedDesignSystem,
    dependencies: &[(String, String)],
    package_manager: Option<&str>,
) -> WritePlan
where
    Tz::Offset: std::fmt::Display,
{
    let mut plan = WritePlan::new();
    let doc_date = documents::format_document_date(dt);
    let is_spectrum_two = resolved_ds.name == resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME;

    plan.add(".gitignore", documents::gitignore_content());
    plan.add(
        "anchor.json",
        manifest::generate_manifest(dt, resolved_ds, resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME),
    );
    plan.add(
        "package.json",
        manifest::generate_package_manifest(dependencies, package_manager),
    );
    plan.add("README.md", documents::render_readme());
    plan.add(
        "docs/architecture.md",
        documents::render_architecture_doc(&doc_date),
    );
    plan.add(
        "docs/design.md",
        documents::render_design_doc(&doc_date, &resolved_ds.name),
    );
    plan.add(
        "docs/glossary.md",
        documents::render_glossary_doc(&doc_date, &resolved_ds.name),
    );

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

    plan
}

/// Assembles the write plan with no dependencies and no recorded package manager.
#[cfg(test)]
pub(crate) fn build_write_plan<Tz: chrono::TimeZone>(
    dt: &chrono::DateTime<Tz>,
    resolved_ds: &resolve::ResolvedDesignSystem,
) -> WritePlan
where
    Tz::Offset: std::fmt::Display,
{
    build_write_plan_with(dt, resolved_ds, &[], None)
}

/// Builds the ordered list of collision candidates for a project initialization run.
///
/// Returns write-plan files in plan order, then the intermediate directories those
/// files need, then `.git/`, followed by the reserved dependency state: legacy
/// `wda.json`, `node_modules`, the stage directory and every lock file of a supported
/// or commonly used manager.
pub(crate) fn build_collision_candidates(plan: &WritePlan) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    for entry in &plan.entries {
        candidates.push(entry.relative_path.clone());
    }

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

    candidates.push(PathBuf::from(".git"));

    for reserved in RESERVED_DEPENDENCY_STATE {
        let reserved = PathBuf::from(reserved);
        if !candidates.contains(&reserved) {
            candidates.push(reserved);
        }
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

type InitResult = Result<(ValidationReport, resolve::ResolvedDesignSystem), ToolFault>;

const SKILL_HINT: &str =
    "WDA Minimal was created. Ask the WDA Skill to add Spectrum 2 and Lit later";

/// Collapses `text` to one bounded line so third-party output cannot flood a reason.
fn one_line(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() > 300 {
        let mut cut: String = collapsed.chars().take(300).collect();
        cut.push_str("...");
        cut
    } else {
        collapsed
    }
}

fn describe_manager_error(error: &resolve::ManagerError) -> String {
    use resolve::ManagerError as E;
    match error {
        E::NotFound => "not found on PATH".to_string(),
        E::CannotStart(detail) => format!("found but could not start: {}", one_line(detail)),
        E::VersionFailure(detail) => format!("version check failed: {}", one_line(detail)),
        E::InstallFailure(detail) => format!("install failed: {}", one_line(detail)),
        E::RegistryUnreachable(detail) => format!("registry unreachable ({})", one_line(detail)),
        E::RegistryHttp(status) => format!("registry answered HTTP {status}"),
        E::RegistryTls(detail) => format!("registry TLS or client failure ({})", one_line(detail)),
        E::RegistryFormat(detail) => {
            format!("registry response was malformed ({})", one_line(detail))
        }
    }
}

fn fallback_reason(cause: &str) -> String {
    format!("{cause}. {SKILL_HINT}")
}

/// Reads the exact release version of the registry `latest` dist-tag.
fn latest_release_version(metadata: &serde_json::Value) -> Result<String, String> {
    let latest = metadata
        .pointer("/dist-tags/latest")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "the registry response has no 'latest' dist-tag".to_string())?;
    let parts: Vec<&str> = latest.split('.').collect();
    let exact = parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
    if !exact {
        return Err(format!("the latest dist-tag '{}' is not an exact release", one_line(latest)));
    }
    if parts[0].parse::<u64>().unwrap_or(0) < 1 {
        return Err(format!("the latest dist-tag '{latest}' is below 1.0.0"));
    }
    Ok(latest.to_string())
}

/// What the manager and registry probes decided.
enum Decision {
    Minimal(resolve::ResolvedDesignSystem),
    Default {
        design_system: resolve::ResolvedDesignSystem,
        manager: resolve::PackageManager,
        dependencies: Vec<(String, String)>,
    },
    Cancelled,
}

/// Reads one package's exact release version from the registry, recording in `online`
/// whether the registry answered at all (an HTTP or format error still means reachable).
fn registry_release_version(
    ports: &dyn InitPorts,
    package: &str,
    online: &mut bool,
) -> Result<String, String> {
    match ports.registry_metadata(package) {
        Ok(metadata) => {
            *online = true;
            latest_release_version(&metadata)
        }
        Err(error) => {
            if !matches!(error, resolve::ManagerError::RegistryUnreachable(_)) {
                *online = true;
            }
            Err(describe_manager_error(&error))
        }
    }
}

/// Decides between the default Spectrum 2 and Lit project and WDA Minimal, and reports
/// whether the registry was reachable.
///
/// Managers are probed first (probing never downloads and never touches the registry);
/// with none usable the result is Minimal. Then the registry is probed: when it is
/// unreachable the result is Minimal whatever a local package cache holds. A named
/// manager that is unavailable never switches to another. Several available managers
/// are asked for only once the registry is known to be reachable.
fn decide(
    root: &Path,
    options: &InitOptions,
    ports: &dyn InitPorts,
) -> Result<(Decision, bool), ToolFault> {
    let supported = resolve::supported_package_managers();
    let named = options.package_manager.as_deref();
    if let Some(name) = named {
        if !supported.iter().any(|candidate| candidate == name) {
            return Err(ToolFault::new(
                format!(
                    "Unknown package manager '{}'; supported: {}",
                    one_line(name),
                    supported.join(", ")
                ),
                Some(root.to_path_buf()),
            ));
        }
    }

    let mut candidates: Vec<String> = match named {
        Some(name) => vec![name.to_string()],
        None => supported.clone(),
    };
    candidates.sort();
    let mut available = Vec::new();
    let mut failures = Vec::new();
    for name in &candidates {
        match ports.probe_manager(name, root) {
            Ok(manager) => available.push(manager),
            Err(error) => failures.push(format!("{name}: {}", describe_manager_error(&error))),
        }
    }
    if available.is_empty() {
        let cause = if named.is_some() {
            format!("Package manager {} is not available", failures.join("; "))
        } else {
            format!(
                "No supported package manager is available ({})",
                failures.join("; ")
            )
        };
        return Ok((
            Decision::Minimal(resolve::builtin_fallback(&fallback_reason(&cause))),
            false,
        ));
    }

    let mut online = false;
    let versions = registry_release_version(ports, resolve::SPECTRUM_NPM_PACKAGE, &mut online)
        .map_err(|cause| format!("Default Spectrum 2 and Lit were not resolved: {cause}"))
        .and_then(|spectrum| {
            registry_release_version(ports, "lit", &mut online)
                .map(|lit| (spectrum, lit))
                .map_err(|cause| format!("Lit version was not resolved: {cause}"))
        });
    let (spectrum_version, lit_version) = match versions {
        Ok(pair) => pair,
        Err(cause) => {
            return Ok((
                Decision::Minimal(resolve::builtin_fallback(&fallback_reason(&cause))),
                online,
            ));
        }
    };

    let manager = if available.len() == 1 {
        available.remove(0)
    } else {
        let names: Vec<String> = available.iter().map(|m| m.name.clone()).collect();
        match ports.choose_manager(&names) {
            MenuChoice::Selected(chosen) => {
                match available.into_iter().find(|manager| manager.name == chosen) {
                    Some(manager) => manager,
                    None => return Ok((Decision::Cancelled, online)),
                }
            }
            MenuChoice::Cancelled => return Ok((Decision::Cancelled, online)),
            MenuChoice::NonInteractive => {
                return Err(ToolFault::new(
                    format!(
                        "Several package managers are available ({}) and there is no interactive terminal; pass --package-manager <name> to choose one",
                        names.join(", ")
                    ),
                    Some(root.to_path_buf()),
                ));
            }
        }
    };

    Ok((
        Decision::Default {
            design_system: resolve::ResolvedDesignSystem {
                name: resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME.to_string(),
                version: spectrum_version.clone(),
                fallback_reason: None,
            },
            dependencies: spectrum_two_dependencies(&spectrum_version, &lit_version),
            manager,
        },
        online,
    ))
}

fn collision_report(collisions: Vec<PathBuf>) -> ValidationReport {
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
    report
}

/// Removes everything this run created, then returns `fault`. A cleanup failure is the
/// error instead, naming the residual paths.
fn fail_after_cleanup<T>(
    root: &Path,
    created: &[PathBuf],
    ports: &dyn InitPorts,
    fault: ToolFault,
) -> Result<T, ToolFault> {
    rollback(root, created, |base, rel| ports.remove(base, rel))?;
    Err(fault)
}

fn apply_git_env(cmd: &mut std::process::Command, ports: &dyn InitPorts) {
    if let Some(path) = ports.path_env() {
        cmd.env("PATH", path);
    }
    if let Some(home) = ports.home_env() {
        cmd.env("HOME", home);
    }
    if let Some(global) = ports.git_config_global_env() {
        cmd.env("GIT_CONFIG_GLOBAL", global);
    }
}

/// Runs one git invocation, mapping a launch failure or non-zero exit to a message.
fn run_git(
    git: &Path,
    ports: &dyn InitPorts,
    root: &Path,
    args: &[&str],
    label: &str,
) -> Result<std::process::Output, String> {
    let mut cmd = std::process::Command::new(git);
    cmd.args(args).current_dir(root);
    apply_git_env(&mut cmd, ports);
    cmd.output()
        .map_err(|e| format!("Failed to execute '{label}': {e}"))
}

fn git_failure_message(label: &str, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let trimmed = stderr.trim();
    if trimmed.is_empty() {
        format!("'{label}' failed with status {}", output.status)
    } else {
        format!("'{label}' failed: {trimmed}")
    }
}

/// Locates a startable git. Missing or unlaunchable git is only a Warning, so this
/// returns the warning text instead of failing.
fn locate_git(ports: &dyn InitPorts) -> Result<PathBuf, String> {
    use crate::tool_probe::{resolve_tool_candidate_with_path, ToolCandidateResolution};
    match resolve_tool_candidate_with_path("git", ports.path_env().as_deref()) {
        ToolCandidateResolution::Startable(path) => Ok(path),
        ToolCandidateResolution::NotFound => Err("Required tool 'git' was not found on PATH".to_string()),
        ToolCandidateResolution::FoundButUnstartable(diagnostic)
        | ToolCandidateResolution::InvalidName(diagnostic) => Err(diagnostic.message),
    }
}

/// Runs the whole init transaction.
///
/// `decided` bypasses the registry and manager probes (used by callers that already
/// know the design system). Every path shares this one transaction: collision scan,
/// decision, stage install, git init, write plan, commit of install output, first
/// commit, then the best-effort tool-cache restore.
fn init_inner<Tz: chrono::TimeZone>(
    root: &Path,
    dt: &chrono::DateTime<Tz>,
    decided: Option<resolve::ResolvedDesignSystem>,
    options: &InitOptions,
    ports: &dyn InitPorts,
) -> InitResult
where
    Tz::Offset: std::fmt::Display,
{
    if !root.is_dir() {
        return Err(ToolFault::new(
            "Project root directory is invalid or unreadable",
            Some(root.to_path_buf()),
        ));
    }

    let placeholder = || resolve::builtin_fallback("init refused before any file was written");
    let minimal_probe = resolve::builtin_fallback("collision probe");
    let probe_plan = build_write_plan_with(dt, &minimal_probe, &[], None);
    let collisions = scan_collision_candidates(root, &build_collision_candidates(&probe_plan))?;
    if !collisions.is_empty() {
        return Ok((collision_report(collisions), decided.unwrap_or_else(placeholder)));
    }

    let (decision, online) = match decided {
        Some(design_system) => (Decision::Minimal(design_system), false),
        None => decide(root, options, ports)?,
    };

    let mut report = ValidationReport::new();
    let mut created: Vec<PathBuf> = Vec::new();
    let resolved_ds;
    let dependencies;
    let package_manager_field;
    let mut stage_result: Option<PathBuf> = None;

    match decision {
        Decision::Cancelled => {
            let mut report = ValidationReport::new();
            report.add(Diagnostic::new(
                codes::CLI_USAGE,
                Severity::Error,
                None,
                "Package manager choice was cancelled; no project was created",
                "Run 'wda init' again and choose a package manager, or pass --package-manager <name>",
            ));
            return Ok((report, placeholder()));
        }
        Decision::Minimal(design_system) => {
            resolved_ds = design_system;
            dependencies = Vec::new();
            package_manager_field = None;
        }
        Decision::Default {
            design_system,
            manager,
            dependencies: default_dependencies,
        } => {
            let spectrum_plan = build_write_plan_with(dt, &design_system, &[], None);
            let collisions =
                scan_collision_candidates(root, &build_collision_candidates(&spectrum_plan))?;
            if !collisions.is_empty() {
                return Ok((collision_report(collisions), design_system));
            }

            let manager_field = format!("{}@{}", manager.name, manager.version);
            let stage = PathBuf::from(STAGE_DIR);
            let stage_plan = {
                let mut plan = WritePlan::new();
                plan.add(
                    stage.join("package.json"),
                    manifest::generate_package_manifest(&default_dependencies, Some(&manager_field)),
                );
                plan
            };
            let mut staged = Ok(());
            if let Err(fault) = fs::create_dir(root.join(&stage)).map_err(|e| {
                ToolFault::new(
                    format!("Failed to create install stage: {e}"),
                    Some(root.to_path_buf()),
                )
            }) {
                staged = Err(fault);
            } else {
                created.push(stage.clone());
                for entry in &stage_plan.entries {
                    if let Err(fault) = ports.write(root, entry, &mut created) {
                        staged = Err(fault);
                        break;
                    }
                }
            }
            if let Err(fault) = staged {
                return fail_after_cleanup(root, &created, ports, fault);
            }

            let stage_abs = root.join(&stage);
            let installed = ports.install(&manager, &stage_abs).and_then(|()| {
                let has_modules = fs::symlink_metadata(stage_abs.join("node_modules")).is_ok();
                let lock = LOCKFILES
                    .iter()
                    .find(|name| fs::symlink_metadata(stage_abs.join(name)).is_ok());
                match (has_modules, lock) {
                    (true, Some(name)) => Ok(PathBuf::from(name)),
                    _ => Err(resolve::ManagerError::InstallFailure(
                        "the manager reported success but left no node_modules or lockfile"
                            .to_string(),
                    )),
                }
            });
            match installed {
                Ok(lock) => {
                    stage_result = Some(lock);
                    resolved_ds = design_system;
                    dependencies = default_dependencies;
                    package_manager_field = Some(manager_field);
                }
                Err(error) => {
                    // Nothing of the default project exists outside the stage, so
                    // removing the stage removes this run's Spectrum-only content.
                    rollback(root, &created, |base, rel| ports.remove(base, rel))?;
                    created.clear();
                    resolved_ds = resolve::builtin_fallback(&fallback_reason(&format!(
                        "Installing the default packages with {} failed: {}",
                        manager.name,
                        describe_manager_error(&error)
                    )));
                    dependencies = Vec::new();
                    package_manager_field = None;
                }
            }
        }
    }

    // Transition shim: callers that pre-decide the design system with the retired
    // Deno-absent reason still get the Deno-missing Warning that build fixtures count.
    // The new decision path never produces that reason.
    if resolved_ds
        .fallback_reason
        .as_deref()
        .is_some_and(|reason| reason.starts_with(resolve::FALLBACK_REASON_DENO_ABSENT))
    {
        report.add(Diagnostic::new(
            codes::TOOLCHAIN_DENO_MISSING,
            Severity::Warning,
            None,
            "Deno was not found on PATH. Init used WDA Minimal.",
            "Deno is no longer needed to initialize a project",
        ));
    }

    let plan = build_write_plan_with(
        dt,
        &resolved_ds,
        &dependencies,
        package_manager_field.as_deref(),
    );

    let git = match locate_git(ports) {
        Ok(path) => Some(path),
        Err(cause) => {
            report.add(Diagnostic::new(
                codes::TOOLCHAIN_GIT_MISSING,
                Severity::Warning,
                None,
                format!("{cause}. The project was created without a git repository or first commit"),
                "Install git and run 'git init' in the project, or ask the WDA Skill to help",
            ));
            None
        }
    };

    if let Some(git) = &git {
        match run_git(git, ports, root, &["init"], "git init") {
            Ok(output) if output.status.success() => created.push(PathBuf::from(".git")),
            Ok(output) => {
                let fault = ToolFault::new(
                    git_failure_message("git init", &output),
                    Some(root.to_path_buf()),
                );
                if root.join(".git").exists() {
                    created.push(PathBuf::from(".git"));
                }
                return fail_after_cleanup(root, &created, ports, fault);
            }
            Err(message) => {
                let fault = ToolFault::new(message, Some(root.to_path_buf()));
                return fail_after_cleanup(root, &created, ports, fault);
            }
        }
    }

    // `.gitignore` is the first plan entry, so it exists before any `git add`.
    for entry in &plan.entries {
        if let Err(fault) = ports.write(root, entry, &mut created) {
            return fail_after_cleanup(root, &created, ports, fault);
        }
    }

    if let Some(lock) = stage_result {
        let stage = PathBuf::from(STAGE_DIR);
        // Recheck right before the commit: another process may have created install
        // state since the first scan. Anything found here is not ours to replace.
        let late = scan_collision_candidates(
            root,
            &["node_modules", "package-lock.json", "bun.lock", "bun.lockb"]
                .map(PathBuf::from),
        )?;
        if !late.is_empty() {
            rollback(root, &created, |base, rel| ports.remove(base, rel))?;
            return Ok((collision_report(late), resolved_ds));
        }
        let moves = [
            (stage.join("node_modules"), PathBuf::from("node_modules")),
            (stage.join(&lock), lock.clone()),
        ];
        for (from, to) in moves {
            if let Err(error) = fs::rename(root.join(&from), root.join(&to)) {
                let fault = ToolFault::new(
                    format!("Failed to commit install output {}: {error}", to.display()),
                    Some(root.to_path_buf()),
                );
                return fail_after_cleanup(root, &created, ports, fault);
            }
            created.push(to);
        }
        // The stage still holds its package.json; removing it is a required cleanup.
        let stage_index = created.iter().position(|p| p == &stage);
        if let Err(cleanup) = rollback(root, &[stage.clone()], |base, rel| ports.remove(base, rel)) {
            let mut remaining = created.clone();
            if let Some(index) = stage_index {
                remaining.remove(index);
            }
            return match rollback(root, &remaining, |base, rel| ports.remove(base, rel)) {
                Ok(()) => Err(cleanup),
                Err(other) => Err(other),
            };
        }
        created.retain(|p| p != &stage);
    }

    if let Some(git) = &git {
        match run_git(git, ports, root, &["add", "-A"], "git add -A") {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                let fault = ToolFault::new(
                    git_failure_message("git add -A", &output),
                    Some(root.to_path_buf()),
                );
                return fail_after_cleanup(root, &created, ports, fault);
            }
            Err(message) => {
                let fault = ToolFault::new(message, Some(root.to_path_buf()));
                return fail_after_cleanup(root, &created, ports, fault);
            }
        }

        let commit = match run_git(git, ports, root, &["commit", "-m", "wda init"], "git commit") {
            Ok(output) => output,
            Err(message) => {
                let fault = ToolFault::new(message, Some(root.to_path_buf()));
                return fail_after_cleanup(root, &created, ports, fault);
            }
        };
        if !commit.status.success() {
            let stderr = String::from_utf8_lossy(&commit.stderr);
            // Global git configuration is never written: the fallback identity is
            // passed through `-c` on this one invocation only. A retry happens only
            // for the "no identity configured" failure; any other refusal (a
            // `commit.gpgsign` requirement, a pre-commit hook, etc.) is reported
            // as-is with git's own stderr.
            let looks_like_missing_identity = stderr.contains("Please tell me who you are")
                || stderr.contains("user.email")
                || stderr.contains("user.name");
            if !looks_like_missing_identity {
                let fault = ToolFault::new(
                    git_failure_message("git commit", &commit),
                    Some(root.to_path_buf()),
                );
                return fail_after_cleanup(root, &created, ports, fault);
            }
            let fallback = run_git(
                git,
                ports,
                root,
                &[
                    "-c",
                    "user.name=WDA",
                    "-c",
                    "user.email=wda@localhost",
                    "commit",
                    "-m",
                    "wda init",
                ],
                "git commit with fallback identity",
            );
            match fallback {
                Ok(output) if output.status.success() => {
                    report.add(Diagnostic::new(
                        codes::INIT_GIT_IDENTITY_FALLBACK,
                        Severity::Warning,
                        None,
                        "No git identity was configured, so the initial commit was authored as WDA <wda@localhost>",
                        "Configure 'git config user.name' and 'git config user.email' to author future commits under your own identity",
                    ));
                }
                Ok(output) => {
                    let fault = ToolFault::new(
                        git_failure_message("git commit with fallback identity", &output),
                        Some(root.to_path_buf()),
                    );
                    return fail_after_cleanup(root, &created, ports, fault);
                }
                Err(message) => {
                    let fault = ToolFault::new(message, Some(root.to_path_buf()));
                    return fail_after_cleanup(root, &created, ports, fault);
                }
            }
        }
    }

    // The project is complete and usable at this point. Restoring the embedded tool
    // cache is best effort and only tried when this run saw the registry answer: a
    // local cache never changes the decision above, and an offline run does not wait
    // on a download. A failure is a Warning, never an init failure.
    if let Err(cause) = if online { ports.ensure_tools() } else { Ok(()) } {
        report.add(Diagnostic::new(
            codes::INIT_TOOL_UNAVAILABLE,
            Severity::Warning,
            None,
            format!(
                "The TypeScript tool package could not be restored: {}",
                one_line(&cause)
            ),
            "Run 'wda tools fetch' when online; the project is otherwise complete",
        ));
    }

    Ok((report, resolved_ds))
}

/// Seam to initialize a project at `root` with a specific date-time and options.
pub(crate) fn init_project_with_time_and_options<Tz: chrono::TimeZone>(
    root: &Path,
    dt: &chrono::DateTime<Tz>,
    options: &InitOptions,
) -> InitResult
where
    Tz::Offset: std::fmt::Display,
{
    init_inner(root, dt, None, options, &SystemPorts)
}

/// Seam to initialize a project at `root` with a specific date-time.
///
/// Checks that `root` has no colliding paths with the write plan, the reserved
/// dependency state or an initialized repository. If any candidate path already
/// exists, returns a `ValidationReport` with Errors carrying
/// [`codes::INIT_PATH_CONFLICT`] for each colliding path in candidate order.
#[cfg(test)]
pub fn init_project_with_time<Tz: chrono::TimeZone>(
    root: &Path,
    dt: &chrono::DateTime<Tz>,
) -> InitResult
where
    Tz::Offset: std::fmt::Display,
{
    init_project_with_time_and_options(root, dt, &InitOptions::default())
}

/// Variant of [`init_project_with_time`] accepting a pre-decided design system,
/// bypassing the registry and manager probes.
///
/// Callers pass `Some(resolved_ds)` to stay offline and deterministic: the project is
/// created as that design system with no package install and no tool fetch.
#[cfg(test)]
pub(crate) fn init_project_with_time_and_resolution<Tz: chrono::TimeZone>(
    root: &Path,
    dt: &chrono::DateTime<Tz>,
    resolved_ds: Option<resolve::ResolvedDesignSystem>,
) -> InitResult
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
#[cfg(test)]
pub(crate) fn init_project_with_time_resolution_and_path<Tz: chrono::TimeZone>(
    root: &Path,
    dt: &chrono::DateTime<Tz>,
    resolved_ds: Option<resolve::ResolvedDesignSystem>,
    path_env: Option<&std::ffi::OsStr>,
) -> InitResult
where
    Tz::Offset: std::fmt::Display,
{
    init_project_with_time_path_and_git_env(root, dt, resolved_ds, path_env, None, None)
}

/// Variant additionally accepting explicit `HOME` and `GIT_CONFIG_GLOBAL` overrides for
/// the git child processes, so tests can isolate git identity resolution without
/// mutating the process environment (racy under parallel `cargo test`).
#[cfg(test)]
pub(crate) fn init_project_with_time_path_and_git_env<Tz: chrono::TimeZone>(
    root: &Path,
    dt: &chrono::DateTime<Tz>,
    resolved_ds: Option<resolve::ResolvedDesignSystem>,
    path_env: Option<&std::ffi::OsStr>,
    home_env: Option<&std::ffi::OsStr>,
    git_config_global_env: Option<&std::ffi::OsStr>,
) -> InitResult
where
    Tz::Offset: std::fmt::Display,
{
    let decided = resolved_ds.unwrap_or_else(|| {
        resolve::builtin_fallback("Default packages were not requested by this caller")
    });
    let ports = DecidedPorts {
        path_env: path_env.map(|value| value.to_os_string()),
        home_env: home_env.map(|value| value.to_os_string()),
        git_config_global_env: git_config_global_env.map(|value| value.to_os_string()),
    };
    init_inner(root, dt, Some(decided), &InitOptions::default(), &ports)
}

/// Seam to initialize a project at `root`.
///
/// Equivalent to [`init_project_with_options`] with default options: detect the
/// package manager and fall back to WDA Minimal when the default cannot be installed.
pub fn init_project(root: &Path) -> InitResult {
    init_project_with_options(root, &InitOptions::default())
}

/// Seam to initialize a project at `root` with explicit [`InitOptions`].
pub fn init_project_with_options(root: &Path, options: &InitOptions) -> InitResult {
    let now = chrono::Local::now();
    init_project_with_time_and_options(root, &now, options)
}

/// Variant of [`init_project`] accepting a pre-resolved design system, for
/// deterministic, offline-only initialization in tests. See
/// [`init_project_with_time_and_resolution`].
#[cfg(test)]
pub(crate) fn init_project_with_resolution(
    root: &Path,
    resolved_ds: resolve::ResolvedDesignSystem,
) -> InitResult {
    let now = chrono::Local::now();
    init_project_with_time_and_resolution(root, &now, Some(resolved_ds))
}

/// Variant of [`init_project_with_resolution`] accepting an explicit PATH environment value.
#[cfg(test)]
pub(crate) fn init_project_with_resolution_and_path(
    root: &Path,
    resolved_ds: resolve::ResolvedDesignSystem,
    path_env: Option<&std::ffi::OsStr>,
) -> InitResult {
    let now = chrono::Local::now();
    init_project_with_time_resolution_and_path(root, &now, Some(resolved_ds), path_env)
}

/// Variant of [`init_project_with_resolution_and_path`] additionally accepting explicit
/// `HOME` and `GIT_CONFIG_GLOBAL` overrides for the git child processes.
#[cfg(test)]
pub(crate) fn init_project_with_resolution_path_and_git_env(
    root: &Path,
    resolved_ds: resolve::ResolvedDesignSystem,
    path_env: Option<&std::ffi::OsStr>,
    home_env: Option<&std::ffi::OsStr>,
    git_config_global_env: Option<&std::ffi::OsStr>,
) -> InitResult {
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

    fn has_no_errors_or_unexpected_init_diagnostics(report: &ValidationReport) -> bool {
        report
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != Severity::Error)
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
            has_no_errors_or_unexpected_init_diagnostics(&report),
            "init report must contain zero diagnostics, got: {:?}",
            report.diagnostics
        );

        assert!(temp_dir.join(".hidden").exists(), ".hidden must be preserved");
        assert_eq!(
            fs::read_to_string(temp_dir.join(".hidden")).unwrap(),
            "content",
            "content of pre-existing hidden file must remain unchanged"
        );
        assert!(temp_dir.join("anchor.json").exists(), "anchor.json must be written");
        assert!(temp_dir.join("package.json").exists(), "package.json must be written");

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
    fn test_init_commits_whole_directory_with_wda_init_message_and_no_remote() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_commit_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("notes.txt"), "designer notes").unwrap();

        let (report, _) = init_project_with_resolution(&temp_dir, offline_wda_minimal())
            .expect("init with a pre-existing identity must succeed");
        assert!(
            has_no_errors_or_unexpected_init_diagnostics(&report),
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

        let identity_diagnostic = report
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == codes::INIT_GIT_IDENTITY_FALLBACK)
            .expect("report must retain the identity-fallback diagnostic");
        assert_eq!(identity_diagnostic.severity, Severity::Warning);
        assert!(
            report.diagnostics.iter().all(|diagnostic| {
                diagnostic.code == codes::INIT_GIT_IDENTITY_FALLBACK
                    || diagnostic.code == codes::TOOLCHAIN_DENO_MISSING
            }),
            "report must contain no other diagnostics, got: {:?}",
            report.diagnostics
        );
        assert!(report.diagnostics.len() <= 2);
        assert!(
            identity_diagnostic.message.contains("WDA")
                && identity_diagnostic.message.contains("wda@localhost"),
            "fallback report line must name the WDA <wda@localhost> identity, got: {}",
            identity_diagnostic.message
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
    fn test_init_project_empty_directory_produces_nine_paths_and_validates_clean() {
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
            Some(resolve::FALLBACK_REASON_DENO_ABSENT),
            "a pre-decided design system is reported back unchanged"
        );
        assert!(
            has_no_errors_or_unexpected_init_diagnostics(&report),
            "init report must contain zero diagnostics: {:?}",
            report.diagnostics
        );
        assert_eq!(report.diagnostics.len(), 1, "got: {:?}", report.diagnostics);
        assert_eq!(report.diagnostics[0].code, codes::TOOLCHAIN_DENO_MISSING);

        let expected_paths = [
            "anchor.json",
            "package.json",
            "README.md",
            "docs/architecture.md",
            "docs/design.md",
            "docs/glossary.md",
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
            crate::project_diagnostics(&validation_report.diagnostics).is_empty(),
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
            "anchor.json",
            "package.json",
            "README.md",
            "docs/architecture.md",
            "docs/design.md",
            "docs/glossary.md",
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

        assert_eq!(plan.entries.len(), 9);
        assert_eq!(
            plan.entries[0].relative_path,
            Path::new(".gitignore"),
            ".gitignore is first so it exists before any git add"
        );
        assert_eq!(plan.entries[1].relative_path, Path::new("anchor.json"));
        assert_eq!(plan.entries[2].relative_path, Path::new("package.json"));
        assert_eq!(plan.entries[3].relative_path, Path::new("README.md"));
        assert_eq!(
            plan.entries[4].relative_path,
            Path::new("docs/architecture.md")
        );
        assert_eq!(plan.entries[5].relative_path, Path::new("docs/design.md"));
        assert_eq!(plan.entries[6].relative_path, Path::new("docs/glossary.md"));
        assert_eq!(
            plan.entries[7].relative_path,
            Path::new("tokens/tokens.json")
        );
        assert_eq!(plan.entries[8].relative_path, Path::new("pages/index.html"));

        assert!(plan.entries[0].content.contains("/dist/"));
        assert!(plan.entries[0].content.contains("/node_modules/"));
        assert_eq!(
            plan.entries[7].content,
            documents::tokens_json_content(),
            "Minimal resolution must source tokens/tokens.json from the Minimal module"
        );
        assert_eq!(
            plan.entries[8].content,
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
            10,
            "Spectrum 2 path adds scripts/main.ts on top of the base nine paths"
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
        plan.add("docs/glossary.md", "# Naming");
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
        plan.add("docs/glossary.md", "# Naming");
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
        plan.add("docs/glossary.md", "# Naming");
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
        let candidates = build_collision_candidates(&plan);

        let expected = vec![
            PathBuf::from(".gitignore"),
            PathBuf::from("anchor.json"),
            PathBuf::from("package.json"),
            PathBuf::from("README.md"),
            PathBuf::from("docs/architecture.md"),
            PathBuf::from("docs/design.md"),
            PathBuf::from("docs/glossary.md"),
            PathBuf::from("tokens/tokens.json"),
            PathBuf::from("pages/index.html"),
            PathBuf::from("docs"),
            PathBuf::from("tokens"),
            PathBuf::from("pages"),
            PathBuf::from(".git"),
            PathBuf::from("wda.json"),
            PathBuf::from("node_modules"),
            PathBuf::from(".wda-init-stage"),
            PathBuf::from("package-lock.json"),
            PathBuf::from("npm-shrinkwrap.json"),
            PathBuf::from("bun.lock"),
            PathBuf::from("bun.lockb"),
            PathBuf::from("yarn.lock"),
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
        let candidates = build_collision_candidates(&plan);

        let expected = vec![
            PathBuf::from(".gitignore"),
            PathBuf::from("anchor.json"),
            PathBuf::from("package.json"),
            PathBuf::from("README.md"),
            PathBuf::from("docs/architecture.md"),
            PathBuf::from("docs/design.md"),
            PathBuf::from("docs/glossary.md"),
            PathBuf::from("tokens/tokens.json"),
            PathBuf::from("pages/index.html"),
            PathBuf::from("scripts/main.ts"),
            PathBuf::from("docs"),
            PathBuf::from("tokens"),
            PathBuf::from("pages"),
            PathBuf::from("scripts"),
            PathBuf::from(".git"),
            PathBuf::from("wda.json"),
            PathBuf::from("node_modules"),
            PathBuf::from(".wda-init-stage"),
            PathBuf::from("package-lock.json"),
            PathBuf::from("npm-shrinkwrap.json"),
            PathBuf::from("bun.lock"),
            PathBuf::from("bun.lockb"),
            PathBuf::from("yarn.lock"),
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
            PathBuf::from("deno.lock"),
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
            has_no_errors_or_unexpected_init_diagnostics(&report),
            "init report must contain zero diagnostics, got: {:?}",
            report.diagnostics
        );

        assert_eq!(fs::read(&notes_path).unwrap(), b"designer notes");
        assert_eq!(fs::read(&logo_path).unwrap(), b"fake png logo");
        assert_eq!(fs::read(&hero_path).unwrap(), b"fake png hero");
        assert_eq!(fs::read(&ds_store_path).unwrap(), b"\x00\x00\x00\x01Mac OS X");

        assert!(temp_dir.join("anchor.json").exists());
        assert!(temp_dir.join("package.json").exists());
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
        let wda_json_path = temp_dir.join("anchor.json");
        let notes_path = temp_dir.join("notes.txt");

        fs::write(&readme_path, "original readme").unwrap();
        fs::write(&wda_json_path, "original anchor.json").unwrap();
        fs::write(&notes_path, "original notes").unwrap();

        let before_count = count_entries(&temp_dir).unwrap();
        assert_eq!(before_count, 3);

        let result = init_project_with_resolution(&temp_dir, offline_wda_minimal());
        let (report, _) = result.expect("refusal returns Ok((report, resolved_ds))");

        assert_eq!(report.diagnostics.len(), 2);

        // First collision is anchor.json (index 1 in candidate plan)
        assert_eq!(report.diagnostics[0].code, codes::INIT_PATH_CONFLICT);
        assert_eq!(report.diagnostics[0].severity, Severity::Error);
        assert_eq!(
            report.diagnostics[0].location.as_ref().unwrap().path,
            Path::new("anchor.json")
        );
        assert!(report.diagnostics[0].message.contains("anchor.json"));

        // Second collision is README.md (index 3 in candidate plan)
        assert_eq!(report.diagnostics[1].code, codes::INIT_PATH_CONFLICT);
        assert_eq!(report.diagnostics[1].severity, Severity::Error);
        assert_eq!(
            report.diagnostics[1].location.as_ref().unwrap().path,
            Path::new("README.md")
        );
        assert!(report.diagnostics[1].message.contains("README.md"));

        assert_eq!(fs::read_to_string(&readme_path).unwrap(), "original readme");
        assert_eq!(fs::read_to_string(&wda_json_path).unwrap(), "original anchor.json");
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
            has_no_errors_or_unexpected_init_diagnostics(&report),
            "init report must contain zero diagnostics, got: {:?}",
            report.diagnostics
        );

        assert_eq!(
            fs::read_to_string(&old_component_path).unwrap(),
            "<section>component</section>"
        );
        assert!(temp_dir.join("anchor.json").exists());
        assert!(temp_dir.join("package.json").exists());

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
        assert!(has_no_errors_or_unexpected_init_diagnostics(&report_ok));
        assert!(temp_dir_ok.join(".DS_Store").exists());
        assert_eq!(fs::read(temp_dir_ok.join(".DS_Store")).unwrap(), b"metadata");
        assert!(temp_dir_ok.join("anchor.json").exists());
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

    #[test]
    fn test_late_rollback_removes_in_reverse_order_including_dependency_files() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_rollback_rev_deps_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let created = vec![
            PathBuf::from(".git"),
            PathBuf::from("wda.json"),
            PathBuf::from("README.md"),
            PathBuf::from("deno.jsonc"),
            PathBuf::from("deno.lock"),
        ];

        let mut recorded_removals = Vec::new();
        let recording_remover = |_: &Path, rel_path: &Path| {
            recorded_removals.push(rel_path.to_path_buf());
            Ok(())
        };

        let rollback_res = rollback(&temp_dir, &created, recording_remover);
        assert!(rollback_res.is_ok(), "rollback must succeed");

        let expected_order = vec![
            PathBuf::from("deno.lock"),
            PathBuf::from("deno.jsonc"),
            PathBuf::from("README.md"),
            PathBuf::from("wda.json"),
            PathBuf::from(".git"),
        ];
        assert_eq!(
            recorded_removals, expected_order,
            "recorded removals must be in exact reverse order of created, including dependency files"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    // ---- native init transaction: injected ports, no terminal, network or registry ----

    use std::cell::RefCell;
    use std::collections::BTreeSet;

    struct FakePorts {
        path: Option<std::ffi::OsString>,
        spectrum: Result<String, resolve::ManagerError>,
        lit: Result<String, resolve::ManagerError>,
        managers: Vec<(&'static str, Result<&'static str, resolve::ManagerError>)>,
        interactive: bool,
        menu_input: &'static str,
        install: Result<(), resolve::ManagerError>,
        tools: Result<(), String>,
        fail_remove_on: Option<PathBuf>,
        fail_write_on: Option<PathBuf>,
        on_write: Option<Box<dyn Fn(&Path, &WritePlanEntry)>>,
        on_install: Option<Box<dyn Fn(&Path)>>,
        home: Option<std::ffi::OsString>,
        git_config: Option<std::ffi::OsString>,
        calls: RefCell<Vec<String>>,
        stage_manifest: RefCell<Option<String>>,
    }

    impl FakePorts {
        fn online(managers: &[&'static str]) -> Self {
            Self {
                path: std::env::var_os("PATH"),
                spectrum: Ok("1.2.3".to_string()),
                lit: Ok("3.4.5".to_string()),
                managers: managers.iter().map(|name| (*name, Ok("9.8.7"))).collect(),
                interactive: false,
                menu_input: "",
                install: Ok(()),
                tools: Ok(()),
                fail_remove_on: None,
                fail_write_on: None,
                on_write: None,
                on_install: None,
                home: None,
                git_config: None,
                calls: RefCell::new(Vec::new()),
                stage_manifest: RefCell::new(None),
            }
        }

        fn offline(managers: &[&'static str]) -> Self {
            let mut ports = Self::online(managers);
            ports.spectrum = Err(resolve::ManagerError::RegistryUnreachable(
                "dns error".to_string(),
            ));
            ports
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl InitPorts for FakePorts {
        fn path_env(&self) -> Option<std::ffi::OsString> {
            self.path.clone()
        }

        fn home_env(&self) -> Option<std::ffi::OsString> {
            self.home.clone()
        }

        fn git_config_global_env(&self) -> Option<std::ffi::OsString> {
            self.git_config.clone()
        }

        fn registry_metadata(
            &self,
            package: &str,
        ) -> Result<serde_json::Value, resolve::ManagerError> {
            self.calls.borrow_mut().push(format!("metadata:{package}"));
            let version = if package == resolve::SPECTRUM_NPM_PACKAGE {
                self.spectrum.clone()
            } else {
                self.lit.clone()
            }?;
            Ok(serde_json::json!({ "dist-tags": { "latest": version } }))
        }

        fn probe_manager(
            &self,
            name: &str,
            _cwd: &Path,
        ) -> Result<resolve::PackageManager, resolve::ManagerError> {
            self.calls.borrow_mut().push(format!("probe:{name}"));
            match self.managers.iter().find(|(known, _)| *known == name) {
                Some((_, Ok(version))) => Ok(resolve::PackageManager {
                    name: name.to_string(),
                    executable: PathBuf::from(name),
                    version: (*version).to_string(),
                }),
                Some((_, Err(error))) => Err(error.clone()),
                None => Err(resolve::ManagerError::NotFound),
            }
        }

        fn install(
            &self,
            manager: &resolve::PackageManager,
            cwd: &Path,
        ) -> Result<(), resolve::ManagerError> {
            self.calls
                .borrow_mut()
                .push(format!("install:{}", manager.name));
            *self.stage_manifest.borrow_mut() = fs::read_to_string(cwd.join("package.json")).ok();
            if let Some(hook) = &self.on_install {
                hook(cwd);
            }
            self.install.clone()?;
            fs::create_dir_all(cwd.join("node_modules/lit")).unwrap();
            fs::write(cwd.join("node_modules/lit/index.js"), "export {};").unwrap();
            let lock = if manager.name == "bun" {
                "bun.lock"
            } else {
                "package-lock.json"
            };
            fs::write(cwd.join(lock), "{}").unwrap();
            Ok(())
        }

        fn ensure_tools(&self) -> Result<(), String> {
            self.calls.borrow_mut().push("tools".to_string());
            self.tools.clone()
        }

        fn choose_manager(&self, names: &[String]) -> MenuChoice {
            self.calls
                .borrow_mut()
                .push(format!("choose:{}", names.join(",")));
            if !self.interactive {
                return MenuChoice::NonInteractive;
            }
            let mut input = io::Cursor::new(self.menu_input.to_string());
            let mut sink = Vec::new();
            run_menu(names, &mut input, &mut sink)
        }

        fn write(
            &self,
            root: &Path,
            entry: &WritePlanEntry,
            created: &mut Vec<PathBuf>,
        ) -> Result<(), ToolFault> {
            if self.fail_write_on.as_deref() == Some(entry.relative_path.as_path()) {
                return Err(ToolFault::new(
                    format!("injected write failure at {}", entry.relative_path.display()),
                    Some(root.to_path_buf()),
                ));
            }
            real_writer(root, entry, created)?;
            if let Some(hook) = &self.on_write {
                hook(root, entry);
            }
            Ok(())
        }

        fn remove(&self, root: &Path, rel_path: &Path) -> Result<(), ToolFault> {
            if self.fail_remove_on.as_deref() == Some(rel_path) {
                return Err(ToolFault::new(
                    format!("injected removal failure at {}", rel_path.display()),
                    Some(root.to_path_buf()),
                ));
            }
            real_remover(root, rel_path)
        }
    }

    fn native_temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "wda_native_init_{tag}_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn top_level(root: &Path) -> BTreeSet<String> {
        fs::read_dir(root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect()
    }

    fn names(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|item| (*item).to_string()).collect()
    }

    fn native_dt() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 11, 11, 12, 0, 0).unwrap()
    }

    fn run_native(
        root: &Path,
        options: &InitOptions,
        ports: &FakePorts,
    ) -> InitResult {
        init_inner(root, &native_dt(), None, options, ports)
    }

    fn json_file(root: &Path, name: &str) -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(root.join(name)).unwrap()).unwrap()
    }

    const MINIMAL_TOP_LEVEL: [&str; 7] = [
        ".git",
        ".gitignore",
        "README.md",
        "anchor.json",
        "docs",
        "package.json",
        "pages",
    ];

    #[test]
    fn native_init_transaction_unit_spectrum_dependencies_pin_every_direct_package() {
        let dependencies = spectrum_two_dependencies("1.2.3", "3.0.0");
        assert_eq!(dependencies.len(), 7);
        assert!(dependencies
            .iter()
            .take(6)
            .all(|(name, version)| name.starts_with("@spectrum-web-components/")
                && version == "1.2.3"));
        assert_eq!(dependencies[6], ("lit".to_string(), "3.0.0".to_string()));
    }

    #[test]
    fn native_init_transaction_unit_offline_with_cache_creates_minimal_without_install() {
        let root = native_temp_root("offline");
        // A complete tool cache and a manager on PATH must not switch init to Spectrum.
        let ports = FakePorts::offline(&["npm"]);
        let (report, ds) = run_native(&root, &InitOptions::default(), &ports).unwrap();

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        assert_eq!(ds.name, resolve::BUILTIN_DESIGN_SYSTEM_NAME);
        let reason = ds.fallback_reason.unwrap();
        assert!(reason.contains("registry unreachable"), "{reason}");
        assert!(reason.contains("WDA Skill"), "{reason}");
        assert!(!ports.calls().iter().any(|call| call.starts_with("install")));
        assert!(
            !ports.calls().iter().any(|call| call == "tools"),
            "an offline run does not wait on a tool download"
        );
        let mut expected = names(&MINIMAL_TOP_LEVEL);
        expected.extend(names(&["tokens"]));
        assert_eq!(top_level(&root), expected);
        let package = json_file(&root, "package.json");
        assert_eq!(package["dependencies"], serde_json::json!({}));
        assert!(package.get("packageManager").is_none());
        let anchor = json_file(&root, "anchor.json");
        assert_eq!(anchor["designSystem"]["name"], "WDA Minimal");
        assert_eq!(anchor["initialization"]["fallback"]["requestedDesignSystem"], "Spectrum 2");
        let gitignore = fs::read_to_string(root.join(".gitignore")).unwrap();
        assert!(gitignore.contains("/dist/") && gitignore.contains("/node_modules/"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_init_transaction_unit_zero_managers_creates_minimal_without_registry_access() {
        let root = native_temp_root("zero");
        let ports = FakePorts::online(&[]);
        let (report, ds) = run_native(&root, &InitOptions::default(), &ports).unwrap();

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        assert_eq!(ds.name, resolve::BUILTIN_DESIGN_SYSTEM_NAME);
        assert!(ds
            .fallback_reason
            .unwrap()
            .contains("No supported package manager"));
        assert!(
            !ports.calls().iter().any(|call| call.starts_with("metadata")),
            "no manager means the registry is never contacted: {:?}",
            ports.calls()
        );
        assert!(root.join("anchor.json").is_file() && root.join("package.json").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_init_transaction_unit_one_manager_installs_default_in_stage_then_commits() {
        let root = native_temp_root("one");
        let ports = FakePorts::online(&["npm"]);
        let (report, ds) = run_native(&root, &InitOptions::default(), &ports).unwrap();

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        assert_eq!(ds.name, resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME);
        assert_eq!(ds.version, "1.2.3");
        assert!(ds.fallback_reason.is_none());
        let mut expected = names(&MINIMAL_TOP_LEVEL);
        expected.extend(names(&["tokens", "scripts", "node_modules", "package-lock.json"]));
        assert_eq!(top_level(&root), expected, "the stage directory must be gone");

        let package = json_file(&root, "package.json");
        assert_eq!(package["packageManager"], "npm@9.8.7");
        assert_eq!(package["dependencies"]["lit"], "3.4.5");
        assert_eq!(
            package["dependencies"]["@spectrum-web-components/button"],
            "1.2.3"
        );
        let staged: serde_json::Value =
            serde_json::from_str(&ports.stage_manifest.borrow().clone().unwrap()).unwrap();
        assert_eq!(staged["dependencies"], package["dependencies"]);
        assert_eq!(staged["packageManager"], "npm@9.8.7");

        let anchor = json_file(&root, "anchor.json");
        assert_eq!(anchor["designSystem"]["name"], "Spectrum 2");
        assert!(anchor.get("initialization").is_none());
        assert_eq!(
            ports.calls(),
            vec![
                "probe:bun",
                "probe:npm",
                "metadata:@spectrum-web-components/bundle",
                "metadata:lit",
                "install:npm",
                "tools"
            ]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_init_transaction_unit_two_managers_on_a_terminal_follow_the_choice() {
        let root = native_temp_root("two_tty");
        let mut ports = FakePorts::online(&["npm", "bun"]);
        ports.interactive = true;
        ports.menu_input = "1\n";
        let (_, ds) = run_native(&root, &InitOptions::default(), &ports).unwrap();

        assert_eq!(ds.name, resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME);
        assert!(ports.calls().contains(&"choose:bun,npm".to_string()));
        assert!(ports.calls().contains(&"install:bun".to_string()));
        assert_eq!(json_file(&root, "package.json")["packageManager"], "bun@9.8.7");
        assert!(root.join("bun.lock").is_file());
        assert!(!root.join("package-lock.json").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_init_transaction_unit_cancel_or_eof_ends_before_writing() {
        for input in ["\n", "q\n", ""] {
            let root = native_temp_root("cancel");
            let mut ports = FakePorts::online(&["npm", "bun"]);
            ports.interactive = true;
            ports.menu_input = input;
            let (report, _) = run_native(&root, &InitOptions::default(), &ports).unwrap();

            assert!(report.has_error(), "input {input:?}");
            assert_eq!(report.diagnostics[0].code, codes::CLI_USAGE);
            assert!(report.diagnostics[0].message.contains("cancelled"));
            assert_eq!(count_entries(&root).unwrap(), 0, "input {input:?}");
            assert!(!ports.calls().iter().any(|call| call.starts_with("install")));
            let _ = fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn native_init_transaction_unit_two_managers_without_a_terminal_is_a_usage_fault() {
        let root = native_temp_root("two_pipe");
        let ports = FakePorts::online(&["npm", "bun"]);
        let fault = run_native(&root, &InitOptions::default(), &ports)
            .expect_err("ambiguity without a terminal must not guess");

        assert!(fault.message.contains("--package-manager"), "{}", fault.message);
        assert!(fault.message.contains("bun, npm"), "{}", fault.message);
        assert_eq!(count_entries(&root).unwrap(), 0);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_init_transaction_unit_named_manager_needs_no_menu_and_never_switches() {
        let root = native_temp_root("named_ok");
        let ports = FakePorts::online(&["npm", "bun"]);
        let options = InitOptions {
            package_manager: Some("npm".to_string()),
        };
        let (_, ds) = run_native(&root, &options, &ports).unwrap();
        assert_eq!(ds.name, resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME);
        assert!(!ports.calls().iter().any(|call| call.starts_with("choose")));
        assert!(!ports.calls().contains(&"probe:bun".to_string()));
        let _ = fs::remove_dir_all(&root);

        let root = native_temp_root("named_missing");
        let ports = FakePorts::online(&["npm"]);
        let options = InitOptions {
            package_manager: Some("bun".to_string()),
        };
        let (report, ds) = run_native(&root, &options, &ports).unwrap();
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        assert_eq!(ds.name, resolve::BUILTIN_DESIGN_SYSTEM_NAME);
        let reason = ds.fallback_reason.unwrap();
        assert!(reason.contains("bun") && reason.contains("not available"), "{reason}");
        assert!(!ports.calls().iter().any(|call| call == "install:npm"));
        assert!(!ports.calls().contains(&"probe:npm".to_string()));
        assert!(!root.join("package-lock.json").exists());
        let _ = fs::remove_dir_all(&root);

        let root = native_temp_root("named_unknown");
        let options = InitOptions {
            package_manager: Some("pnpm".to_string()),
        };
        let fault = run_native(&root, &options, &FakePorts::online(&["npm"])).unwrap_err();
        assert!(fault.message.contains("Unknown package manager"));
        assert_eq!(count_entries(&root).unwrap(), 0);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_init_transaction_unit_install_failure_falls_back_to_complete_minimal() {
        let root = native_temp_root("install_fail");
        let mut ports = FakePorts::online(&["npm"]);
        ports.install = Err(resolve::ManagerError::InstallFailure(
            "EACCES: permission denied".to_string(),
        ));
        let (report, ds) = run_native(&root, &InitOptions::default(), &ports).unwrap();

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        assert_eq!(ds.name, resolve::BUILTIN_DESIGN_SYSTEM_NAME);
        let reason = ds.fallback_reason.unwrap();
        assert!(reason.contains("EACCES"), "{reason}");
        assert!(reason.contains("npm"), "{reason}");
        assert!(reason.contains("WDA Skill"), "{reason}");
        let mut expected = names(&MINIMAL_TOP_LEVEL);
        expected.extend(names(&["tokens"]));
        assert_eq!(top_level(&root), expected, "no stage, no node_modules, no scripts");
        assert_eq!(
            json_file(&root, "package.json")["dependencies"],
            serde_json::json!({})
        );
        assert_eq!(
            json_file(&root, "anchor.json")["initialization"]["fallback"]["reason"],
            serde_json::Value::String(reason)
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_init_transaction_unit_registry_failures_are_classified_and_fall_back() {
        let cases = [
            (resolve::ManagerError::RegistryHttp(503), "HTTP 503"),
            (
                resolve::ManagerError::RegistryTls("bad certificate".to_string()),
                "TLS",
            ),
            (
                resolve::ManagerError::RegistryFormat("not json".to_string()),
                "malformed",
            ),
        ];
        for (error, expected) in cases {
            let root = native_temp_root("registry_class");
            let mut ports = FakePorts::online(&["npm"]);
            ports.spectrum = Err(error);
            let (_, ds) = run_native(&root, &InitOptions::default(), &ports).unwrap();
            assert_eq!(ds.name, resolve::BUILTIN_DESIGN_SYSTEM_NAME);
            let reason = ds.fallback_reason.unwrap();
            assert!(reason.contains(expected), "{reason}");
            assert!(
                !reason.contains("unreachable"),
                "only a connection failure is 'registry unreachable': {reason}"
            );
            assert!(
                ports.calls().iter().any(|call| call == "tools"),
                "a registry that answered counts as online for the tool restore"
            );
            let _ = fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn native_init_transaction_unit_collisions_are_errors_before_any_probe_or_write() {
        for existing in [
            "package.json",
            "anchor.json",
            "node_modules",
            "package-lock.json",
            "bun.lock",
            "yarn.lock",
            ".wda-init-stage",
        ] {
            let root = native_temp_root("collide");
            let path = root.join(existing);
            if existing == "node_modules" || existing == ".wda-init-stage" {
                fs::create_dir_all(path.join("x")).unwrap();
            } else {
                fs::write(&path, "designer content").unwrap();
            }
            let before = top_level(&root);
            let ports = FakePorts::online(&["npm"]);
            let (report, _) = run_native(&root, &InitOptions::default(), &ports).unwrap();

            assert!(report.has_error(), "{existing}");
            assert_eq!(report.diagnostics.len(), 1, "{existing}: {:?}", report.diagnostics);
            assert_eq!(report.diagnostics[0].code, codes::INIT_PATH_CONFLICT);
            assert_eq!(
                report.diagnostics[0].location.as_ref().unwrap().path,
                Path::new(existing)
            );
            assert!(ports.calls().is_empty(), "{existing}: {:?}", ports.calls());
            assert_eq!(top_level(&root), before, "{existing}");
            let _ = fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn native_init_transaction_unit_install_state_appearing_mid_run_is_refused_and_untouched() {
        let root = native_temp_root("late_collision");
        let mut ports = FakePorts::online(&["npm"]);
        let foreign = root.clone();
        ports.on_install = Some(Box::new(move |_| {
            fs::create_dir_all(foreign.join("node_modules/foreign")).unwrap();
            fs::write(foreign.join("node_modules/foreign/a.js"), "theirs").unwrap();
        }));
        let (report, _) = run_native(&root, &InitOptions::default(), &ports).unwrap();

        assert!(report.has_error());
        assert_eq!(report.diagnostics[0].code, codes::INIT_PATH_CONFLICT);
        assert_eq!(top_level(&root), names(&["node_modules"]));
        assert_eq!(
            fs::read_to_string(root.join("node_modules/foreign/a.js")).unwrap(),
            "theirs",
            "content this run did not create is never removed"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_init_transaction_unit_cleanup_failure_is_an_error_naming_residue() {
        let root = native_temp_root("cleanup_fail");
        let mut ports = FakePorts::online(&["npm"]);
        ports.install = Err(resolve::ManagerError::InstallFailure("boom".to_string()));
        ports.fail_remove_on = Some(PathBuf::from(STAGE_DIR));
        let fault = run_native(&root, &InitOptions::default(), &ports)
            .expect_err("a failed required cleanup is never a claimed Minimal success");

        assert!(fault.message.contains("residual paths remain"), "{}", fault.message);
        assert!(fault.message.contains(".wda-init-stage"), "{}", fault.message);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_init_transaction_unit_write_failure_rolls_back_install_and_keeps_unrelated_files() {
        let root = native_temp_root("write_fail");
        fs::write(root.join("notes.txt"), "designer notes").unwrap();
        let mut ports = FakePorts::online(&["npm"]);
        ports.fail_write_on = Some(PathBuf::from("pages/index.html"));
        let fault = run_native(&root, &InitOptions::default(), &ports).unwrap_err();

        assert!(fault.message.contains("injected write failure"));
        assert_eq!(top_level(&root), names(&["notes.txt"]));
        assert_eq!(
            fs::read_to_string(root.join("notes.txt")).unwrap(),
            "designer notes"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_init_transaction_unit_git_add_failure_rolls_back_install_output() {
        let root = native_temp_root("add_fail");
        fs::write(root.join("notes.txt"), "designer notes").unwrap();
        let mut ports = FakePorts::online(&["npm"]);
        ports.on_write = Some(Box::new(|root, entry| {
            if entry.relative_path == Path::new("README.md") {
                fs::write(root.join(".git/index.lock"), "").unwrap();
            }
        }));
        let fault = run_native(&root, &InitOptions::default(), &ports).unwrap_err();

        assert!(fault.message.contains("'git add -A' failed"), "{}", fault.message);
        assert_eq!(
            top_level(&root),
            names(&["notes.txt"]),
            "node_modules, lockfile, .git and every written file are removed"
        );
        let _ = fs::remove_dir_all(&root);
    }

    fn assert_late_commit_failure_rolls_back(tag: &str, mut ports: FakePorts) {
        let root = native_temp_root(tag);
        let home = native_temp_root(&format!("{tag}_home"));
        let config = home.join("failing-gitconfig");
        fs::write(
            &config,
            "[user]\n    name = Test User\n    email = test@example.com\n    signingkey = fake\n[commit]\n    gpgsign = true\n",
        )
        .unwrap();
        ports.home = Some(home.clone().into_os_string());
        ports.git_config = Some(config.into_os_string());
        let notes = "designer notes that must survive git commit rollback";
        fs::write(root.join("notes.txt"), notes).unwrap();

        let fault = run_native(&root, &InitOptions::default(), &ports)
            .expect_err("init must fail when git commit fails");

        assert!(fault.message.contains("'git commit' failed"), "{}", fault.message);
        assert_eq!(
            top_level(&root),
            names(&["notes.txt"]),
            ".git, node_modules, lockfile, scripts and every written file are removed"
        );
        assert_eq!(fs::read_to_string(root.join("notes.txt")).unwrap(), notes);
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn native_init_transaction_unit_late_git_commit_failure_rolls_back_minimal() {
        assert_late_commit_failure_rolls_back("commit_fail_min", FakePorts::offline(&["npm"]));
    }

    #[test]
    fn native_init_transaction_unit_late_git_commit_failure_rolls_back_spectrum() {
        let ports = FakePorts::online(&["npm"]);
        assert_late_commit_failure_rolls_back("commit_fail_spec", ports);
    }

    #[test]
    fn native_init_transaction_unit_missing_git_is_a_warning_and_gitignore_is_first() {
        for managers in [&[][..], &["npm"][..]] {
            let root = native_temp_root("no_git");
            let empty_path_dir = native_temp_root("no_git_path");
            let mut ports = FakePorts::online(managers);
            ports.path = Some(empty_path_dir.clone().into_os_string());
            let (report, ds) = run_native(&root, &InitOptions::default(), &ports).unwrap();

            assert!(!report.has_error(), "{:?}", report.diagnostics);
            assert_eq!(report.diagnostics[0].code, codes::TOOLCHAIN_GIT_MISSING);
            assert_eq!(report.diagnostics[0].severity, Severity::Warning);
            assert!(!root.join(".git").exists());
            let gitignore = fs::read_to_string(root.join(".gitignore")).unwrap();
            assert!(gitignore.contains("/dist/") && gitignore.contains("/node_modules/"));
            assert!(root.join("anchor.json").is_file() && root.join("package.json").is_file());
            if !managers.is_empty() {
                assert_eq!(ds.name, resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME);
                assert!(root.join("node_modules").is_dir());
                assert!(root.join("package-lock.json").is_file());
            }
            let _ = fs::remove_dir_all(&root);
            let _ = fs::remove_dir_all(&empty_path_dir);
        }
    }

    #[test]
    fn native_init_transaction_unit_tool_restore_failure_warns_and_keeps_project() {
        let root = native_temp_root("tools_fail");
        let mut ports = FakePorts::online(&["npm"]);
        ports.tools = Err("download failed".to_string());
        let (report, ds) = run_native(&root, &InitOptions::default(), &ports).unwrap();

        assert!(!report.has_error());
        assert_eq!(ds.name, resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME);
        let warning = &report.diagnostics[0];
        assert_eq!(warning.code, codes::INIT_TOOL_UNAVAILABLE);
        assert_eq!(warning.severity, Severity::Warning);
        assert!(warning.next_action.contains("wda tools fetch"));
        assert!(root.join("node_modules").is_dir());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_init_transaction_unit_menu_accepts_number_or_name_and_asks_again() {
        let list = vec!["bun".to_string(), "npm".to_string()];
        let mut out = Vec::new();
        assert_eq!(
            run_menu(&list, &mut io::Cursor::new("2\n"), &mut out),
            MenuChoice::Selected("npm".to_string())
        );
        assert_eq!(
            run_menu(&list, &mut io::Cursor::new("bun\n"), &mut out),
            MenuChoice::Selected("bun".to_string())
        );
        assert_eq!(
            run_menu(&list, &mut io::Cursor::new("9\nzzz\n1\n"), &mut out),
            MenuChoice::Selected("bun".to_string())
        );
        assert_eq!(
            run_menu(&list, &mut io::Cursor::new("9\n"), &mut out),
            MenuChoice::Cancelled
        );
        let shown = String::from_utf8(out).unwrap();
        assert!(shown.contains("1) bun") && shown.contains("2) npm"));
    }
}
