use std::fs;
use std::path::{Path, PathBuf};
use wda_core::validate_project;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
        report.diagnostics.is_empty(),
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
        "wda.json",
        "README.md",
        "docs/architecture.md",
        "docs/naming.md",
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
    let boundary_path = manifest_dir().join("docs/design-systems/adapter-boundary.md");
    let content = fs::read_to_string(&boundary_path).expect("read adapter-boundary.md");

    assert!(content.starts_with("---"));
    assert!(content.contains("title: \"Design System Adapter Boundary\""));
    assert!(content.contains("status: stable"));

    let required_facets = [
        "1. Token Mapping",
        "2. Styles",
        "3. Fonts",
        "4. Icons",
        "5. Theme",
        "6. Component Dependencies",
        "7. Dependency Integration",
    ];

    for facet in required_facets {
        assert!(
            content.contains(facet),
            "adapter boundary document must contain facet section '{}'",
            facet
        );
    }

    assert!(content.contains("**WDA Minimal**"));
    assert!(content.contains("**Spectrum**"));
}

#[test]
fn test_wda_minimal_spectrum_paper_adapter_document_completeness() {
    let paper_adapter_path = manifest_dir().join("docs/design-systems/spectrum-paper-adapter.md");
    let content = fs::read_to_string(&paper_adapter_path).expect("read spectrum-paper-adapter.md");

    assert!(content.starts_with("---"));
    assert!(content.contains("title: \"Paper Spectrum Adapter Mapping\""));
    assert!(content.contains("status: stable"));

    let required_mappings = [
        "1. Spectrum Tokens",
        "2. Spectrum Components",
        "3. Spectrum Theme",
        "4. Spectrum Component Dependencies",
        "5. Spectrum NPM Dependency Integration",
    ];

    for mapping in required_mappings {
        assert!(
            content.contains(mapping),
            "spectrum paper adapter document must contain mapping section '{}'",
            mapping
        );
    }

    assert!(
        content.contains("Spectrum Adapter Mapping Matrix"),
        "spectrum paper adapter document must contain summary matrix"
    );
}
