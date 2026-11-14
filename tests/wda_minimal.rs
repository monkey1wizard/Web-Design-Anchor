use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use wda_core::validate_project;

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

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn test_wda_minimal_notice_records_impeccable_snapshot() {
    let notice_path = manifest_dir().join("NOTICE");
    let notice = fs::read_to_string(&notice_path).expect("read root NOTICE");

    assert!(notice.contains("https://github.com/pbakaus/impeccable"));
    assert!(notice.contains("9d715cc4f5564a990ca8345abfdd5df6dc9b41c8"));
}

#[test]
fn test_wda_minimal_embedded_assets_validity() {
    let source_dir = manifest_dir()
        .join("src")
        .join("builtins")
        .join("wda_minimal");

    let tokens_path = source_dir.join("tokens/tokens.json");
    let html_path = source_dir.join("pages/index.html");
    let design_path = source_dir.join("docs/design.md");

    assert!(tokens_path.is_file(), "embedded tokens.json must exist");
    assert!(html_path.is_file(), "embedded index.html must exist");
    assert!(design_path.is_file(), "embedded design.md must exist");

    let tokens_content = fs::read_to_string(&tokens_path).expect("read source tokens.json");
    let parsed: serde_json::Value =
        serde_json::from_str(&tokens_content).expect("source tokens.json must be valid JSON");
    assert!(parsed.is_object(), "source tokens.json must be a JSON object");

    let html_content = fs::read_to_string(&html_path).expect("read source index.html");
    assert!(
        html_content.contains("<!DOCTYPE html>"),
        "source html must contain doctype"
    );
    assert!(
        html_content.contains("<html lang=\"en\">"),
        "source html must contain lang tag"
    );

    let design_content = fs::read_to_string(&design_path).expect("read source design.md");
    assert!(
        design_content.contains("# Design System & Visual Review Criteria"),
        "source design.md must contain title"
    );
}

#[test]
fn test_wda_minimal_fixture_validation_and_byte_identity() {
    let fixture_dir = manifest_dir()
        .join("tests")
        .join("fixtures")
        .join("wda_minimal_starter");
    assert!(
        fixture_dir.is_dir(),
        "fixture directory 'wda_minimal_starter' must exist"
    );

    let report = validate_project(&fixture_dir).expect("validation report");
    assert!(
        project_diagnostics(&report.diagnostics).is_empty(),
        "wda_minimal_starter fixture must produce zero diagnostics, got: {:?}",
        report.diagnostics
    );
    assert!(!report.has_error());

    let source_dir = manifest_dir()
        .join("src")
        .join("builtins")
        .join("wda_minimal");

    let mirrored_files = [
        ("tokens/tokens.json", "tokens/tokens.json"),
        ("pages/index.html", "pages/index.html"),
        ("docs/design.md", "docs/design.md"),
    ];

    for (source_rel, fixture_rel) in mirrored_files {
        let source_path = source_dir.join(source_rel);
        let fixture_path = fixture_dir.join(fixture_rel);

        assert!(
            source_path.is_file(),
            "source asset {:?} must exist",
            source_path
        );
        assert!(
            fixture_path.is_file(),
            "fixture asset {:?} must exist",
            fixture_path
        );

        let source_bytes = fs::read(&source_path).expect("read source asset");
        let fixture_bytes = fs::read(&fixture_path).expect("read fixture asset");

        assert!(
            !source_bytes.is_empty(),
            "source asset {:?} must not be empty",
            source_path
        );
        assert_eq!(
            source_bytes, fixture_bytes,
            "fixture file {:?} must be byte-identical to embedded source asset {:?}",
            fixture_rel, source_rel
        );
    }

    let required_fixture_files = [
        "anchor.json",
        "package.json",
        "README.md",
        "docs/architecture.md",
        "docs/glossary.md",
    ];

    for rel in required_fixture_files {
        let path = fixture_dir.join(rel);
        assert!(path.is_file(), "required fixture file {:?} must exist", rel);
        let metadata = fs::metadata(&path).expect("metadata for fixture file");
        assert!(metadata.len() > 0, "fixture file {:?} must not be empty", rel);
    }
}

#[test]
fn test_wda_minimal_no_placeholder_directories_or_files() {
    let source_dir = manifest_dir()
        .join("src")
        .join("builtins")
        .join("wda_minimal");
    let fixture_dir = manifest_dir()
        .join("tests")
        .join("fixtures")
        .join("wda_minimal_starter");

    let inspect_tree = |root: &Path| {
        let mut checked_dirs = 0;
        let mut checked_files = 0;

        fn visit(current: &Path, checked_dirs: &mut usize, checked_files: &mut usize) {
            let entries: Vec<_> = fs::read_dir(current)
                .expect("read_dir")
                .filter_map(Result::ok)
                .collect();

            assert!(
                !entries.is_empty(),
                "directory {:?} must not be empty (no placeholder directories allowed)",
                current
            );
            *checked_dirs += 1;

            for entry in entries {
                let path = entry.path();
                let name = path.file_name().unwrap().to_string_lossy().to_lowercase();

                assert!(
                    !name.contains("placeholder"),
                    "path {:?} contains 'placeholder'",
                    path
                );
                assert!(!name.contains("todo"), "path {:?} contains 'todo'", path);
                assert!(
                    !name.contains(".gitkeep"),
                    "path {:?} contains '.gitkeep'",
                    path
                );

                if path.is_dir() {
                    visit(&path, checked_dirs, checked_files);
                } else if path.is_file() {
                    let len = fs::metadata(&path).expect("file metadata").len();
                    assert!(len > 0, "file {:?} must not be 0 bytes", path);
                    *checked_files += 1;
                }
            }
        }

        visit(root, &mut checked_dirs, &mut checked_files);
        (checked_dirs, checked_files)
    };

    let (src_dirs, src_files) = inspect_tree(&source_dir);
    assert!(
        src_dirs > 0 && src_files > 0,
        "source tree must contain files and directories"
    );

    let (fix_dirs, fix_files) = inspect_tree(&fixture_dir);
    assert!(
        fix_dirs > 0 && fix_files > 0,
        "fixture tree must contain files and directories"
    );
}

#[test]
fn test_wda_minimal_vendor_token_exclusion_and_semantic_naming() {
    let token_paths = [
        manifest_dir().join("src/builtins/wda_minimal/tokens/tokens.json"),
        manifest_dir().join("tests/fixtures/wda_minimal_starter/tokens/tokens.json"),
    ];

    let forbidden_vendor_ids = [
        "tailwind",
        "spectrum",
        "adobe",
        "material",
        "bootstrap",
        "chakra",
        "shadcn",
    ];

    for path in token_paths {
        let content = fs::read_to_string(&path).expect("read tokens.json");
        let parsed: serde_json::Value =
            serde_json::from_str(&content).expect("valid tokens JSON");

        fn collect_keys(val: &serde_json::Value, current_path: &str, keys: &mut Vec<String>) {
            if let Some(obj) = val.as_object() {
                for (k, v) in obj {
                    let next_path = if current_path.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", current_path, k)
                    };
                    keys.push(next_path.clone());
                    collect_keys(v, &next_path, keys);
                }
            }
        }

        let mut key_paths = Vec::new();
        collect_keys(&parsed, "", &mut key_paths);

        for key_path in key_paths {
            let lower = key_path.to_lowercase();
            for vendor in forbidden_vendor_ids {
                assert!(
                    !lower.contains(vendor),
                    "token key path '{}' in {:?} contains vendor identifier '{}'",
                    key_path,
                    path,
                    vendor
                );
            }
        }
    }
}

#[test]
fn test_wda_minimal_criteria_variation_and_conditional_rendering() {
    let criteria_path = manifest_dir().join("src/builtins/wda_minimal/docs/design.md");
    let source = fs::read_to_string(&criteria_path).expect("read criteria source design.md");

    assert!(
        source.contains("{{#if_design_system \"WDA Minimal\"}}"),
        "criteria source must contain WDA Minimal conditional block"
    );
    assert!(
        source.contains("{{#if_design_system \"Spectrum 2\"}}"),
        "criteria source must contain Spectrum 2 conditional block"
    );

    let render = |design_system: &str, accessibility_baseline: &str| -> String {
        let mut result = String::with_capacity(source.len());
        let mut include_lines = true;

        for line in source.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("{{#if_design_system \"") {
                if let Some(target) = rest.strip_suffix("\"}}") {
                    include_lines = target == design_system;
                    continue;
                }
            }
            if trimmed == "{{/if_design_system}}" {
                include_lines = true;
                continue;
            }

            if include_lines {
                let rendered_line = line
                    .replace("{{DESIGN_SYSTEM_NAME}}", design_system)
                    .replace("{{ACCESSIBILITY_BASELINE}}", accessibility_baseline);
                result.push_str(&rendered_line);
                result.push('\n');
            }
        }

        result
    };

    let wda_render = render("WDA Minimal", "WCAG 2.2 AA");
    let spectrum_render = render("Spectrum 2", "WCAG 2.1 AA");

    assert_ne!(wda_render, spectrum_render, "rendered criteria must differ");

    assert!(wda_render.contains("Design System**: WDA Minimal"));
    assert!(wda_render.contains("Accessibility Baseline**: WCAG 2.2 AA"));
    assert!(wda_render.contains("2D flat layout model"));
    assert!(!wda_render.contains("reduced-elevation surface model"));
    assert!(!wda_render.contains("{{#if_design_system"));

    assert!(spectrum_render.contains("Design System**: Spectrum 2"));
    assert!(spectrum_render.contains("Accessibility Baseline**: WCAG 2.1 AA"));
    assert!(spectrum_render.contains("reduced-elevation surface model"));
    assert!(!spectrum_render.contains("2D flat layout model"));
    assert!(!spectrum_render.contains("{{#if_design_system"));
}

#[test]
fn test_wda_minimal_adapter_boundary_document_completeness() {
    let design_system_path = manifest_dir().join("docs/design-system.md");
    let content = fs::read_to_string(&design_system_path).expect("read design-system.md");

    assert!(content.starts_with("---"));
    assert!(content.contains("status: stable"));
    assert!(content.contains("## Adapter Boundary"));

    let required_facets = [
        "### 1. Token mapping",
        "### 2. Styles",
        "### 3. Fonts",
        "### 4. Icons",
        "### 5. Theme",
        "### 6. Component dependencies",
        "### 7. Dependency integration",
    ];

    for facet in required_facets {
        assert!(
            content.contains(facet),
            "design-system.md must contain facet section '{}'",
            facet
        );
    }

    assert!(content.contains("WDA Minimal"));
    assert!(content.contains("Spectrum"));
}

#[test]
fn test_wda_minimal_spectrum_adapter_document_completeness() {
    let spectrum_adapter_path = manifest_dir().join("docs/design-systems/spectrum-adapter.md");
    let content = fs::read_to_string(&spectrum_adapter_path).expect("read spectrum-adapter.md");

    assert!(content.starts_with("---"));
    assert!(content.contains("title: \"Spectrum Adapter Mapping\""));
    assert!(content.contains("status: stable"));

    let required_mappings = [
        "### 1. Token mapping",
        "### 2. Styles",
        "### 3. Fonts",
        "### 4. Icons",
        "### 5. Theme",
        "### 6. Component dependencies",
        "### 7. Dependency integration",
    ];

    for mapping in required_mappings {
        assert!(
            content.contains(mapping),
            "spectrum adapter document must contain mapping section '{}'",
            mapping
        );
    }

    assert!(
        content.contains("Summary Matrix"),
        "spectrum adapter document must contain summary matrix"
    );

    assert!(
        content.contains("docs/design-system.md"),
        "spectrum adapter document must point to the adapter boundary in docs/design-system.md"
    );

    assert!(
        content.contains("2026-09-07"),
        "spectrum adapter document must state the implementation landing date"
    );
}

fn fixture_dir() -> PathBuf {
    manifest_dir()
        .join("tests")
        .join("fixtures")
        .join("wda_minimal_starter")
}

fn native_minimal_starter_root(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "wda_minimal_starter_{tag}_{}_{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create project root");
    root
}

fn isolated_home_for(root: &Path) -> PathBuf {
    root.with_file_name(format!(
        "{}-home",
        root.file_name().expect("root has a name").to_string_lossy()
    ))
}

/// Runs `wda` with an empty PATH, so no package manager or git is reachable.
/// Init therefore takes the manager-less Minimal path with no network use.
fn run_wda_without_tools(root: &Path, args: &[&str]) -> Output {
    let home = isolated_home_for(root);
    fs::create_dir_all(&home).expect("create isolated home");
    Command::new(env!("CARGO_BIN_EXE_wda"))
        .args(args)
        .current_dir(root)
        .env("PATH", "")
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("GIT_CONFIG_GLOBAL", home.join("nonexistent-gitconfig"))
        .env(
            "GIT_CONFIG_SYSTEM",
            home.join("nonexistent-gitconfig-system"),
        )
        .output()
        .expect("run wda")
}

fn top_level_entries(root: &Path) -> BTreeSet<String> {
    fs::read_dir(root)
        .expect("read project root")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

/// Removes the fields that depend on the run: the creation instant, the WDA
/// version, and the fallback reason, which lists whatever probes failed on this machine.
fn without_run_bound_fields(mut manifest: serde_json::Value) -> serde_json::Value {
    let object = manifest.as_object_mut().expect("anchor.json root object");
    object.remove("projectVersion");
    object.remove("wdaVersion");
    if let Some(design_system) = object
        .get_mut("designSystem")
        .and_then(serde_json::Value::as_object_mut)
    {
        design_system.remove("version");
    }
    if let Some(fallback) = object
        .get_mut("initialization")
        .and_then(|initialization| initialization.get_mut("fallback"))
        .and_then(serde_json::Value::as_object_mut)
    {
        fallback.remove("reason");
    }
    manifest
}

fn cleanup(root: &Path) {
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(isolated_home_for(root));
}

#[test]
fn native_minimal_starter_golden_manifests_page_and_tokens() {
    let root = native_minimal_starter_root("golden");
    let output = run_wda_without_tools(&root, &["init"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "offline Minimal init must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let fixture = fixture_dir();

    let generated_package = fs::read(root.join("package.json")).expect("generated package.json");
    let golden_package = fs::read(fixture.join("package.json")).expect("golden package.json");
    assert_eq!(
        generated_package, golden_package,
        "Minimal package.json must match the golden sample byte for byte"
    );

    let generated_anchor: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("anchor.json")).expect("generated anchor.json"))
            .expect("generated anchor.json is JSON");
    let golden_anchor: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.join("anchor.json")).expect("golden anchor.json"))
            .expect("golden anchor.json is JSON");
    assert!(
        generated_anchor["initialization"]["fallback"]["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()),
        "offline Minimal must record a non-empty fallback reason"
    );
    assert_eq!(
        without_run_bound_fields(generated_anchor),
        without_run_bound_fields(golden_anchor),
        "Minimal anchor.json must match the golden sample apart from run-bound fields"
    );

    let golden_page = fs::read_to_string(fixture.join("pages/index.html"))
        .expect("golden index.html")
        .replace(
            "<title>WDA Minimal Starter</title>",
            "<title>Web Design Anchor Project</title>",
        );
    let generated_page =
        fs::read_to_string(root.join("pages/index.html")).expect("generated index.html");
    assert_eq!(
        generated_page, golden_page,
        "Minimal starter page must equal the golden page with the project title"
    );

    assert_eq!(
        fs::read(root.join("tokens/tokens.json")).expect("generated tokens.json"),
        fs::read(fixture.join("tokens/tokens.json")).expect("golden tokens.json"),
        "Minimal tokens must match the golden sample byte for byte"
    );

    let report = validate_project(&root).expect("validation report");
    assert!(
        project_diagnostics(&report.diagnostics).is_empty(),
        "generated Minimal project must validate with zero diagnostics, got: {:?}",
        report.diagnostics
    );

    cleanup(&root);
}

#[test]
fn native_minimal_starter_offline_check_and_build_add_no_directories() {
    let root = native_minimal_starter_root("build");
    let output = run_wda_without_tools(&root, &["init"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "offline Minimal init must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let after_init = top_level_entries(&root);
    assert!(
        !after_init.contains("node_modules") && !after_init.contains("dist"),
        "init must not create node_modules or dist, got: {after_init:?}"
    );

    let check = run_wda_without_tools(&root, &["check"]);
    assert_eq!(
        check.status.code(),
        Some(0),
        "check on Minimal must exit 0, stderr: {}",
        String::from_utf8_lossy(&check.stderr)
    );
    assert_eq!(
        top_level_entries(&root),
        after_init,
        "check must not add or remove top-level entries"
    );

    let build = run_wda_without_tools(&root, &["build"]);
    assert_eq!(
        build.status.code(),
        Some(0),
        "build on Minimal must exit 0, stderr: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let mut expected = after_init;
    expected.insert("dist".to_string());
    assert_eq!(
        top_level_entries(&root),
        expected,
        "build must add only dist/"
    );
    assert!(
        root.join("dist").join("pages").join("index.html").is_file()
            || root.join("dist").join("index.html").is_file(),
        "build must write the starter page into dist/"
    );
    assert!(
        !root.join("node_modules").exists() && !root.join("package-lock.json").exists(),
        "check and build must never install packages"
    );

    cleanup(&root);
}

/// Replaces the `updated:` front matter line with a placeholder, so the run date
/// does not decide the comparison. The exact date is covered by the fixed-time unit probe.
fn with_document_date_placeholder(text: &str) -> String {
    text.replace("\r\n", "\n")
        .split_inclusive('\n')
        .map(|line| {
            if line.starts_with("updated: ") {
                "updated: <DATE>\n".to_string()
            } else {
                line.to_string()
            }
        })
        .collect()
}

#[test]
fn native_generated_documents_integration_minimal_matches_golden_and_checks() {
    let root = native_minimal_starter_root("documents");
    let output = run_wda_without_tools(&root, &["init"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "offline Minimal init must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let fixture = fixture_dir();
    for rel in ["README.md", "docs/architecture.md", "docs/glossary.md"] {
        let generated =
            fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("generated {rel}: {e}"));
        let golden =
            fs::read_to_string(fixture.join(rel)).unwrap_or_else(|e| panic!("golden {rel}: {e}"));
        assert_eq!(
            with_document_date_placeholder(&generated),
            with_document_date_placeholder(&golden),
            "generated {rel} must equal the golden sample apart from the run date"
        );
    }

    let architecture =
        fs::read_to_string(root.join("docs/architecture.md")).expect("generated architecture.md");
    let updated = architecture
        .lines()
        .find_map(|line| line.strip_prefix("updated: "))
        .expect("architecture.md must carry an updated date");
    assert!(
        updated.len() == 10
            && updated.bytes().enumerate().all(|(i, b)| match i {
                4 | 7 => b == b'-',
                _ => b.is_ascii_digit(),
            }),
        "updated date must be YYYY-MM-DD, got {updated:?}"
    );

    let design = fs::read_to_string(root.join("docs/design.md")).expect("generated design.md");
    assert!(
        design.starts_with("---\n") && design.contains("Design System**: WDA Minimal"),
        "design.md must be generated with front matter and the actual Minimal result"
    );

    let anchor: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("anchor.json")).expect("anchor.json"))
            .expect("anchor.json is JSON");
    assert_eq!(
        anchor["designSystem"]["name"].as_str(),
        Some("WDA Minimal"),
        "anchor.json must record the actual Minimal result"
    );
    let glossary = fs::read_to_string(root.join("docs/glossary.md")).expect("glossary.md");
    assert!(
        glossary.contains("`WDA Minimal`"),
        "glossary.md must name the actual Design System"
    );

    let check = run_wda_without_tools(&root, &["check"]);
    assert_eq!(
        check.status.code(),
        Some(0),
        "check on generated documents must exit 0, stderr: {}",
        String::from_utf8_lossy(&check.stderr)
    );
    let report = validate_project(&root).expect("validation report");
    assert!(
        project_diagnostics(&report.diagnostics).is_empty(),
        "generated documents must validate with zero diagnostics, got: {:?}",
        report.diagnostics
    );

    cleanup(&root);
}
