use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use wda_core::codes;
use wda_core::validate_project;

fn get_dir_state(dir: &Path) -> BTreeMap<PathBuf, u64> {
    let mut state = BTreeMap::new();
    fn visit(base: &Path, current: &Path, state: &mut BTreeMap<PathBuf, u64>) {
        if let Ok(entries) = std::fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                let rel_path = path.strip_prefix(base).unwrap().to_path_buf();
                if path.is_dir() {
                    state.insert(rel_path.clone(), 0);
                    visit(base, &path, state);
                } else if path.is_file() {
                    let contents = std::fs::read(&path).unwrap_or_default();
                    let mut hasher = DefaultHasher::new();
                    contents.hash(&mut hasher);
                    state.insert(rel_path, hasher.finish());
                }
            }
        }
    }
    visit(dir, dir, &mut state);
    state
}

#[test]
fn test_validate_project_integration_probe_all_rule_families() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("probe_all_families");

    assert!(
        fixture_dir.is_dir(),
        "fixture directory 'probe_all_families' must exist"
    );

    let state_before = get_dir_state(&fixture_dir);

    let report = validate_project(&fixture_dir).expect("validation report");

    let state_after = get_dir_state(&fixture_dir);

    assert_eq!(
        state_before, state_after,
        "validate_project must write nothing to the fixture directory"
    );

    assert_eq!(
        report.diagnostics.len(),
        4,
        "expected exactly 4 diagnostics (one from each rule family)"
    );

    // Verify diagnostic codes from all four rule families
    let d0 = &report.diagnostics[0];
    let d1 = &report.diagnostics[1];
    let d2 = &report.diagnostics[2];
    let d3 = &report.diagnostics[3];

    assert_eq!(
        d0.code,
        codes::DOCS_REQUIRED_DOCUMENT_MISSING,
        "first diagnostic must be from documents family"
    );
    assert_eq!(
        d0.location.as_ref().unwrap().path,
        PathBuf::from("docs/naming.md")
    );

    assert_eq!(
        d1.code,
        codes::HTML_LANG_MISSING,
        "second diagnostic must be from HTML family"
    );
    assert_eq!(
        d1.location.as_ref().unwrap().path,
        PathBuf::from("pages/index.html")
    );

    assert_eq!(
        d2.code,
        codes::TOKENS_MALFORMED_JSON,
        "third diagnostic must be from tokens family"
    );
    assert_eq!(
        d2.location.as_ref().unwrap().path,
        PathBuf::from("tokens/tokens.json")
    );

    assert_eq!(
        d3.code,
        codes::CONTRACT_MISSING,
        "fourth diagnostic must be from contract family"
    );
    assert_eq!(
        d3.location.as_ref().unwrap().path,
        PathBuf::from("wda.json")
    );

    // Verify total ordering invariant
    assert!(
        d0 <= d1 && d1 <= d2 && d2 <= d3,
        "diagnostics must be returned in declared total order"
    );
}

#[test]
fn test_validate_project_clean_project_fixture() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("clean_project");

    assert!(
        fixture_dir.is_dir(),
        "fixture directory 'clean_project' must exist"
    );

    let state_before = get_dir_state(&fixture_dir);

    let report = validate_project(&fixture_dir).expect("validation report");

    let state_after = get_dir_state(&fixture_dir);

    assert_eq!(
        state_before, state_after,
        "validate_project must write nothing to clean project fixture"
    );

    assert!(
        report.diagnostics.is_empty(),
        "clean project fixture must produce zero diagnostics, got: {:?}",
        report.diagnostics
    );
    assert!(!report.has_error());
}

#[test]
fn test_validate_project_conformant_fixture() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("conformant");

    assert!(
        fixture_dir.is_dir(),
        "fixture directory 'conformant' must exist"
    );

    let state_before = get_dir_state(&fixture_dir);

    let report = validate_project(&fixture_dir).expect("validation report");

    let state_after = get_dir_state(&fixture_dir);

    assert_eq!(
        state_before, state_after,
        "validate_project must write nothing to conformant fixture"
    );

    assert!(
        report.diagnostics.is_empty(),
        "conformant fixture must produce zero diagnostics, got: {:?}",
        report.diagnostics
    );
    assert!(!report.has_error());
}

#[test]
fn test_validate_project_absent_or_invalid_wda_json_does_not_early_return() {
    let temp_dir = std::env::temp_dir().join(format!("wda_test_no_early_return_{}", std::process::id()));
    let _ = std::fs::create_dir_all(temp_dir.join("pages"));
    let _ = std::fs::create_dir_all(temp_dir.join("tokens"));

    // wda.json is malformed (invalid JSON)
    std::fs::write(temp_dir.join("wda.json"), "{ invalid wda.json }").unwrap();

    // pages/index.html is missing lang attribute
    std::fs::write(
        temp_dir.join("pages/index.html"),
        "<!DOCTYPE html>\n<html><body></body></html>",
    )
    .unwrap();

    // tokens/tokens.json has leaf token missing value
    std::fs::write(
        temp_dir.join("tokens/tokens.json"),
        r#"{"color": {"primary": {"$type": "color"}}}"#,
    )
    .unwrap();

    let report = validate_project(&temp_dir).expect("validation report");
    let _ = std::fs::remove_dir_all(&temp_dir);

    let has_contract_diag = report
        .diagnostics
        .iter()
        .any(|d| d.code == codes::CONTRACT_MALFORMED_JSON);
    let has_docs_diag = report
        .diagnostics
        .iter()
        .any(|d| d.code == codes::DOCS_REQUIRED_DOCUMENT_MISSING);
    let has_html_diag = report
        .diagnostics
        .iter()
        .any(|d| d.code == codes::HTML_LANG_MISSING);
    let has_tokens_diag = report
        .diagnostics
        .iter()
        .any(|d| d.code == codes::TOKENS_VALUE_MISSING);

    assert!(
        has_contract_diag,
        "report must contain malformed contract diagnostic"
    );
    assert!(
        has_docs_diag,
        "report must contain documents diagnostic despite malformed wda.json"
    );
    assert!(
        has_html_diag,
        "report must contain HTML diagnostic despite malformed wda.json"
    );
    assert!(
        has_tokens_diag,
        "report must contain tokens diagnostic despite malformed wda.json"
    );
}

#[test]
fn test_validate_project_invalid_root_returns_tool_fault() {
    let non_existent_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("non_existent_directory_12345");

    let err = validate_project(&non_existent_dir).expect_err("non-existent directory must fail with ToolFault");
    assert!(
        err.message.contains("Project root directory is invalid or unreadable"),
        "ToolFault message must state root directory is invalid or unreadable, got: {}",
        err.message
    );
    assert_eq!(err.path, Some(non_existent_dir));
}
