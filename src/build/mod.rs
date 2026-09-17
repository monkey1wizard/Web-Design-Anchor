//! Build pipeline orchestration and execution for Web Design Anchor (WDA).
//!
//! Provides the internal build pipeline stages, including validation, tool resolution,
//! `<wda-include>` expansion, token emission, script transformation, reachability walking,
//! staged atomic output replacement, and build reporting.

pub mod include;
pub mod layout;
pub mod report;
pub mod scripts;
pub mod tokens;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::diagnostics::{ToolFault, ValidationReport};
use crate::{validate_project_with, ValidationScope};

const DIST_HELD_HINT: &str = "dist/ is probably holding an open handle; close any preview server, file watcher, or terminal rooted at dist/, then run the build again";

/// Build outcome report containing diagnostics, the absolute `dist/` path,
/// and the HTTP-server requirement flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildReport {
    pub validation: ValidationReport,
    pub dist_path: PathBuf,
    pub needs_http: bool,
}

/// Builds output in a sibling directory and swaps it into `dist/` only after the
/// stage has completed successfully.
///
/// The stage writer owns all output-producing work. Keeping it behind this seam
/// means a failed stage cannot expose a partial deployment tree. The remover is
/// also injected so callers can exercise cleanup failures without changing the
/// filesystem implementation used by the build pipeline.
pub(crate) fn stage_and_replace<W, R>(root: &Path, writer: W, remover: R) -> Result<(), ToolFault>
where
    W: FnMut(&Path) -> Result<(), ToolFault>,
    R: FnMut(&Path, &Path) -> Result<(), ToolFault>,
{
    stage_and_replace_with_rename(root, writer, remover, |from, to| {
        fs::rename(from, to).map_err(|error| error.to_string())
    })
}

fn stage_and_replace_with_rename<W, R, N>(
    root: &Path,
    mut writer: W,
    mut remover: R,
    mut rename: N,
) -> Result<(), ToolFault>
where
    W: FnMut(&Path) -> Result<(), ToolFault>,
    R: FnMut(&Path, &Path) -> Result<(), ToolFault>,
    N: FnMut(&Path, &Path) -> Result<(), String>,
{
    let stage = create_stage_directory(root)?;
    let stage_name = stage
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("<staging>"));

    if let Err(stage_error) = writer(&stage) {
        return match remove_tree(
            root,
            &stage,
            &stage_name,
            "Failed to clean up build staging output",
            &mut remover,
        ) {
            Ok(()) => Err(stage_error),
            Err(cleanup_error) => Err(cleanup_error),
        };
    }

    let dist = root.join("dist");
    let backup = root.join(format!(
        ".{}.wda-old-{}",
        root.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("project"),
        unique_suffix()
    ));

    // Rename the old tree out of the way, then rename the completed sibling
    // tree into its canonical location. On platforms with atomic rename this
    // makes the visible replacement a single directory-entry transition.
    let had_dist = dist.exists();
    if had_dist {
        if let Err(error) = rename(&dist, &backup) {
            return cleanup_stage_with_error(
                root,
                &stage,
                &stage_name,
                &mut remover,
                ToolFault::new(
                    format!("Failed to prepare dist/ replacement: {DIST_HELD_HINT}: {error}"),
                    Some(dist),
                ),
            );
        }
    }

    if let Err(error) = rename(&stage, &dist) {
        let restore_error = if had_dist {
            rename(&backup, &dist).err()
        } else {
            None
        };
        let message = match restore_error {
            Some(restore) => format!(
                "Failed to install staged dist/: {DIST_HELD_HINT}: {error}; restoring the previous dist/ also failed: {restore}"
            ),
            None => format!("Failed to install staged dist/: {DIST_HELD_HINT}: {error}"),
        };
        return cleanup_stage_with_error(
            root,
            &stage,
            &stage_name,
            &mut remover,
            ToolFault::new(message, Some(dist)),
        );
    }

    if had_dist {
        // The backup lives directly beneath `root` (it is a sibling of `dist/`,
        // not of `root` itself), so the walk base for removal is `backup`, not
        // `root.parent()`. It is removed only after the new tree is installed,
        // so a failed stage never touches the old tree.
        let backup_name = backup.file_name().map(PathBuf::from).unwrap_or_default();
        remove_tree(
            root,
            &backup,
            &backup_name,
            "Failed to remove old deployment backup",
            &mut remover,
        )?;
    }

    Ok(())
}

fn create_stage_directory(root: &Path) -> Result<PathBuf, ToolFault> {
    let parent = root.parent().unwrap_or_else(|| Path::new("."));
    let root_name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project");

    for attempt in 0..100 {
        let candidate = parent.join(format!(
            ".{root_name}.wda-stage-{}-{attempt}",
            unique_suffix()
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(ToolFault::new(
                    format!("Failed to create build staging directory: {error}"),
                    Some(candidate),
                ));
            }
        }
    }

    Err(ToolFault::new(
        "Unable to allocate a unique build staging directory",
        Some(parent.to_path_buf()),
    ))
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

/// Recursively removes every entry under `dir` (files first, then directories,
/// bottom-up), using `remover` for each individual filesystem entry so callers
/// can inject failures without changing the filesystem implementation.
///
/// `dir_name` is used only to build human-readable residual paths in the error
/// message; `root` is used only to render those paths as project-relative.
pub(crate) fn remove_tree<R>(
    root: &Path,
    dir: &Path,
    dir_name: &Path,
    failure_message: &str,
    remover: &mut R,
) -> Result<(), ToolFault>
where
    R: FnMut(&Path, &Path) -> Result<(), ToolFault>,
{
    let mut entries = Vec::new();
    collect_stage_entries(dir, Path::new(""), &mut entries);
    entries.reverse();
    // The directory itself must be removed after all of its children.
    entries.push(PathBuf::new());

    let mut residuals = Vec::new();
    for relative in entries {
        if remover(dir, &relative).is_err() {
            residuals.push(if relative.as_os_str().is_empty() {
                dir_name.to_path_buf()
            } else {
                dir_name.join(relative)
            });
        }
    }

    if residuals.is_empty() {
        Ok(())
    } else {
        let paths = residuals
            .iter()
            .map(|path| project_relative_path(root, path))
            .collect::<Vec<_>>()
            .join(", ");
        Err(ToolFault::new(
            format!("{failure_message}; residual paths remain: {paths}"),
            Some(root.to_path_buf()),
        ))
    }
}

fn cleanup_stage_with_error<R>(
    root: &Path,
    stage: &Path,
    stage_name: &Path,
    remover: &mut R,
    original: ToolFault,
) -> Result<(), ToolFault>
where
    R: FnMut(&Path, &Path) -> Result<(), ToolFault>,
{
    match remove_tree(
        root,
        stage,
        stage_name,
        "Failed to clean up build staging output",
        remover,
    ) {
        Ok(()) => Err(original),
        Err(cleanup) => Err(cleanup),
    }
}

fn collect_stage_entries(dir: &Path, relative: &Path, entries: &mut Vec<PathBuf>) {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return;
    };
    let mut children = read_dir.flatten().collect::<Vec<_>>();
    children.sort_by_key(|entry| entry.file_name());
    for entry in children {
        let child_relative = relative.join(entry.file_name());
        // `entry.file_type()` uses `lstat`/`readdir` metadata and does not follow
        // symlinks, unlike `entry.path().is_dir()`. A symlink is therefore never
        // recursed into here: it is pushed as a single leaf entry (matching the
        // real filesystem entry's own type), so a symlink pointing outside the
        // project tree can never contribute the symlink target's own children
        // to the removal list. Any error reading the entry's type is treated as
        // "do not recurse" rather than guessed at.
        let is_real_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        if is_real_dir {
            entries.push(child_relative.clone());
            collect_stage_entries(&entry.path(), &child_relative, entries);
        } else {
            entries.push(child_relative);
        }
    }
}

fn project_relative_path(root: &Path, path: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(root) {
        return relative.to_string_lossy().replace('\\', "/");
    }
    if let Some(parent) = root.parent() {
        if let Ok(name) = path.strip_prefix(parent) {
            return Path::new("..")
                .join(name)
                .to_string_lossy()
                .replace('\\', "/");
        }
    }
    path.to_string_lossy().replace('\\', "/")
}

impl BuildReport {
    pub fn new(validation: ValidationReport, dist_path: PathBuf, needs_http: bool) -> Self {
        Self {
            validation,
            dist_path,
            needs_http,
        }
    }
}

/// Executes the build pipeline for a project at `root`.
///
/// Runs the build stages into a sibling staging directory and only exposes the resulting tree
/// after every output-producing stage has succeeded.
pub(crate) fn build_project(root: &Path) -> Result<BuildReport, ToolFault> {
    if !root.is_dir() {
        return Err(ToolFault::new(
            "Project root directory is invalid or unreadable",
            Some(root.to_path_buf()),
        ));
    }

    let mut report = validate_project_with(root, ValidationScope::Build)?;
    let dist_path = root.join("dist");
    if report.has_error() {
        return Ok(BuildReport::new(report, dist_path, false));
    }

    // Resolve the tool before any build work can produce output. The type-check and transform
    // helpers resolve it again at their own seams, but this early gate makes the stage explicit.
    if let Err(diagnostic) = scripts::resolve_deno() {
        report.add(diagnostic);
        return Ok(BuildReport::new(report, dist_path, false));
    }

    let pages = layout::discover_pages(root);
    let mut expanded_pages = std::collections::BTreeMap::new();
    for page in pages {
        let content = fs::read_to_string(root.join(&page)).map_err(|error| {
            ToolFault::new(
                format!("Failed to read page '{}': {error}", page.display()),
                Some(root.join(&page)),
            )
        })?;
        match include::expand_includes_for_page(&content, &page, root) {
            Ok(expanded) => {
                expanded_pages.insert(page, expanded);
            }
            Err(include_report) => {
                report.diagnostics.extend(include_report.diagnostics);
            }
        }
    }
    report.sort();
    if report.has_error() {
        return Ok(BuildReport::new(report, dist_path, false));
    }

    // This gate deliberately precedes Deno: Deno resolves imports itself, and must not obscure
    // an unlocked dependency with a less honest type-error diagnostic.
    let dependency_report = scripts::check_locked_dependencies(root, &expanded_pages);
    report.diagnostics.extend(dependency_report.diagnostics);
    report.sort();
    if report.has_error() {
        return Ok(BuildReport::new(report, dist_path, false));
    }

    // The reachable script set is computed here, ahead of the workspace, so the same
    // set can be copied into the workspace below and reused for the transform call further
    // down without walking the pages twice.
    let script_entries = {
        let mut entries = std::collections::BTreeSet::new();
        for (page, content) in &expanded_pages {
            for reference in layout::extract_references_from_html(content, page) {
                if matches!(
                    layout::classify_reference(&reference.raw_url),
                    layout::ReferenceKind::FilePath(_)
                ) {
                    let candidate = layout::resolve_reference_target(&reference.raw_url, page);
                    let normalized = candidate.to_string_lossy().replace('\\', "/");
                    if normalized.starts_with("scripts/")
                        && normalized.ends_with(".ts")
                        && root.join(&candidate).is_file()
                    {
                        entries.insert(candidate);
                    }
                }
            }
        }
        entries.into_iter().collect::<Vec<_>>()
    };

    let path_env = std::env::var_os("PATH");

    // The workspace is created and populated with the reachable script set
    // right after the lock gate passes. `deno install --frozen` then runs against
    // that copy so `deno.lock` is never rewritten and `node_modules/` never appears in the
    // project root. The workspace stays a local variable — never moved into the stage
    // closure below — so it drops after `stage_and_replace` returns, on every path.
    let workspace = scripts::BuildWorkspace::new(root)?;
    let workspace_reachable = scripts::reachable_script_paths(root, script_entries.clone());
    workspace.copy_scripts(root, &workspace_reachable)?;
    if let Err(diagnostic) = workspace.install_dependencies(path_env.as_deref()) {
        report.add(diagnostic);
        return Ok(BuildReport::new(report, dist_path, false));
    }

    let mut type_report = ValidationReport::new();
    if let Err(diagnostic) = scripts::type_check_entries_with_path(
        root,
        &expanded_pages,
        &workspace,
        path_env.as_deref(),
    ) {
        type_report.add(diagnostic);
    }
    type_report.sort();
    report.diagnostics.extend(type_report.diagnostics);
    report.sort();
    if report.has_error() {
        return Ok(BuildReport::new(report, dist_path, false));
    }

    let mapping = match layout::discover_and_map_pages(root) {
        Ok(mapping) => mapping,
        Err(mapping_report) => {
            report.diagnostics.extend(mapping_report.diagnostics);
            report.sort();
            return Ok(BuildReport::new(report, dist_path, false));
        }
    };
    let (reachable, reachability_report) = layout::compute_reachability(root, &expanded_pages);
    report.diagnostics.extend(reachability_report.diagnostics);
    report.sort();
    if report.has_error() {
        return Ok(BuildReport::new(report, dist_path, false));
    }

    let token_css = root
        .join("tokens/tokens.json")
        .is_file()
        .then(|| {
            fs::read_to_string(root.join("tokens/tokens.json"))
                .map_err(|error| {
                    ToolFault::new(
                        format!("Failed to read tokens/tokens.json: {error}"),
                        Some(root.join("tokens/tokens.json")),
                    )
                })
                .and_then(|content| {
                    serde_json::from_str::<serde_json::Value>(&content)
                        .map(|value| tokens::emit_token_stylesheet(&value))
                        .map_err(|error| {
                            ToolFault::new(
                                format!("Failed to parse tokens/tokens.json: {error}"),
                                Some(root.join("tokens/tokens.json")),
                            )
                        })
                })
        })
        .transpose()?;

    let mut transform_error = None;
    let stage_result = stage_and_replace(
        root,
        |stage| {
            // `mapping` is computed before staging so all layout errors leave the old dist intact.
            let _ = &mapping;
            let copy_report = layout::copy_reachable_set(root, stage, &expanded_pages, &reachable)
                .map_err(|error| {
                    ToolFault::new(
                        format!("Failed to assemble build output: {error}"),
                        Some(stage.to_path_buf()),
                    )
                })?;
            report.diagnostics.extend(copy_report.diagnostics);
            report.sort();
            if report.has_error() {
                return Err(ToolFault::new(
                    "Build output validation failed",
                    Some(stage.to_path_buf()),
                ));
            }
            if let Some(css) = &token_css {
                let path = stage.join("styles/tokens.css");
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|error| {
                        ToolFault::new(
                            format!("Failed to prepare token stylesheet: {error}"),
                            Some(path.clone()),
                        )
                    })?;
                }
                fs::write(&path, css).map_err(|error| {
                    ToolFault::new(
                        format!("Failed to write token stylesheet: {error}"),
                        Some(path),
                    )
                })?;
            }
            if let Err(diagnostic) = scripts::transform_script_entries_with_path(
                &workspace,
                stage,
                &script_entries,
                path_env.as_deref(),
            ) {
                transform_error = Some(diagnostic);
                return Err(ToolFault::new(
                    "Script transformation failed",
                    Some(stage.to_path_buf()),
                ));
            }
            Ok(())
        },
        |base, relative| crate::init::real_remover(base, relative),
    );
    if let Some(diagnostic) = transform_error {
        report.add(diagnostic);
        report.sort();
        return Ok(BuildReport::new(report, dist_path, false));
    }
    stage_result?;

    let needs_http = report::detect_http_requirement(&dist_path);
    crate::init::manifest::update_wda_version(root)?;
    Ok(BuildReport::new(report, dist_path, needs_http))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init::init_project_with_resolution;
    use std::collections::BTreeMap;

    /// Fixture standing in for offline, injected design-system resolution so build
    /// tests never reach the real online resolver's network call during their
    /// `init_project` setup step.
    fn offline_wda_minimal() -> crate::init::resolve::ResolvedDesignSystem {
        crate::init::resolve::ResolvedDesignSystem {
            name: crate::init::resolve::BUILTIN_DESIGN_SYSTEM_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            fallback_reason: Some(
                crate::init::resolve::FALLBACK_REASON_DENO_ABSENT.to_string(),
            ),
        }
    }

    fn snapshot_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn visit(root: &Path, current: &Path, snapshot: &mut BTreeMap<PathBuf, Vec<u8>>) {
            let mut entries = fs::read_dir(current)
                .unwrap()
                .map(|entry| entry.unwrap())
                .collect::<Vec<_>>();
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let path = entry.path();
                let relative = path.strip_prefix(root).unwrap().to_path_buf();
                if path.is_dir() {
                    visit(root, &path, snapshot);
                } else {
                    snapshot.insert(relative, fs::read(path).unwrap());
                }
            }
        }

        let mut snapshot = BTreeMap::new();
        visit(root, root, &mut snapshot);
        snapshot
    }

    fn assert_no_stage_directory(root: &Path) {
        let parent = root.parent().unwrap();
        let root_name = root.file_name().unwrap().to_string_lossy();
        let leftovers = fs::read_dir(parent)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| {
                name.to_string_lossy()
                    .starts_with(&format!(".{root_name}.wda-stage-"))
            })
            .collect::<Vec<_>>();
        assert!(
            leftovers.is_empty(),
            "staging directories remain: {leftovers:?}"
        );
    }

    fn write_fixture(stage: &Path) -> Result<(), ToolFault> {
        fs::create_dir_all(stage.join("nested")).unwrap();
        fs::write(stage.join("index.html"), b"new index").unwrap();
        fs::write(stage.join("nested/app.js"), b"new app").unwrap();
        Ok(())
    }

    #[test]
    fn test_stage_writer_failure_preserves_dist_and_cleans_stage() {
        for fail_position in 0..3 {
            let root = std::env::temp_dir().join(format!(
                "wda_test_stage_writer_{}_{}",
                std::process::id(),
                unique_suffix()
            ));
            fs::create_dir_all(root.join("dist")).unwrap();
            fs::write(root.join("dist/old.txt"), b"old").unwrap();
            let before = snapshot_tree(&root.join("dist"));
            let mut position = 0;

            let result = stage_and_replace(
                &root,
                |stage| {
                    for (index, (relative, content)) in [
                        (Path::new("index.html"), b"new index".as_slice()),
                        (Path::new("nested/app.js"), b"new app".as_slice()),
                        (Path::new("nested/extra.js"), b"extra".as_slice()),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        if position == fail_position {
                            return Err(ToolFault::new(
                                format!("injected writer failure at {index}"),
                                Some(stage.to_path_buf()),
                            ));
                        }
                        position += 1;
                        if let Some(parent) = relative.parent() {
                            fs::create_dir_all(stage.join(parent)).unwrap();
                        }
                        fs::write(stage.join(relative), content).unwrap();
                    }
                    Ok(())
                },
                |base, relative| crate::init::real_remover(base, relative),
            );

            assert!(result.is_err());
            assert_eq!(snapshot_tree(&root.join("dist")), before);
            assert_no_stage_directory(&root);
            fs::remove_dir_all(&root).unwrap();
        }
    }

    #[test]
    fn test_stage_install_rename_failure_with_failed_restore() {
        let root = std::env::temp_dir().join(format!(
            "wda_test_stage_restore_failure_{}_{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(root.join("dist")).unwrap();
        fs::write(root.join("dist/old.txt"), b"old").unwrap();
        let mut position = 0;

        let result = stage_and_replace_with_rename(
            &root,
            write_fixture,
            |base, relative| crate::init::real_remover(base, relative),
            |from, to| {
                let current = position;
                position += 1;
                if current == 1 || current == 2 {
                    Err(format!("injected rename failure at {current}"))
                } else {
                    fs::rename(from, to).map_err(|error| error.to_string())
                }
            },
        );

        let fault = result.unwrap_err();
        assert!(fault.message.starts_with("Failed to install staged"));
        assert!(fault.message.contains(DIST_HELD_HINT));
        assert!(fault.message.contains("restoring the previous"));
        assert!(!root.join("dist").exists());
        assert_no_stage_directory(&root);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_stage_install_rename_failure_without_previous_dist() {
        let root = std::env::temp_dir().join(format!(
            "wda_test_stage_no_previous_dist_{}_{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&root).unwrap();
        let mut position = 0;

        let result = stage_and_replace_with_rename(
            &root,
            write_fixture,
            |base, relative| crate::init::real_remover(base, relative),
            |from, to| {
                let current = position;
                position += 1;
                if current == 0 {
                    Err("injected install rename failure".to_string())
                } else {
                    fs::rename(from, to).map_err(|error| error.to_string())
                }
            },
        );

        let fault = result.unwrap_err();
        assert!(fault.message.contains("Failed to install staged"));
        assert!(fault.message.contains(DIST_HELD_HINT));
        assert!(!fault.message.contains("restoring the previous"));
        assert!(!root.join("dist").exists());
        assert_no_stage_directory(&root);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_stage_rename_failures_preserve_dist_and_clean_stage() {
        for fail_position in 0..2 {
            let root = std::env::temp_dir().join(format!(
                "wda_test_stage_rename_{}_{}",
                std::process::id(),
                unique_suffix()
            ));
            fs::create_dir_all(root.join("dist")).unwrap();
            fs::write(root.join("dist/old.txt"), b"old").unwrap();
            let before = snapshot_tree(&root.join("dist"));
            let mut position = 0;

            let result = stage_and_replace_with_rename(
                &root,
                write_fixture,
                |base, relative| crate::init::real_remover(base, relative),
                |from, to| {
                    let current = position;
                    position += 1;
                    if current == fail_position {
                        Err(format!("injected rename failure at {current}"))
                    } else {
                        fs::rename(from, to).map_err(|error| error.to_string())
                    }
                },
            );

            let fault = result.unwrap_err();
            let expected_prefix = if fail_position == 0 {
                "Failed to prepare"
            } else {
                "Failed to install staged"
            };
            let other_prefix = if fail_position == 0 {
                "Failed to install staged"
            } else {
                "Failed to prepare"
            };
            assert!(fault.message.starts_with(expected_prefix));
            assert!(fault.message.contains(DIST_HELD_HINT));
            assert!(!fault.message.starts_with(other_prefix));
            assert_eq!(snapshot_tree(&root.join("dist")), before);
            assert_no_stage_directory(&root);
            fs::remove_dir_all(&root).unwrap();
        }
    }

    #[test]
    fn test_stage_replacement_removes_stale_files() {
        let root = std::env::temp_dir().join(format!(
            "wda_test_stage_stale_{}_{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(root.join("dist")).unwrap();
        fs::write(root.join("dist/stale.txt"), b"stale").unwrap();

        stage_and_replace(&root, write_fixture, |base, relative| {
            crate::init::real_remover(base, relative)
        })
        .unwrap();

        assert!(!root.join("dist/stale.txt").exists());
        assert_eq!(
            fs::read(root.join("dist/index.html")).unwrap(),
            b"new index"
        );
        assert_no_stage_directory(&root);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_stage_replacement_removes_non_empty_backup_entirely() {
        let root = std::env::temp_dir().join(format!(
            "wda_test_stage_backup_{}_{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(root.join("dist/nested")).unwrap();
        fs::write(root.join("dist/old.txt"), b"old").unwrap();
        fs::write(root.join("dist/nested/old_nested.txt"), b"old nested").unwrap();

        stage_and_replace(&root, write_fixture, |base, relative| {
            crate::init::real_remover(base, relative)
        })
        .unwrap();

        assert_eq!(
            fs::read(root.join("dist/index.html")).unwrap(),
            b"new index"
        );
        assert_no_stage_directory(&root);

        let leftovers = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| name.to_string_lossy().contains(".wda-old-"))
            .collect::<Vec<_>>();
        assert!(
            leftovers.is_empty(),
            "old dist/ backup must be fully removed, found: {leftovers:?}"
        );

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn test_stage_replacement_does_not_follow_symlink_out_of_backup_tree() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "wda_test_stage_symlink_{}_{}",
            std::process::id(),
            unique_suffix()
        ));
        let outside = std::env::temp_dir().join(format!(
            "wda_test_stage_symlink_outside_{}_{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(root.join("dist")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.txt"), b"do not delete me").unwrap();
        // A symlink inside the pre-existing dist/ pointing at a directory
        // entirely outside the project tree, simulating a maliciously (or
        // accidentally, e.g. via a checked-out repo) planted directory symlink.
        symlink(&outside, root.join("dist/evil")).unwrap();

        let result = stage_and_replace(&root, write_fixture, |base, relative| {
            crate::init::real_remover(base, relative)
        });

        // The symlink itself is a leaf entry under the backup tree; removing it
        // is expected to succeed (it unlinks the symlink, not its target), so
        // the overall replace should succeed rather than fail.
        assert!(result.is_ok(), "replace must succeed: {result:?}");
        assert_eq!(
            fs::read(root.join("dist/index.html")).unwrap(),
            b"new index"
        );
        // The critical assertion: the symlink target outside the project tree
        // must be completely untouched.
        assert!(outside.exists(), "symlink target directory must survive");
        assert_eq!(
            fs::read(outside.join("secret.txt")).unwrap(),
            b"do not delete me",
            "file outside the project tree must never be touched"
        );

        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&outside).unwrap();
    }

    #[test]
    fn test_stage_remover_failure_names_leftover_project_relative_path() {
        let root = std::env::temp_dir().join(format!(
            "wda_test_stage_remover_{}_{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(root.join("dist")).unwrap();
        fs::write(root.join("dist/old.txt"), b"old").unwrap();
        let mut attempted = None;

        let result = stage_and_replace(&root, write_fixture, |base, relative| {
            attempted = Some(relative.to_path_buf());
            Err(ToolFault::new(
                "injected remover failure",
                Some(base.to_path_buf()),
            ))
        });

        let fault = result.unwrap_err();
        let leftover = attempted.unwrap();
        assert!(fault.message.contains("residual paths remain"));
        assert!(fault.message.contains(leftover.to_string_lossy().as_ref()));
        assert!(root.join("dist/index.html").exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_build_project_clean_fixture_succeeds_and_creates_dist() {
        let temp_dir =
            std::env::temp_dir().join(format!("wda_test_build_clean_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let (init_report, _) = init_project_with_resolution(&temp_dir, offline_wda_minimal())
            .expect("init succeeds");
        assert!(init_report.diagnostics.is_empty(), "init must be clean");

        if scripts::resolve_deno().is_err() {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return;
        }

        let manifest_path = temp_dir.join("wda.json");
        let mut manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest["wdaVersion"] = serde_json::Value::String("stale-version".to_owned());
        std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();

        let dist_dir = temp_dir.join("dist");
        assert!(!dist_dir.exists(), "dist/ must not exist before build");

        let report = build_project(&temp_dir).expect("clean project build succeeds");
        assert!(
            !report.validation.has_error(),
            "clean project must not report errors: {:?}",
            report.validation.diagnostics
        );
        assert!(
            dist_dir.join("index.html").is_file(),
            "clean project must emit its page"
        );
        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        assert_eq!(manifest["wdaVersion"], env!("CARGO_PKG_VERSION"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_build_project_error_fixture_returns_report_and_creates_no_dist() {
        let temp_dir =
            std::env::temp_dir().join(format!("wda_test_build_error_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let (init_report, _) = init_project_with_resolution(&temp_dir, offline_wda_minimal())
            .expect("init succeeds");
        assert!(init_report.diagnostics.is_empty(), "init must be clean");

        // Inject a build error: source-side styles/tokens.css
        let styles_dir = temp_dir.join("styles");
        std::fs::create_dir_all(&styles_dir).unwrap();
        std::fs::write(styles_dir.join("tokens.css"), "/* source tokens */").unwrap();

        let dist_dir = temp_dir.join("dist");
        assert!(!dist_dir.exists(), "dist/ must not exist before build");

        let res = build_project(&temp_dir);
        assert!(
            res.is_ok(),
            "error-carrying fixture must return Ok(BuildReport)"
        );
        let report = res.unwrap();
        assert!(
            report.validation.has_error(),
            "returned report must contain validation errors"
        );
        let error_codes: Vec<&str> = report
            .validation
            .diagnostics
            .iter()
            .map(|d| d.code)
            .collect();
        assert!(
            error_codes.contains(&crate::codes::BUILD_TOKENS_CSS_IN_SOURCE),
            "report must contain BUILD_TOKENS_CSS_IN_SOURCE, got: {:?}",
            error_codes
        );
        assert!(
            !dist_dir.exists(),
            "dist/ must not be created when build report has errors"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
