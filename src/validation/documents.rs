use std::fs;
use std::path::{Path, PathBuf};

use crate::codes;
use crate::diagnostics::{Diagnostic, Location, Severity, ToolFault, ValidationReport};

/// List of four required documents for a WDA project.
pub const REQUIRED_DOCUMENTS: &[&str] = &[
    "README.md",
    "docs/architecture.md",
    "docs/design.md",
    "docs/naming.md",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DocumentClass {
    Exempt,
    Required,
    OptIn,
}

fn classify_document(rel_path: &str) -> DocumentClass {
    if rel_path == "README.md" {
        DocumentClass::Exempt
    } else if matches!(
        rel_path,
        "docs/architecture.md" | "docs/design.md" | "docs/naming.md"
    ) {
        DocumentClass::Required
    } else if rel_path.starts_with("docs/") && rel_path.ends_with(".md") {
        DocumentClass::OptIn
    } else {
        DocumentClass::Exempt
    }
}

enum FrontMatterBlock<'a> {
    Missing,
    Unterminated,
    Closed {
        yaml_slice: &'a str,
        line_offset: usize,
    },
}

fn find_line_start_byte_offset(content: &str, line_idx: usize) -> usize {
    if line_idx == 0 {
        return 0;
    }
    let mut current_line = 0;
    for (i, b) in content.bytes().enumerate() {
        if b == b'\n' {
            current_line += 1;
            if current_line == line_idx {
                return i + 1;
            }
        }
    }
    content.len()
}

fn find_front_matter_block(content: &str) -> FrontMatterBlock<'_> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return FrontMatterBlock::Missing;
    }

    let first_line = lines[0].trim_end();
    if first_line != "---" {
        return FrontMatterBlock::Missing;
    }

    let mut close_idx = None;
    for (idx, &line) in lines.iter().enumerate().skip(1) {
        if line.trim_end() == "---" {
            close_idx = Some(idx);
            break;
        }
    }

    let close_idx = match close_idx {
        Some(idx) => idx,
        None => return FrontMatterBlock::Unterminated,
    };

    let slice_start = find_line_start_byte_offset(content, 1);
    let slice_end = find_line_start_byte_offset(content, close_idx);
    let yaml_slice = &content[slice_start..slice_end];

    FrontMatterBlock::Closed {
        yaml_slice,
        line_offset: 1,
    }
}

fn is_valid_yyyy_mm_dd(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let year: u32 = match s[0..4].parse() {
        Ok(y) => y,
        Err(_) => return false,
    };
    let month: u32 = match s[5..7].parse() {
        Ok(m) => m,
        Err(_) => return false,
    };
    let day: u32 = match s[8..10].parse() {
        Ok(d) => d,
        Err(_) => return false,
    };
    if !(1..=9999).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return false;
    }
    let max_days = match month {
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    day <= max_days
}

fn is_root_escaping(path_str: &str) -> bool {
    let bytes = path_str.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    if bytes[0] == b'/' || bytes[0] == b'\\' {
        return true;
    }

    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return true;
    }

    if bytes.len() >= 2
        && ((bytes[0] == b'\\' && bytes[1] == b'\\') || (bytes[0] == b'/' && bytes[1] == b'/'))
    {
        return true;
    }

    if Path::new(path_str).is_absolute() {
        return true;
    }

    let normalized = path_str.replace('\\', "/");
    let mut depth: i32 = 0;
    for component in Path::new(&normalized).components() {
        match component {
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                return true;
            }
            std::path::Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return true;
                }
            }
            std::path::Component::Normal(_) => {
                depth += 1;
            }
            std::path::Component::CurDir => {}
        }
    }

    false
}


fn collect_md_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_md_files(&path, files);
            } else if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.eq_ignore_ascii_case("md") {
                        files.push(path);
                    }
                }
            }
        }
    }
}

/// Validates the presence of required project documents and their front matter.
///
/// Returns a [`ValidationReport`] containing diagnostics.
pub fn validate_documents(root: &Path) -> Result<ValidationReport, ToolFault> {
    let mut report = ValidationReport::new();

    // 1. Validate presence of required documents
    for &rel_path in REQUIRED_DOCUMENTS {
        let doc_path = root.join(rel_path);
        if !doc_path.is_file() {
            report.add(Diagnostic::new(
                codes::DOCS_REQUIRED_DOCUMENT_MISSING,
                Severity::Error,
                Some(Location::path_only(rel_path)),
                format!("Required document '{rel_path}' is missing"),
                format!("Create required document '{rel_path}'"),
            ));
        }
    }

    // 2. Collect all Markdown documents in `docs/`
    let docs_dir = root.join("docs");
    let mut md_files = Vec::new();
    if docs_dir.is_dir() {
        collect_md_files(&docs_dir, &mut md_files);
    }
    md_files.sort();

    for full_path in md_files {
        let rel_path = match full_path.strip_prefix(root) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let rel_path_str = rel_path.to_string_lossy().replace('\\', "/");
        let class = classify_document(&rel_path_str);
        if class == DocumentClass::Exempt {
            continue;
        }

        let content = match fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        match find_front_matter_block(&content) {
            FrontMatterBlock::Missing => {
                if class == DocumentClass::Required {
                    report.add(Diagnostic::new(
                        codes::DOCS_FRONT_MATTER_MISSING,
                        Severity::Error,
                        Some(Location::path_only(&rel_path_str)),
                        format!("Required document '{rel_path_str}' is missing front matter"),
                        format!("Add front matter block to '{rel_path_str}'"),
                    ));
                }
            }
            FrontMatterBlock::Unterminated => {
                if class == DocumentClass::Required {
                    report.add(Diagnostic::new(
                        codes::DOCS_FRONT_MATTER_MALFORMED,
                        Severity::Error,
                        Some(Location::new(&rel_path_str, Some(1), Some(1))),
                        format!("Required document '{rel_path_str}' front matter block is unterminated"),
                        format!("Close front matter block in '{rel_path_str}' with '---'"),
                    ));
                }
            }
            FrontMatterBlock::Closed {
                yaml_slice,
                line_offset,
            } => {
                let severity = if class == DocumentClass::Required {
                    Severity::Error
                } else {
                    Severity::Warning
                };

                match marked_yaml::parse_yaml(0, yaml_slice) {
                    Err(err) => {
                        let marker = match &err {
                            marked_yaml::LoadError::TopLevelMustBeMapping(m)
                            | marked_yaml::LoadError::TopLevelMustBeSequence(m)
                            | marked_yaml::LoadError::UnexpectedAnchor(m)
                            | marked_yaml::LoadError::MappingKeyMustBeScalar(m)
                            | marked_yaml::LoadError::UnexpectedTag(m) => Some(*m),
                            _ => None,
                        };
                        let (line, col) = if let Some(m) = marker {
                            (m.line() + line_offset, m.column())
                        } else {
                            (1, 1)
                        };
                        report.add(Diagnostic::new(
                            codes::DOCS_FRONT_MATTER_MALFORMED,
                            severity,
                            Some(Location::new(&rel_path_str, Some(line), Some(col))),
                            format!("Front matter in '{rel_path_str}' is malformed YAML"),
                            format!("Fix YAML syntax errors in '{rel_path_str}' front matter"),
                        ));
                    }
                    Ok(node) => {
                        let map = match node.as_mapping() {
                            Some(m) => m,
                            None => {
                                report.add(Diagnostic::new(
                                    codes::DOCS_FRONT_MATTER_MALFORMED,
                                    severity,
                                    Some(Location::new(&rel_path_str, Some(1), Some(1))),
                                    format!("Front matter in '{rel_path_str}' is not a YAML mapping"),
                                    format!("Ensure front matter in '{rel_path_str}' is a key-value mapping"),
                                ));
                                continue;
                            }
                        };

                        let required_fields = ["title", "status", "updated"];
                        for field_name in required_fields {
                            let field_entry = map.iter().find(|(k, _)| k.as_str() == field_name);
                            match field_entry {
                                None => {
                                    report.add(Diagnostic::new(
                                        codes::DOCS_FRONT_MATTER_FIELD_INVALID,
                                        severity,
                                        Some(Location::new(&rel_path_str, Some(1), Some(1))),
                                        format!("Front matter field '{field_name}' in '{rel_path_str}' is absent"),
                                        format!("Add required field '{field_name}' to '{rel_path_str}' front matter"),
                                    ));
                                }
                                Some((k, v)) => {
                                    let key_line = k
                                        .span()
                                        .start()
                                        .map(|m| m.line() + line_offset)
                                        .unwrap_or(1);
                                    let key_col =
                                        k.span().start().map(|m| m.column()).unwrap_or(1);
                                    let location = Location::new(
                                        &rel_path_str,
                                        Some(key_line),
                                        Some(key_col),
                                    );

                                    let is_valid = match v {
                                        marked_yaml::Node::Scalar(s) => {
                                            let val_str = s.as_str().trim();
                                            if val_str.is_empty() {
                                                false
                                            } else if field_name == "updated" {
                                                is_valid_yyyy_mm_dd(val_str)
                                            } else {
                                                true
                                            }
                                        }
                                        _ => false,
                                    };

                                    if !is_valid {
                                        report.add(Diagnostic::new(
                                            codes::DOCS_FRONT_MATTER_FIELD_INVALID,
                                            severity,
                                            Some(location),
                                            format!("Front matter field '{field_name}' in '{rel_path_str}' is invalid"),
                                            format!("Provide a valid '{field_name}' in '{rel_path_str}' front matter"),
                                        ));
                                    }
                                }
                            }
                        }

                        if let Some((_, source_refs_node)) =
                            map.iter().find(|(k, _)| k.as_str() == "sourceRefs")
                        {
                            let process_entry =
                                |node: &marked_yaml::Node, rep: &mut ValidationReport| {
                                    if let marked_yaml::Node::Scalar(s) = node {
                                        let entry_str = s.as_str();
                                        let line = s
                                            .span()
                                            .start()
                                            .map(|m| m.line() + line_offset)
                                            .unwrap_or(1);
                                        let col =
                                            s.span().start().map(|m| m.column()).unwrap_or(1);
                                        let location =
                                            Location::new(&rel_path_str, Some(line), Some(col));

                                        let path_part = match entry_str.find(['§', '#']) {
                                            Some(idx) => &entry_str[..idx],
                                            None => entry_str,
                                        };
                                        let trimmed_path = path_part.trim();

                                        if is_root_escaping(trimmed_path) {
                                            rep.add(Diagnostic::new(
                                                codes::DOCS_SOURCE_REF_ESCAPES_ROOT,
                                                Severity::Warning,
                                                Some(location),
                                                format!(
                                                    "Source reference '{entry_str}' in '{rel_path_str}' escapes project root"
                                                ),
                                                format!(
                                                    "Ensure source reference path stays within project root"
                                                ),
                                            ));
                                        } else {
                                            let target_path = root.join(trimmed_path);
                                            if !target_path.is_file() {
                                                rep.add(Diagnostic::new(
                                                    codes::DOCS_SOURCE_REF_UNRESOLVABLE,
                                                    Severity::Warning,
                                                    Some(location),
                                                    format!(
                                                        "Source reference '{entry_str}' in '{rel_path_str}' cannot be resolved"
                                                    ),
                                                    format!(
                                                        "Ensure source reference '{entry_str}' points to an existing file"
                                                    ),
                                                ));
                                            }
                                        }
                                    }
                                };

                            match source_refs_node {
                                marked_yaml::Node::Sequence(seq) => {
                                    for item in seq.iter() {
                                        process_entry(item, &mut report);
                                    }
                                }
                                marked_yaml::Node::Scalar(_) => {
                                    process_entry(source_refs_node, &mut report);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    const VALID_FRONT_MATTER: &str = "---\ntitle: Document Title\nstatus: active\nupdated: 2026-08-25\n---\n# Content\n";

    #[test]
    fn test_required_documents_validation_and_missing_document_diagnostics() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_docs_req_{}", std::process::id()));
        let _ = fs::create_dir_all(temp_dir.join("docs"));

        // 1. Create all four required documents with valid front matter (for required docs/ files)
        fs::write(temp_dir.join("README.md"), "# README Content\n").unwrap();
        for &rel_path in &["docs/architecture.md", "docs/design.md", "docs/naming.md"] {
            fs::write(temp_dir.join(rel_path), VALID_FRONT_MATTER).unwrap();
        }

        // Clean run with all four documents present
        let clean_report = validate_documents(&temp_dir).expect("validation report");
        assert!(
            clean_report.diagnostics.is_empty(),
            "clean fixture with all 4 required documents must yield 0 diagnostics"
        );

        // Conditional document (docs/development.md) missing must NOT yield any diagnostic
        assert!(
            !temp_dir.join("docs/development.md").exists(),
            "docs/development.md should be absent in test setup"
        );
        let cond_report = validate_documents(&temp_dir).expect("validation report");
        assert!(
            cond_report.diagnostics.is_empty(),
            "absence of conditional docs/development.md must yield 0 diagnostics"
        );

        // 2. Test removing each of the four required documents in turn
        for &target_rel_path in REQUIRED_DOCUMENTS {
            let target_path = temp_dir.join(target_rel_path);
            fs::remove_file(&target_path).unwrap();

            let report = validate_documents(&temp_dir).expect("validation report");
            assert_eq!(
                report.diagnostics.len(),
                1,
                "removing '{target_rel_path}' must yield exactly 1 diagnostic"
            );

            let d = &report.diagnostics[0];
            assert_eq!(d.code, codes::DOCS_REQUIRED_DOCUMENT_MISSING);
            assert!(
                !report.diagnostics.iter().any(|diag| diag.code == codes::DOCS_FRONT_MATTER_MISSING),
                "absent required document must not emit front-matter-missing diagnostic"
            );
            assert_eq!(d.severity, Severity::Error);
            let loc = d.location.as_ref().expect("has location");
            assert_eq!(
                loc.path,
                PathBuf::from(target_rel_path),
                "diagnostic location path must match '{target_rel_path}'"
            );
            assert_eq!(loc.line, None);
            assert_eq!(loc.column, None);

            // Restore the document
            if target_rel_path == "README.md" {
                fs::write(&target_path, "# README Content\n").unwrap();
            } else {
                fs::write(&target_path, VALID_FRONT_MATTER).unwrap();
            }
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_front_matter_validation_probe_cases() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_fm_probe_{}", std::process::id()));
        let _ = fs::create_dir_all(temp_dir.join("docs/notes"));

        // Setup base clean required documents
        fs::write(temp_dir.join("README.md"), "# README without front matter\n").unwrap();
        fs::write(temp_dir.join("docs/architecture.md"), VALID_FRONT_MATTER).unwrap();
        fs::write(temp_dir.join("docs/design.md"), VALID_FRONT_MATTER).unwrap();
        fs::write(temp_dir.join("docs/naming.md"), VALID_FRONT_MATTER).unwrap();

        // Case 1: README.md with no front matter yields zero diagnostics
        let r1 = validate_documents(&temp_dir).unwrap();
        assert!(r1.diagnostics.is_empty(), "README with no front matter yields 0 diagnostics");

        // Case 2: Required docs/ file with no opening --- yields front-matter-missing (Error)
        fs::write(temp_dir.join("docs/architecture.md"), "# No front matter\n").unwrap();
        let r2 = validate_documents(&temp_dir).unwrap();
        assert_eq!(r2.diagnostics.len(), 1);
        assert_eq!(r2.diagnostics[0].code, codes::DOCS_FRONT_MATTER_MISSING);
        assert_eq!(r2.diagnostics[0].severity, Severity::Error);

        // Case 3: Required docs/ file with unterminated --- yields front-matter-malformed (Error)
        fs::write(temp_dir.join("docs/architecture.md"), "---\ntitle: Foo\nstatus: active\nno closing delimiter\n").unwrap();
        let r3 = validate_documents(&temp_dir).unwrap();
        assert_eq!(r3.diagnostics.len(), 1);
        assert_eq!(r3.diagnostics[0].code, codes::DOCS_FRONT_MATTER_MALFORMED);
        assert_eq!(r3.diagnostics[0].severity, Severity::Error);

        // Case 4: Required docs/ file with empty title, empty status, or non-YYYY-MM-DD updated yields front-matter-field-invalid (Error)
        fs::write(
            temp_dir.join("docs/architecture.md"),
            "---\ntitle: \"\"\nstatus: active\nupdated: 2026-08-25\n---\n",
        ).unwrap();
        let r4 = validate_documents(&temp_dir).unwrap();
        assert_eq!(r4.diagnostics.len(), 1);
        assert_eq!(r4.diagnostics[0].code, codes::DOCS_FRONT_MATTER_FIELD_INVALID);
        assert_eq!(r4.diagnostics[0].severity, Severity::Error);

        fs::write(
            temp_dir.join("docs/architecture.md"),
            "---\ntitle: Architecture\nstatus: \"\"\nupdated: 2026-08-25\n---\n",
        ).unwrap();
        let r4_status = validate_documents(&temp_dir).unwrap();
        assert_eq!(r4_status.diagnostics.len(), 1);
        assert_eq!(r4_status.diagnostics[0].code, codes::DOCS_FRONT_MATTER_FIELD_INVALID);
        assert_eq!(r4_status.diagnostics[0].severity, Severity::Error);

        fs::write(
            temp_dir.join("docs/architecture.md"),
            "---\ntitle: Architecture\nstatus: active\nupdated: invalid-date\n---\n",
        ).unwrap();
        let r4_date = validate_documents(&temp_dir).unwrap();
        assert_eq!(r4_date.diagnostics.len(), 1);
        assert_eq!(r4_date.diagnostics[0].code, codes::DOCS_FRONT_MATTER_FIELD_INVALID);
        assert_eq!(r4_date.diagnostics[0].severity, Severity::Error);

        // Restore clean architecture.md
        fs::write(temp_dir.join("docs/architecture.md"), VALID_FRONT_MATTER).unwrap();

        // Case 5 & 6: docs/notes/scratch.md without front matter or with unterminated --- yields zero
        fs::write(temp_dir.join("docs/notes/scratch.md"), "# Scratch Note\n").unwrap();
        let r5 = validate_documents(&temp_dir).unwrap();
        assert!(r5.diagnostics.is_empty(), "opt-in doc without front matter yields zero");

        fs::write(temp_dir.join("docs/notes/scratch.md"), "---\n# Just horizontal line\n").unwrap();
        let r6 = validate_documents(&temp_dir).unwrap();
        assert!(r6.diagnostics.is_empty(), "opt-in doc with unterminated --- yields zero");

        // Case 7: docs/notes/scratch.md with closed block and empty title yields front-matter-field-invalid at Warning and run still exits 0 (!has_error())
        fs::write(
            temp_dir.join("docs/notes/scratch.md"),
            "---\ntitle: \"\"\nstatus: draft\nupdated: 2026-08-25\n---\n",
        ).unwrap();
        let r7 = validate_documents(&temp_dir).unwrap();
        assert_eq!(r7.diagnostics.len(), 1);
        assert_eq!(r7.diagnostics[0].code, codes::DOCS_FRONT_MATTER_FIELD_INVALID);
        assert_eq!(r7.diagnostics[0].severity, Severity::Warning);
        assert!(!r7.has_error(), "Warning-only report has_error must be false (exits 0)");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_front_matter_exact_line_and_column_position() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_fm_pos_{}", std::process::id()));
        let _ = fs::create_dir_all(temp_dir.join("docs"));

        fs::write(temp_dir.join("README.md"), "# README\n").unwrap();
        fs::write(temp_dir.join("docs/design.md"), VALID_FRONT_MATTER).unwrap();
        fs::write(temp_dir.join("docs/naming.md"), VALID_FRONT_MATTER).unwrap();

        // title is on line 2, status is on line 3, updated is on line 4
        let fm_content = "---\ntitle: Title\nstatus: \"\"\nupdated: 2026-08-25\n---\n";
        fs::write(temp_dir.join("docs/architecture.md"), fm_content).unwrap();

        let report = validate_documents(&temp_dir).unwrap();
        assert_eq!(report.diagnostics.len(), 1);
        let d = &report.diagnostics[0];
        assert_eq!(d.code, codes::DOCS_FRONT_MATTER_FIELD_INVALID);
        let loc = d.location.as_ref().expect("has location");
        assert_eq!(loc.path, PathBuf::from("docs/architecture.md"));
        assert_eq!(loc.line, Some(3), "expected exact line 3 for status field");
        assert_eq!(loc.column, Some(1), "expected exact column 1 for status field");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_crlf_bytes_produce_identical_diagnostics() {
        let temp_dir_lf = std::env::temp_dir().join(format!("wda_test_crlf_lf_{}", std::process::id()));
        let temp_dir_crlf = std::env::temp_dir().join(format!("wda_test_crlf_crlf_{}", std::process::id()));
        let _ = fs::create_dir_all(temp_dir_lf.join("docs"));
        let _ = fs::create_dir_all(temp_dir_crlf.join("docs"));

        let content_lf = "---\ntitle: Architecture\nstatus: \"\"\nupdated: 2026-08-25\n---\n";
        let content_crlf = "---\r\ntitle: Architecture\r\nstatus: \"\"\r\nupdated: 2026-08-25\r\n---\r\n";

        fs::write(temp_dir_lf.join("README.md"), "# README\n").unwrap();
        fs::write(temp_dir_lf.join("docs/architecture.md"), content_lf).unwrap();
        fs::write(temp_dir_lf.join("docs/design.md"), VALID_FRONT_MATTER).unwrap();
        fs::write(temp_dir_lf.join("docs/naming.md"), VALID_FRONT_MATTER).unwrap();

        fs::write(temp_dir_crlf.join("README.md"), "# README\r\n").unwrap();
        fs::write(temp_dir_crlf.join("docs/architecture.md"), content_crlf).unwrap();
        fs::write(temp_dir_crlf.join("docs/design.md"), VALID_FRONT_MATTER).unwrap();
        fs::write(temp_dir_crlf.join("docs/naming.md"), VALID_FRONT_MATTER).unwrap();

        let report_lf = validate_documents(&temp_dir_lf).unwrap();
        let report_crlf = validate_documents(&temp_dir_crlf).unwrap();

        let _ = fs::remove_dir_all(&temp_dir_lf);
        let _ = fs::remove_dir_all(&temp_dir_crlf);

        assert_eq!(report_lf.diagnostics.len(), 1);
        assert_eq!(report_crlf.diagnostics.len(), 1);
        assert_eq!(report_lf.diagnostics[0].code, report_crlf.diagnostics[0].code);
        assert_eq!(report_lf.diagnostics[0].severity, report_crlf.diagnostics[0].severity);
        assert_eq!(report_lf.diagnostics[0].location, report_crlf.diagnostics[0].location);
    }

    #[test]
    fn test_source_refs_resolution_and_root_escaping_probe_cases() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_sr_{}", std::process::id()));
        let _ = fs::create_dir_all(temp_dir.join("docs"));

        // Setup base clean required documents
        fs::write(temp_dir.join("README.md"), "# README\n").unwrap();
        fs::write(temp_dir.join("docs/design.md"), VALID_FRONT_MATTER).unwrap();
        fs::write(temp_dir.join("docs/naming.md"), VALID_FRONT_MATTER).unwrap();
        fs::write(
            temp_dir.join("docs/my document (v1).md"),
            VALID_FRONT_MATTER,
        )
        .unwrap();

        // Case 1: Absent sourceRefs yields zero diagnostics
        fs::write(temp_dir.join("docs/architecture.md"), VALID_FRONT_MATTER).unwrap();
        let r1 = validate_documents(&temp_dir).unwrap();
        assert!(
            r1.diagnostics.is_empty(),
            "absent sourceRefs must yield 0 diagnostics"
        );

        // Case 2: Non-existent target yields wda.docs.source-ref-unresolvable Warning
        let fm_unresolvable = "---\ntitle: Architecture\nstatus: active\nupdated: 2026-08-25\nsourceRefs:\n  - \"docs/nonexistent.md\"\n---\n";
        fs::write(temp_dir.join("docs/architecture.md"), fm_unresolvable).unwrap();
        let r2 = validate_documents(&temp_dir).unwrap();
        assert_eq!(r2.diagnostics.len(), 1);
        assert_eq!(r2.diagnostics[0].code, codes::DOCS_SOURCE_REF_UNRESOLVABLE);
        assert_eq!(r2.diagnostics[0].severity, Severity::Warning);

        // Case 3: Root-escaping entries (relative traversal, absolute POSIX, Windows drive)
        // each yield wda.docs.source-ref-escapes-root Warning
        let fm_escaping = "---\ntitle: Architecture\nstatus: active\nupdated: 2026-08-25\nsourceRefs:\n  - \"../../etc/passwd\"\n  - \"/etc/passwd\"\n  - \"C:/etc/passwd\"\n---\n";
        fs::write(temp_dir.join("docs/architecture.md"), fm_escaping).unwrap();
        let r3 = validate_documents(&temp_dir).unwrap();
        assert_eq!(r3.diagnostics.len(), 3);
        for d in &r3.diagnostics {
            assert_eq!(d.code, codes::DOCS_SOURCE_REF_ESCAPES_ROOT);
            assert_eq!(d.severity, Severity::Warning);
        }

        // Case 4: Entry with section suffix (§ or #) whose file resolves yields zero diagnostics
        let fm_section = "---\ntitle: Architecture\nstatus: active\nupdated: 2026-08-25\nsourceRefs:\n  - \"docs/design.md §2.1\"\n  - \"docs/naming.md#section-3\"\n---\n";
        fs::write(temp_dir.join("docs/architecture.md"), fm_section).unwrap();
        let r4 = validate_documents(&temp_dir).unwrap();
        assert!(
            r4.diagnostics.is_empty(),
            "resolvable entry with section suffix must yield 0 diagnostics"
        );

        // Case 5: Entry with spaces and parentheses followed by §2 resolves and yields zero
        let fm_spaces_parens = "---\ntitle: Architecture\nstatus: active\nupdated: 2026-08-25\nsourceRefs:\n  - \"docs/my document (v1).md §2\"\n---\n";
        fs::write(temp_dir.join("docs/architecture.md"), fm_spaces_parens).unwrap();
        let r5 = validate_documents(&temp_dir).unwrap();
        assert!(
            r5.diagnostics.is_empty(),
            "entry with spaces and parentheses followed by section suffix must yield 0 diagnostics"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }
}

