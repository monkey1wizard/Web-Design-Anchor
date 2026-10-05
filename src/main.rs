use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use wda_core::cli::{self, DepsCommand, Invocation};
use wda_core::diagnostics::{self, ToolFault, ValidationReport};

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
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let invocation = cli::parse_args(&args);

    let status = match invocation {
        Invocation::Version => {
            println!("wda {}", env!("CARGO_PKG_VERSION"));
            map_outcome_to_exit_status(&InvocationOutcome::Success)
        }
        Invocation::UsageError(diag) => {
            eprintln!("{}", diagnostics::render(&diag));
            map_outcome_to_exit_status(&InvocationOutcome::UsageError)
        }
        Invocation::Deps(cmd) => {
            let root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let result = match &cmd {
                DepsCommand::Add(spec) => wda_core::add_dependency(&root, spec).map(|_| ()),
                DepsCommand::Remove(name) => wda_core::remove_dependency(&root, name).map(|_| ()),
                DepsCommand::Update(name_filter) => {
                    wda_core::update_dependencies(&root, name_filter.as_deref()).map(|_| ())
                }
            };
            match result {
                Ok(()) => {
                    println!("{}", cli::render_deps_summary(&cmd));
                    map_outcome_to_exit_status(&InvocationOutcome::Success)
                }
                Err(wda_core::DepsError::Diagnostic(diag)) => {
                    eprintln!("{}", diagnostics::render(&diag));
                    map_outcome_to_exit_status(&InvocationOutcome::UsageError)
                }
                Err(wda_core::DepsError::ToolFault(fault)) => {
                    eprintln!("{}", diagnostics::render_fault(&fault));
                    map_outcome_to_exit_status(&InvocationOutcome::Fault(&fault))
                }
            }
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
        Invocation::Init => {
            let root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            match wda_core::init_project(&root) {
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
            Some(Location::path_only("wda.json")),
            "Missing wda.json",
            "Run wda init to create wda.json",
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
