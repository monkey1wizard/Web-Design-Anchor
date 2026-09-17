//! Command-line argument parsing and dispatch for Web Design Anchor (WDA).

use crate::codes;
use crate::diagnostics::{Diagnostic, Severity};
use std::path::Path;

/// Subcommands supported by `wda deps`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepsCommand {
    Add(String),
    Remove(String),
    Update(Option<String>),
}

/// Parsed CLI invocation outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation {
    Check,
    Init,
    Deps(DepsCommand),
    Build,
    Version,
    UsageError(Diagnostic),
}

/// Parses an argv slice into an [`Invocation`].
///
/// Handles command dispatch for `check`, `init`, `deps`, `build`, and `--version`.
/// Anything else (unknown command, unknown flag, missing command, extra arguments)
/// returns [`Invocation::UsageError`] carrying `wda.cli.usage`.
pub fn parse_args<S: AsRef<str>>(args: &[S]) -> Invocation {
    let tokens: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    if tokens.is_empty() {
        return Invocation::UsageError(make_usage_error("No command or arguments provided"));
    }

    if tokens.iter().any(|&t| t == "--json") {
        return Invocation::UsageError(make_json_not_supported_error());
    }

    // Determine if first token is a command/flag or binary name
    let (cmd_token, rest) = match tokens[0] {
        "check" | "init" | "deps" | "build" | "--version" => (tokens[0], &tokens[1..]),
        _ if tokens.len() > 1 => match tokens[1] {
            "check" | "init" | "deps" | "build" | "--version" => (tokens[1], &tokens[2..]),
            _ => {
                return Invocation::UsageError(make_usage_error(format!(
                    "Unknown command or flag: {}",
                    tokens[1]
                )))
            }
        },
        _ => {
            return Invocation::UsageError(make_usage_error(format!(
                "Unknown command or flag: {}",
                tokens[0]
            )))
        }
    };

    if cmd_token != "deps" && !rest.is_empty() {
        return Invocation::UsageError(make_usage_error(format!(
            "Unexpected argument: {}",
            rest[0]
        )));
    }

    match cmd_token {
        "check" => Invocation::Check,
        "init" => Invocation::Init,
        "deps" => parse_deps_args(rest),
        "build" => Invocation::Build,
        "--version" => Invocation::Version,
        _ => Invocation::UsageError(make_usage_error(format!(
            "Unknown command or flag: {cmd_token}"
        ))),
    }
}

fn parse_deps_args(rest: &[&str]) -> Invocation {
    if rest.is_empty() {
        return Invocation::UsageError(make_usage_error(
            "Missing subcommand for 'deps': expected add, remove, or update",
        ));
    }

    match rest[0] {
        "add" => {
            if rest.len() < 2 {
                Invocation::UsageError(make_usage_error("Missing package specifier for 'deps add'"))
            } else if rest[1].trim().is_empty() {
                Invocation::UsageError(make_usage_error("Package specifier cannot be empty"))
            } else if rest.len() > 2 {
                Invocation::UsageError(make_usage_error(format!(
                    "Unexpected argument: {}",
                    rest[2]
                )))
            } else {
                Invocation::Deps(DepsCommand::Add(rest[1].to_string()))
            }
        }
        "remove" => {
            if rest.len() < 2 {
                Invocation::UsageError(make_usage_error(
                    "Missing dependency name for 'deps remove'",
                ))
            } else if rest[1].trim().is_empty() {
                Invocation::UsageError(make_usage_error("Dependency name cannot be empty"))
            } else if rest.len() > 2 {
                Invocation::UsageError(make_usage_error(format!(
                    "Unexpected argument: {}",
                    rest[2]
                )))
            } else {
                Invocation::Deps(DepsCommand::Remove(rest[1].to_string()))
            }
        }
        "update" => {
            if rest.len() == 1 {
                Invocation::Deps(DepsCommand::Update(None))
            } else if rest.len() == 2 {
                if rest[1].trim().is_empty() {
                    Invocation::UsageError(make_usage_error(
                        "Dependency name filter cannot be empty",
                    ))
                } else {
                    Invocation::Deps(DepsCommand::Update(Some(rest[1].to_string())))
                }
            } else {
                Invocation::UsageError(make_usage_error(format!(
                    "Unexpected argument: {}",
                    rest[2]
                )))
            }
        }
        unknown => Invocation::UsageError(make_usage_error(format!(
            "Unknown deps subcommand: {unknown}"
        ))),
    }
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
        "Run 'wda --version' or check command usage: wda <check|init|deps|build|--version>",
    )
}

/// Renders the success summary for `wda deps`.
pub fn render_deps_summary(command: &DepsCommand) -> String {
    match command {
        DepsCommand::Add(spec) => format!("Dependencies updated.\nAdded: {spec}"),
        DepsCommand::Remove(name) => format!("Dependencies updated.\nRemoved: {name}"),
        DepsCommand::Update(Some(name)) => format!("Dependencies updated.\nUpdated: {name}"),
        DepsCommand::Update(None) => "Dependencies updated.".to_string(),
    }
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
        assert_eq!(
            parse_args(&["deps", "add", "is-number@7.0.0"]),
            Invocation::Deps(DepsCommand::Add("is-number@7.0.0".to_string()))
        );
        assert_eq!(
            parse_args(&["deps", "remove", "is-number"]),
            Invocation::Deps(DepsCommand::Remove("is-number".to_string()))
        );
        assert_eq!(
            parse_args(&["deps", "update"]),
            Invocation::Deps(DepsCommand::Update(None))
        );
        assert_eq!(
            parse_args(&["deps", "update", "is-number"]),
            Invocation::Deps(DepsCommand::Update(Some("is-number".to_string())))
        );
        assert_eq!(parse_args(&["build"]), Invocation::Build);
        assert_eq!(parse_args(&["--version"]), Invocation::Version);

        assert_eq!(parse_args(&["wda", "check"]), Invocation::Check);
        assert_eq!(parse_args(&["wda", "init"]), Invocation::Init);
        assert_eq!(
            parse_args(&["wda", "deps", "add", "is-number@7.0.0"]),
            Invocation::Deps(DepsCommand::Add("is-number@7.0.0".to_string()))
        );
        assert_eq!(
            parse_args(&["wda", "deps", "remove", "is-number"]),
            Invocation::Deps(DepsCommand::Remove("is-number".to_string()))
        );
        assert_eq!(
            parse_args(&["wda", "deps", "update"]),
            Invocation::Deps(DepsCommand::Update(None))
        );
        assert_eq!(
            parse_args(&["wda", "deps", "update", "is-number"]),
            Invocation::Deps(DepsCommand::Update(Some("is-number".to_string())))
        );
        assert_eq!(parse_args(&["wda", "build"]), Invocation::Build);
        assert_eq!(parse_args(&["wda", "--version"]), Invocation::Version);
    }

    #[test]
    fn test_parse_args_deps_missing_subcommand_or_spec_carries_cli_usage() {
        let missing_sub = parse_args(&["deps"]);
        if let Invocation::UsageError(diag) = missing_sub {
            assert_eq!(diag.code, codes::CLI_USAGE);
        } else {
            panic!("Expected UsageError for bare deps, got {:?}", missing_sub);
        }

        let missing_add_spec = parse_args(&["deps", "add"]);
        if let Invocation::UsageError(diag) = missing_add_spec {
            assert_eq!(diag.code, codes::CLI_USAGE);
        } else {
            panic!("Expected UsageError for deps add without spec, got {:?}", missing_add_spec);
        }

        let missing_remove_name = parse_args(&["deps", "remove"]);
        if let Invocation::UsageError(diag) = missing_remove_name {
            assert_eq!(diag.code, codes::CLI_USAGE);
        } else {
            panic!("Expected UsageError for deps remove without name, got {:?}", missing_remove_name);
        }

        let unknown_sub = parse_args(&["deps", "foo"]);
        if let Invocation::UsageError(diag) = unknown_sub {
            assert_eq!(diag.code, codes::CLI_USAGE);
        } else {
            panic!("Expected UsageError for deps foo, got {:?}", unknown_sub);
        }

        let extra_add = parse_args(&["deps", "add", "pkg", "extra"]);
        if let Invocation::UsageError(diag) = extra_add {
            assert_eq!(diag.code, codes::CLI_USAGE);
        } else {
            panic!("Expected UsageError for deps add with extra arg, got {:?}", extra_add);
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
    fn test_parse_args_empty_or_binary_only_carries_cli_usage() {
        let outcome_empty = parse_args::<&str>(&[]);
        if let Invocation::UsageError(diag) = outcome_empty {
            assert_eq!(diag.code, codes::CLI_USAGE);
        } else {
            panic!("Expected UsageError for empty args, got {:?}", outcome_empty);
        }

        let outcome_bin_only = parse_args(&["wda"]);
        if let Invocation::UsageError(diag) = outcome_bin_only {
            assert_eq!(diag.code, codes::CLI_USAGE);
        } else {
            panic!("Expected UsageError for binary-only args, got {:?}", outcome_bin_only);
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
    fn test_render_deps_summary() {
        let add_summary = render_deps_summary(&DepsCommand::Add("is-number@7.0.0".to_string()));
        assert!(add_summary.contains("Dependencies updated"));
        assert!(add_summary.contains("is-number@7.0.0"));

        let remove_summary = render_deps_summary(&DepsCommand::Remove("is-number".to_string()));
        assert!(remove_summary.contains("Dependencies updated"));
        assert!(remove_summary.contains("is-number"));

        let update_filtered =
            render_deps_summary(&DepsCommand::Update(Some("is-number".to_string())));
        assert!(update_filtered.contains("Dependencies updated"));
        assert!(update_filtered.contains("is-number"));

        let update_unfiltered = render_deps_summary(&DepsCommand::Update(None));
        assert_eq!(update_unfiltered, "Dependencies updated.");
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
