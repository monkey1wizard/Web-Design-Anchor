use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use wda_core::codes;
use wda_core::validate_project;

mod project_fixture;

#[test]
fn author_token_profile_validation_reports_source() {
    let dir = tempfile::tempdir().unwrap();
    project_fixture::write_manifests(dir.path());
    std::fs::create_dir(dir.path().join("tokens")).unwrap();
    let tokens = dir.path().join("tokens/tokens.json");
    for value in ["null", "true", "[]", "{}", "\"\"", "\"red; color: blue\""] {
        std::fs::write(
            &tokens,
            format!("{{\n  \"sample\": {{\"$type\": \"color\", \"$value\": {value}}}\n}}"),
        )
        .unwrap();
        let report = validate_project(dir.path()).unwrap();
        let diagnostic = report
            .diagnostics
            .iter()
            .find(|d| d.code == codes::TOKENS_UNSUPPORTED)
            .unwrap();
        let location = diagnostic.location.as_ref().unwrap();
        assert_eq!(location.path, Path::new("tokens/tokens.json"));
        assert_eq!(location.line, Some(2));
    }
}

#[test]
fn native_manifest_integration_selection_and_diagnostics() {
    use project_fixture::write_manifests;
    let dir = std::env::temp_dir().join(format!("wda_native_manifest_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    write_manifests(&dir);
    let before = get_dir_state(&dir);
    let report = validate_project(&dir).unwrap();
    assert!(!report
        .diagnostics
        .iter()
        .any(|d| d.location.as_ref().is_some_and(
            |l| l.path == Path::new("anchor.json") || l.path == Path::new("package.json")
        )));
    assert_eq!(before, get_dir_state(&dir));

    std::fs::write(
        dir.join("package.json"),
        "{\"dependencies\":{\"lit\":\"^3\"},\"packageManager\":\"npm@10.2.3\"}",
    )
    .unwrap();
    let report = validate_project(&dir).unwrap();
    assert!(!report.diagnostics.iter().any(|d| d
        .location
        .as_ref()
        .is_some_and(|l| l.path == Path::new("package.json"))));

    std::fs::remove_file(dir.join("package.json")).unwrap();
    let report = validate_project(&dir).unwrap();
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.code == codes::CONTRACT_MISSING
            && d.location
                .as_ref()
                .is_some_and(|l| l.path == Path::new("package.json"))));
    for invalid in [
        "[]",
        "{\"dependencies\": []}",
        "{\"packageManager\": \"npm@latest\"}",
        "{\n broken",
    ] {
        std::fs::write(dir.join("package.json"), invalid).unwrap();
        let report = validate_project(&dir).unwrap();
        assert!(report.diagnostics.iter().any(|d| d
            .location
            .as_ref()
            .is_some_and(|l| l.path == Path::new("package.json"))));
    }

    std::fs::write(
        dir.join("package.json"),
        "{\n  \"dependencies\": {\"x\": 1}\n}",
    )
    .unwrap();
    let report = validate_project(&dir).unwrap();
    assert!(report.diagnostics.iter().any(|d| d
        .location
        .as_ref()
        .is_some_and(|l| l.path == Path::new("package.json") && l.line.is_some())));
    std::fs::write(dir.join("anchor.json"), "{}").unwrap();
    let report = validate_project(&dir).unwrap();
    assert!(report.diagnostics.iter().any(|d| d
        .location
        .as_ref()
        .is_some_and(|l| l.path == Path::new("anchor.json"))));
    std::fs::remove_file(dir.join("anchor.json")).unwrap();
    std::fs::remove_file(dir.join("package.json")).unwrap();
    let report = validate_project(&dir).unwrap();
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.code == codes::CONTRACT_MISSING
            && d.location
                .as_ref()
                .is_some_and(|l| l.path == Path::new("anchor.json"))));
    std::fs::remove_dir_all(dir).unwrap();
}

/// The strict cutover accepts only `anchor.json` and `package.json`. A legacy
/// `wda.json` never satisfies the contract, never fills a missing file, and is
/// left byte-for-byte unchanged.
#[test]
fn native_strict_manifest_cutover_legacy_and_mixed_inputs_are_rejected_unchanged() {
    let legacy = include_str!("fixtures/clean_project/anchor.json");
    let cases: [(&str, bool, bool); 3] = [
        ("legacy_only", false, false),
        ("anchor_only", true, false),
        ("package_only", false, true),
    ];
    for (name, has_anchor, has_package) in cases {
        let dir =
            std::env::temp_dir().join(format!("wda_strict_cutover_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("wda.json"), legacy).unwrap();
        if has_anchor {
            std::fs::write(dir.join("anchor.json"), legacy).unwrap();
        }
        if has_package {
            std::fs::write(dir.join("package.json"), "{\"dependencies\":{}}").unwrap();
        }

        let before = get_dir_state(&dir);
        let report = validate_project(&dir).unwrap();
        assert_eq!(
            before,
            get_dir_state(&dir),
            "{name}: inputs must stay unchanged"
        );
        assert!(report.has_error(), "{name}: must report a named Error");

        let missing: Vec<PathBuf> = report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::CONTRACT_MISSING)
            .filter_map(|d| d.location.as_ref().map(|l| l.path.clone()))
            .collect();
        if !has_anchor {
            assert!(
                missing.contains(&PathBuf::from("anchor.json")),
                "{name}: {missing:?}"
            );
        }
        if !has_package {
            assert!(
                missing.contains(&PathBuf::from("package.json")),
                "{name}: {missing:?}"
            );
        }
        assert!(
            !report.diagnostics.iter().any(|d| d
                .location
                .as_ref()
                .is_some_and(|l| l.path == Path::new("wda.json"))),
            "{name}: wda.json must never be read or reported as a contract file"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Copies a checked-in fixture into a temporary project and gives it the strict
/// two-file contract, so existing fixture tests exercise the new format only.
fn new_format_copy(fixture: &str) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    copy_tree(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(fixture),
        project.path(),
    );
    std::fs::write(project.path().join("package.json"), "{\"dependencies\":{}}").unwrap();
    project
}

#[test]
fn native_strict_manifest_cutover_new_format_fixture_has_no_contract_diagnostics() {
    let project = new_format_copy("clean_project");
    let before = get_dir_state(project.path());
    let report = validate_project(project.path()).expect("validation report");
    assert_eq!(
        before,
        get_dir_state(project.path()),
        "validation must write nothing"
    );
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|d| d.location.as_ref().is_some_and(
                |l| l.path == Path::new("anchor.json") || l.path == Path::new("package.json")
            )),
        "new-format clean project must produce no contract diagnostics: {:?}",
        report.diagnostics
    );
}

fn project_diagnostics(
    diagnostics: &[wda_core::diagnostics::Diagnostic],
) -> Vec<&wda_core::diagnostics::Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| {
            !(diagnostic.severity == wda_core::diagnostics::Severity::Warning
                && (diagnostic.code.starts_with("wda.toolchain.")
                    || diagnostic.code == "wda.tool.fault"))
        })
        .collect()
}

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
        5,
        "expected 5 diagnostics: docs, HTML, tokens, and the two missing strict contract files"
    );

    // The declared total order places the missing anchor.json first, then the
    // documents family, the missing package.json, the HTML family and the tokens family.
    let d0 = &report.diagnostics[0];
    let d1 = &report.diagnostics[1];
    let d2 = &report.diagnostics[2];
    let d3 = &report.diagnostics[3];
    let d4 = &report.diagnostics[4];

    assert_eq!(
        d0.code,
        codes::CONTRACT_MISSING,
        "first diagnostic must be the missing anchor.json"
    );
    assert_eq!(
        d0.location.as_ref().unwrap().path,
        PathBuf::from("anchor.json")
    );
    assert_eq!(
        d1.code,
        codes::DOCS_REQUIRED_DOCUMENT_MISSING,
        "second diagnostic must be from documents family"
    );
    assert_eq!(
        d1.location.as_ref().unwrap().path,
        PathBuf::from("docs/glossary.md")
    );
    assert_eq!(
        d2.code,
        codes::CONTRACT_MISSING,
        "third diagnostic must be the missing package.json"
    );
    assert_eq!(
        d2.location.as_ref().unwrap().path,
        PathBuf::from("package.json")
    );
    assert_eq!(
        d3.code,
        codes::HTML_LANG_MISSING,
        "fourth diagnostic must be from HTML family"
    );
    assert_eq!(
        d3.location.as_ref().unwrap().path,
        PathBuf::from("pages/index.html")
    );
    assert_eq!(
        d4.code,
        codes::TOKENS_MALFORMED_JSON,
        "fifth diagnostic must be from tokens family"
    );
    assert_eq!(
        d4.location.as_ref().unwrap().path,
        PathBuf::from("tokens/tokens.json")
    );

    assert!(
        d0 <= d1 && d1 <= d2 && d2 <= d3 && d3 <= d4,
        "diagnostics must be returned in declared total order"
    );
}

#[test]
fn test_validate_project_clean_project_fixture() {
    let project = new_format_copy("clean_project");
    let fixture_dir = project.path().to_path_buf();

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
        project_diagnostics(&report.diagnostics).is_empty(),
        "clean project fixture must produce zero diagnostics, got: {:?}",
        report.diagnostics
    );
    assert!(!report.has_error());
}

#[test]
fn test_validate_project_conformant_fixture() {
    let project = new_format_copy("conformant");
    let fixture_dir = project.path().to_path_buf();

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
        project_diagnostics(&report.diagnostics).is_empty(),
        "conformant fixture must produce zero diagnostics, got: {:?}",
        report.diagnostics
    );
    assert!(!report.has_error());
}

#[test]
fn test_legacy_naming_document_does_not_satisfy_glossary_requirement() {
    let temp_dir = std::env::temp_dir().join(format!(
        "wda_test_legacy_naming_document_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("docs")).unwrap();
    std::fs::write(
        temp_dir.join("docs/naming.md"),
        "---\ntitle: Naming Conventions\nstatus: active\nupdated: 2026-10-14\n---\n",
    )
    .unwrap();

    let state_before = get_dir_state(&temp_dir);
    let report = validate_project(&temp_dir).expect("validation report");
    let state_after = get_dir_state(&temp_dir);

    assert_eq!(
        state_before, state_after,
        "validation must not modify the project tree"
    );
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == codes::DOCS_REQUIRED_DOCUMENT_MISSING
            && diagnostic
                .location
                .as_ref()
                .map(|location| location.path.as_path())
                == Some(Path::new("docs/glossary.md"))
    }));
    assert!(temp_dir.join("docs/naming.md").is_file());
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_validate_project_absent_or_invalid_anchor_json_does_not_early_return() {
    let temp_dir =
        std::env::temp_dir().join(format!("wda_test_no_early_return_{}", std::process::id()));
    let _ = std::fs::create_dir_all(temp_dir.join("pages"));
    let _ = std::fs::create_dir_all(temp_dir.join("tokens"));

    // anchor.json is malformed (invalid JSON)
    std::fs::write(temp_dir.join("anchor.json"), "{ invalid anchor.json }").unwrap();

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
        "report must contain documents diagnostic despite malformed anchor.json"
    );
    assert!(
        has_html_diag,
        "report must contain HTML diagnostic despite malformed anchor.json"
    );
    assert!(
        has_tokens_diag,
        "report must contain tokens diagnostic despite malformed anchor.json"
    );
}

#[test]
fn test_validate_project_invalid_root_returns_tool_fault() {
    let non_existent_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("non_existent_directory_12345");

    let err = validate_project(&non_existent_dir)
        .expect_err("non-existent directory must fail with ToolFault");
    assert!(
        err.message
            .contains("Project root directory is invalid or unreadable"),
        "ToolFault message must state root directory is invalid or unreadable, got: {}",
        err.message
    );
    assert_eq!(err.path, Some(non_existent_dir));
}

fn copy_tree(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap().flatten() {
        let target = destination.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn preflight_stderr(project: &Path, cache: &Path) -> (Option<i32>, String) {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_wda"))
        .arg("check")
        .current_dir(project)
        .env("PATH", "")
        .env("LOCALAPPDATA", cache)
        .env("XDG_CACHE_HOME", cache)
        .env("HOME", cache)
        .output()
        .unwrap();
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn native_check_preflight_integration_missing_manager_alone_does_not_fail_an_installed_project() {
    let project = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    copy_tree(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("clean_project"),
        project.path(),
    );
    std::fs::write(
        project.path().join("package.json"),
        "{\"dependencies\":{\"lit\":\"3.3.0\"},\"packageManager\":\"bun@1.3.0\"}",
    )
    .unwrap();

    let before = get_dir_state(project.path());
    let (code, stderr) = preflight_stderr(project.path(), cache.path());
    assert_eq!(code, Some(0), "{stderr}");
    assert!(
        stderr.contains(codes::BUILD_DEPENDENCY_UNLOCKED),
        "{stderr}"
    );
    assert!(
        stderr.contains("bun"),
        "the hint names the manager from package.json: {stderr}"
    );
    assert!(!stderr.contains("wda.toolchain.deno-missing"), "{stderr}");
    assert_eq!(before, get_dir_state(project.path()), "check is read-only");

    std::fs::create_dir_all(project.path().join("node_modules/lit")).unwrap();
    let before = get_dir_state(project.path());
    let (code, stderr) = preflight_stderr(project.path(), cache.path());
    assert_eq!(code, Some(0), "{stderr}");
    assert!(
        !stderr.contains(codes::BUILD_DEPENDENCY_UNLOCKED),
        "{stderr}"
    );
    assert!(
        !stderr.contains(codes::BUILD_TOOL_UNAVAILABLE),
        "pure HTML needs no cache: {stderr}"
    );
    assert_eq!(before, get_dir_state(project.path()));
    assert!(std::fs::read_dir(cache.path()).unwrap().next().is_none());
}

#[test]
fn native_strict_manifest_cutover_legacy_project_is_rejected_by_check_unchanged() {
    let project = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    copy_tree(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("clean_project"),
        project.path(),
    );
    std::fs::rename(
        project.path().join("anchor.json"),
        project.path().join("wda.json"),
    )
    .unwrap();
    std::fs::write(
        project.path().join("package.json"),
        "{\"dependencies\":{\"lit\":\"3.3.0\"}}",
    )
    .unwrap();
    let before = get_dir_state(project.path());
    let (code, stderr) = preflight_stderr(project.path(), cache.path());
    assert_ne!(
        code,
        Some(0),
        "a wda.json-only project must not check clean: {stderr}"
    );
    assert!(
        stderr.contains("anchor.json"),
        "the error names the missing file: {stderr}"
    );
    assert!(
        !stderr.contains(codes::BUILD_DEPENDENCY_UNLOCKED),
        "{stderr}"
    );
    assert_eq!(
        before,
        get_dir_state(project.path()),
        "check must not rewrite inputs"
    );
}
