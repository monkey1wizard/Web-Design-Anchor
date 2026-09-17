pub(crate) mod build;
pub(crate) mod builtins;
pub mod cli;
pub mod codes;
pub(crate) mod deps;
pub mod diagnostics;
pub(crate) mod init;
pub(crate) mod validation {
    pub mod build_output;
    pub mod documents;
    pub mod html;
    pub mod project_contract;
    pub mod tokens;
}

/// Validation scope controlling which rules run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValidationScope {
    Check,
    Build,
}

/// Seam to validate a project at `root`.
///
/// Returns a [`diagnostics::ValidationReport`] containing diagnostics,
/// or a [`diagnostics::ToolFault`] if an internal tool failure occurs.
pub fn validate_project(
    root: &std::path::Path,
) -> Result<diagnostics::ValidationReport, diagnostics::ToolFault> {
    validate_project_with(root, ValidationScope::Check)
}

/// Internal validation seam parameterized by [`ValidationScope`].
pub(crate) fn validate_project_with(
    root: &std::path::Path,
    scope: ValidationScope,
) -> Result<diagnostics::ValidationReport, diagnostics::ToolFault> {
    if !root.is_dir() {
        return Err(diagnostics::ToolFault::new(
            "Project root directory is invalid or unreadable",
            Some(root.to_path_buf()),
        ));
    }
    let mut report = validation::project_contract::validate_project_contract(root)?;
    let docs_report = validation::documents::validate_documents(root)?;
    for diagnostic in docs_report.diagnostics {
        report.add(diagnostic);
    }
    let html_report = validation::html::validate_html(root)?;
    for diagnostic in html_report.diagnostics {
        report.add(diagnostic);
    }
    let tokens_report = validation::tokens::validate_tokens(root)?;
    for diagnostic in tokens_report.diagnostics {
        report.add(diagnostic);
    }
    if scope == ValidationScope::Build {
        let build_output_report = validation::build_output::validate_build_output(root)?;
        for diagnostic in build_output_report.diagnostics {
            report.add(diagnostic);
        }
    }
    report.sort();
    Ok(report)
}

pub use build::BuildReport;
pub use init::resolve::ResolvedDesignSystem;

/// Seam to initialize a project at `root`.
///
/// Returns a tuple of [`diagnostics::ValidationReport`] and [`ResolvedDesignSystem`]
/// containing diagnostics if the gate refuses or success report with resolved design system,
/// or a [`diagnostics::ToolFault`] if an internal tool failure occurs.
pub fn init_project(
    root: &std::path::Path,
) -> Result<(diagnostics::ValidationReport, ResolvedDesignSystem), diagnostics::ToolFault> {
    init::init_project(root)
}

/// Seam to build a project at `root`.
///
/// Returns a [`BuildReport`] containing diagnostics, the absolute `dist/` path,
/// and the HTTP-server requirement flag, or a [`diagnostics::ToolFault`] if an internal tool failure occurs.
pub fn build_project(
    root: &std::path::Path,
) -> Result<BuildReport, diagnostics::ToolFault> {
    build::build_project(root)
}

pub use deps::resolve::StagedFiles;
pub use deps::{CommitOutcome, DepsError};

/// Seam to add a dependency specification to the project at `root`.
///
/// Returns [`StagedFiles`] on success, or [`DepsError`] on failure.
pub fn add_dependency(
    root: &std::path::Path,
    spec: &str,
) -> Result<StagedFiles, DepsError> {
    deps::add(root, spec)
}

/// Direct alias for [`add_dependency`].
pub fn deps_add(
    root: &std::path::Path,
    spec: &str,
) -> Result<StagedFiles, DepsError> {
    deps::add(root, spec)
}

/// Direct alias for [`add_dependency`].
pub fn add(
    root: &std::path::Path,
    spec: &str,
) -> Result<StagedFiles, DepsError> {
    deps::add(root, spec)
}

/// Seam to remove a dependency from the project at `root`.
///
/// Returns [`CommitOutcome`] on success, or [`DepsError`] on failure.
pub fn remove_dependency(
    root: &std::path::Path,
    name: &str,
) -> Result<CommitOutcome, DepsError> {
    deps::remove(root, name)
}

/// Direct alias for [`remove_dependency`].
pub fn deps_remove(
    root: &std::path::Path,
    name: &str,
) -> Result<CommitOutcome, DepsError> {
    deps::remove(root, name)
}

/// Direct alias for [`remove_dependency`].
pub fn remove(
    root: &std::path::Path,
    name: &str,
) -> Result<CommitOutcome, DepsError> {
    deps::remove(root, name)
}

/// Seam to update dependencies in the project at `root`.
///
/// Returns `Option<StagedFiles>` on success (`None` when zero dependencies are present),
/// or [`DepsError`] on failure.
pub fn update_dependencies(
    root: &std::path::Path,
    name_filter: Option<&str>,
) -> Result<Option<StagedFiles>, DepsError> {
    deps::update(root, name_filter)
}

/// Direct alias for [`update_dependencies`].
pub fn update_dependency(
    root: &std::path::Path,
    name_filter: Option<&str>,
) -> Result<Option<StagedFiles>, DepsError> {
    deps::update(root, name_filter)
}

/// Direct alias for [`update_dependencies`].
pub fn deps_update(
    root: &std::path::Path,
    name_filter: Option<&str>,
) -> Result<Option<StagedFiles>, DepsError> {
    deps::update(root, name_filter)
}

/// Direct alias for [`update_dependencies`].
pub fn update(
    root: &std::path::Path,
    name_filter: Option<&str>,
) -> Result<Option<StagedFiles>, DepsError> {
    deps::update(root, name_filter)
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture standing in for offline, injected design-system resolution so these
    /// tests never reach the real online resolver's network call.
    fn offline_wda_minimal() -> ResolvedDesignSystem {
        ResolvedDesignSystem {
            name: init::resolve::BUILTIN_DESIGN_SYSTEM_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            fallback_reason: Some(init::resolve::FALLBACK_REASON_DENO_ABSENT.to_string()),
        }
    }

    #[test]
    fn test_tool_fault_renders_without_host_directory_in_message() {
        let absolute_root = if cfg!(windows) {
            std::path::PathBuf::from("C:\\Users\\testuser\\projects\\my_project")
        } else {
            std::path::PathBuf::from("/home/testuser/projects/my_project")
        };

        let err = validate_project(&absolute_root).unwrap_err();
        assert_eq!(err.message, "Project root directory is invalid or unreadable");
        let rendered = diagnostics::render_fault(&err);
        assert!(!rendered.contains("testuser"));
        assert!(!rendered.contains("projects"));
        assert!(rendered.contains("my_project"));
    }

    #[test]
    fn test_validate_project_includes_document_validation_diagnostics() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_project_val_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let report = validate_project(&temp_dir).expect("validation report");
        let missing_docs_count = report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::DOCS_REQUIRED_DOCUMENT_MISSING)
            .count();
        assert_eq!(
            missing_docs_count, 4,
            "an empty directory should produce 4 missing document diagnostics"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_validate_project_includes_html_validation_diagnostics() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_project_html_val_{}", std::process::id()));
        let pages_dir = temp_dir.join("pages");
        std::fs::create_dir_all(&pages_dir).unwrap();

        std::fs::write(
            pages_dir.join("index.html"),
            "<!DOCTYPE html>\n<html>\n<head></head>\n<body></body>\n</html>\n",
        )
        .unwrap();

        let report = validate_project(&temp_dir).expect("validation report");
        let html_missing_count = report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::HTML_LANG_MISSING)
            .count();
        assert_eq!(
            html_missing_count, 1,
            "project validation must include html lang-missing diagnostics"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_validate_project_includes_token_validation_diagnostics() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_project_tokens_val_{}", std::process::id()));
        let tokens_dir = temp_dir.join("tokens");
        std::fs::create_dir_all(&tokens_dir).unwrap();

        std::fs::write(
            tokens_dir.join("tokens.json"),
            "{ invalid json }",
        )
        .unwrap();

        let report = validate_project(&temp_dir).expect("validation report");
        let token_malformed_count = report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::TOKENS_MALFORMED_JSON)
            .count();
        assert_eq!(
            token_malformed_count, 1,
            "project validation must include token validation diagnostics"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_total_order_independent_of_rule_execution_order() {
        let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("probe_all_families");

        assert!(
            fixture_dir.is_dir(),
            "fixture directory 'probe_all_families' must exist"
        );

        // Forward execution order: contract -> docs -> html -> tokens
        let mut forward_report = diagnostics::ValidationReport::new();
        let contract_rep = validation::project_contract::validate_project_contract(&fixture_dir)
            .expect("contract report");
        for d in contract_rep.diagnostics {
            forward_report.add(d);
        }
        let docs_rep = validation::documents::validate_documents(&fixture_dir)
            .expect("documents report");
        for d in docs_rep.diagnostics {
            forward_report.add(d);
        }
        let html_rep = validation::html::validate_html(&fixture_dir)
            .expect("html report");
        for d in html_rep.diagnostics {
            forward_report.add(d);
        }
        let tokens_rep = validation::tokens::validate_tokens(&fixture_dir)
            .expect("tokens report");
        for d in tokens_rep.diagnostics {
            forward_report.add(d);
        }
        forward_report.sort();

        // Reversed execution order: tokens -> html -> docs -> contract
        let mut reversed_report = diagnostics::ValidationReport::new();
        let tokens_rep = validation::tokens::validate_tokens(&fixture_dir)
            .expect("tokens report");
        for d in tokens_rep.diagnostics {
            reversed_report.add(d);
        }
        let html_rep = validation::html::validate_html(&fixture_dir)
            .expect("html report");
        for d in html_rep.diagnostics {
            reversed_report.add(d);
        }
        let docs_rep = validation::documents::validate_documents(&fixture_dir)
            .expect("documents report");
        for d in docs_rep.diagnostics {
            reversed_report.add(d);
        }
        let contract_rep = validation::project_contract::validate_project_contract(&fixture_dir)
            .expect("contract report");
        for d in contract_rep.diagnostics {
            reversed_report.add(d);
        }
        reversed_report.sort();

        assert_eq!(
            forward_report.diagnostics.len(),
            4,
            "expected exactly 4 diagnostics in forward report"
        );

        // Normalize insertion ordinals to compare the diagnostic finding sequences
        for d in &mut forward_report.diagnostics {
            d.ordinal = 0;
        }
        for d in &mut reversed_report.diagnostics {
            d.ordinal = 0;
        }

        assert_eq!(
            forward_report.diagnostics, reversed_report.diagnostics,
            "sorted report diagnostic sequences must be equal regardless of rule family execution order"
        );
    }

    #[test]
    fn test_init_project_seam_gate() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_lib_init_gate_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);

        // Empty directory initializes successfully with zero diagnostics
        let (empty_report, empty_ds) = init::init_project_with_resolution(&temp_dir, offline_wda_minimal())
            .expect("empty directory succeeds");
        assert!(
            empty_report.diagnostics.is_empty(),
            "empty directory must produce zero diagnostics, got: {:?}",
            empty_report.diagnostics
        );
        assert_eq!(empty_ds.name, "WDA Minimal");

        // Second call on already-initialized root collides on planned names
        let hidden_file = temp_dir.join(".hidden");
        std::fs::write(&hidden_file, "content").unwrap();
        let (report, _) = init::init_project_with_resolution(&temp_dir, offline_wda_minimal())
            .expect("report on colliding dir");
        assert!(
            !report.diagnostics.is_empty(),
            "colliding directory must produce diagnostics"
        );
        for diagnostic in &report.diagnostics {
            assert_eq!(diagnostic.code, codes::INIT_PATH_CONFLICT);
            assert_eq!(diagnostic.severity, diagnostics::Severity::Error);
            let path_str = diagnostic
                .location
                .as_ref()
                .unwrap()
                .path
                .to_string_lossy()
                .replace('\\', "/");
            assert!(diagnostic.message.contains(&path_str));
        }
        let colliding_paths: Vec<String> = report
            .diagnostics
            .iter()
            .map(|d| {
                d.location
                    .as_ref()
                    .unwrap()
                    .path
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        for planned_name in [
            "wda.json",
            "README.md",
            "docs/architecture.md",
            "docs/design.md",
            "docs/naming.md",
            "tokens/tokens.json",
            "pages/index.html",
            ".gitignore",
            "docs",
            "tokens",
            "pages",
        ] {
            assert!(
                colliding_paths.contains(&planned_name.to_string()),
                "colliding paths must include {planned_name}, got: {colliding_paths:?}"
            );
        }
        assert!(
            !colliding_paths.contains(&".hidden".to_string()),
            ".hidden must not collide"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_validation_scope_and_build_only_tokens_css_rule() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_scope_tokens_css_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 1. Initialize a clean, valid WDA project
        let (init_report, _) = init::init_project_with_resolution(&temp_dir, offline_wda_minimal())
            .expect("init succeeds");
        assert!(init_report.diagnostics.is_empty(), "init must be clean");

        // 2. Add source-side styles/tokens.css
        let styles_dir = temp_dir.join("styles");
        std::fs::create_dir_all(&styles_dir).unwrap();
        std::fs::write(styles_dir.join("tokens.css"), "/* source tokens css */").unwrap();

        // Scope::Check must yield zero diagnostics
        let check_report = validate_project_with(&temp_dir, ValidationScope::Check)
            .expect("check scope report");
        assert!(
            check_report.diagnostics.is_empty(),
            "Check scope must produce zero diagnostics for source styles/tokens.css, got: {:?}",
            check_report.diagnostics
        );

        // validate_project (public seam) must also yield zero diagnostics
        let public_report = validate_project(&temp_dir).expect("public validate_project report");
        assert!(
            public_report.diagnostics.is_empty(),
            "validate_project must produce zero diagnostics, got: {:?}",
            public_report.diagnostics
        );

        // Scope::Build must yield exactly one Error with BUILD_TOKENS_CSS_IN_SOURCE
        let build_report = validate_project_with(&temp_dir, ValidationScope::Build)
            .expect("build scope report");
        assert_eq!(
            build_report.diagnostics.len(),
            1,
            "Build scope must produce exactly one diagnostic, got: {:?}",
            build_report.diagnostics
        );
        let diag = &build_report.diagnostics[0];
        assert_eq!(diag.code, codes::BUILD_TOKENS_CSS_IN_SOURCE);
        assert_eq!(diag.severity, diagnostics::Severity::Error);
        assert_eq!(
            diag.location.as_ref().unwrap().path,
            std::path::PathBuf::from("styles/tokens.css")
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_build_project_seam() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_lib_build_seam_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 1. Clean fixture initialized with init_project
        let (init_report, _) = init::init_project_with_resolution(&temp_dir, offline_wda_minimal())
            .expect("init succeeds");
        assert!(init_report.diagnostics.is_empty());

        let dist_dir = temp_dir.join("dist");
        assert!(!dist_dir.exists());

        if build::scripts::resolve_deno().is_err() {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return;
        }

        // Calling public build_project on a clean fixture succeeds and creates dist/
        let report = build_project(&temp_dir).expect("clean project build succeeds");
        assert!(!report.validation.has_error());
        let index_path = dist_dir.join("index.html");
        assert!(index_path.is_file());
        let clean_index = std::fs::read(&index_path).unwrap();

        // 2. Error fixture (source-side styles/tokens.css)
        let styles_dir = temp_dir.join("styles");
        std::fs::create_dir_all(&styles_dir).unwrap();
        std::fs::write(styles_dir.join("tokens.css"), "/* source tokens */").unwrap();

        // Calling public build_project on an error fixture leaves the existing dist/ unchanged.
        let report = build_project(&temp_dir).expect("error fixture returns report");
        assert!(report.validation.has_error());
        assert_eq!(report.dist_path, dist_dir);
        assert!(!report.needs_http);
        assert!(dist_dir.is_dir(), "existing dist/ must remain on error fixture");
        assert_eq!(std::fs::read(index_path).unwrap(), clean_index);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_deps_project_seams() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_lib_deps_seam_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 1. update_dependencies with zero dependencies is a no-op returning Ok(None)
        let update_res = update_dependencies(&temp_dir, None).expect("zero deps update succeeds");
        assert!(update_res.is_none());

        // Test update aliases match
        let update_alias_res = update_dependency(&temp_dir, None).expect("update_dependency alias succeeds");
        assert!(update_alias_res.is_none());
        let deps_update_res = deps_update(&temp_dir, None).expect("deps_update alias succeeds");
        assert!(deps_update_res.is_none());
        let bare_update_res = update(&temp_dir, None).expect("update alias succeeds");
        assert!(bare_update_res.is_none());

        // 2. remove_dependency with absent package returns CLI_USAGE error
        let remove_res = remove_dependency(&temp_dir, "missing-pkg");
        match remove_res {
            Err(DepsError::Diagnostic(diag)) => {
                assert_eq!(diag.code, codes::CLI_USAGE);
            }
            other => panic!("expected Diagnostic(CLI_USAGE), got: {other:?}"),
        }

        // Test remove aliases match
        let deps_remove_res = deps_remove(&temp_dir, "missing-pkg");
        assert!(deps_remove_res.is_err());
        let bare_remove_res = remove(&temp_dir, "missing-pkg");
        assert!(bare_remove_res.is_err());

        // 3. add_dependency on non-existent directory returns ToolFault
        let non_existent = temp_dir.join("non_existent_subdir");
        let add_res = add_dependency(&non_existent, "pkg@1.0.0");
        match add_res {
            Err(DepsError::ToolFault(fault)) => {
                assert!(fault.message.contains("not found"));
            }
            other => panic!("expected ToolFault, got: {other:?}"),
        }

        // Test add aliases match
        let deps_add_res = deps_add(&non_existent, "pkg@1.0.0");
        assert!(deps_add_res.is_err());
        let bare_add_res = add(&non_existent, "pkg@1.0.0");
        assert!(bare_add_res.is_err());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}





