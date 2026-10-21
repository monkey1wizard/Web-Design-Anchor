use std::path::Path;

use crate::codes;
use crate::diagnostics::{Diagnostic, Location, Severity, ToolFault, ValidationReport};

/// Validates build-only rules against `root`.
///
/// Under `Build` scope:
/// - A source-side `styles/tokens.css` yields an Error carrying `wda.build.tokens-css-in-source`.
pub fn validate_build_output(root: &Path) -> Result<ValidationReport, ToolFault> {
    let mut report = ValidationReport::new();
    let rel_path = "styles/tokens.css";
    let tokens_css_path = root.join("styles").join("tokens.css");

    if tokens_css_path.exists() {
        report.add(Diagnostic::new(
            codes::BUILD_TOKENS_CSS_IN_SOURCE,
            Severity::Error,
            Some(Location::path_only(rel_path)),
            format!("Source-side file '{rel_path}' is forbidden; tokens.css is generated at build time in dist/styles/tokens.css"),
            format!("Remove '{rel_path}' from source; edit tokens/tokens.json instead"),
        ));
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_tokens_css_in_source_yields_error() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_build_output_err_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let styles_dir = temp_dir.join("styles");
        fs::create_dir_all(&styles_dir).unwrap();
        fs::write(styles_dir.join("tokens.css"), "/* custom tokens */").unwrap();

        let report = validate_build_output(&temp_dir).expect("validation report");
        assert_eq!(report.diagnostics.len(), 1);
        let diag = &report.diagnostics[0];
        assert_eq!(diag.code, codes::BUILD_TOKENS_CSS_IN_SOURCE);
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(
            diag.location.as_ref().unwrap().path,
            std::path::PathBuf::from("styles/tokens.css")
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_tokens_css_absent_yields_zero_diagnostics() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_build_output_clean_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let styles_dir = temp_dir.join("styles");
        fs::create_dir_all(&styles_dir).unwrap();
        fs::write(styles_dir.join("main.css"), "/* main css */").unwrap();

        let report = validate_build_output(&temp_dir).expect("validation report");
        assert!(report.diagnostics.is_empty());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
