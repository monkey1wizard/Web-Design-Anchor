use serde_json::{json, Value};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use wda_core::cli::{self, Invocation};
use wda_core::diagnostics::Severity;
use wda_core::diagnostics::{self, ToolFault, ValidationReport};

fn json_path(path: &std::path::Path) -> String {
    let path = if path.is_absolute() {
        path.file_name().map(std::path::Path::new).unwrap_or(path)
    } else {
        path
    };
    path.to_string_lossy().replace('\\', "/")
}

fn json_report(
    command: &str,
    status: &str,
    report: &ValidationReport,
    dist: Option<&std::path::Path>,
    needs_http: Option<bool>,
) -> Value {
    let diagnostics: Vec<Value> = report.diagnostics.iter().map(|d| {
        let location = d.location.as_ref();
        json!({"code": d.code, "severity": if d.severity == Severity::Error {"error"} else {"warning"},
            "path": location.map(|l| json_path(&l.path)),
            "line": location.and_then(|l| l.line), "column": location.and_then(|l| l.column),
            "message": d.message, "next_action": d.next_action})
    }).collect();
    let errors = report
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warnings = report.diagnostics.len() - errors;
    json!({"schema_version": 1, "command": command, "status": status, "diagnostics": diagnostics,
        "summary": {"errors": errors, "warnings": warnings, "dist_path": dist.map(|p| p.to_string_lossy().replace('\\', "/")), "needs_http": needs_http}})
}

fn run_json(args: &[String]) -> Option<u8> {
    let command = args.first()?.as_str();
    if !matches!(command, "check" | "build")
        || !args
            .iter()
            .skip(1)
            .take_while(|a| *a != "--")
            .any(|a| a == "--json")
    {
        return None;
    }
    let remaining: Vec<&str> = args
        .iter()
        .skip(1)
        .map(String::as_str)
        .filter(|a| *a != "--json")
        .collect();
    let count = args.iter().skip(1).filter(|a| *a == "--json").count();
    let parsed = if count == 1 && remaining.is_empty() {
        cli::parse_args(&[command])
    } else {
        cli::parse_args(&[command, "unexpected argument"])
    };
    let mut report = ValidationReport::new();
    let mut dist = None;
    let mut needs_http = None;
    let status = if let Invocation::UsageError(d) = parsed {
        report.add(d);
        "usage-error"
    } else {
        let root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let result = if command == "check" {
            wda_core::validate_project(&root).map(|r| (r, None))
        } else {
            wda_core::build_project(&root)
                .map(|r| (r.validation, Some((r.dist_path, r.needs_http))))
        };
        match result {
            Ok((mut validation, artifact)) => {
                validation.sort();
                report = validation;
                if report.has_error() {
                    "project-error"
                } else {
                    if let Some((path, http)) = artifact {
                        dist = Some(path.strip_prefix(&root).unwrap_or(&path).to_path_buf());
                        needs_http = Some(http);
                    }
                    "success"
                }
            }
            Err(fault) => {
                report.add(diagnostics::Diagnostic::new(
                    wda_core::codes::TOOL_FAULT,
                    Severity::Error,
                    fault.path.map(diagnostics::Location::path_only),
                    fault.message,
                    "Resolve the tool failure and try again",
                ));
                "tool-fault"
            }
        }
    };
    println!(
        "{}",
        json_report(command, status, &report, dist.as_deref(), needs_http)
    );
    Some(match status {
        "success" => 0,
        "project-error" => 1,
        _ => 2,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum InvocationOutcome<'a> {
    Success,
    Report(&'a ValidationReport),
    UsageError,
    Fault(&'a ToolFault),
}

pub fn map_outcome_to_exit_status(outcome: &InvocationOutcome) -> u8 {
    match outcome {
        InvocationOutcome::Success => 0,
        InvocationOutcome::Report(report) => {
            if report.has_error() {
                1
            } else {
                0
            }
        }
        InvocationOutcome::UsageError | InvocationOutcome::Fault(_) => 2,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    if let Some(status) = run_json(&args) {
        return ExitCode::from(status);
    }
    let invocation = cli::parse_args(&args);

    let status = match &invocation {
        Invocation::ToolsFetch => match wda_core::tools::fetch() {
            Ok(path) => {
                println!("Toolchain ready: {}", path.display());
                0
            }
            Err(error) => {
                eprintln!("wda.tools.fetch: {error}");
                1
            }
        },
        Invocation::Version => {
            println!("wda {}", env!("CARGO_PKG_VERSION"));
            map_outcome_to_exit_status(&InvocationOutcome::Success)
        }
        Invocation::Usage => {
            println!("{}", cli::USAGE);
            map_outcome_to_exit_status(&InvocationOutcome::Success)
        }
        Invocation::UsageError(diag) => {
            eprintln!("{}", diagnostics::render(&diag));
            map_outcome_to_exit_status(&InvocationOutcome::UsageError)
        }
        Invocation::Build => {
            let root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            match wda_core::build_project(&root) {
                Ok(mut report) => {
                    report.validation.sort();
                    for diag in &report.validation.diagnostics {
                        eprintln!("{}", diagnostics::render(diag));
                    }
                    if !report.validation.has_error() {
                        println!(
                            "{}",
                            cli::render_build_summary(&report.dist_path, report.needs_http)
                        );
                    }
                    map_outcome_to_exit_status(&InvocationOutcome::Report(&report.validation))
                }
                Err(fault) => {
                    eprintln!("{}", diagnostics::render_fault(&fault));
                    map_outcome_to_exit_status(&InvocationOutcome::Fault(&fault))
                }
            }
        }
        Invocation::Init | Invocation::InitManager(_) => {
            let root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let options = wda_core::InitOptions {
                package_manager: match &invocation {
                    Invocation::InitManager(name) => Some(name.clone()),
                    _ => None,
                },
            };
            match wda_core::init_project_with_options(&root, &options) {
                Ok((mut report, resolved_ds)) => {
                    report.sort();
                    for diag in &report.diagnostics {
                        eprintln!("{}", diagnostics::render(diag));
                    }
                    if !report.has_error() {
                        println!(
                            "{}",
                            cli::render_init_summary(
                                &resolved_ds.name,
                                resolved_ds.fallback_reason.as_deref(),
                            )
                        );
                    }
                    map_outcome_to_exit_status(&InvocationOutcome::Report(&report))
                }
                Err(fault) => {
                    eprintln!("{}", diagnostics::render_fault(&fault));
                    map_outcome_to_exit_status(&InvocationOutcome::Fault(&fault))
                }
            }
        }
        Invocation::Check => {
            let root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            match wda_core::validate_project(&root) {
                Ok(mut report) => {
                    report.sort();
                    for diag in &report.diagnostics {
                        eprintln!("{}", diagnostics::render(diag));
                    }
                    if !report.has_error() {
                        println!("{}", cli::render_check_summary());
                    }
                    map_outcome_to_exit_status(&InvocationOutcome::Report(&report))
                }
                Err(fault) => {
                    eprintln!("{}", diagnostics::render_fault(&fault));
                    map_outcome_to_exit_status(&InvocationOutcome::Fault(&fault))
                }
            }
        }
    };

    ExitCode::from(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wda_core::codes;
    use wda_core::diagnostics::{Diagnostic, Location, Severity, ToolFault, ValidationReport};

    #[test]
    fn json_report_preserves_diagnostics_and_nulls() {
        let mut report = ValidationReport::new();
        report.add(Diagnostic::new(
            codes::TOOL_FAULT,
            Severity::Error,
            None,
            "fault\ntext",
            "retry\tplease",
        ));
        report.add(Diagnostic::new(
            codes::TOOLCHAIN_GIT_MISSING,
            Severity::Warning,
            Some(Location::new("pages/test.html", Some(2), Some(3))),
            "warning",
            "act",
        ));
        report.sort();
        let value = json_report("build", "tool-fault", &report, None, None);
        assert_eq!(value["diagnostics"][0]["message"], "fault\ntext");
        assert_eq!(value["diagnostics"][0]["path"], Value::Null);
        assert_eq!(value["diagnostics"][1]["line"], 2);
        assert_eq!(value["summary"]["errors"], 1);
        assert_eq!(value["summary"]["warnings"], 1);
        assert_eq!(value["summary"]["dist_path"], Value::Null);
    }

    #[test]
    fn test_map_outcome_to_exit_status_success() {
        assert_eq!(map_outcome_to_exit_status(&InvocationOutcome::Success), 0);
    }

    #[test]
    fn test_map_outcome_to_exit_status_clean_report() {
        let report = ValidationReport::new();
        assert_eq!(
            map_outcome_to_exit_status(&InvocationOutcome::Report(&report)),
            0
        );
    }

    #[test]
    fn test_map_outcome_to_exit_status_error_report() {
        let mut report = ValidationReport::new();
        report.add(Diagnostic::new(
            codes::CONTRACT_MISSING,
            Severity::Error,
            Some(Location::path_only("anchor.json")),
            "Missing anchor.json",
            "Run wda init to create anchor.json",
        ));
        assert_eq!(
            map_outcome_to_exit_status(&InvocationOutcome::Report(&report)),
            1
        );
    }

    #[test]
    fn test_map_outcome_to_exit_status_usage_error() {
        assert_eq!(
            map_outcome_to_exit_status(&InvocationOutcome::UsageError),
            2
        );
    }

    #[test]
    fn test_map_outcome_to_exit_status_tool_fault() {
        let fault = ToolFault::new(
            "Project root directory is invalid or unreadable",
            Some(PathBuf::from("invalid")),
        );
        assert_eq!(
            map_outcome_to_exit_status(&InvocationOutcome::Fault(&fault)),
            2
        );
    }
}
