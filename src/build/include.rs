//! `<wda-include>` expansion with root containment and cycle detection.
//!
//! Provides expansion of build-time `<wda-include src="...">` primitives into static HTML.
//! Includes are resolved relative to the including file and canonicalized against the project root.
//! Emits diagnostics when an include leaves the project root, cannot be resolved, or forms a cycle.
//! Non-include constructs such as `<slot>`, `{{ }}`, and control flow are passed through untouched.

#![allow(dead_code)]

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::codes;
use crate::diagnostics::{Diagnostic, Location, Severity, ValidationReport};

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

#[derive(Debug, Clone)]
struct IncludeTag {
    src: Option<String>,
    start_offset: usize,
    end_offset: usize,
    line: usize,
    col: usize,
}

#[derive(Debug)]
struct PendingInclude {
    src: Option<String>,
    start_offset: usize,
    tag_end_offset: usize,
    line: usize,
    col: usize,
}

fn parse_include_tags(content: &str) -> Vec<IncludeTag> {
    let emitter = html5gum::DefaultEmitter::<usize>::new_with_span();
    let tokenizer = html5gum::Tokenizer::new_with_emitter(content, emitter);

    let mut includes = Vec::new();
    let mut pending: Option<PendingInclude> = None;

    for token in tokenizer {
        match token {
            Ok(html5gum::Token::StartTag(tag)) => {
                let name = tag.name.as_slice();
                if name.eq_ignore_ascii_case(b"wda-include") {
                    if let Some(prev) = pending.take() {
                        includes.push(IncludeTag {
                            src: prev.src,
                            start_offset: prev.start_offset,
                            end_offset: prev.tag_end_offset,
                            line: prev.line,
                            col: prev.col,
                        });
                    }

                    let mut src = None;
                    for (attr_name, attr_val) in &tag.attributes {
                        if attr_name.as_slice().eq_ignore_ascii_case(b"src") {
                            let s = std::str::from_utf8(attr_val.as_slice())
                                .unwrap_or("")
                                .trim()
                                .to_string();
                            src = Some(s);
                            break;
                        }
                    }

                    let (line, col) = byte_offset_to_line_col(content, tag.span.start);

                    if tag.self_closing {
                        includes.push(IncludeTag {
                            src,
                            start_offset: tag.span.start,
                            end_offset: tag.span.end,
                            line,
                            col,
                        });
                    } else {
                        pending = Some(PendingInclude {
                            src,
                            start_offset: tag.span.start,
                            tag_end_offset: tag.span.end,
                            line,
                            col,
                        });
                    }
                }
            }
            Ok(html5gum::Token::EndTag(tag)) => {
                let name = tag.name.as_slice();
                if name.eq_ignore_ascii_case(b"wda-include") {
                    if let Some(prev) = pending.take() {
                        includes.push(IncludeTag {
                            src: prev.src,
                            start_offset: prev.start_offset,
                            end_offset: tag.span.end,
                            line: prev.line,
                            col: prev.col,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(prev) = pending.take() {
        includes.push(IncludeTag {
            src: prev.src,
            start_offset: prev.start_offset,
            end_offset: prev.tag_end_offset,
            line: prev.line,
            col: prev.col,
        });
    }

    includes
}

fn is_include_root_escaping(src: &str, base_rel: &Path) -> bool {
    let bytes = src.as_bytes();
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

    if Path::new(src).is_absolute() {
        return true;
    }

    let mut depth: i32 = 0;
    for comp in base_rel.components() {
        if let Component::Normal(_) = comp {
            depth += 1;
        }
    }

    let normalized = src.replace('\\', "/");
    for comp in Path::new(&normalized).components() {
        match comp {
            Component::Prefix(_) | Component::RootDir => return true,
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return true;
                }
            }
            Component::Normal(_) => {
                depth += 1;
            }
            Component::CurDir => {}
        }
    }

    false
}

fn resolve_target_rel_path(src: &str, base_rel: &Path) -> PathBuf {
    let mut stack: Vec<String> = Vec::new();
    for comp in base_rel.components() {
        if let Component::Normal(n) = comp {
            stack.push(n.to_string_lossy().to_string());
        }
    }

    let normalized = src.replace('\\', "/");
    for comp in Path::new(&normalized).components() {
        match comp {
            Component::ParentDir => {
                stack.pop();
            }
            Component::Normal(n) => {
                stack.push(n.to_string_lossy().to_string());
            }
            _ => {}
        }
    }

    stack.iter().map(PathBuf::from).collect()
}

fn expand_internal(
    content: &str,
    current_file_rel: &Path,
    root: &Path,
    canon_root: &Path,
    include_stack: &mut Vec<PathBuf>,
    report: &mut ValidationReport,
) -> String {
    let includes = parse_include_tags(content);
    if includes.is_empty() {
        return content.to_string();
    }

    let rel_path_str = current_file_rel.to_string_lossy().replace('\\', "/");
    let base_rel = current_file_rel.parent().unwrap_or(Path::new(""));

    let mut replacements: Vec<(usize, usize, String)> = Vec::new();

    for inc in includes {
        let src = match &inc.src {
            Some(s) if !s.is_empty() => s.clone(),
            _ => {
                report.add(Diagnostic::new(
                    codes::BUILD_INCLUDE_UNRESOLVABLE,
                    Severity::Error,
                    Some(Location::new(&rel_path_str, Some(inc.line), Some(inc.col))),
                    format!("Include tag in '{rel_path_str}' is missing or has empty 'src' attribute"),
                    format!("Provide a valid 'src' attribute on the <wda-include> tag"),
                ));
                continue;
            }
        };

        if is_include_root_escaping(&src, base_rel) {
            report.add(Diagnostic::new(
                codes::BUILD_INCLUDE_ESCAPES_ROOT,
                Severity::Error,
                Some(Location::new(&rel_path_str, Some(inc.line), Some(inc.col))),
                format!("Include path '{src}' in '{rel_path_str}' escapes project root"),
                format!("Ensure include path stays within project root"),
            ));
            continue;
        }

        let rel_target = resolve_target_rel_path(&src, base_rel);
        let target_path = root.join(&rel_target);

        if !target_path.is_file() {
            report.add(Diagnostic::new(
                codes::BUILD_INCLUDE_UNRESOLVABLE,
                Severity::Error,
                Some(Location::new(&rel_path_str, Some(inc.line), Some(inc.col))),
                format!("Include path '{src}' in '{rel_path_str}' cannot be resolved"),
                format!("Ensure include path points to an existing file"),
            ));
            continue;
        }

        let canon_target = match target_path.canonicalize() {
            Ok(c) => c,
            Err(_) => {
                report.add(Diagnostic::new(
                    codes::BUILD_INCLUDE_UNRESOLVABLE,
                    Severity::Error,
                    Some(Location::new(&rel_path_str, Some(inc.line), Some(inc.col))),
                    format!("Include path '{src}' in '{rel_path_str}' cannot be resolved"),
                    format!("Ensure include path points to an existing file"),
                ));
                continue;
            }
        };

        if !canon_target.starts_with(canon_root) {
            report.add(Diagnostic::new(
                codes::BUILD_INCLUDE_ESCAPES_ROOT,
                Severity::Error,
                Some(Location::new(&rel_path_str, Some(inc.line), Some(inc.col))),
                format!("Include path '{src}' in '{rel_path_str}' escapes project root"),
                format!("Ensure include path stays within project root"),
            ));
            continue;
        }

        if include_stack.contains(&canon_target) {
            report.add(Diagnostic::new(
                codes::BUILD_INCLUDE_CYCLE,
                Severity::Error,
                Some(Location::new(&rel_path_str, Some(inc.line), Some(inc.col))),
                format!("Cyclic include detected for '{src}' in '{rel_path_str}'"),
                format!("Remove circular <wda-include> dependency"),
            ));
            continue;
        }

        let child_content = match fs::read_to_string(&target_path) {
            Ok(c) => c,
            Err(err) => {
                report.add(Diagnostic::new(
                    codes::BUILD_INCLUDE_UNRESOLVABLE,
                    Severity::Error,
                    Some(Location::new(&rel_path_str, Some(inc.line), Some(inc.col))),
                    format!("Failed to read include target '{src}': {err}"),
                    format!("Ensure include target is readable"),
                ));
                continue;
            }
        };

        include_stack.push(canon_target);
        let child_expanded = expand_internal(
            &child_content,
            &rel_target,
            root,
            canon_root,
            include_stack,
            report,
        );
        include_stack.pop();

        replacements.push((inc.start_offset, inc.end_offset, child_expanded));
    }

    let mut output = content.to_string();
    replacements.sort_by(|a, b| b.0.cmp(&a.0));
    for (start, end, replacement) in replacements {
        output.replace_range(start..end, &replacement);
    }

    output
}

/// Expands all `<wda-include>` tags in `content` assuming the page is located at `page_path` within `root`.
///
/// Returns the expanded content string if expansion succeeded with zero errors, or a [`ValidationReport`]
/// containing all collected diagnostics.
pub fn expand_includes_for_page(
    content: &str,
    page_path: &Path,
    root: &Path,
) -> Result<String, ValidationReport> {
    let mut report = ValidationReport::new();

    let canon_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    let page_rel = if page_path.is_absolute() {
        page_path.strip_prefix(root).unwrap_or(page_path)
    } else {
        page_path
    };

    let page_full = if page_path.is_absolute() {
        page_path.to_path_buf()
    } else {
        root.join(page_path)
    };

    let canon_page = page_full
        .canonicalize()
        .unwrap_or_else(|_| canon_root.join(page_rel));

    let mut include_stack = vec![canon_page];

    let expanded = expand_internal(
        content,
        page_rel,
        root,
        &canon_root,
        &mut include_stack,
        &mut report,
    );

    crate::validation::html::check_duplicate_ids(&expanded, page_rel, &mut report);

    report.sort();

    if report.has_error() {
        Err(report)
    } else {
        Ok(expanded)
    }
}

/// Expands all `<wda-include>` tags in a page's `content` relative to the project `root`.
///
/// Top-level include paths are resolved relative to the project root (`index.html`).
///
/// Returns the expanded content on success, or a [`ValidationReport`] when diagnostics are emitted.
pub fn expand_includes(content: &str, root: &Path) -> Result<String, ValidationReport> {
    expand_includes_for_page(content, Path::new("index.html"), root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nested_include_and_template_passthrough() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_include_nested_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let components_dir = temp_dir.join("components");
        fs::create_dir_all(&components_dir).unwrap();

        fs::write(
            components_dir.join("nav.html"),
            "<nav><wda-include src=\"links.html\"></wda-include></nav>",
        )
        .unwrap();

        fs::write(
            components_dir.join("links.html"),
            "<a href=\"/\">Home</a><a href=\"/about\">About</a>",
        )
        .unwrap();

        let page_content = r#"<!DOCTYPE html>
<html lang="en">
<head><title>Test</title></head>
<body>
  <slot name="header">Default Header</slot>
  <div class="user">{{ user_name }}</div>
  {% if active %}
  <wda-include src="components/nav.html"></wda-include>
</body>
</html>"#;

        let res = expand_includes(page_content, &temp_dir);
        assert!(res.is_ok(), "nested include expansion must succeed");
        let expanded = res.unwrap();

        // 1. Zero <wda-include> residue left
        let residue_count = expanded.matches("<wda-include").count();
        assert_eq!(residue_count, 0, "must have zero <wda-include> remaining");

        // 2. Full nested expansion present
        assert!(
            expanded.contains("<nav><a href=\"/\">Home</a><a href=\"/about\">About</a></nav>"),
            "expanded output must contain fully resolved nested components"
        );

        // 3. <slot>, {{ }}, and control flow survive byte-for-byte
        assert!(
            expanded.contains(r#"<slot name="header">Default Header</slot>"#),
            "<slot> construct must survive byte-for-byte"
        );
        assert!(
            expanded.contains("{{ user_name }}"),
            "{{ }} construct must survive byte-for-byte"
        );
        assert!(
            expanded.contains("{% if active %}"),
            "control flow construct must survive byte-for-byte"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_root_escaping_include_yields_error() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_include_escape_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let page_content = r#"<div><wda-include src="../outside.html"></wda-include></div>"#;

        let res = expand_includes(page_content, &temp_dir);
        assert!(res.is_err(), "root-escaping include must fail");
        let report = res.unwrap_err();
        assert!(report.has_error(), "report must have errors");
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(
            report.diagnostics[0].code,
            codes::BUILD_INCLUDE_ESCAPES_ROOT,
            "must emit BUILD_INCLUDE_ESCAPES_ROOT"
        );
        assert_eq!(report.diagnostics[0].severity, Severity::Error);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_missing_target_yields_unresolvable_error() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_include_missing_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let page_content = r#"<div><wda-include src="components/missing.html"></wda-include></div>"#;

        let res = expand_includes(page_content, &temp_dir);
        assert!(res.is_err(), "missing include target must fail");
        let report = res.unwrap_err();
        assert!(report.has_error(), "report must have errors");
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(
            report.diagnostics[0].code,
            codes::BUILD_INCLUDE_UNRESOLVABLE,
            "must emit BUILD_INCLUDE_UNRESOLVABLE"
        );
        assert_eq!(report.diagnostics[0].severity, Severity::Error);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cyclic_include_yields_error() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_include_cycle_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let comp_dir = temp_dir.join("components");
        fs::create_dir_all(&comp_dir).unwrap();

        fs::write(
            comp_dir.join("cycle_a.html"),
            "<div><wda-include src=\"cycle_b.html\"></wda-include></div>",
        )
        .unwrap();
        fs::write(
            comp_dir.join("cycle_b.html"),
            "<div><wda-include src=\"cycle_a.html\"></wda-include></div>",
        )
        .unwrap();

        let page_content = r#"<div><wda-include src="components/cycle_a.html"></wda-include></div>"#;

        let res = expand_includes(page_content, &temp_dir);
        assert!(res.is_err(), "cyclic include must fail");
        let report = res.unwrap_err();
        assert!(report.has_error(), "report must have errors");
        assert_eq!(
            report.diagnostics[0].code,
            codes::BUILD_INCLUDE_CYCLE,
            "must emit BUILD_INCLUDE_CYCLE"
        );
        assert_eq!(report.diagnostics[0].severity, Severity::Error);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_self_cycle_yields_error() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_include_self_cycle_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let comp_dir = temp_dir.join("components");
        fs::create_dir_all(&comp_dir).unwrap();

        fs::write(
            comp_dir.join("self.html"),
            "<div><wda-include src=\"self.html\"></wda-include></div>",
        )
        .unwrap();

        let page_content = r#"<div><wda-include src="components/self.html"></wda-include></div>"#;

        let res = expand_includes(page_content, &temp_dir);
        assert!(res.is_err(), "self-cyclic include must fail");
        let report = res.unwrap_err();
        assert!(report.has_error(), "report must have errors");
        assert_eq!(report.diagnostics[0].code, codes::BUILD_INCLUDE_CYCLE);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_self_closing_and_void_tag_expansion() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_include_self_closing_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let comp_dir = temp_dir.join("components");
        fs::create_dir_all(&comp_dir).unwrap();

        fs::write(comp_dir.join("widget.html"), "<span>Widget</span>").unwrap();

        let page_content = r#"<div><wda-include src="components/widget.html" /></div>"#;

        let res = expand_includes(page_content, &temp_dir);
        assert!(res.is_ok(), "self-closing include must succeed");
        let expanded = res.unwrap();
        assert_eq!(expanded.matches("<wda-include").count(), 0);
        assert_eq!(expanded, "<div><span>Widget</span></div>");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_duplicate_id_introduced_by_include_expansion_yields_duplicate_id_error() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_include_dup_id_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let pages_dir = temp_dir.join("pages");
        let components_dir = temp_dir.join("components");
        fs::create_dir_all(&pages_dir).unwrap();
        fs::create_dir_all(&components_dir).unwrap();

        fs::write(
            components_dir.join("header.html"),
            "<header id=\"navigation\">Header Content</header>\n",
        )
        .unwrap();

        fs::write(
            components_dir.join("footer.html"),
            "<footer id=\"navigation\">Footer Content</footer>\n",
        )
        .unwrap();

        let page_content = r#"<!DOCTYPE html>
<html lang="en">
<head><title>Test</title></head>
<body>
  <wda-include src="../components/header.html"></wda-include>
  <wda-include src="../components/footer.html"></wda-include>
</body>
</html>"#;
        fs::write(pages_dir.join("index.html"), page_content).unwrap();

        // 1. Verify that each source file alone is clean under validate_html
        let validate_report = crate::validation::html::validate_html(&temp_dir)
            .expect("validate_html succeeded");
        let dup_id_diags: Vec<_> = validate_report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::HTML_DUPLICATE_ID)
            .collect();
        assert!(
            dup_id_diags.is_empty(),
            "each source file alone must yield zero duplicate-id diagnostics"
        );

        // 2. Expanding includes on pages/index.html yields wda.html.duplicate-id
        let res = expand_includes_for_page(page_content, Path::new("pages/index.html"), &temp_dir);
        assert!(
            res.is_err(),
            "expansion introducing duplicate IDs must fail"
        );
        let report = res.unwrap_err();
        assert!(report.has_error());

        let expansion_dup_ids: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::HTML_DUPLICATE_ID)
            .collect();
        assert_eq!(
            expansion_dup_ids.len(),
            1,
            "must emit duplicate-id diagnostic for duplicated ID"
        );

        let diag = expansion_dup_ids[0];
        assert_eq!(diag.code, codes::HTML_DUPLICATE_ID);
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(
            diag.location.as_ref().unwrap().path,
            PathBuf::from("pages/index.html")
        );
        assert!(
            diag.message.contains("navigation"),
            "diagnostic message must identify duplicate ID 'navigation'"
        );

        // 3. Also verify top-level expand_includes
        let top_content = r#"<div>
  <wda-include src="components/header.html"></wda-include>
  <wda-include src="components/footer.html"></wda-include>
</div>"#;
        let res_top = expand_includes(top_content, &temp_dir);
        assert!(res_top.is_err());
        let top_report = res_top.unwrap_err();
        assert_eq!(
            top_report
                .diagnostics
                .iter()
                .filter(|d| d.code == codes::HTML_DUPLICATE_ID)
                .count(),
            1
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
