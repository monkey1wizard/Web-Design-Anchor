use std::path::Path;
use std::sync::OnceLock;
use jsonschema::Validator;
use serde_json::Value;

use crate::codes;
use crate::diagnostics::{Diagnostic, Location, Severity, ToolFault, ValidationReport};

/// Error type when reading or compiling the canonical JSON schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaCompileError {
    Json(String),
    Schema(String),
}

impl std::fmt::Display for SchemaCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaCompileError::Json(msg) => write!(f, "schema JSON parse error: {msg}"),
            SchemaCompileError::Schema(msg) => write!(f, "schema compile error: {msg}"),
        }
    }
}

impl std::error::Error for SchemaCompileError {}

impl From<serde_json::Error> for SchemaCompileError {
    fn from(err: serde_json::Error) -> Self {
        SchemaCompileError::Json(err.to_string())
    }
}

impl From<jsonschema::ValidationError<'static>> for SchemaCompileError {
    fn from(err: jsonschema::ValidationError<'static>) -> Self {
        SchemaCompileError::Schema(err.to_string())
    }
}

/// Canonical raw `wda.schema.json` string embedded at compile time.
pub const WDA_SCHEMA_JSON: &str = include_str!("../../schemas/wda.schema.json");

/// Maximum allowed JSON nesting depth (128).
///
/// Design token files (`tokens/tokens.json`) and project contracts (`wda.json`) in practice are nested
/// no more than 10–20 levels deep. Setting a cap at 128 prevents stack buffer overruns caused by unbounded
/// recursion in `json_sourcemap::parse` or AST traversals on deeply nested inputs while accommodating
/// all legitimate documents.
pub const MAX_JSON_DEPTH: usize = 128;

/// Computes the maximum `{`/`[` nesting depth in `content`, ignoring braces and brackets inside string literals
/// and honoring backslash escapes.
pub fn compute_max_json_depth(content: &str) -> usize {
    let mut in_string = false;
    let mut escaped = false;
    let mut current_depth = 0usize;
    let mut max_depth = 0usize;

    for &b in content.as_bytes() {
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
        } else {
            match b {
                b'"' => in_string = true,
                b'{' | b'[' => {
                    current_depth += 1;
                    if current_depth > max_depth {
                        max_depth = current_depth;
                    }
                }
                b'}' | b']' => {
                    current_depth = current_depth.saturating_sub(1);
                }
                _ => {}
            }
        }
    }

    max_depth
}

static VALIDATOR: OnceLock<Result<Validator, SchemaCompileError>> = OnceLock::new();

/// Returns a reference to the statically compiled [`Validator`], compiling it on first use.
pub fn get_validator() -> Result<&'static Validator, SchemaCompileError> {
    VALIDATOR
        .get_or_init(compile_schema)
        .as_ref()
        .map_err(|e| e.clone())
}

/// Compiles the embedded `wda.schema.json` schema.
///
/// Returns a compiled [`Validator`] or a [`SchemaCompileError`].
pub fn compile_schema() -> Result<Validator, SchemaCompileError> {
    let schema_val: Value = serde_json::from_str(WDA_SCHEMA_JSON)?;
    let validator = jsonschema::validator_for(&schema_val)?;
    Ok(validator)
}

/// Helper function to validate a JSON [`Value`] against the canonical schema.
#[cfg(test)]
pub fn validate_json(instance: &Value) -> Result<bool, SchemaCompileError> {
    let validator = get_validator()?;
    Ok(validator.is_valid(instance))
}

pub(crate) fn escape_json_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

/// Validates the `wda.json` file in `root` against the embedded project contract schema.
///
/// Returns a [`ValidationReport`] containing diagnostics, or a [`ToolFault`] if schema compilation fails.
pub fn validate_project_contract(root: &Path) -> Result<ValidationReport, ToolFault> {
    let validator = match get_validator() {
        Ok(v) => v,
        Err(err) => {
            return Err(ToolFault::new(
                format!("Failed to compile project contract schema: {err}"),
                None::<std::path::PathBuf>,
            ));
        }
    };

    let mut report = ValidationReport::new();
    let rel_path = "wda.json";
    let wda_json_path = root.join(rel_path);

    if !wda_json_path.exists() {
        report.add(Diagnostic::new(
            codes::CONTRACT_MISSING,
            Severity::Error,
            Some(Location::path_only(rel_path)),
            "Project contract file 'wda.json' is missing",
            "Create a valid wda.json file at the project root",
        ));
        return Ok(report);
    }

    let content = match std::fs::read_to_string(&wda_json_path) {
        Ok(c) => c,
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                report.add(Diagnostic::new(
                    codes::CONTRACT_MISSING,
                    Severity::Error,
                    Some(Location::path_only(rel_path)),
                    "Project contract file 'wda.json' is missing",
                    "Create a valid wda.json file at the project root",
                ));
            } else {
                report.add(Diagnostic::new(
                    codes::CONTRACT_MALFORMED_JSON,
                    Severity::Error,
                    Some(Location::path_only(rel_path)),
                    format!("Failed to read wda.json: {err}"),
                    "Ensure wda.json is readable and valid UTF-8 JSON",
                ));
            }
            return Ok(report);
        }
    };

    if compute_max_json_depth(&content) > MAX_JSON_DEPTH {
        report.add(Diagnostic::new(
            codes::CONTRACT_MALFORMED_JSON,
            Severity::Error,
            Some(Location::path_only(rel_path)),
            format!("Malformed JSON in wda.json: nesting depth exceeds maximum allowed limit of {MAX_JSON_DEPTH}"),
            "Fix JSON syntax errors in wda.json",
        ));
        return Ok(report);
    }

    let parsed_sourcemap = match json_sourcemap::parse(&content, json_sourcemap::Options::default()) {
        Ok(res) => res,
        Err(_) => {
            let (loc, msg) = match serde_json::from_str::<Value>(&content) {
                Err(json_err) => {
                    let location = if json_err.line() > 0 {
                        Location::new(rel_path, Some(json_err.line()), Some(json_err.column()))
                    } else {
                        Location::path_only(rel_path)
                    };
                    (location, format!("Malformed JSON in wda.json: {json_err}"))
                }
                Ok(_) => (
                    Location::path_only(rel_path),
                    "Malformed JSON in wda.json: sourcemap parse failed".to_string(),
                ),
            };
            report.add(Diagnostic::new(
                codes::CONTRACT_MALFORMED_JSON,
                Severity::Error,
                Some(loc),
                msg,
                "Fix JSON syntax errors in wda.json",
            ));
            return Ok(report);
        }
    };

    // Note: `json_sourcemap` returns 0-based line and column numbers (`line = 0` for line 1, `column = 0` for column 1).
    // We add 1 to both line and column to convert to 1-based positions as required by the Diagnostic Location contract.
    for err in validator.iter_errors(&parsed_sourcemap.value) {
        if let jsonschema::error::ValidationErrorKind::AdditionalProperties { unexpected } = err.kind() {
            let base_pointer = err.instance_path().to_string();
            for field_name in unexpected {
                let escaped = escape_json_pointer_token(field_name);
                let full_pointer = if base_pointer.is_empty() {
                    format!("/{escaped}")
                } else {
                    format!("{base_pointer}/{escaped}")
                };

                let location = if let Some(pos) = parsed_sourcemap.pointers.get(&full_pointer) {
                    let loc = if full_pointer.is_empty() {
                        pos.value()
                    } else {
                        pos.key()
                    };
                    Location::new(rel_path, Some(loc.line + 1), Some(loc.column + 1))
                } else {
                    Location::path_only(rel_path)
                };

                report.add(Diagnostic::new(
                    codes::CONTRACT_UNKNOWN_FIELD,
                    Severity::Error,
                    Some(location),
                    format!("Unknown field '{field_name}' in project contract wda.json"),
                    format!("Remove unknown field '{field_name}' from wda.json"),
                ));
            }
        } else {
            let pointer = err.instance_path().to_string();
            let location = if let Some(pos) = parsed_sourcemap.pointers.get(&pointer) {
                let loc = if pointer.is_empty() {
                    pos.value()
                } else {
                    pos.key()
                };
                Location::new(rel_path, Some(loc.line + 1), Some(loc.column + 1))
            } else {
                Location::path_only(rel_path)
            };

            report.add(Diagnostic::new(
                codes::CONTRACT_SCHEMA_VIOLATION,
                Severity::Error,
                Some(location),
                format!("Schema violation in wda.json: {err}"),
                "Update wda.json to satisfy the project contract schema",
            ));
        }
    }

    if report.diagnostics.is_empty() {
        let _contract = project_contract(&parsed_sourcemap.value)?;
    }

    Ok(report)
}

/// Typed representation of the page architecture setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageArchitecture {
    Mpa,
    Spa,
}

/// Typed representation of the browser baseline setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserBaseline {
    BaselineWidelyAvailable,
}

/// Typed representation of the accessibility baseline setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityBaseline {
    Wcag22Aa,
}

/// Typed representation of the design system contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignSystem {
    pub name: String,
    pub version: String,
}

/// Typed representation of the validated `wda.json` project contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContract {
    pub project_name: String,
    pub project_version: String,
    pub wda_version: String,
    pub page_architecture: PageArchitecture,
    pub browser_baseline: BrowserBaseline,
    pub accessibility_baseline: AccessibilityBaseline,
    pub design_system: DesignSystem,
}

/// Projects a JSON [`Value`] instance into a typed [`ProjectContract`].
///
/// Runs post-schema validation. If any field type or enum variant mismatches the invariant,
/// an internal invariant fault is returned as a [`ToolFault`] without generating field-level diagnostics.
pub fn project_contract(instance: &Value) -> Result<ProjectContract, ToolFault> {
    let project_name = instance
        .get("projectName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ToolFault::new(
                "Projection fault: missing or non-string 'projectName'",
                None::<std::path::PathBuf>,
            )
        })?
        .to_string();

    let project_version = instance
        .get("projectVersion")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ToolFault::new(
                "Projection fault: missing or non-string 'projectVersion'",
                None::<std::path::PathBuf>,
            )
        })?
        .to_string();

    let wda_version = instance
        .get("wdaVersion")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ToolFault::new(
                "Projection fault: missing or non-string 'wdaVersion'",
                None::<std::path::PathBuf>,
            )
        })?
        .to_string();

    let page_architecture = match instance.get("pageArchitecture").and_then(|v| v.as_str()) {
        Some("mpa") => PageArchitecture::Mpa,
        Some("spa") => PageArchitecture::Spa,
        _ => {
            return Err(ToolFault::new(
                "Projection fault: invalid or non-string 'pageArchitecture'",
                None::<std::path::PathBuf>,
            ));
        }
    };

    let browser_baseline = match instance.get("browserBaseline").and_then(|v| v.as_str()) {
        Some("baseline-widely-available") => BrowserBaseline::BaselineWidelyAvailable,
        _ => {
            return Err(ToolFault::new(
                "Projection fault: invalid or non-string 'browserBaseline'",
                None::<std::path::PathBuf>,
            ));
        }
    };

    let accessibility_baseline =
        match instance.get("accessibilityBaseline").and_then(|v| v.as_str()) {
            Some("wcag-2.2-aa") => AccessibilityBaseline::Wcag22Aa,
            _ => {
                return Err(ToolFault::new(
                    "Projection fault: invalid or non-string 'accessibilityBaseline'",
                    None::<std::path::PathBuf>,
                ));
            }
        };

    let ds_val = instance.get("designSystem").ok_or_else(|| {
        ToolFault::new(
            "Projection fault: missing 'designSystem'",
            None::<std::path::PathBuf>,
        )
    })?;

    let name = ds_val
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ToolFault::new(
                "Projection fault: missing or non-string 'designSystem.name'",
                None::<std::path::PathBuf>,
            )
        })?
        .to_string();

    let version = ds_val
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ToolFault::new(
                "Projection fault: missing or non-string 'designSystem.version'",
                None::<std::path::PathBuf>,
            )
        })?
        .to_string();

    Ok(ProjectContract {
        project_name,
        project_version,
        wda_version,
        page_architecture,
        browser_baseline,
        accessibility_baseline,
        design_system: DesignSystem { name, version },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid_instance() -> Value {
        json!({
            "projectName": "my-project",
            "projectVersion": "1.0.0",
            "wdaVersion": "0.1.0",
            "pageArchitecture": "mpa",
            "browserBaseline": "baseline-widely-available",
            "accessibilityBaseline": "wcag-2.2-aa",
            "designSystem": {
                "name": "default",
                "version": "1.0.0"
            }
        })
    }

    #[test]
    fn test_seven_field_instance_is_valid() {
        let instance = valid_instance();
        assert!(validate_json(&instance).unwrap());

        // spa is also a valid pageArchitecture
        let mut spa_instance = valid_instance();
        spa_instance["pageArchitecture"] = json!("spa");
        assert!(validate_json(&spa_instance).unwrap());
    }

    #[test]
    fn test_excluded_top_level_fields_are_invalid() {
        let excluded_fields = [
            "$schema",
            "metadata",
            "projectDescription",
            "projectFormatVersion",
            "deploymentDirectory",
            "extraField",
        ];

        for field in excluded_fields {
            let mut instance = valid_instance();
            instance[field] = json!("some_value");
            assert!(
                !validate_json(&instance).unwrap(),
                "Expected instance with top-level field '{field}' to be invalid"
            );
        }
    }

    #[test]
    fn test_missing_required_fields_are_invalid() {
        let required_fields = [
            "projectName",
            "projectVersion",
            "wdaVersion",
            "pageArchitecture",
            "browserBaseline",
            "accessibilityBaseline",
            "designSystem",
        ];

        for field in required_fields {
            let mut instance = valid_instance();
            if let Value::Object(ref mut map) = instance {
                map.remove(field);
            }
            assert!(
                !validate_json(&instance).unwrap(),
                "Expected instance missing required field '{field}' to be invalid"
            );
        }
    }

    #[test]
    fn test_unknown_key_nested_inside_design_system_is_invalid() {
        let mut instance = valid_instance();
        instance["designSystem"]["unknownKey"] = json!("value");
        assert!(!validate_json(&instance).unwrap());
    }

    #[test]
    fn test_missing_design_system_required_fields_are_invalid() {
        let mut no_name = valid_instance();
        no_name["designSystem"]
            .as_object_mut()
            .unwrap()
            .remove("name");
        assert!(!validate_json(&no_name).unwrap());

        let mut no_version = valid_instance();
        no_version["designSystem"]
            .as_object_mut()
            .unwrap()
            .remove("version");
        assert!(!validate_json(&no_version).unwrap());
    }

    #[test]
    fn test_invalid_enum_and_string_values() {
        let mut invalid_arch = valid_instance();
        invalid_arch["pageArchitecture"] = json!("invalid");
        assert!(!validate_json(&invalid_arch).unwrap());

        let mut invalid_browser = valid_instance();
        invalid_browser["browserBaseline"] = json!("baseline-2023");
        assert!(!validate_json(&invalid_browser).unwrap());

        let mut invalid_a11y = valid_instance();
        invalid_a11y["accessibilityBaseline"] = json!("wcag-2.1-aa");
        assert!(!validate_json(&invalid_a11y).unwrap());

        let mut empty_name = valid_instance();
        empty_name["projectName"] = json!("");
        assert!(!validate_json(&empty_name).unwrap());
    }

    #[test]
    fn test_compile_schema_success() {
        assert!(compile_schema().is_ok());
    }

    #[test]
    fn test_unknown_field_yields_exact_line_and_column() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_unknown_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let wda_json_path = temp_dir.join("wda.json");

        let json_str = r#"{
  "projectName": "my-project",
  "projectVersion": "1.0.0",
  "wdaVersion": "0.1.0",
  "pageArchitecture": "mpa",
  "browserBaseline": "baseline-widely-available",
  "accessibilityBaseline": "wcag-2.2-aa",
  "designSystem": {
    "name": "default",
    "version": "1.0.0"
  },
  "unknownField": "value"
}"#;
        std::fs::write(&wda_json_path, json_str).unwrap();

        let report = validate_project_contract(&temp_dir).expect("validation report");
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert_eq!(report.diagnostics.len(), 1);
        let d = &report.diagnostics[0];
        assert_eq!(d.code, codes::CONTRACT_UNKNOWN_FIELD);
        let loc = d.location.as_ref().expect("has location");
        assert_eq!(loc.path, std::path::PathBuf::from("wda.json"));
        assert_eq!(loc.line, Some(12), "expected exact line 12 for unknownField");
        assert_eq!(loc.column, Some(3), "expected exact column 3 for unknownField");
    }

    #[test]
    fn test_missing_required_field_yields_schema_violation() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_missing_req_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let wda_json_path = temp_dir.join("wda.json");

        let json_str = r#"{
  "projectVersion": "1.0.0",
  "wdaVersion": "0.1.0",
  "pageArchitecture": "mpa",
  "browserBaseline": "baseline-widely-available",
  "accessibilityBaseline": "wcag-2.2-aa",
  "designSystem": {
    "name": "default",
    "version": "1.0.0"
  }
}"#;
        std::fs::write(&wda_json_path, json_str).unwrap();

        let report = validate_project_contract(&temp_dir).expect("validation report");
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert_eq!(report.diagnostics.len(), 1);
        let d = &report.diagnostics[0];
        assert_eq!(d.code, codes::CONTRACT_SCHEMA_VIOLATION);
    }

    #[test]
    fn test_absent_file_yields_missing_path_only() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_absent_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let report = validate_project_contract(&temp_dir).expect("validation report");
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert_eq!(report.diagnostics.len(), 1);
        let d = &report.diagnostics[0];
        assert_eq!(d.code, codes::CONTRACT_MISSING);
        let loc = d.location.as_ref().expect("has location");
        assert_eq!(loc.path, std::path::PathBuf::from("wda.json"));
        assert_eq!(loc.line, None, "absent file location must be path only");
        assert_eq!(loc.column, None, "absent file location must be path only");
    }

    #[test]
    fn test_non_json_content_yields_malformed_json() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_malformed_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let wda_json_path = temp_dir.join("wda.json");

        std::fs::write(&wda_json_path, "not valid json content {").unwrap();

        let report = validate_project_contract(&temp_dir).expect("validation report");
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert_eq!(report.diagnostics.len(), 1);
        let d = &report.diagnostics[0];
        assert_eq!(d.code, codes::CONTRACT_MALFORMED_JSON);
    }

    #[test]
    fn test_position_base_convention_json_sourcemap() {
        let json_str = "{\n  \"foo\": 123\n}";
        let res = json_sourcemap::parse(json_str, json_sourcemap::Options::default()).unwrap();
        let key_pos = res.pointers.get("/foo").unwrap().key();
        // json_sourcemap returns 0-based positions (line 1 is 0, column 1 is 0).
        // Line 2 -> 1, column 3 (2 leading spaces) -> 2.
        assert_eq!(key_pos.line, 1, "json_sourcemap line is 0-indexed");
        assert_eq!(key_pos.column, 2, "json_sourcemap column is 0-indexed");
    }

    #[test]
    fn test_validate_project_contract_valid_file_returns_clean_report() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_valid_contract_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let wda_json_path = temp_dir.join("wda.json");

        let json_str = r#"{
  "projectName": "my-project",
  "projectVersion": "1.0.0",
  "wdaVersion": "0.1.0",
  "pageArchitecture": "mpa",
  "browserBaseline": "baseline-widely-available",
  "accessibilityBaseline": "wcag-2.2-aa",
  "designSystem": {
    "name": "default",
    "version": "1.0.0"
  }
}"#;
        std::fs::write(&wda_json_path, json_str).unwrap();

        let report = validate_project_contract(&temp_dir).expect("valid wda.json must return Ok report");
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert!(
            report.diagnostics.is_empty(),
            "valid wda.json must yield zero diagnostics"
        );
        assert!(!report.has_error());
    }

    #[test]
    fn test_project_schema_valid_instance_succeeds() {
        let instance = valid_instance();
        let projected = project_contract(&instance).expect("schema-valid instance projects successfully");

        assert_eq!(projected.project_name, "my-project");
        assert_eq!(projected.project_version, "1.0.0");
        assert_eq!(projected.wda_version, "0.1.0");
        assert_eq!(projected.page_architecture, PageArchitecture::Mpa);
        assert_eq!(
            projected.browser_baseline,
            BrowserBaseline::BaselineWidelyAvailable
        );
        assert_eq!(
            projected.accessibility_baseline,
            AccessibilityBaseline::Wcag22Aa
        );
        assert_eq!(projected.design_system.name, "default");
        assert_eq!(projected.design_system.version, "1.0.0");

        let mut spa_instance = valid_instance();
        spa_instance["pageArchitecture"] = json!("spa");
        let spa_projected = project_contract(&spa_instance).expect("spa instance projects successfully");
        assert_eq!(spa_projected.page_architecture, PageArchitecture::Spa);
    }

    #[test]
    fn test_project_injected_mismatch_returns_tool_fault_without_diagnostics() {
        let mut instance = valid_instance();
        instance["pageArchitecture"] = json!("invalid_architecture_variant");

        let res = project_contract(&instance);
        assert!(res.is_err(), "injected mismatch must return error");
        let fault = res.unwrap_err();
        assert!(
            fault.message.contains("Projection fault"),
            "mismatch must surface as ToolFault: {}",
            fault.message
        );

        // Additional mismatch checks (non-string type, missing subfield)
        let mut non_string_name = valid_instance();
        non_string_name["projectName"] = json!(12345);
        let res2 = project_contract(&non_string_name);
        assert!(res2.is_err());
        assert!(res2.unwrap_err().message.contains("Projection fault"));

        let mut invalid_design_system = valid_instance();
        invalid_design_system["designSystem"]["name"] = json!(null);
        let res3 = project_contract(&invalid_design_system);
        assert!(res3.is_err());
        assert!(res3.unwrap_err().message.contains("Projection fault"));
    }

    #[test]
    fn test_wda_json_exceeding_max_depth_yields_malformed_json_without_panic() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_deep_contract_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let wda_json_path = temp_dir.join("wda.json");

        let mut deep_json = String::new();
        for _ in 0..(MAX_JSON_DEPTH + 10) {
            deep_json.push_str("{\"nested\":");
        }
        deep_json.push_str("1");
        for _ in 0..(MAX_JSON_DEPTH + 10) {
            deep_json.push('}');
        }

        std::fs::write(&wda_json_path, deep_json).unwrap();

        let report = validate_project_contract(&temp_dir).expect("validation report");
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert_eq!(report.diagnostics.len(), 1);
        let d = &report.diagnostics[0];
        assert_eq!(d.code, codes::CONTRACT_MALFORMED_JSON);
        assert_eq!(d.severity, Severity::Error);
        let loc = d.location.as_ref().expect("has location");
        assert_eq!(loc.path, std::path::PathBuf::from("wda.json"));
        assert_eq!(loc.line, None, "deep depth guard location must be path only");
        assert_eq!(loc.column, None, "deep depth guard location must be path only");
    }

    #[test]
    fn test_compute_max_json_depth_cases() {
        assert_eq!(compute_max_json_depth("{}"), 1);
        assert_eq!(compute_max_json_depth("{\"a\": {\"b\": [1]}}"), 3);
        assert_eq!(compute_max_json_depth("{\"str\": \"{ [{}]\"}"), 1);
        assert_eq!(compute_max_json_depth("{\"escaped\": \"\\\"{\" }"), 1);
    }
}
