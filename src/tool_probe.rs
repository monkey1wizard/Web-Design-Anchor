//! Tool-launch probing shared by `init`, `validation::toolchain`, and their tests.
//!
//! Resolves a tool name against a PATH value and checks that it starts. This
//! module does not resolve, stage, or lock website packages.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::codes;
use crate::diagnostics::{Diagnostic, Severity};

#[derive(Debug)]
pub(crate) enum ToolCandidateResolution {
    NotFound,
    Startable(PathBuf),
    FoundButUnstartable(Diagnostic),
    InvalidName(Diagnostic),
}

pub(crate) fn resolve_tool_candidate_with_path(
    tool_name: &str,
    path_env: Option<&OsStr>,
) -> ToolCandidateResolution {
    let trimmed = tool_name.trim();
    if trimmed.is_empty() {
        return ToolCandidateResolution::InvalidName(Diagnostic::new(
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
                return verify_can_start_candidate(trimmed, &candidate);
            }
        }
        return ToolCandidateResolution::NotFound;
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
                return verify_can_start_candidate(trimmed, &candidate_path);
            }
        }
    }

    ToolCandidateResolution::NotFound
}

fn verify_can_start_candidate(tool_name: &str, path: &Path) -> ToolCandidateResolution {
    match verify_can_start(tool_name, path) {
        Ok(path) => ToolCandidateResolution::Startable(path),
        Err(diagnostic) => ToolCandidateResolution::FoundButUnstartable(diagnostic),
    }
}

#[cfg(windows)]
fn candidate_names(p: &Path) -> Vec<String> {
    let base = p.to_string_lossy().into_owned();
    if p.extension().is_some() {
        return vec![base];
    }

    let mut names: Vec<String> = Vec::new();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn probe_dir(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("wda_probe_{name}_{}_{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_test_tool(dir: &Path, name: &str, contents: &[u8]) -> PathBuf {
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;

        let path = dir.join(name);
        std::fs::write(&path, contents).unwrap();
        #[cfg(unix)]
        {
            let mut permissions = std::fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&path, permissions).unwrap();
        }
        path
    }

    #[test]
    fn typed_resolver_distinguishes_absent_tool() {
        let dir = probe_dir("absent");
        assert!(matches!(
            resolve_tool_candidate_with_path("missing-tool", Some(dir.as_os_str())),
            ToolCandidateResolution::NotFound
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn typed_resolver_accepts_startable_tool_with_nonzero_version_status() {
        let dir = probe_dir("startable");
        #[cfg(windows)]
        let (name, contents) = ("git.cmd", b"@exit /b 7\r\n".as_slice());
        #[cfg(not(windows))]
        let (name, contents) = ("git", b"#!/bin/sh\nexit 7\n".as_slice());
        let expected = write_test_tool(&dir, name, contents);
        assert!(matches!(
            resolve_tool_candidate_with_path("git", Some(dir.as_os_str())),
            ToolCandidateResolution::Startable(path) if paths_match_case_insensitively(&path, &expected)
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    fn paths_match_case_insensitively(left: &Path, right: &Path) -> bool {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }

    #[cfg(not(windows))]
    fn paths_match_case_insensitively(left: &Path, right: &Path) -> bool {
        left == right
    }

    #[test]
    fn typed_resolver_distinguishes_found_but_unstartable_tool() {
        let dir = probe_dir("unstartable");
        #[cfg(windows)]
        let name = "broken-tool.exe";
        #[cfg(not(windows))]
        let name = "broken-tool";
        #[cfg(windows)]
        use std::os::windows::fs::OpenOptionsExt;
        #[cfg(windows)]
        let locked_file = {
            let path = dir.join(name);
            std::fs::write(&path, b"MZ").unwrap();
            std::fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(path)
                .unwrap()
        };
        #[cfg(not(windows))]
        write_test_tool(&dir, name, b"not a valid executable format\n");
        assert!(matches!(
            resolve_tool_candidate_with_path("broken-tool", Some(dir.as_os_str())),
            ToolCandidateResolution::FoundButUnstartable(_)
        ));
        #[cfg(windows)]
        drop(locked_file);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn windows_candidate_order_prefers_pathext_over_bare_name() {
        let dir = probe_dir("windows_candidate");
        write_test_tool(&dir, "fake-npm", b"#!/bin/sh\nexit 1\n");
        write_test_tool(&dir, "fake-npm.cmd", b"@exit /b 0\r\n");
        match resolve_tool_candidate_with_path("fake-npm", Some(dir.as_os_str())) {
            ToolCandidateResolution::Startable(path) => {
                assert!(
                    path.to_string_lossy()
                        .to_ascii_lowercase()
                        .ends_with("fake-npm.cmd"),
                    "expected fake-npm.cmd, got {}",
                    path.display()
                );
            }
            other => panic!("expected Startable fake-npm.cmd, got {other:?}"),
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn native_remove_deps_unit_probe_rejects_absent_and_empty_names() {
        let dir = probe_dir("impossible");
        assert!(matches!(
            resolve_tool_candidate_with_path(
                "impossible_tool_name_wda_test_nonexistent",
                Some(dir.as_os_str())
            ),
            ToolCandidateResolution::NotFound
        ));
        match resolve_tool_candidate_with_path("", None) {
            ToolCandidateResolution::InvalidName(diag) => {
                assert_eq!(diag.code, codes::TOOL_FAULT);
                assert_eq!(diag.severity, Severity::Error);
                assert!(diag.location.is_none());
            }
            other => panic!("empty tool name must be InvalidName, got {other:?}"),
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
