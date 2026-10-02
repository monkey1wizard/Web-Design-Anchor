//! Page discovery, source-to-dist mapping, and collision detection.
//!
//! Provides page discovery over `pages/**/*.html` only (never scanning `components/`,
//! `docs/`, or `dist/`) and computes the fixed canonical mapping from source files
//! to `dist/` relative paths per WDA build specification. Emits `wda.build.output-path-collision`
//! when two source paths map to the same `dist/` target path.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::codes;
use crate::diagnostics::{Diagnostic, Location, Severity, ValidationReport};

/// Discovers all HTML page files under `pages/` within `root`.
///
/// Discovers files matching `pages/**/*.html` only. Directories such as `components/`,
/// `docs/`, and `dist/` are never scanned for page entries.
///
/// Returns a sorted vector of project-relative paths (e.g. `pages/index.html`).
pub fn discover_pages(root: &Path) -> Vec<PathBuf> {
    let mut pages = Vec::new();
    let pages_dir = root.join("pages");
    if pages_dir.is_dir() {
        collect_html_files(&pages_dir, &mut pages);
    }
    pages.sort();

    pages
        .into_iter()
        .filter_map(|full_path| full_path.strip_prefix(root).ok().map(|p| p.to_path_buf()))
        .collect()
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

/// Maps a project-relative source path to its canonical `dist/`-relative target path.
///
/// Implements the fixed mapping table:
/// - `pages/<rel>.html` -> `dist/<rel>.html` (extension kept; no nested `index.html` directory)
/// - `styles/<rel>.css` -> `dist/styles/<rel>.css`
/// - `scripts/<rel>.ts` -> `dist/scripts/<rel>.js`
/// - `assets/<rel>` -> `dist/assets/<rel>`
/// - `tokens/tokens.json` -> `dist/styles/tokens.css`
/// - `components/**`, `wda.json`, `README.md`, `docs/**`, `.gitignore`, dotfiles, `.env*` -> `None` (never copied)
pub fn map_source_to_dist(source_rel: &Path) -> Option<PathBuf> {
    let rel_str = source_rel.to_string_lossy().replace('\\', "/");

    // Secret / ignored / internal sources never map to dist
    if rel_str == "wda.json"
        || rel_str == "README.md"
        || rel_str == ".gitignore"
        || rel_str.starts_with("docs/")
        || rel_str.starts_with("components/")
        || rel_str.starts_with("dist/")
    {
        return None;
    }

    // Dotfiles and .env* files are never copied
    for comp in source_rel.components() {
        let comp_str = comp.as_os_str().to_string_lossy();
        if comp_str.starts_with('.') {
            return None;
        }
    }

    if rel_str == "tokens/tokens.json" {
        return Some(PathBuf::from("dist/styles/tokens.css"));
    }

    if let Ok(stripped) = source_rel.strip_prefix("pages") {
        if let Some(ext) = stripped.extension() {
            if ext.eq_ignore_ascii_case("html") {
                return Some(Path::new("dist").join(stripped));
            }
        }
    }

    if let Ok(stripped) = source_rel.strip_prefix("styles") {
        if let Some(ext) = stripped.extension() {
            if ext.eq_ignore_ascii_case("css") {
                return Some(Path::new("dist/styles").join(stripped));
            }
        }
    }

    if let Ok(stripped) = source_rel.strip_prefix("scripts") {
        if let Some(ext) = stripped.extension() {
            if ext.eq_ignore_ascii_case("ts") {
                let mut js_path = stripped.to_path_buf();
                js_path.set_extension("js");
                return Some(Path::new("dist/scripts").join(js_path));
            }
        }
    }

    if let Ok(stripped) = source_rel.strip_prefix("assets") {
        return Some(Path::new("dist/assets").join(stripped));
    }

    None
}

/// Computes the source-to-dist mapping for a collection of source paths, checking for collisions.
///
/// Returns a map of `source_rel_path -> dist_rel_path` on success.
/// If two or more source paths map to the same `dist/` destination path, returns a
/// [`ValidationReport`] containing `wda.build.output-path-collision` Error diagnostics.
pub fn compute_source_mapping<'a, I>(sources: I) -> Result<BTreeMap<PathBuf, PathBuf>, ValidationReport>
where
    I: IntoIterator<Item = &'a Path>,
{
    let mut dist_to_sources: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    let mut mapping: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();

    for src in sources {
        if let Some(dist_dest) = map_source_to_dist(src) {
            dist_to_sources
                .entry(dist_dest.clone())
                .or_default()
                .push(src.to_path_buf());
            mapping.insert(src.to_path_buf(), dist_dest);
        }
    }

    let mut report = ValidationReport::new();

    for (dist_path, src_list) in dist_to_sources {
        if src_list.len() > 1 {
            let dist_str = dist_path.to_string_lossy().replace('\\', "/");
            let mut sorted_sources: Vec<String> = src_list
                .iter()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .collect();
            sorted_sources.sort();
            let sources_str = sorted_sources.join("', '");

            for src in &src_list {
                let src_str = src.to_string_lossy().replace('\\', "/");
                report.add(Diagnostic::new(
                    codes::BUILD_OUTPUT_PATH_COLLISION,
                    Severity::Error,
                    Some(Location::path_only(&src_str)),
                    format!(
                        "Output path '{dist_str}' collides: multiple source paths ('{sources_str}') map to the same destination"
                    ),
                    format!("Ensure each source file maps to a unique destination path in dist/"),
                ));
            }
        }
    }

    report.sort();

    if report.has_error() {
        Err(report)
    } else {
        Ok(mapping)
    }
}

/// Discovers all pages under `pages/` in `root` and computes their `dist/` mappings.
///
/// Emits `wda.build.output-path-collision` if any page collision occurs.
pub fn discover_and_map_pages(root: &Path) -> Result<BTreeMap<PathBuf, PathBuf>, ValidationReport> {
    let pages = discover_pages(root);
    let page_refs: Vec<&Path> = pages.iter().map(|p| p.as_path()).collect();
    compute_source_mapping(page_refs)
}

/// Computes the reachable static copy set and validates all project-relative references.
///
/// Traverses expanded pages under `pages/**/*.html`, inline `<style>` blocks in those pages,
/// and stylesheets under `styles/**/*.css`.
///
/// Classifies references:
/// - Scheme-bearing (`http:`, `https:`, `data:`, `mailto:`, etc.), protocol-relative (`//host`),
///   and pure fragment (`#id`) references are untouched, unvalidated, and never in copy set.
/// - The Known Virtual Asset `styles/tokens.css` is exempt from unresolvable errors and never copied.
/// - Dotfiles and `.env*` paths are strictly excluded from copying; if referenced, emits
///   `wda.build.secret-path-referenced`.
/// - Project-relative references to non-existent files emit `wda.build.reference-unresolvable`.
///
/// Returns `(copy_set, report)`. The `copy_set` is a sorted, deduplicated set of project-relative
/// source paths that are reachable and eligible for copying into `dist/`.
pub fn compute_reachability(
    root: &Path,
    expanded_pages: &BTreeMap<PathBuf, String>,
) -> (std::collections::BTreeSet<PathBuf>, ValidationReport) {
    let mut reachable_copy_set = std::collections::BTreeSet::new();
    let mut report = ValidationReport::new();

    // 1. Collect references from expanded pages (HTML attributes + inline <style> blocks)
    for (page_rel, html_content) in expanded_pages {
        let refs = extract_references_from_html(html_content, page_rel);
        for r in refs {
            process_reference(root, &r, &mut reachable_copy_set, &mut report);
        }
    }

    // 2. Collect references from styles/**/*.css
    let styles_dir = root.join("styles");
    if styles_dir.is_dir() {
        let mut css_files = Vec::new();
        collect_css_files(&styles_dir, &mut css_files);
        css_files.sort();

        for css_full in css_files {
            if let Ok(css_rel) = css_full.strip_prefix(root) {
                if let Ok(css_content) = fs::read_to_string(&css_full) {
                    let refs = extract_references_from_css(&css_content, css_rel, 1, 1);
                    for r in refs {
                        process_reference(root, &r, &mut reachable_copy_set, &mut report);
                    }
                }
            }
        }
    }

    report.sort();
    (reachable_copy_set, report)
}

/// A raw reference extracted from HTML or CSS with source file location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedReference {
    pub raw_url: String,
    pub source_file: PathBuf,
    pub line: Option<usize>,
    pub col: Option<usize>,
}

/// Classification of a reference URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceKind {
    /// Scheme-bearing URL (e.g. `http://...`, `https://...`, `data:...`, `mailto:...`)
    Scheme(String),
    /// Protocol-relative URL (starts with `//`)
    ProtocolRelative,
    /// Pure fragment reference (starts with `#`)
    Fragment,
    /// Project-relative or file-relative file path
    FilePath(String),
}

/// Classifies a reference string into Scheme, ProtocolRelative, Fragment, or FilePath.
pub fn classify_reference(raw: &str) -> ReferenceKind {
    let trimmed = raw.trim();

    if trimmed.starts_with('#') {
        return ReferenceKind::Fragment;
    }

    if trimmed.starts_with("//") {
        return ReferenceKind::ProtocolRelative;
    }

    // Check for URI scheme (RFC 3986: ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) ":")
    if let Some(colon_idx) = trimmed.find(':') {
        let prefix = &trimmed[..colon_idx];
        if !prefix.is_empty() {
            let first_byte = prefix.as_bytes()[0];
            if first_byte.is_ascii_alphabetic() {
                let rest_valid = prefix[1..]
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.');
                if rest_valid {
                    return ReferenceKind::Scheme(prefix.to_ascii_lowercase());
                }
            }
        }
    }

    ReferenceKind::FilePath(trimmed.to_string())
}

/// Resolves a file-path reference relative to the containing document or root.
///
/// Strips query parameters (`?query`) and fragments (`#fragment`).
/// If `file_path` starts with `../`, it is resolved relative to `source_file`'s parent directory.
/// Otherwise, it is resolved as a project-relative path from project root.
pub fn resolve_reference_target(file_path: &str, source_file: &Path) -> PathBuf {
    // Strip query or fragment from path
    let path_only = if let Some(idx) = file_path.find(|c| c == '?' || c == '#') {
        &file_path[..idx]
    } else {
        file_path
    };

    let normalized = path_only.replace('\\', "/");

    // If the path starts with "../", resolve relative to source_file parent
    if normalized.starts_with("../") {
        let base_dir = source_file.parent().unwrap_or(Path::new(""));
        let mut stack: Vec<String> = Vec::new();
        for comp in base_dir.components() {
            if let std::path::Component::Normal(n) = comp {
                stack.push(n.to_string_lossy().to_string());
            }
        }
        for comp in Path::new(&normalized).components() {
            match comp {
                std::path::Component::ParentDir => {
                    stack.pop();
                }
                std::path::Component::Normal(n) => {
                    stack.push(n.to_string_lossy().to_string());
                }
                _ => {}
            }
        }
        return stack.iter().map(PathBuf::from).collect();
    }

    let trimmed = normalized.trim_start_matches('/');
    let mut stack: Vec<String> = Vec::new();
    for comp in Path::new(trimmed).components() {
        match comp {
            std::path::Component::ParentDir => {
                stack.pop();
            }
            std::path::Component::Normal(n) => {
                stack.push(n.to_string_lossy().to_string());
            }
            _ => {}
        }
    }
    stack.iter().map(PathBuf::from).collect()
}

/// Checks if a path is a secret path (dotfile or starts with `.env`).
pub fn is_secret_path(path: &Path) -> bool {
    let rel_str = path.to_string_lossy().replace('\\', "/");
    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    if file_name.starts_with(".env") || rel_str.starts_with(".env") || rel_str.contains("/.env") {
        return true;
    }

    for comp in path.components() {
        if let std::path::Component::Normal(n) = comp {
            let comp_str = n.to_string_lossy();
            if comp_str.starts_with('.') {
                return true;
            }
        }
    }

    false
}

fn process_reference(
    root: &Path,
    r: &ExtractedReference,
    copy_set: &mut std::collections::BTreeSet<PathBuf>,
    report: &mut ValidationReport,
) {
    let kind = classify_reference(&r.raw_url);
    let file_rel_str = match kind {
        ReferenceKind::Scheme(_) | ReferenceKind::ProtocolRelative | ReferenceKind::Fragment => {
            return;
        }
        ReferenceKind::FilePath(ref s) => s.clone(),
    };

    let resolved_rel = resolve_reference_target(&file_rel_str, &r.source_file);
    let resolved_str = resolved_rel.to_string_lossy().replace('\\', "/");
    let source_str = r.source_file.to_string_lossy().replace('\\', "/");

    // Check secret path exclusion first
    if is_secret_path(&resolved_rel) || is_secret_path(Path::new(&file_rel_str)) {
        report.add(Diagnostic::new(
            codes::BUILD_SECRET_PATH_REFERENCED,
            Severity::Error,
            Some(Location::new(&source_str, r.line, r.col)),
            format!(
                "Reference to secret path '{resolved_str}' is forbidden and will not be copied into dist/"
            ),
            format!("Remove reference to secret file or dotfile"),
        ));
        return;
    }

    let parent_rel = r
        .source_file
        .parent()
        .unwrap_or(Path::new(""))
        .join(&resolved_rel);
    let parent_str = parent_rel.to_string_lossy().replace('\\', "/");

    // Known Virtual Asset: styles/tokens.css is generated at build time; exempt from Error and never copied
    if resolved_str == "styles/tokens.css" || parent_str == "styles/tokens.css" {
        return;
    }

    // Check if target file can be produced or copied
    // Check project-relative first, then document-relative if not found directly
    let candidate = if root.join(&resolved_rel).is_file() {
        resolved_rel
    } else if root.join(&parent_rel).is_file() {
        parent_rel
    } else {
        resolved_rel
    };

    let full_target = root.join(&candidate);
    let exists_in_source = full_target.is_file();
    let can_produce_or_copy = exists_in_source && map_source_to_dist(&candidate).is_some();

    if !can_produce_or_copy {
        report.add(Diagnostic::new(
            codes::BUILD_REFERENCE_UNRESOLVABLE,
            Severity::Error,
            Some(Location::new(&source_str, r.line, r.col)),
            format!(
                "Static reference '{raw_ref}' in '{source_str}' cannot be resolved to a source file",
                raw_ref = r.raw_url
            ),
            format!("Ensure referenced file exists or update reference path"),
        ));
        return;
    }

    let cand_str = candidate.to_string_lossy().replace('\\', "/");
    if (candidate.starts_with("assets") || candidate.starts_with("styles"))
        && cand_str != "tokens/tokens.json"
    {
        copy_set.insert(candidate);
    }
}

/// Extracts static references from HTML content.
pub fn extract_references_from_html(
    content: &str,
    source_file: &Path,
) -> Vec<ExtractedReference> {
    let mut refs = Vec::new();
    let emitter = html5gum::DefaultEmitter::<usize>::new_with_span();
    let tokenizer = html5gum::Tokenizer::new_with_emitter(content, emitter);

    let mut in_style_tag = false;
    let mut style_start_offset = 0;
    let mut style_line = 1;
    let mut style_col = 1;

    for token in tokenizer {
        match token {
            Ok(html5gum::Token::StartTag(tag)) => {
                let name = tag.name.as_slice();
                let is_style = name.eq_ignore_ascii_case(b"style");

                if is_style {
                    in_style_tag = true;
                    style_start_offset = tag.span.end;
                    let (l, c) = byte_offset_to_line_col(content, tag.span.end);
                    style_line = l;
                    style_col = c;
                }

                // Check standard referencing attributes: href, src, poster
                for (attr_name, attr_val) in &tag.attributes {
                    let is_ref_attr = attr_name.eq_ignore_ascii_case(b"href")
                        || attr_name.eq_ignore_ascii_case(b"src")
                        || attr_name.eq_ignore_ascii_case(b"poster");

                    if is_ref_attr {
                        let val_str = std::str::from_utf8(attr_val.as_slice())
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        if !val_str.is_empty() {
                            let (line, col) = byte_offset_to_line_col(content, tag.span.start);
                            refs.push(ExtractedReference {
                                raw_url: val_str,
                                source_file: source_file.to_path_buf(),
                                line: Some(line),
                                col: Some(col),
                            });
                        }
                    }

                    // Check inline style attribute: style="..."
                    if attr_name.eq_ignore_ascii_case(b"style") {
                        let val_str = std::str::from_utf8(attr_val.as_slice()).unwrap_or("");
                        let (line, col) = byte_offset_to_line_col(content, tag.span.start);
                        let css_refs = extract_references_from_css(val_str, source_file, line, col);
                        refs.extend(css_refs);
                    }
                }
            }
            Ok(html5gum::Token::EndTag(tag)) => {
                let name = tag.name.as_slice();
                if name.eq_ignore_ascii_case(b"style") && in_style_tag {
                    in_style_tag = false;
                    let style_end_offset = tag.span.start;
                    if style_end_offset >= style_start_offset && style_end_offset <= content.len() {
                        let css_content = &content[style_start_offset..style_end_offset];
                        let css_refs = extract_references_from_css(
                            css_content,
                            source_file,
                            style_line,
                            style_col,
                        );
                        refs.extend(css_refs);
                    }
                }
            }
            _ => {}
        }
    }

    refs
}

/// Extracts static `url(...)` and `@import` references from CSS content.
pub fn extract_references_from_css(
    css: &str,
    source_file: &Path,
    base_line: usize,
    base_col: usize,
) -> Vec<ExtractedReference> {
    let mut refs = Vec::new();
    let bytes = css.as_bytes();
    let len = bytes.len();
    let mut idx = 0;

    while idx < len {
        // Skip CSS comments /* ... */
        if idx + 2 <= len && bytes[idx..idx + 2] == *b"/*" {
            idx += 2;
            while idx + 2 <= len && bytes[idx..idx + 2] != *b"*/" {
                idx += 1;
            }
            if idx + 2 <= len {
                idx += 2;
            } else {
                idx = len;
            }
            continue;
        }

        // Match url(...)
        if idx + 4 <= len && bytes[idx..idx + 4].eq_ignore_ascii_case(b"url(") {
            let url_call_start = idx;
            let mut inside_start = idx + 4;
            while inside_start < len
                && (bytes[inside_start] == b' '
                    || bytes[inside_start] == b'\t'
                    || bytes[inside_start] == b'\n'
                    || bytes[inside_start] == b'\r')
            {
                inside_start += 1;
            }

            let mut end = inside_start;
            if inside_start < len && (bytes[inside_start] == b'"' || bytes[inside_start] == b'\'') {
                let quote = bytes[inside_start];
                end = inside_start + 1;
                while end < len && bytes[end] != quote {
                    if bytes[end] == b'\\' && end + 1 < len {
                        end += 2;
                    } else {
                        end += 1;
                    }
                }
                if end < len && bytes[end] == quote {
                    end += 1;
                }
                while end < len && bytes[end] != b')' {
                    end += 1;
                }
            } else {
                while end < len && bytes[end] != b')' {
                    end += 1;
                }
            }

            if end < len {
                let raw_arg = &css[inside_start..end].trim();
                let clean_url = unquote(raw_arg);
                if !clean_url.is_empty() {
                    let (rel_l, rel_c) = byte_offset_to_line_col(css, url_call_start);
                    let line = if rel_l == 1 {
                        base_line
                    } else {
                        base_line + rel_l - 1
                    };
                    let col = if rel_l == 1 {
                        base_col + rel_c - 1
                    } else {
                        rel_c
                    };
                    refs.push(ExtractedReference {
                        raw_url: clean_url.to_string(),
                        source_file: source_file.to_path_buf(),
                        line: Some(line),
                        col: Some(col),
                    });
                }
                idx = end + 1;
                continue;
            }
        }

        // Match @import ...
        if idx + 7 <= len && bytes[idx..idx + 7].eq_ignore_ascii_case(b"@import") {
            let import_start = idx;
            idx += 7;
            while idx < len && (bytes[idx] == b' ' || bytes[idx] == b'\t') {
                idx += 1;
            }
            if idx < len && (bytes[idx] == b'"' || bytes[idx] == b'\'') {
                let quote = bytes[idx];
                idx += 1;
                let start_str = idx;
                while idx < len && bytes[idx] != quote && bytes[idx] != b';' && bytes[idx] != b'\n' {
                    idx += 1;
                }
                if idx < len && bytes[idx] == quote {
                    let clean_url = &css[start_str..idx];
                    if !clean_url.is_empty() {
                        let (rel_l, rel_c) = byte_offset_to_line_col(css, import_start);
                        let line = if rel_l == 1 {
                            base_line
                        } else {
                            base_line + rel_l - 1
                        };
                        let col = if rel_l == 1 {
                            base_col + rel_c - 1
                        } else {
                            rel_c
                        };
                        refs.push(ExtractedReference {
                            raw_url: clean_url.to_string(),
                            source_file: source_file.to_path_buf(),
                            line: Some(line),
                            col: Some(col),
                        });
                    }
                    idx += 1;
                    continue;
                }
            }
        }

        idx += 1;
    }

    refs
}

fn unquote(s: &str) -> &str {
    let trimmed = s.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2)
    {
        &trimmed[1..trimmed.len() - 1].trim()
    } else {
        trimmed
    }
}

fn collect_css_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_css_files(&path, files);
            } else if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.eq_ignore_ascii_case("css") {
                        files.push(path);
                    }
                }
            }
        }
    }
}

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

/// Computes a relative path from `from_dir` to `to_file`.
///
/// Both paths are assumed to share a base prefix or be relative to the same root.
pub fn make_relative_path(from_dir: &Path, to_file: &Path) -> PathBuf {
    let from_comps: Vec<_> = from_dir
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(p) => Some(p.to_string_lossy().to_string()),
            _ => None,
        })
        .collect();

    let to_comps: Vec<_> = to_file
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(p) => Some(p.to_string_lossy().to_string()),
            _ => None,
        })
        .collect();

    let mut common = 0;
    while common < from_comps.len() && common < to_comps.len() && from_comps[common] == to_comps[common] {
        common += 1;
    }

    let mut rel = PathBuf::new();
    for _ in common..from_comps.len() {
        rel.push("..");
    }
    for comp in &to_comps[common..] {
        rel.push(comp);
    }

    rel
}

/// Rewrites a single reference URL so that it resolves against the `dist/` layout shape.
///
/// Scheme-bearing, protocol-relative, and pure fragment references are preserved byte-for-byte.
/// Script references ending in `.ts` (such as `scripts/<rel>.ts`) are rewritten to `.js`.
/// Other file references are rewritten to their mapped `dist/` relative paths.
pub fn rewrite_reference(raw_url: &str, source_file: &Path, root: &Path) -> String {
    let kind = classify_reference(raw_url);
    match kind {
        ReferenceKind::Scheme(_) | ReferenceKind::ProtocolRelative | ReferenceKind::Fragment => {
            return raw_url.to_string();
        }
        ReferenceKind::FilePath(_) => {}
    }

    let (path_only, suffix) = if let Some(idx) = raw_url.find(|c| c == '?' || c == '#') {
        (&raw_url[..idx], &raw_url[idx..])
    } else {
        (raw_url, "")
    };

    let trimmed = path_only.trim();
    if trimmed.is_empty() {
        return raw_url.to_string();
    }

    let dist_source = match map_source_to_dist(source_file) {
        Some(p) => p,
        None => return raw_url.to_string(),
    };
    let dist_from_dir = match dist_source.parent() {
        Some(p) => p,
        None => Path::new("dist"),
    };

    let normalized = trimmed.replace('\\', "/");
    let dist_target = if normalized == "styles/tokens.css" || normalized == "tokens/tokens.json" {
        PathBuf::from("dist/styles/tokens.css")
    } else {
        let resolved_rel = resolve_reference_target(&normalized, source_file);
        let parent_rel = source_file
            .parent()
            .unwrap_or(Path::new(""))
            .join(&resolved_rel);

        let candidate = if root.join(&resolved_rel).is_file() {
            resolved_rel
        } else if root.join(&parent_rel).is_file() {
            parent_rel
        } else if resolved_rel.extension().map(|e| e.eq_ignore_ascii_case("ts")).unwrap_or(false)
            || normalized.ends_with(".ts")
        {
            if resolved_rel.starts_with("scripts") {
                resolved_rel
            } else if parent_rel.starts_with("scripts") {
                parent_rel
            } else {
                Path::new("scripts").join(&resolved_rel)
            }
        } else {
            resolved_rel
        };

        match map_source_to_dist(&candidate) {
            Some(dest) => dest,
            None => {
                if normalized.ends_with(".ts") {
                    let mut js = PathBuf::from(&normalized);
                    js.set_extension("js");
                    Path::new("dist").join(js)
                } else {
                    return raw_url.to_string();
                }
            }
        }
    };

    let rel_path = make_relative_path(dist_from_dir, &dist_target);
    let rel_str = rel_path.to_string_lossy().replace('\\', "/");

    let final_path = if trimmed.starts_with("./") && !rel_str.starts_with("../") {
        format!("./{rel_str}")
    } else if trimmed.starts_with('/') {
        if let Ok(after_dist) = dist_target.strip_prefix("dist") {
            format!("/{}", after_dist.to_string_lossy().replace('\\', "/"))
        } else {
            format!("/{rel_str}")
        }
    } else {
        rel_str
    };

    format!("{final_path}{suffix}")
}

/// Rewrites static `url(...)` and `@import` references in CSS content.
pub fn rewrite_css_references(css: &str, source_file: &Path, root: &Path) -> String {
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();
    let bytes = css.as_bytes();
    let len = bytes.len();
    let mut idx = 0;

    while idx < len {
        // Skip CSS comments /* ... */
        if idx + 2 <= len && bytes[idx..idx + 2] == *b"/*" {
            idx += 2;
            while idx + 2 <= len && bytes[idx..idx + 2] != *b"*/" {
                idx += 1;
            }
            if idx + 2 <= len {
                idx += 2;
            } else {
                idx = len;
            }
            continue;
        }

        // Match url(...)
        if idx + 4 <= len && bytes[idx..idx + 4].eq_ignore_ascii_case(b"url(") {
            let mut inside_start = idx + 4;
            while inside_start < len
                && (bytes[inside_start] == b' '
                    || bytes[inside_start] == b'\t'
                    || bytes[inside_start] == b'\n'
                    || bytes[inside_start] == b'\r')
            {
                inside_start += 1;
            }

            let mut end = inside_start;
            let (url_start, url_end) = if inside_start < len
                && (bytes[inside_start] == b'"' || bytes[inside_start] == b'\'')
            {
                let quote = bytes[inside_start];
                let u_start = inside_start + 1;
                end = inside_start + 1;
                while end < len && bytes[end] != quote {
                    if bytes[end] == b'\\' && end + 1 < len {
                        end += 2;
                    } else {
                        end += 1;
                    }
                }
                let u_end = end;
                if end < len && bytes[end] == quote {
                    end += 1;
                }
                while end < len && bytes[end] != b')' {
                    end += 1;
                }
                (u_start, u_end)
            } else {
                let u_start = inside_start;
                while end < len
                    && bytes[end] != b')'
                    && !bytes[end].is_ascii_whitespace()
                {
                    end += 1;
                }
                let u_end = end;
                while end < len && bytes[end] != b')' {
                    end += 1;
                }
                (u_start, u_end)
            };

            if end < len {
                if url_start < url_end {
                    let clean_url = &css[url_start..url_end];
                    let new_url = rewrite_reference(clean_url, source_file, root);
                    if new_url != clean_url {
                        replacements.push((url_start, url_end, new_url));
                    }
                }
                idx = end + 1;
                continue;
            }
        }

        // Match @import ...
        if idx + 7 <= len && bytes[idx..idx + 7].eq_ignore_ascii_case(b"@import") {
            idx += 7;
            while idx < len && (bytes[idx] == b' ' || bytes[idx] == b'\t') {
                idx += 1;
            }
            if idx < len && (bytes[idx] == b'"' || bytes[idx] == b'\'') {
                let quote = bytes[idx];
                idx += 1;
                let start_str = idx;
                while idx < len && bytes[idx] != quote && bytes[idx] != b';' && bytes[idx] != b'\n' {
                    idx += 1;
                }
                let end_str = idx;
                if idx < len && bytes[idx] == quote {
                    let clean_url = &css[start_str..end_str];
                    let new_url = rewrite_reference(clean_url, source_file, root);
                    if new_url != clean_url {
                        replacements.push((start_str, end_str, new_url));
                    }
                    idx += 1;
                    continue;
                }
            }
        }

        idx += 1;
    }

    if replacements.is_empty() {
        return css.to_string();
    }

    replacements.sort_by_key(|r| r.0);
    let mut rewritten = String::with_capacity(css.len());
    let mut last = 0;
    for (start, end, replacement) in replacements {
        if start >= last {
            rewritten.push_str(&css[last..start]);
            rewritten.push_str(&replacement);
            last = end;
        }
    }
    rewritten.push_str(&css[last..]);
    rewritten
}

fn find_attribute_value_span(tag_text: &str, target_attr: &[u8]) -> Option<(usize, usize)> {
    let bytes = tag_text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    if i < len && bytes[i] == b'<' {
        i += 1;
    }
    if i < len && bytes[i] == b'/' {
        i += 1;
    }

    while i < len && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' && bytes[i] != b'/' {
        i += 1;
    }

    while i < len {
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        if i >= len || bytes[i] == b'>' || bytes[i] == b'/' {
            break;
        }

        let attr_start = i;
        while i < len
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'='
            && bytes[i] != b'>'
            && bytes[i] != b'/'
        {
            i += 1;
        }
        let attr_name = &bytes[attr_start..i];

        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        if i < len && bytes[i] == b'=' {
            i += 1; // skip '='

            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }

            if i < len && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let quote = bytes[i];
                let val_start = i + 1;
                i += 1;
                while i < len && bytes[i] != quote {
                    i += 1;
                }
                let val_end = i;
                if i < len && bytes[i] == quote {
                    i += 1;
                }
                if attr_name.eq_ignore_ascii_case(target_attr) {
                    return Some((val_start, val_end));
                }
            } else {
                let val_start = i;
                while i < len
                    && !bytes[i].is_ascii_whitespace()
                    && bytes[i] != b'>'
                    && bytes[i] != b'/'
                {
                    i += 1;
                }
                let val_end = i;
                if attr_name.eq_ignore_ascii_case(target_attr) {
                    return Some((val_start, val_end));
                }
            }
        }
    }

    None
}

/// Rewrites static references in HTML content.
pub fn rewrite_html_references(content: &str, source_file: &Path, root: &Path) -> String {
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();
    let emitter = html5gum::DefaultEmitter::<usize>::new_with_span();
    let tokenizer = html5gum::Tokenizer::new_with_emitter(content, emitter);

    let mut in_style_tag = false;
    let mut style_start = 0;

    for token in tokenizer {
        match token {
            Ok(html5gum::Token::StartTag(tag)) => {
                let name = tag.name.as_slice();
                if name.eq_ignore_ascii_case(b"style") {
                    in_style_tag = true;
                    style_start = tag.span.end;
                }

                let tag_text = &content[tag.span.start..tag.span.end];
                for (attr_name, _) in &tag.attributes {
                    let is_ref = attr_name.eq_ignore_ascii_case(b"href")
                        || attr_name.eq_ignore_ascii_case(b"src")
                        || attr_name.eq_ignore_ascii_case(b"poster");

                    if is_ref {
                        if let Some((v_start, v_end)) =
                            find_attribute_value_span(tag_text, attr_name.as_slice())
                        {
                            let abs_start = tag.span.start + v_start;
                            let abs_end = tag.span.start + v_end;
                            let raw_val = &content[abs_start..abs_end];
                            let new_val = rewrite_reference(raw_val, source_file, root);
                            if new_val != raw_val {
                                replacements.push((abs_start, abs_end, new_val));
                            }
                        }
                    }

                    if attr_name.eq_ignore_ascii_case(b"style") {
                        if let Some((v_start, v_end)) =
                            find_attribute_value_span(tag_text, b"style")
                        {
                            let abs_start = tag.span.start + v_start;
                            let abs_end = tag.span.start + v_end;
                            let raw_css = &content[abs_start..abs_end];
                            let new_css = rewrite_css_references(raw_css, source_file, root);
                            if new_css != raw_css {
                                replacements.push((abs_start, abs_end, new_css));
                            }
                        }
                    }
                }
            }
            Ok(html5gum::Token::EndTag(tag)) => {
                let name = tag.name.as_slice();
                if name.eq_ignore_ascii_case(b"style") && in_style_tag {
                    in_style_tag = false;
                    let style_end = tag.span.start;
                    if style_end >= style_start && style_end <= content.len() {
                        let raw_css = &content[style_start..style_end];
                        let new_css = rewrite_css_references(raw_css, source_file, root);
                        if new_css != raw_css {
                            replacements.push((style_start, style_end, new_css));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if replacements.is_empty() {
        return content.to_string();
    }

    replacements.sort_by_key(|r| r.0);
    let mut rewritten = String::with_capacity(content.len());
    let mut last = 0;
    for (start, end, replacement) in replacements {
        if start >= last {
            rewritten.push_str(&content[last..start]);
            rewritten.push_str(&replacement);
            last = end;
        }
    }
    rewritten.push_str(&content[last..]);
    rewritten
}

fn collect_asset_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_asset_files(&path, files);
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
}

/// Copies the reachable set of assets and stylesheets into `output_dir` using the source-to-dist mapping,
/// writes the expanded HTML pages into `output_dir`, and rewrites references inside HTML and CSS
/// so they resolve against the `dist/` shape.
///
/// References classified as scheme-bearing (`http:`, `data:`, etc.), protocol-relative (`//`),
/// or fragment-only (`#...`) are preserved byte-for-byte.
/// Script specifiers referencing `scripts/<rel>.ts` are rewritten to `scripts/<rel>.js`.
///
/// Emits `wda.build.unreferenced-asset` as a Warning for each non-secret file under `assets/`
/// that is not in `reachable_set`.
pub fn copy_reachable_set(
    root: &Path,
    output_dir: &Path,
    expanded_pages: &BTreeMap<PathBuf, String>,
    reachable_set: &std::collections::BTreeSet<PathBuf>,
) -> Result<ValidationReport, std::io::Error> {
    let mut report = ValidationReport::new();

    // 1. Detect unreferenced assets under assets/
    let assets_dir = root.join("assets");
    if assets_dir.is_dir() {
        let mut all_assets = Vec::new();
        collect_asset_files(&assets_dir, &mut all_assets);
        all_assets.sort();

        for full_path in all_assets {
            if let Ok(rel_path) = full_path.strip_prefix(root) {
                if !is_secret_path(rel_path) && !reachable_set.contains(rel_path) {
                    let rel_str = rel_path.to_string_lossy().replace('\\', "/");
                    report.add(Diagnostic::new(
                        codes::BUILD_UNREFERENCED_ASSET,
                        Severity::Warning,
                        Some(Location::path_only(&rel_str)),
                        format!(
                            "Asset '{rel_str}' is under 'assets/' but not referenced by any page or stylesheet and will not be copied into dist/"
                        ),
                        format!("Reference this asset from a page or stylesheet, or remove it from assets/"),
                    ));
                }
            }
        }
    }

    report.sort();

    // 2. Write expanded HTML pages with rewritten references
    for (page_rel, page_content) in expanded_pages {
        if let Some(dist_path) = map_source_to_dist(page_rel) {
            let out_rel = dist_path.strip_prefix("dist").unwrap_or(&dist_path);
            let out_full = output_dir.join(out_rel);
            if let Some(parent) = out_full.parent() {
                fs::create_dir_all(parent)?;
            }
            let rewritten_html = rewrite_html_references(page_content, page_rel, root);
            fs::write(&out_full, rewritten_html.as_bytes())?;
        }
    }

    // 3. Copy reachable set (assets and stylesheets)
    for src_rel in reachable_set {
        if let Some(dist_path) = map_source_to_dist(src_rel) {
            let out_rel = dist_path.strip_prefix("dist").unwrap_or(&dist_path);
            let out_full = output_dir.join(out_rel);
            if let Some(parent) = out_full.parent() {
                fs::create_dir_all(parent)?;
            }

            let is_css = src_rel
                .extension()
                .map(|e| e.eq_ignore_ascii_case("css"))
                .unwrap_or(false);

            if is_css {
                let css_content = fs::read_to_string(root.join(src_rel))?;
                let rewritten_css = rewrite_css_references(&css_content, src_rel, root);
                fs::write(&out_full, rewritten_css.as_bytes())?;
            } else {
                fs::copy(root.join(src_rel), &out_full)?;
            }
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_page_fixture_maps_to_dist_index_and_dist_about_without_nested_index() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_layout_two_page_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let pages_dir = temp_dir.join("pages");
        fs::create_dir_all(&pages_dir).unwrap();

        fs::write(pages_dir.join("index.html"), "<!DOCTYPE html><title>Home</title>").unwrap();
        fs::write(pages_dir.join("about.html"), "<!DOCTYPE html><title>About</title>").unwrap();

        let mapping = discover_and_map_pages(&temp_dir).expect("mapping must succeed");

        assert_eq!(mapping.len(), 2);
        assert_eq!(
            mapping.get(Path::new("pages/index.html")),
            Some(&PathBuf::from("dist/index.html"))
        );
        assert_eq!(
            mapping.get(Path::new("pages/about.html")),
            Some(&PathBuf::from("dist/about.html"))
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_components_docs_and_dist_are_never_in_page_set() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_layout_excluded_dirs_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);

        let pages_dir = temp_dir.join("pages");
        let components_dir = temp_dir.join("components");
        let docs_dir = temp_dir.join("docs");
        let dist_dir = temp_dir.join("dist");

        fs::create_dir_all(&pages_dir).unwrap();
        fs::create_dir_all(&components_dir).unwrap();
        fs::create_dir_all(&docs_dir).unwrap();
        fs::create_dir_all(&dist_dir).unwrap();

        fs::write(pages_dir.join("index.html"), "<html></html>").unwrap();
        fs::write(components_dir.join("nav.html"), "<nav></nav>").unwrap();
        fs::write(components_dir.join("header.html"), "<header></header>").unwrap();
        fs::write(docs_dir.join("architecture.html"), "<article></article>").unwrap();
        fs::write(dist_dir.join("stale.html"), "<main></main>").unwrap();

        let pages = discover_pages(&temp_dir);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0], PathBuf::from("pages/index.html"));

        let page_strs: Vec<String> = pages
            .iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();
        assert!(!page_strs.contains(&"components/nav.html".to_string()));
        assert!(!page_strs.contains(&"components/header.html".to_string()));
        assert!(!page_strs.contains(&"docs/architecture.html".to_string()));
        assert!(!page_strs.contains(&"dist/stale.html".to_string()));

        let mapping = discover_and_map_pages(&temp_dir).expect("mapping must succeed");
        assert_eq!(mapping.len(), 1);
        assert_eq!(
            mapping.get(Path::new("pages/index.html")),
            Some(&PathBuf::from("dist/index.html"))
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_colliding_source_paths_yield_output_path_collision_error() {
        let sources = [
            Path::new("pages/styles/tokens.css.html"),
            Path::new("styles/tokens.css"),
            Path::new("tokens/tokens.json"),
        ];

        let res = compute_source_mapping(sources.iter().copied());
        assert!(res.is_err(), "colliding sources must yield error");
        let report = res.unwrap_err();
        assert!(report.has_error());

        let error_codes: Vec<&str> = report.diagnostics.iter().map(|d| d.code).collect();
        assert!(error_codes.contains(&codes::BUILD_OUTPUT_PATH_COLLISION));

        let diag = report
            .diagnostics
            .iter()
            .find(|d| d.code == codes::BUILD_OUTPUT_PATH_COLLISION)
            .unwrap();
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("dist/styles/tokens.css"));
    }

    #[test]
    fn test_fixed_mapping_table_cases() {
        assert_eq!(
            map_source_to_dist(Path::new("pages/index.html")),
            Some(PathBuf::from("dist/index.html"))
        );
        assert_eq!(
            map_source_to_dist(Path::new("pages/blog/post.html")),
            Some(PathBuf::from("dist/blog/post.html"))
        );
        assert_eq!(
            map_source_to_dist(Path::new("styles/main.css")),
            Some(PathBuf::from("dist/styles/main.css"))
        );
        assert_eq!(
            map_source_to_dist(Path::new("scripts/main.ts")),
            Some(PathBuf::from("dist/scripts/main.js"))
        );
        assert_eq!(
            map_source_to_dist(Path::new("assets/images/hero.png")),
            Some(PathBuf::from("dist/assets/images/hero.png"))
        );
        assert_eq!(
            map_source_to_dist(Path::new("tokens/tokens.json")),
            Some(PathBuf::from("dist/styles/tokens.css"))
        );

        // Never copied / excluded
        assert_eq!(map_source_to_dist(Path::new("components/header.html")), None);
        assert_eq!(map_source_to_dist(Path::new("wda.json")), None);
        assert_eq!(map_source_to_dist(Path::new("README.md")), None);
        assert_eq!(map_source_to_dist(Path::new("docs/architecture.md")), None);
        assert_eq!(map_source_to_dist(Path::new(".gitignore")), None);
        assert_eq!(map_source_to_dist(Path::new(".env")), None);
        assert_eq!(map_source_to_dist(Path::new(".env.local")), None);
        assert_eq!(map_source_to_dist(Path::new("assets/.DS_Store")), None);
    }

    #[test]
    fn test_reference_classification_scheme_protocol_relative_fragment_and_filepath() {
        assert_eq!(classify_reference("#overview"), ReferenceKind::Fragment);
        assert_eq!(classify_reference("#"), ReferenceKind::Fragment);
        assert_eq!(
            classify_reference("//cdn.example.com/lib.js"),
            ReferenceKind::ProtocolRelative
        );
        assert_eq!(
            classify_reference("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg'%3E%3C/svg%3E"),
            ReferenceKind::Scheme("data".to_string())
        );
        assert_eq!(
            classify_reference("https://example.com/style.css"),
            ReferenceKind::Scheme("https".to_string())
        );
        assert_eq!(
            classify_reference("http://example.com"),
            ReferenceKind::Scheme("http".to_string())
        );
        assert_eq!(
            classify_reference("mailto:user@example.com"),
            ReferenceKind::Scheme("mailto".to_string())
        );
        assert_eq!(
            classify_reference("tel:+1234567890"),
            ReferenceKind::Scheme("tel".to_string())
        );
        assert_eq!(
            classify_reference("styles/tokens.css"),
            ReferenceKind::FilePath("styles/tokens.css".to_string())
        );
        assert_eq!(
            classify_reference("assets/images/hero.png"),
            ReferenceKind::FilePath("assets/images/hero.png".to_string())
        );
        assert_eq!(
            classify_reference(".env"),
            ReferenceKind::FilePath(".env".to_string())
        );
    }

    #[test]
    fn test_secret_path_detection_cases() {
        assert!(is_secret_path(Path::new(".env")));
        assert!(is_secret_path(Path::new(".env.local")));
        assert!(is_secret_path(Path::new(".env.production")));
        assert!(is_secret_path(Path::new(".git/config")));
        assert!(is_secret_path(Path::new(".DS_Store")));
        assert!(is_secret_path(Path::new("sub/.env")));
        assert!(is_secret_path(Path::new("assets/.secret")));

        assert!(!is_secret_path(Path::new("styles/tokens.css")));
        assert!(!is_secret_path(Path::new("assets/hero.png")));
        assert!(!is_secret_path(Path::new("pages/index.html")));
    }

    #[test]
    fn test_reachability_probe_covering_all_classification_forms_and_copy_set() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_reachability_probe_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);

        let pages_dir = temp_dir.join("pages");
        let assets_dir = temp_dir.join("assets");
        let styles_dir = temp_dir.join("styles");

        fs::create_dir_all(&pages_dir).unwrap();
        fs::create_dir_all(&assets_dir).unwrap();
        fs::create_dir_all(&styles_dir).unwrap();

        // 1. Existing assets: one will be referenced, one will be unreferenced
        let hero_img = assets_dir.join("hero.png");
        let unused_img = assets_dir.join("unused.png");
        let inline_img = assets_dir.join("inline.png");
        fs::write(&hero_img, b"hero-png-content").unwrap();
        fs::write(&unused_img, b"unused-png-content").unwrap();
        fs::write(&inline_img, b"inline-png-content").unwrap();

        // 2. Unreferenced .env file in project root
        let env_file = temp_dir.join(".env");
        fs::write(&env_file, b"SECRET_KEY=12345").unwrap();

        // 3. HTML with:
        // - pure fragment: #overview, href="#"
        // - scheme: data:image/svg+xml,...
        // - protocol-relative: //cdn.example.com/lib.js
        // - known virtual asset: styles/tokens.css
        // - referenced image: assets/hero.png
        // - inline <style> url reference: assets/inline.png
        // - CSS comment with non-existent url: skipped
        let html_content = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <link rel="stylesheet" href="styles/tokens.css">
  <style>
    /* background: url('assets/commented_out.png'); */
    .banner {
      background-image: url('assets/inline.png');
    }
  </style>
  <script src="//cdn.example.com/lib.js"></script>
</head>
<body>
  <a href="#" class="brand">
    <img src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg'%3E%3C/svg%3E" alt="Mark">
  </a>
  <nav>
    <a href="#overview">Overview</a>
  </nav>
  <img src="assets/hero.png" alt="Hero">
</body>
</html>"##;

        let mut expanded_pages = BTreeMap::new();
        expanded_pages.insert(PathBuf::from("pages/index.html"), html_content.to_string());

        let (copy_set, report) = compute_reachability(&temp_dir, &expanded_pages);

        // Assertions:
        // - Zero diagnostics (all references are valid, schemes/fragments skipped, virtual asset exempt, unreferenced .env ignored)
        assert_eq!(report.diagnostics.len(), 0, "report should have no diagnostics: {:?}", report.diagnostics);

        // - Referenced image is in copy set
        assert!(copy_set.contains(Path::new("assets/hero.png")), "referenced hero.png must be in copy set");

        // - Unreferenced image is NOT in copy set
        assert!(!copy_set.contains(Path::new("assets/unused.png")), "unreferenced unused.png must not be in copy set");

        // - Image referenced only from inline <style> url() is in copy set
        assert!(copy_set.contains(Path::new("assets/inline.png")), "inline style url image must be in copy set");

        // - Virtual asset styles/tokens.css has no copy entry
        assert!(!copy_set.contains(Path::new("styles/tokens.css")), "virtual asset styles/tokens.css must not be copied");

        // - Unreferenced .env has no copy entry
        assert!(!copy_set.contains(Path::new(".env")), "unreferenced .env must not be in copy set");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_reachability_missing_file_yields_reference_unresolvable_error() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_reachability_missing_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let pages_dir = temp_dir.join("pages");
        fs::create_dir_all(&pages_dir).unwrap();

        let html_content = r#"<!DOCTYPE html>
<html lang="en">
<body>
  <a href="missing.html">Missing Page</a>
  <img src="assets/missing.png" alt="Missing Image">
</body>
</html>"#;

        let mut expanded_pages = BTreeMap::new();
        expanded_pages.insert(PathBuf::from("pages/index.html"), html_content.to_string());

        let (copy_set, report) = compute_reachability(&temp_dir, &expanded_pages);

        assert!(copy_set.is_empty(), "copy set must be empty when references are unresolvable");
        assert!(report.has_error(), "report must contain errors");

        let unresolvable_diags: Vec<&Diagnostic> = report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::BUILD_REFERENCE_UNRESOLVABLE)
            .collect();
        assert_eq!(unresolvable_diags.len(), 2);
        assert_eq!(unresolvable_diags[0].severity, Severity::Error);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_reachability_reference_to_secret_path_yields_secret_path_referenced_error() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_reachability_secret_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let pages_dir = temp_dir.join("pages");
        fs::create_dir_all(&pages_dir).unwrap();

        let env_file = temp_dir.join(".env");
        fs::write(&env_file, b"SECRET_TOKEN=xyz").unwrap();

        let html_content = r#"<!DOCTYPE html>
<html lang="en">
<body>
  <a href=".env">Download Secrets</a>
</body>
</html>"#;

        let mut expanded_pages = BTreeMap::new();
        expanded_pages.insert(PathBuf::from("pages/index.html"), html_content.to_string());

        let (copy_set, report) = compute_reachability(&temp_dir, &expanded_pages);

        assert!(!copy_set.contains(Path::new(".env")), ".env must never be in copy set");
        assert!(report.has_error(), "referencing .env must produce an error");

        let secret_diags: Vec<&Diagnostic> = report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::BUILD_SECRET_PATH_REFERENCED)
            .collect();
        assert_eq!(secret_diags.len(), 1);
        assert_eq!(secret_diags[0].severity, Severity::Error);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_reachability_css_stylesheet_references() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_reachability_css_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let pages_dir = temp_dir.join("pages");
        let styles_dir = temp_dir.join("styles");
        let assets_dir = temp_dir.join("assets");

        fs::create_dir_all(&pages_dir).unwrap();
        fs::create_dir_all(&styles_dir).unwrap();
        fs::create_dir_all(&assets_dir).unwrap();

        let bg_img = assets_dir.join("pattern.svg");
        fs::write(&bg_img, b"<svg></svg>").unwrap();

        let css_file = styles_dir.join("main.css");
        fs::write(
            &css_file,
            r#"
            @import url("styles/tokens.css");
            body {
                background: url("assets/pattern.svg");
            }
            "#,
        )
        .unwrap();

        let html_content = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <link rel="stylesheet" href="styles/main.css">
</head>
<body>
</body>
</html>"#;

        let mut expanded_pages = BTreeMap::new();
        expanded_pages.insert(PathBuf::from("pages/index.html"), html_content.to_string());

        let (copy_set, report) = compute_reachability(&temp_dir, &expanded_pages);

        assert_eq!(report.diagnostics.len(), 0, "no diagnostics expected: {:?}", report.diagnostics);
        assert!(copy_set.contains(Path::new("styles/main.css")));
        assert!(copy_set.contains(Path::new("assets/pattern.svg")));
        assert!(!copy_set.contains(Path::new("styles/tokens.css")));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_copy_reachable_set_and_rewrite_paths() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_copy_rewrite_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);

        let pages_dir = temp_dir.join("pages");
        let assets_dir = temp_dir.join("assets");
        let styles_dir = temp_dir.join("styles");
        let scripts_dir = temp_dir.join("scripts");
        let dist_out = temp_dir.join("output_dist");

        fs::create_dir_all(&pages_dir).unwrap();
        fs::create_dir_all(&assets_dir).unwrap();
        fs::create_dir_all(&styles_dir).unwrap();
        fs::create_dir_all(&scripts_dir).unwrap();

        // 1. Assets: one referenced (logo.png), one unreferenced (unused.jpg)
        let logo_img = assets_dir.join("logo.png");
        let unused_img = assets_dir.join("unused.jpg");
        fs::write(&logo_img, b"logo-bytes").unwrap();
        fs::write(&unused_img, b"unused-bytes").unwrap();

        // 2. Scripts: scripts/main.ts
        let script_file = scripts_dir.join("main.ts");
        fs::write(&script_file, b"console.log('hello');").unwrap();

        // 3. Styles: styles/main.css with asset reference
        let style_file = styles_dir.join("main.css");
        fs::write(
            &style_file,
            b"body { background: url('../assets/logo.png'); }",
        )
        .unwrap();

        // 4. HTML page with:
        // - scripts/main.ts reference (must rewrite to scripts/main.js)
        // - styles/main.css reference (must rewrite to styles/main.css)
        // - assets/logo.png reference (must rewrite to assets/logo.png)
        // - #overview link (must remain untouched byte-for-byte)
        // - data:image/svg+xml image (must remain untouched byte-for-byte)
        let page_source = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <link rel="stylesheet" href="styles/main.css">
  <script src="scripts/main.ts"></script>
</head>
<body>
  <a href="#overview">Overview</a>
  <img src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg'%3E%3C/svg%3E" alt="Vector">
  <img src="assets/logo.png" alt="Logo">
</body>
</html>"##;

        let mut expanded_pages = BTreeMap::new();
        expanded_pages.insert(PathBuf::from("pages/index.html"), page_source.to_string());

        let mut reachable_set = std::collections::BTreeSet::new();
        reachable_set.insert(PathBuf::from("assets/logo.png"));
        reachable_set.insert(PathBuf::from("styles/main.css"));

        let report = copy_reachable_set(
            &temp_dir,
            &dist_out,
            &expanded_pages,
            &reachable_set,
        )
        .expect("copy_reachable_set must succeed");

        // Verification 1: Report contains exactly one Warning for unreferenced asset
        let warnings: Vec<&Diagnostic> = report
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::BUILD_UNREFERENCED_ASSET)
            .collect();
        assert_eq!(
            warnings.len(),
            1,
            "expected exactly one unreferenced asset warning: {:?}",
            report.diagnostics
        );
        assert_eq!(warnings[0].severity, Severity::Warning);
        let loc = warnings[0].location.as_ref().unwrap();
        assert_eq!(loc.path, std::path::Path::new("assets/unused.jpg"));

        // Verification 2: Output files written
        // - dist/index.html exists
        // - dist/assets/logo.png exists
        // - dist/assets/unused.jpg does NOT exist
        // - dist/styles/main.css exists
        let emitted_html_path = dist_out.join("index.html");
        let emitted_logo_path = dist_out.join("assets/logo.png");
        let emitted_unused_path = dist_out.join("assets/unused.jpg");
        let emitted_css_path = dist_out.join("styles/main.css");

        assert!(emitted_html_path.is_file(), "emitted HTML page must exist");
        assert!(emitted_logo_path.is_file(), "referenced logo.png must exist in dist/assets/");
        assert!(
            !emitted_unused_path.exists(),
            "unreferenced unused.jpg must not exist in output"
        );
        assert!(emitted_css_path.is_file(), "referenced main.css must exist in dist/styles/");

        // Verification 3: Content checks in emitted HTML
        let emitted_html = fs::read_to_string(&emitted_html_path).unwrap();

        // - scripts/main.ts rewritten to scripts/main.js
        assert!(
            emitted_html.contains(r#"<script src="scripts/main.js"></script>"#),
            "script specifier scripts/main.ts must be rewritten to scripts/main.js; got: {emitted_html}"
        );
        assert!(
            !emitted_html.contains("scripts/main.ts"),
            "scripts/main.ts must not appear in emitted HTML"
        );

        // - #overview unchanged byte-for-byte
        assert!(
            emitted_html.contains(r##"<a href="#overview">Overview</a>"##),
            "#overview link must be unchanged byte-for-byte"
        );

        // - data:image/svg+xml unchanged byte-for-byte
        assert!(
            emitted_html.contains(r#"<img src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg'%3E%3C/svg%3E" alt="Vector">"#),
            "data: URI must be unchanged byte-for-byte"
        );

        // - assets/logo.png resolves against dist/
        assert!(
            emitted_html.contains(r#"<img src="assets/logo.png" alt="Logo">"#),
            "assets/logo.png must be present"
        );

        // Verification 4: Content check in emitted CSS
        let emitted_css = fs::read_to_string(&emitted_css_path).unwrap();
        assert!(
            emitted_css.contains("url('../assets/logo.png')"),
            "CSS relative reference should resolve in dist/: {emitted_css}"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }
}

