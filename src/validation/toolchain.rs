//! Check-scope preflight: Git launchability plus what the project itself needs installed.
//!
//! The preflight is read-only. It never launches a package manager, never installs anything and
//! never writes a file. A prepared project does not need its package manager on PATH: only the
//! installed `node_modules` content and the native tool cache matter.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use crate::codes;
use crate::diagnostics::{Diagnostic, Location, Severity, ValidationReport};
use crate::tool_probe::{resolve_tool_candidate_with_path, ToolCandidateResolution};
use crate::validation::project_contract::project_projection;

/// Seam answering where the cached native TypeScript tool lives. The production seam is
/// [`crate::tools::cache_status`]; tests inject their own answer so no cache or network is touched.
pub(crate) type ToolCache<'a> = &'a dyn Fn() -> Result<Option<PathBuf>, String>;

pub(crate) fn validate_toolchain(
    root: &Path,
    path: Option<&OsStr>,
    tool_cache: ToolCache,
) -> ValidationReport {
    let mut report = ValidationReport::new();
    check_git(path, &mut report);
    if let Some((package, dependencies)) = read_project(root) {
        check_installed_packages(root, &package, &dependencies, &mut report);
    }
    check_tool_cache(root, tool_cache, &mut report);
    report
}

fn check_git(path: Option<&OsStr>, report: &mut ValidationReport) {
    match resolve_tool_candidate_with_path("git", path) {
        ToolCandidateResolution::NotFound => report.add(Diagnostic::new(
            codes::TOOLCHAIN_GIT_MISSING,
            Severity::Warning,
            None,
            "Git was not found on PATH",
            "Install Git for a later 'wda init'; 'wda check' does not need Git",
        )),
        ToolCandidateResolution::FoundButUnstartable(fault) => report.add(Diagnostic::new(
            codes::TOOL_FAULT,
            Severity::Warning,
            None,
            fault.message,
            "Repair or reinstall 'git' so it can start",
        )),
        ToolCandidateResolution::Startable(_) | ToolCandidateResolution::InvalidName(_) => {}
    }
}

/// Reads the manifests through the shared projection. A missing or invalid manifest is reported
/// by the project contract rules, so the preflight stays silent about it.
fn read_project(root: &Path) -> Option<(Value, BTreeMap<String, String>)> {
    let anchor: Value =
        serde_json::from_slice(&std::fs::read(root.join("anchor.json")).ok()?).ok()?;
    let package: Value =
        serde_json::from_slice(&std::fs::read(root.join("package.json")).ok()?).ok()?;
    let projection = project_projection(&anchor, &package).ok()?;
    Some((package, projection.dependencies))
}

fn safe_package_name(name: &str) -> bool {
    !name.is_empty()
        && Path::new(name)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn check_installed_packages(
    root: &Path,
    package: &Value,
    dependencies: &BTreeMap<String, String>,
    report: &mut ValidationReport,
) {
    let manager = package
        .get("packageManager")
        .and_then(Value::as_str)
        .and_then(|field| field.split('@').next())
        .filter(|name| !name.is_empty());
    let install_hint = match manager {
        Some(name) => format!(
            "Install the project's packages with {name} (named in package.json), then run 'wda check' again; 'wda check' never installs packages"
        ),
        None => "Install the project's packages with your package manager, then run 'wda check' again; 'wda check' never installs packages".to_string(),
    };
    for name in dependencies.keys().filter(|name| safe_package_name(name)) {
        if root.join("node_modules").join(name).is_dir() {
            continue;
        }
        report.add(Diagnostic::new(
            codes::BUILD_DEPENDENCY_UNLOCKED,
            Severity::Warning,
            Some(Location::path_only("node_modules")),
            format!(
                "The package '{name}' is declared in package.json but is not installed in node_modules"
            ),
            install_hint.clone(),
        ));
    }
}

/// True when an expanded page references a `scripts/**/*.ts` entry, the only case where a build
/// type-checks and so needs the tool cache. A pure-HTML project needs no cache.
fn project_has_script_entries(root: &Path) -> bool {
    let mut pages = BTreeMap::new();
    for page in crate::build::layout::discover_pages(root) {
        let Ok(content) = std::fs::read_to_string(root.join(&page)) else {
            continue;
        };
        if let Ok(expanded) = crate::build::include::expand_includes_for_page(&content, &page, root)
        {
            pages.insert(page, expanded);
        }
    }
    !crate::build::scripts::discover_script_entries(root, &pages).is_empty()
}

fn check_tool_cache(root: &Path, tool_cache: ToolCache, report: &mut ValidationReport) {
    if !project_has_script_entries(root) {
        return;
    }
    let (message, next_action) = match tool_cache() {
        Ok(Some(_)) => return,
        Ok(None) => (
            "The TypeScript tool cache is missing, so a build cannot type-check the scripts"
                .to_string(),
            "Run `wda tools fetch` once while online; 'wda check' and 'wda build' never download tools",
        ),
        Err(reason) => (
            format!("The TypeScript tool cache cannot be located: {reason}"),
            "Run `wda tools fetch` on a supported platform; 'wda check' and 'wda build' never download tools",
        ),
    };
    report.add(Diagnostic::new(
        codes::BUILD_TOOL_UNAVAILABLE,
        Severity::Warning,
        None,
        message,
        next_action,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wda_toolchain_{label}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn no_cache() -> Result<Option<PathBuf>, String> {
        Ok(None)
    }

    const ANCHOR: &str = include_str!("../../tests/fixtures/clean_project/anchor.json");

    fn write_project(dir: &Path, package: &str, script_page: bool) {
        std::fs::write(dir.join("anchor.json"), ANCHOR).unwrap();
        std::fs::write(dir.join("package.json"), package).unwrap();
        std::fs::create_dir_all(dir.join("pages")).unwrap();
        let script = if script_page {
            "<script type=\"module\" src=\"../scripts/main.ts\"></script>"
        } else {
            ""
        };
        std::fs::write(
            dir.join("pages/index.html"),
            format!("<!DOCTYPE html><html lang=\"en\"><head><title>x</title>{script}</head><body></body></html>"),
        )
        .unwrap();
        if script_page {
            std::fs::create_dir_all(dir.join("scripts")).unwrap();
            std::fs::write(dir.join("scripts/main.ts"), "export {};\n").unwrap();
        }
    }

    #[test]
    fn native_check_preflight_unit_git_missing_only_warns_about_git() {
        let dir = temp_dir("missing");
        let report = validate_toolchain(&dir, Some(dir.as_os_str()), &no_cache);
        assert_eq!(report.diagnostics.len(), 1);
        let git = &report.diagnostics[0];
        assert_eq!(git.code, codes::TOOLCHAIN_GIT_MISSING);
        assert_eq!(git.severity, Severity::Warning);
        assert!(git.message.contains("Git was not found"));
        assert!(git.next_action.contains("wda check"));
        assert!(git.next_action.contains("does not need Git"));
        assert!(git.next_action.contains("later 'wda init'"));
        assert!(!report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("Deno") || d.next_action.contains("Deno")));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn native_check_preflight_unit_unstartable_git_is_warning_fault_not_missing() {
        let dir = temp_dir("unstartable");
        #[cfg(windows)]
        let path = dir.join("git.exe");
        #[cfg(not(windows))]
        let path = dir.join("git");
        #[cfg(windows)]
        let locked_file = {
            use std::os::windows::fs::OpenOptionsExt;
            std::fs::write(&path, b"MZ").unwrap();
            std::fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(&path)
                .unwrap()
        };
        #[cfg(not(windows))]
        std::fs::write(&path, b"invalid executable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let report = validate_toolchain(&dir, Some(dir.as_os_str()), &no_cache);
        let fault = report
            .diagnostics
            .iter()
            .find(|d| d.code == codes::TOOL_FAULT)
            .unwrap();
        assert_eq!(fault.severity, Severity::Warning);
        assert!(!report
            .diagnostics
            .iter()
            .any(|d| d.code == codes::TOOLCHAIN_GIT_MISSING));
        #[cfg(windows)]
        drop(locked_file);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn native_check_preflight_unit_missing_packages_are_named_without_a_manager_on_path() {
        let dir = temp_dir("packages");
        write_project(
            &dir,
            "{\"dependencies\":{\"lit\":\"3.3.0\",\"@scope/pkg\":\"1.0.0\"},\"packageManager\":\"npm@10.2.3\"}",
            false,
        );
        std::fs::create_dir_all(dir.join("node_modules/lit")).unwrap();
        let empty_path = temp_dir("empty_path");
        let report = validate_toolchain(&dir, Some(empty_path.as_os_str()), &no_cache);
        let packages: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::BUILD_DEPENDENCY_UNLOCKED)
            .collect();
        assert_eq!(packages.len(), 1, "only the missing package is reported");
        assert_eq!(packages[0].severity, Severity::Warning);
        assert!(packages[0].message.contains("@scope/pkg"));
        assert!(packages[0].next_action.contains("npm"));
        assert!(packages[0].next_action.contains("never installs"));
        assert!(!report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("'lit'")));

        std::fs::create_dir_all(dir.join("node_modules/@scope/pkg")).unwrap();
        let report = validate_toolchain(&dir, Some(empty_path.as_os_str()), &no_cache);
        assert!(!report
            .diagnostics
            .iter()
            .any(|d| d.code == codes::BUILD_DEPENDENCY_UNLOCKED));
        std::fs::remove_dir_all(dir).unwrap();
        std::fs::remove_dir_all(empty_path).unwrap();
    }

    #[test]
    fn native_check_preflight_unit_dependency_names_cannot_escape_node_modules() {
        let dir = temp_dir("escape");
        write_project(&dir, "{\"dependencies\":{\"../outside\":\"1.0.0\"}}", false);
        let report = validate_toolchain(&dir, Some(dir.as_os_str()), &no_cache);
        assert!(!report
            .diagnostics
            .iter()
            .any(|d| d.code == codes::BUILD_DEPENDENCY_UNLOCKED));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn native_check_preflight_unit_tool_cache_only_matters_with_script_entries() {
        let pure = temp_dir("pure_html");
        write_project(&pure, "{\"dependencies\":{}}", false);
        let lookups = std::cell::Cell::new(0);
        let counting = || {
            lookups.set(lookups.get() + 1);
            Ok(None)
        };
        let report = validate_toolchain(&pure, Some(pure.as_os_str()), &counting);
        assert!(!report
            .diagnostics
            .iter()
            .any(|d| d.code == codes::BUILD_TOOL_UNAVAILABLE));
        assert_eq!(
            lookups.get(),
            0,
            "a pure-HTML project never looks up the cache"
        );

        let scripted = temp_dir("scripted");
        write_project(&scripted, "{\"dependencies\":{}}", true);
        let report = validate_toolchain(&scripted, Some(scripted.as_os_str()), &no_cache);
        let tool = report
            .diagnostics
            .iter()
            .find(|d| d.code == codes::BUILD_TOOL_UNAVAILABLE)
            .expect("missing cache is named");
        assert_eq!(tool.severity, Severity::Warning);
        assert!(tool.next_action.contains("wda tools fetch"));

        let ready = || Ok(Some(PathBuf::from("tsc")));
        let report = validate_toolchain(&scripted, Some(scripted.as_os_str()), &ready);
        assert!(!report
            .diagnostics
            .iter()
            .any(|d| d.code == codes::BUILD_TOOL_UNAVAILABLE));

        let unlocatable = || Err("cache directory unavailable".to_string());
        let report = validate_toolchain(&scripted, Some(scripted.as_os_str()), &unlocatable);
        let tool = report
            .diagnostics
            .iter()
            .find(|d| d.code == codes::BUILD_TOOL_UNAVAILABLE)
            .unwrap();
        assert!(tool.message.contains("cache directory unavailable"));
        std::fs::remove_dir_all(pure).unwrap();
        std::fs::remove_dir_all(scripted).unwrap();
    }
}
