use std::fs;
use std::path::{Path, PathBuf};

use crate::codes;
use crate::diagnostics::{Diagnostic, Location, Severity, ToolFault, ValidationReport};

fn byte_offset_to_line_col(content: &str, target_offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, b) in content.bytes().enumerate() {
        if i == target_offset {
            break;
        }
        if b == b'\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn collect_html_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_html_files(&path, files);
            } else if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.eq_ignore_ascii_case("html") {
                        files.push(path);
                    }
                }
            }
        }
    }
}

/// Validates document ID uniqueness in HTML content.
///
/// Records `wda.html.duplicate-id` diagnostics into `report` for any duplicate ID attributes found in `content`.
pub(crate) fn check_duplicate_ids(
    content: &str,
    rel_path: &Path,
    report: &mut ValidationReport,
) {
    let rel_path_str = rel_path.to_string_lossy().replace('\\', "/");
    let emitter = html5gum::DefaultEmitter::<usize>::new_with_span();
    let tokenizer = html5gum::Tokenizer::new_with_emitter(content, emitter);

    let mut seen_ids = std::collections::HashSet::new();

    for token in tokenizer {
        if let Ok(html5gum::Token::StartTag(tag)) = token {
            for (attr_name, attr_val) in &tag.attributes {
                if attr_name.as_slice().eq_ignore_ascii_case(b"id") {
                    let id_str = std::str::from_utf8(attr_val.as_slice()).unwrap_or("");
                    if !seen_ids.insert(id_str.to_string()) {
                        let (line, col) = byte_offset_to_line_col(content, tag.span.start);
                        report.add(Diagnostic::new(
                            codes::HTML_DUPLICATE_ID,
                            Severity::Error,
                            Some(Location::new(&rel_path_str, Some(line), Some(col))),
                            format!("Duplicate ID '{id_str}' found in HTML document '{rel_path_str}'"),
                            format!("Ensure ID attributes are unique within document '{rel_path_str}'"),
                        ));
                    }
                    break;
                }
            }
        }
    }
}

/// Validates HTML files in the project.
///
/// Checks `pages/**/*.html` and `components/**/*.html` for HTML accessibility rules:
/// - `wda.html.lang-missing` (pages only)
/// - `wda.html.img-alt-missing` (pages and components)
/// - `wda.html.duplicate-id` (pages and components)
pub fn validate_html(root: &Path) -> Result<ValidationReport, ToolFault> {
    let mut report = ValidationReport::new();

    let mut html_files = Vec::new();

    let pages_dir = root.join("pages");
    if pages_dir.is_dir() {
        collect_html_files(&pages_dir, &mut html_files);
    }

    let components_dir = root.join("components");
    if components_dir.is_dir() {
        collect_html_files(&components_dir, &mut html_files);
    }

    html_files.sort();

    for full_path in html_files {
        let rel_path = match full_path.strip_prefix(root) {
            Ok(p) => p,
            Err(_) => &full_path,
        };
        let rel_path_str = rel_path.to_string_lossy().replace('\\', "/");
        let is_page = rel_path_str.starts_with("pages/");

        let content = match fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(err) => {
                return Err(ToolFault::new(
                    format!("Failed to read HTML file '{rel_path_str}': {err}"),
                    Some(full_path),
                ));
            }
        };

        check_duplicate_ids(&content, rel_path, &mut report);

        let emitter = html5gum::DefaultEmitter::<usize>::new_with_span();
        let tokenizer = html5gum::Tokenizer::new_with_emitter(&content, emitter);

        let mut html_tag_found = false;
        let mut lang_valid = false;
        let mut html_tag_offset = 0;

        for token in tokenizer {
            if let Ok(html5gum::Token::StartTag(tag)) = token {
                let tag_name = tag.name.as_slice();

                if is_page && tag_name.eq_ignore_ascii_case(b"html") {
                    html_tag_found = true;
                    html_tag_offset = tag.span.start;

                    for (attr_name, attr_val) in &tag.attributes {
                        if attr_name.as_slice().eq_ignore_ascii_case(b"lang") {
                            let val_str = std::str::from_utf8(attr_val.as_slice()).unwrap_or("");
                            if !val_str.trim().is_empty() {
                                lang_valid = true;
                            }
                            break;
                        }
                    }
                }

                if tag_name.eq_ignore_ascii_case(b"img") {
                    let mut alt_found = false;
                    for (attr_name, _) in &tag.attributes {
                        if attr_name.as_slice().eq_ignore_ascii_case(b"alt") {
                            alt_found = true;
                            break;
                        }
                    }

                    if !alt_found {
                        let (line, col) = byte_offset_to_line_col(&content, tag.span.start);
                        report.add(Diagnostic::new(
                            codes::HTML_IMG_ALT_MISSING,
                            Severity::Error,
                            Some(Location::new(&rel_path_str, Some(line), Some(col))),
                            format!("HTML 'img' element in '{rel_path_str}' is missing an 'alt' attribute"),
                            format!("Add an 'alt' attribute to 'img' element in '{rel_path_str}'"),
                        ));
                    }
                }
            }
        }

        if is_page && (!html_tag_found || !lang_valid) {
            let (line, col) = if html_tag_found {
                byte_offset_to_line_col(&content, html_tag_offset)
            } else {
                (1, 1)
            };

            report.add(Diagnostic::new(
                codes::HTML_LANG_MISSING,
                Severity::Error,
                Some(Location::new(&rel_path_str, Some(line), Some(col))),
                format!("HTML document '{rel_path_str}' is missing a valid 'lang' attribute on <html> element"),
                format!("Add a non-empty 'lang' attribute to <html> element in '{rel_path_str}'"),
            ));
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_pages_html_missing_lang_attribute_yields_lang_missing_error() {
        let temp_dir =
            std::env::temp_dir().join(format!("wda_test_html_missing_lang_{}", std::process::id()));
        let pages_dir = temp_dir.join("pages");
        fs::create_dir_all(&pages_dir).unwrap();

        fs::write(
            pages_dir.join("index.html"),
            "<!DOCTYPE html>\n<html>\n<head></head>\n<body></body>\n</html>\n",
        )
        .unwrap();

        let report = validate_html(&temp_dir).expect("validation report");
        assert_eq!(report.diagnostics.len(), 1);
        let d = &report.diagnostics[0];
        assert_eq!(d.code, codes::HTML_LANG_MISSING);
        assert_eq!(d.severity, Severity::Error);
        let loc = d.location.as_ref().expect("has location");
        assert_eq!(loc.path, PathBuf::from("pages/index.html"));
        assert_eq!(loc.line, Some(2));
        assert_eq!(loc.column, Some(1));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pages_html_empty_lang_attribute_yields_lang_missing_error() {
        let temp_dir =
            std::env::temp_dir().join(format!("wda_test_html_empty_lang_{}", std::process::id()));
        let pages_dir = temp_dir.join("pages");
        fs::create_dir_all(&pages_dir).unwrap();

        fs::write(
            pages_dir.join("index.html"),
            "<!DOCTYPE html>\n<html lang=\"\">\n<head></head>\n<body></body>\n</html>\n",
        )
        .unwrap();

        let report = validate_html(&temp_dir).expect("validation report");
        assert_eq!(report.diagnostics.len(), 1);
        let d = &report.diagnostics[0];
        assert_eq!(d.code, codes::HTML_LANG_MISSING);
        assert_eq!(d.severity, Severity::Error);
        let loc = d.location.as_ref().expect("has location");
        assert_eq!(loc.path, PathBuf::from("pages/index.html"));
        assert_eq!(loc.line, Some(2));
        assert_eq!(loc.column, Some(1));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pages_html_whitespace_lang_attribute_yields_lang_missing_error() {
        let temp_dir =
            std::env::temp_dir().join(format!("wda_test_html_ws_lang_{}", std::process::id()));
        let pages_dir = temp_dir.join("pages");
        fs::create_dir_all(&pages_dir).unwrap();

        fs::write(
            pages_dir.join("index.html"),
            "<!DOCTYPE html>\n<html lang=\"   \">\n<head></head>\n<body></body>\n</html>\n",
        )
        .unwrap();

        let report = validate_html(&temp_dir).expect("validation report");
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].code, codes::HTML_LANG_MISSING);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pages_html_valid_lang_attribute_yields_zero_diagnostics() {
        let temp_dir =
            std::env::temp_dir().join(format!("wda_test_html_valid_lang_{}", std::process::id()));
        let pages_dir = temp_dir.join("pages");
        fs::create_dir_all(&pages_dir).unwrap();

        fs::write(
            pages_dir.join("index.html"),
            "<!DOCTYPE html>\n<html lang=\"en\">\n<head></head>\n<body></body>\n</html>\n",
        )
        .unwrap();

        let report = validate_html(&temp_dir).expect("validation report");
        assert!(
            report.diagnostics.is_empty(),
            "valid lang attribute must yield zero diagnostics"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pages_html_synthesized_html_element_without_tag_yields_lang_missing_error() {
        let temp_dir =
            std::env::temp_dir().join(format!("wda_test_html_no_tag_{}", std::process::id()));
        let pages_dir = temp_dir.join("pages");
        fs::create_dir_all(&pages_dir).unwrap();

        fs::write(pages_dir.join("index.html"), "<h1>No html tag at all</h1>\n").unwrap();

        let report = validate_html(&temp_dir).expect("validation report");
        assert_eq!(report.diagnostics.len(), 1);
        let d = &report.diagnostics[0];
        assert_eq!(d.code, codes::HTML_LANG_MISSING);
        let loc = d.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(1));
        assert_eq!(loc.column, Some(1));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_components_html_fragment_yields_zero_diagnostics() {
        let temp_dir =
            std::env::temp_dir().join(format!("wda_test_html_components_{}", std::process::id()));
        let components_dir = temp_dir.join("components");
        fs::create_dir_all(&components_dir).unwrap();

        fs::write(
            components_dir.join("card.html"),
            "<div class=\"card\"><h2>Title</h2></div>\n",
        )
        .unwrap();

        let report = validate_html(&temp_dir).expect("validation report");
        assert!(
            report.diagnostics.is_empty(),
            "components/*.html fragment must yield zero diagnostics"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_html_exact_line_and_column_position() {
        let temp_dir =
            std::env::temp_dir().join(format!("wda_test_html_exact_pos_{}", std::process::id()));
        let pages_dir = temp_dir.join("pages");
        fs::create_dir_all(&pages_dir).unwrap();

        let content = "\n\n<html lang=\"\">\n<body></body>\n</html>";
        fs::write(pages_dir.join("index.html"), content).unwrap();

        let report = validate_html(&temp_dir).expect("validation report");
        assert_eq!(report.diagnostics.len(), 1);
        let d = &report.diagnostics[0];
        assert_eq!(d.code, codes::HTML_LANG_MISSING);
        let loc = d.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(3));
        assert_eq!(loc.column, Some(1));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pages_html_invalid_utf8_yields_tool_fault() {
        let temp_dir =
            std::env::temp_dir().join(format!("wda_test_html_invalid_utf8_{}", std::process::id()));
        let pages_dir = temp_dir.join("pages");
        fs::create_dir_all(&pages_dir).unwrap();

        fs::write(pages_dir.join("index.html"), &[0xFF, 0xFE, 0xFD]).unwrap();

        let err = validate_html(&temp_dir).unwrap_err();
        assert!(err.message.contains("Failed to read HTML file"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_page_and_fragment_img_missing_alt_yields_img_alt_missing_error() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_html_img_missing_alt_{}",
            std::process::id()
        ));
        let pages_dir = temp_dir.join("pages");
        let components_dir = temp_dir.join("components");
        fs::create_dir_all(&pages_dir).unwrap();
        fs::create_dir_all(&components_dir).unwrap();

        fs::write(
            pages_dir.join("index.html"),
            "<!DOCTYPE html>\n<html lang=\"en\">\n<body>\n<img src=\"page.jpg\">\n</body>\n</html>\n",
        )
        .unwrap();

        fs::write(
            components_dir.join("card.html"),
            "<div class=\"card\">\n<img src=\"fragment.jpg\">\n</div>\n",
        )
        .unwrap();

        let report = validate_html(&temp_dir).expect("validation report");
        let alt_missing_diags: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::HTML_IMG_ALT_MISSING)
            .collect();
        assert_eq!(alt_missing_diags.len(), 2);

        let d1 = alt_missing_diags
            .iter()
            .find(|d| d.location.as_ref().unwrap().path == PathBuf::from("components/card.html"))
            .expect("has component diagnostic");
        assert_eq!(d1.severity, Severity::Error);

        let d2 = alt_missing_diags
            .iter()
            .find(|d| d.location.as_ref().unwrap().path == PathBuf::from("pages/index.html"))
            .expect("has page diagnostic");
        assert_eq!(d2.severity, Severity::Error);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_img_empty_alt_attribute_yields_zero_img_alt_missing_diagnostics() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_html_img_empty_alt_{}",
            std::process::id()
        ));
        let pages_dir = temp_dir.join("pages");
        let components_dir = temp_dir.join("components");
        fs::create_dir_all(&pages_dir).unwrap();
        fs::create_dir_all(&components_dir).unwrap();

        fs::write(
            pages_dir.join("index.html"),
            "<!DOCTYPE html>\n<html lang=\"en\">\n<body>\n<img src=\"page.jpg\" alt=\"\">\n</body>\n</html>\n",
        )
        .unwrap();

        fs::write(
            components_dir.join("card.html"),
            "<div class=\"card\">\n<img src=\"fragment.jpg\" alt=\"\">\n</div>\n",
        )
        .unwrap();

        let report = validate_html(&temp_dir).expect("validation report");
        let alt_missing_count = report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::HTML_IMG_ALT_MISSING)
            .count();
        assert_eq!(
            alt_missing_count, 0,
            "alt=\"\" must yield zero img-alt-missing diagnostics"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_repeated_id_in_single_document_yields_duplicate_id_error() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_html_duplicate_id_{}",
            std::process::id()
        ));
        let pages_dir = temp_dir.join("pages");
        fs::create_dir_all(&pages_dir).unwrap();

        fs::write(
            pages_dir.join("index.html"),
            "<!DOCTYPE html>\n<html lang=\"en\">\n<body>\n<div id=\"header\">First</div>\n<span id=\"header\">Second</span>\n</body>\n</html>\n",
        )
        .unwrap();

        let report = validate_html(&temp_dir).expect("validation report");
        let dup_id_diags: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::HTML_DUPLICATE_ID)
            .collect();
        assert_eq!(dup_id_diags.len(), 1);

        let d = dup_id_diags[0];
        assert_eq!(d.severity, Severity::Error);
        let loc = d.location.as_ref().expect("has location");
        assert_eq!(loc.path, PathBuf::from("pages/index.html"));
        assert_eq!(loc.line, Some(5));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_same_id_across_different_documents_yields_zero_duplicate_id_diagnostics() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_html_multi_doc_id_{}",
            std::process::id()
        ));
        let pages_dir = temp_dir.join("pages");
        let components_dir = temp_dir.join("components");
        fs::create_dir_all(&pages_dir).unwrap();
        fs::create_dir_all(&components_dir).unwrap();

        fs::write(
            pages_dir.join("index.html"),
            "<!DOCTYPE html>\n<html lang=\"en\">\n<body>\n<div id=\"header\">Main</div>\n</body>\n</html>\n",
        )
        .unwrap();

        fs::write(
            components_dir.join("card.html"),
            "<div id=\"header\">Header Component</div>\n",
        )
        .unwrap();

        let report = validate_html(&temp_dir).expect("validation report");
        let dup_id_count = report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::HTML_DUPLICATE_ID)
            .count();
        assert_eq!(
            dup_id_count, 0,
            "same ID across different documents must yield zero duplicate-id diagnostics"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }
}

