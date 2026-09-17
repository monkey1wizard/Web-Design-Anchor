//! Reporting helpers for completed build output.

use std::fs;
use std::path::Path;

/// Detects output constructs that are known to require HTTP serving in a browser.
///
/// The scan is deterministic: directory entries and files are visited in sorted
/// path order, and unreadable entries are ignored because they cannot contribute
/// observable output. It recognizes module script attributes and calls to `fetch`.
pub fn detect_http_requirement(dist_path: &Path) -> bool {
    let mut files = Vec::new();
    collect_files(dist_path, &mut files);
    files.sort();

    files.into_iter().any(|path| {
        fs::read(&path)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .is_some_and(|content| contains_module_script(&content) || contains_fetch_call(&content))
    })
}

fn collect_files(path: &Path, files: &mut Vec<std::path::PathBuf>) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };

    if metadata.is_file() {
        files.push(path.to_path_buf());
        return;
    }

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            collect_files(&entry.path(), files);
        }
    }
}

fn contains_module_script(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    let mut remainder = lower.as_str();

    while let Some(start) = remainder.find("<script") {
        let after_tag = &remainder[start + "<script".len()..];
        let boundary = after_tag.chars().next();
        remainder = after_tag;
        if boundary.is_some_and(|c| c.is_ascii_alphanumeric()) {
            continue;
        }
        let Some(end) = remainder.find('>') else {
            break;
        };
        let attributes = &remainder[..end];
        if has_module_type_attribute(attributes) {
            return true;
        }
        remainder = &remainder[end + 1..];
    }

    false
}

fn has_module_type_attribute(attributes: &str) -> bool {
    let mut remainder = attributes;
    while let Some(start) = remainder.find("type") {
        let before = remainder[..start].chars().next_back();
        let after = remainder[start + "type".len()..].chars().next();
        if before.is_none_or(|c| c.is_ascii_whitespace())
            && after.is_none_or(|c| c.is_ascii_whitespace() || c == '=')
        {
            let value = remainder[start + "type".len()..].trim_start();
            if let Some(value) = value.strip_prefix('=') {
                if attribute_value_is_module(value.trim_start()) {
                    return true;
                }
            }
        }
        remainder = &remainder[start + "type".len()..];
    }
    false
}

/// Reports whether an attribute value, taken from just after `type=`, is `module`.
///
/// Handles the three forms HTML allows: double-quoted, single-quoted, and
/// unquoted. An unquoted value ends at the first ASCII whitespace, and the
/// caller has already cut the slice at the tag's closing `>`, so a trailing `/`
/// from a self-closing tag is the only other terminator left to strip. The
/// content reaching this function is already lowercased, and HTML strips ASCII
/// whitespace around a script `type` before comparing it, so both sides are
/// compared trimmed.
fn attribute_value_is_module(value: &str) -> bool {
    let unquoted = if let Some(rest) = value.strip_prefix('"') {
        rest.split('"').next()
    } else if let Some(rest) = value.strip_prefix('\'') {
        rest.split('\'').next()
    } else {
        value
            .split(|c: char| c.is_ascii_whitespace())
            .next()
            .map(|candidate| candidate.strip_suffix('/').unwrap_or(candidate))
    };

    unquoted.is_some_and(|candidate| candidate.trim() == "module")
}

fn contains_fetch_call(content: &str) -> bool {
    let bytes = content.as_bytes();
    let needle = b"fetch";
    let mut offset = 0;

    while let Some(relative) = content[offset..].find("fetch") {
        let start = offset + relative;
        let end = start + needle.len();
        let before_ok = start == 0 || !is_identifier_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_identifier_byte(bytes[end]);
        let call_ok = content[end..].chars().next().is_some_and(|c| c.is_ascii_whitespace() || c == '(');
        if before_ok && after_ok && call_ok {
            return true;
        }
        offset = end;
    }

    false
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_module_script_in_nested_output() {
        let root = std::env::temp_dir().join(format!("wda_http_module_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("nested/index.html"), r#"<script type="module" src="app.js"></script>"#).unwrap();

        assert!(detect_http_requirement(&root));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn detects_module_script_preceded_by_document_preamble() {
        // Mirrors the real `dist/` shape observed in the G8 browser probe: a
        // `<!DOCTYPE html>` and `<head>` preamble pushes the `<script>` match off
        // offset 0, which previously made the boundary check read an unrelated
        // character and miss the `type="module"` attribute entirely.
        let root = std::env::temp_dir().join(format!("wda_http_module_preamble_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("index.html"),
            "<!DOCTYPE html>\n<html lang=\"en\"><head>\n  <meta charset=\"utf-8\"><title>Mapping</title>\n  <link rel=\"stylesheet\" href=\"styles/tokens.css\">\n  <script type=\"module\" src=\"scripts/main.js\"></script>\n</head><body><h1>Mapping</h1></body></html>",
        )
        .unwrap();

        assert!(detect_http_requirement(&root));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn detects_module_script_in_every_attribute_quoting_form() {
        // `type=module` without quotes is valid HTML5 and blocks under `file://`
        // exactly like the quoted form, so a report that misses it sends the
        // designer to a CORS error with no warning.
        let cases = [
            ("double", r#"<script type="module"></script>"#, true),
            ("single", r#"<script type='module'></script>"#, true),
            ("unquoted", r#"<script type=module></script>"#, true),
            ("spaced", r#"<script type = module></script>"#, true),
            ("self-closing", r#"<script type=module/>"#, true),
            ("trailing", r#"<script src="a.js" type=module>"#, true),
            ("not-module", r#"<script type=text/javascript>"#, false),
            ("prefix-only", r#"<script type=moduleish></script>"#, false),
        ];

        for (label, html, expected) in cases {
            let root = std::env::temp_dir().join(format!(
                "wda_http_module_form_{label}_{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            fs::write(root.join("index.html"), html).unwrap();

            assert_eq!(
                detect_http_requirement(&root),
                expected,
                "case '{label}' with markup {html}"
            );
            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn detects_fetch_and_rejects_plain_script_output() {
        let root = std::env::temp_dir().join(format!("wda_http_fetch_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("fetch.js"), "window.fetch('/data.json');").unwrap();
        assert!(detect_http_requirement(&root));
        fs::write(root.join("fetch.js"), "document.querySelector('main');").unwrap();
        assert!(!detect_http_requirement(&root));
        let _ = fs::remove_dir_all(root);
    }
}
