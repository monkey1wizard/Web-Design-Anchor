//! Checks that Git and Deno can be started from the supplied PATH.

use std::ffi::OsStr;

use crate::codes;
use crate::deps::resolve::{resolve_tool_candidate_with_path, ToolCandidateResolution};
use crate::diagnostics::{Diagnostic, Severity, ValidationReport};

pub(crate) fn validate_toolchain(path: Option<&OsStr>) -> ValidationReport {
    let mut report = ValidationReport::new();
    for (name, missing_code) in [
        ("git", codes::TOOLCHAIN_GIT_MISSING),
        ("deno", codes::TOOLCHAIN_DENO_MISSING),
    ] {
        match resolve_tool_candidate_with_path(name, path) {
            ToolCandidateResolution::NotFound => report.add(Diagnostic::new(
                missing_code,
                Severity::Warning,
                None,
                match name {
                    "git" => "Git was not found on PATH".to_string(),
                    "deno" => "Deno was not found on PATH".to_string(),
                    _ => unreachable!(),
                },
                match name {
                    "git" => "Install Git for a later 'wda init'; 'wda check' does not need Git".to_string(),
                    "deno" => "Install Deno for 'wda build', 'wda deps', and online Design System resolution in 'wda init'; 'wda check' does not need Deno".to_string(),
                    _ => unreachable!(),
                },
            )),
            ToolCandidateResolution::FoundButUnstartable(fault) => report.add(Diagnostic::new(
                codes::TOOL_FAULT,
                Severity::Warning,
                None,
                fault.message,
                format!("Repair or reinstall '{name}' so it can start"),
            )),
            ToolCandidateResolution::Startable(_) | ToolCandidateResolution::InvalidName(_) => {}
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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

    #[test]
    fn missing_tools_use_specific_warning_codes() {
        let dir = temp_dir("missing");
        let report = validate_toolchain(Some(dir.as_os_str()));
        assert_eq!(report.diagnostics.len(), 2);
        let git = report
            .diagnostics
            .iter()
            .find(|d| d.code == codes::TOOLCHAIN_GIT_MISSING)
            .unwrap();
        assert!(git.message.contains("Git was not found"));
        assert!(git.next_action.contains("wda check"));
        assert!(git.next_action.contains("does not need Git"));
        assert!(git.next_action.contains("later 'wda init'"));
        let deno = report
            .diagnostics
            .iter()
            .find(|d| d.code == codes::TOOLCHAIN_DENO_MISSING)
            .unwrap();
        assert!(deno.message.contains("Deno was not found"));
        assert!(deno.next_action.contains("wda build"));
        assert!(deno.next_action.contains("wda deps"));
        assert!(deno.next_action.contains("online Design System resolution"));
        assert!(deno.next_action.contains("wda init"));
        assert!(deno.next_action.contains("wda check"));
        assert!(deno.next_action.contains("does not need Deno"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unstartable_tool_is_warning_fault_not_missing() {
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
        let report = validate_toolchain(Some(dir.as_os_str()));
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
}
