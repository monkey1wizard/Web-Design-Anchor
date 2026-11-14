//! Command-line argument parsing and dispatch for Web Design Anchor (WDA).

use crate::codes;
use crate::diagnostics::{Diagnostic, Severity};
use std::path::Path;

/// Parsed CLI invocation outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation {
    Check,
    Init,
    /// `wda init --package-manager <name>` with a supported manager name.
    InitManager(String),
    Build,
    ToolsFetch,
    Version,
    /// Bare invocation with no arguments: print [`USAGE`] and exit successfully.
    Usage,
    UsageError(Diagnostic),
}

/// Usage text printed to stdout for a bare `wda` invocation.
pub const USAGE: &str = "\
Usage: wda <command>

Commands:
  init         Create a new project (optional: --package-manager <name>)
  check        Validate the project without producing output
  build        Validate the project, then build dist/
  tools fetch  Restore the embedded TypeScript tool cache (no arguments)

Options:
  --version    Print the version

Website packages are not managed by wda. Use your package manager, for
example `npm install <package>` or `bun add <package>`.";

/// Parses user arguments into an [`Invocation`].
///
/// `args` must not include the program name (`argv[0]`). An empty slice is a
/// bare invocation and returns [`Invocation::Usage`]. Handles command dispatch
/// for `check`, `init`, `build`, `tools fetch`, and `--version`. Anything else
/// (unknown command, unknown flag, extra arguments) returns
/// [`Invocation::UsageError`] carrying `wda.cli.usage`.
pub fn parse_args<S: AsRef<str>>(args: &[S]) -> Invocation {
    let tokens: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    let Some((&cmd_token, rest)) = tokens.split_first() else {
        return Invocation::Usage;
    };

    if tokens.contains(&"--json") {
        return Invocation::UsageError(make_json_not_supported_error());
    }

    if cmd_token == "deps" {
        return Invocation::UsageError(Diagnostic::new(
            codes::CLI_USAGE,
            Severity::Error,
            None,
            "'wda deps' no longer exists; wda does not manage website packages",
            "Use your package manager, for example 'npm install <package>' or 'bun add <package>', then run 'wda check'",
        ));
    }

    if !matches!(cmd_token, "check" | "init" | "build" | "tools" | "--version") {
        return Invocation::UsageError(make_usage_error(format!(
            "Unknown command or flag: {cmd_token}"
        )));
    }

    if cmd_token == "init" && !rest.is_empty() {
        return parse_init_args(rest);
    }

    if cmd_token != "tools" && !rest.is_empty() {
        return Invocation::UsageError(make_usage_error(format!(
            "Unexpected argument: {}",
            rest[0]
        )));
    }

    match cmd_token {
        "check" => Invocation::Check,
        "init" => Invocation::Init,
        "tools" if rest == ["fetch"] => Invocation::ToolsFetch,
        "tools" => Invocation::UsageError(make_usage_error("Expected: wda tools fetch")),
        "build" => Invocation::Build,
        // Only `--version` remains after the allow-list check above.
        _ => Invocation::Version,
    }
}

/// Parses the arguments after `init`: only `--package-manager <name>` (or
/// `--package-manager=<name>`) naming a supported manager is accepted.
fn parse_init_args(rest: &[&str]) -> Invocation {
    let (flag, value, extra) = match rest {
        [single] => match single.split_once('=') {
            Some((flag, value)) => (flag, Some(value), &rest[1..]),
            None => (*single, None, &rest[1..]),
        },
        [flag, value, extra @ ..] => (*flag, Some(*value), extra),
        [] => unreachable!("empty init arguments are handled by the caller"),
    };
    if flag != "--package-manager" {
        return Invocation::UsageError(make_usage_error(format!("Unexpected argument: {}", rest[0])));
    }
    let Some(name) = value else {
        return Invocation::UsageError(make_usage_error(
            "Missing package manager name for '--package-manager'",
        ));
    };
    if let Some(unexpected) = extra.first() {
        return Invocation::UsageError(make_usage_error(format!(
            "Unexpected argument: {unexpected}"
        )));
    }
    let supported = crate::init::resolve::supported_package_managers();
    if !supported.iter().any(|candidate| candidate == name) {
        return Invocation::UsageError(make_usage_error(format!(
            "Unknown package manager '{name}'; supported: {}",
            supported.join(", ")
        )));
    }
    Invocation::InitManager(name.to_string())
}

fn make_json_not_supported_error() -> Diagnostic {
    Diagnostic::new(
        codes::CLI_JSON_NOT_SUPPORTED,
        Severity::Error,
        None,
        "JSON output mode (--json) is not supported",
        "Remove the --json flag and run the command again",
    )
}

fn make_usage_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        codes::CLI_USAGE,
        Severity::Error,
        None,
        message,
        "Run 'wda --version' or check command usage: wda <check|init|build|tools fetch|--version>",
    )
}

/// Accessibility capability-boundary scope note included in the stdout success summary.
pub const A11Y_SCOPE_NOTE: &str =
    "Note: Automated checks do not guarantee WCAG accessibility compliance. Contextual criteria require manual review.";

/// Renders the success summary for `wda check` including the accessibility capability-boundary note.
pub fn render_check_summary() -> String {
    format!("Validation succeeded.\n{}", A11Y_SCOPE_NOTE)
}

/// Renders the success summary for `wda init` naming the Design System used and the optional fallback reason.
pub fn render_init_summary(design_system_name: &str, fallback_reason: Option<&str>) -> String {
    match fallback_reason {
        Some(reason) => format!(
            "Project initialized.\nDesign System: {design_system_name}\nNote: {reason}"
        ),
        None => format!("Project initialized.\nDesign System: {design_system_name}"),
    }
}

/// Renders the success summary for `wda build`.
pub fn render_build_summary(dist_path: &Path, needs_http: bool) -> String {
    let absolute_path = if dist_path.is_absolute() {
        dist_path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|current| current.join(dist_path))
            .unwrap_or_else(|_| dist_path.to_path_buf())
    };
    let requirement = if needs_http {
        "HTTP server required; opening via file:// will not work."
    } else {
        "No known HTTP requirement was detected."
    };
    format!(
        "Build succeeded.\nOutput: {}\n{}",
        absolute_path.display(),
        requirement
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_five_commands_and_version() {
        assert_eq!(parse_args(&["check"]), Invocation::Check);
        assert_eq!(parse_args(&["init"]), Invocation::Init);
        assert_eq!(parse_args(&["build"]), Invocation::Build);
        assert_eq!(parse_args(&["--version"]), Invocation::Version);
    }

    #[test]
    fn native_check_preflight_unit_deps_and_tools_syntax() {
        assert_eq!(parse_args(&["tools", "fetch"]), Invocation::ToolsFetch);
        for case in [
            vec!["tools"],
            vec!["tools", "fetch", "7.0.2"],
            vec!["tools", "7.0.2"],
        ] {
            assert!(
                matches!(parse_args(&case), Invocation::UsageError(ref d) if d.code == codes::CLI_USAGE),
                "{case:?} must be a usage error"
            );
        }
        for case in [
            vec!["deps"],
            vec!["deps", "add", "lit"],
            vec!["deps", "remove", "lit"],
            vec!["deps", "update"],
            vec!["deps", "sync"],
        ] {
            let Invocation::UsageError(diag) = parse_args(&case) else {
                panic!("{case:?} must be a usage error");
            };
            assert_eq!(diag.code, codes::CLI_USAGE);
            assert!(diag.next_action.contains("npm install"), "{}", diag.next_action);
            assert!(diag.next_action.contains("bun add"), "{}", diag.next_action);
        }
    }

    #[test]
    fn test_parse_args_leading_program_name_is_unknown_command() {
        let program_path = r"C:\Users\Validator\AppData\Local\Microsoft\WinGet\wda.exe";
        for case in [vec!["wda", "check"], vec![program_path]] {
            let outcome = parse_args(&case);
            if let Invocation::UsageError(diag) = outcome {
                assert_eq!(diag.code, codes::CLI_USAGE);
                assert!(diag.message.contains(case[0]));
            } else {
                panic!("Expected UsageError for {:?}, got {:?}", case, outcome);
            }
        }
    }

    #[test]
    fn test_parse_args_unknown_command_carries_cli_usage() {
        let outcome = parse_args(&["unknown"]);
        if let Invocation::UsageError(diag) = outcome {
            assert_eq!(diag.code, codes::CLI_USAGE);
            assert_eq!(diag.severity, Severity::Error);
            assert!(diag.location.is_none());
        } else {
            panic!("Expected UsageError for unknown command, got {:?}", outcome);
        }

        let outcome_wda = parse_args(&["wda", "unknown"]);
        if let Invocation::UsageError(diag) = outcome_wda {
            assert_eq!(diag.code, codes::CLI_USAGE);
        } else {
            panic!("Expected UsageError for wda unknown, got {:?}", outcome_wda);
        }
    }

    #[test]
    fn test_parse_args_unknown_flag_carries_cli_usage() {
        let outcome = parse_args(&["--unknown-flag"]);
        if let Invocation::UsageError(diag) = outcome {
            assert_eq!(diag.code, codes::CLI_USAGE);
            assert_eq!(diag.severity, Severity::Error);
            assert!(diag.location.is_none());
        } else {
            panic!("Expected UsageError for unknown flag, got {:?}", outcome);
        }

        let outcome_wda = parse_args(&["wda", "--foo"]);
        if let Invocation::UsageError(diag) = outcome_wda {
            assert_eq!(diag.code, codes::CLI_USAGE);
        } else {
            panic!("Expected UsageError for wda --foo, got {:?}", outcome_wda);
        }
    }

    #[test]
    fn test_parse_args_empty_returns_usage() {
        assert_eq!(parse_args::<&str>(&[]), Invocation::Usage);
    }

    #[test]
    fn test_usage_lists_every_command_and_version() {
        for token in ["init", "check", "build", "tools fetch", "--version", "npm install", "bun add"] {
            assert!(USAGE.contains(token), "USAGE must mention {token}");
        }
    }

    #[test]
    fn test_parse_args_unexpected_extra_arguments() {
        let outcome = parse_args(&["check", "extra"]);
        if let Invocation::UsageError(diag) = outcome {
            assert_eq!(diag.code, codes::CLI_USAGE);
        } else {
            panic!("Expected UsageError for extra arguments, got {:?}", outcome);
        }
    }

    #[test]
    fn test_parse_args_reject_json_flag() {
        let cases = vec![
            vec!["wda", "--version", "--json"],
            vec!["wda", "--json", "check"],
            vec!["wda", "check", "--json"],
            vec!["--json"],
            vec!["--version", "--json"],
        ];

        for case in cases {
            let outcome = parse_args(&case);
            if let Invocation::UsageError(diag) = outcome {
                assert_eq!(diag.code, codes::CLI_JSON_NOT_SUPPORTED);
                assert_eq!(diag.severity, Severity::Error);
                assert!(diag.location.is_none());
            } else {
                panic!(
                    "Expected UsageError carrying CLI_JSON_NOT_SUPPORTED for {:?}, got {:?}",
                    case, outcome
                );
            }
        }
    }

    #[test]
    fn test_render_check_summary_includes_a11y_scope_note() {
        let summary = render_check_summary();
        assert!(summary.contains(A11Y_SCOPE_NOTE));
        assert!(summary.contains("WCAG"));
        assert!(summary.contains("Validation succeeded"));
    }

    #[test]
    fn test_render_init_summary_with_fallback_reason() {
        let summary = render_init_summary(
            "WDA Minimal",
            Some("Online Design System resolution is unavailable in this build"),
        );
        assert!(summary.contains("WDA Minimal"));
        assert!(summary.contains("Online Design System resolution is unavailable in this build"));
        assert!(summary.contains("Design System: WDA Minimal"));
        assert!(summary.contains("Note: Online Design System resolution is unavailable in this build"));
    }

    #[test]
    fn test_render_init_summary_without_fallback_reason() {
        let summary = render_init_summary("Spectrum", None);
        assert!(summary.contains("Spectrum"));
        assert!(summary.contains("Design System: Spectrum"));
        assert!(!summary.contains("Note:"));
    }

    #[test]
    fn test_render_build_summary_reports_http_limitation() {
        let path = if cfg!(windows) {
            std::path::PathBuf::from(r"C:\projects\site\dist")
        } else {
            std::path::PathBuf::from("/projects/site/dist")
        };
        let summary = render_build_summary(&path, true);
        assert!(summary.contains(&path.display().to_string()));
        assert!(summary.contains("HTTP server required"));
        assert!(summary.contains("file:// will not work"));
    }

    #[test]
    fn test_render_build_summary_does_not_guarantee_file_urls() {
        let path = std::path::PathBuf::from("dist");
        let summary = render_build_summary(&path, false);
        assert!(summary.contains("No known HTTP requirement was detected"));
        assert!(summary.contains("dist"));
        assert!(!summary.contains("file://"));
    }
}
