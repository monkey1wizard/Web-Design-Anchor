use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Error,
    Warning,
}

/// Version compatibility status for project format (§2.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VersionStatus {
    Current,
    SupportedLegacy,
    DeprecatedLegacy,
    Unsupported,
}

impl VersionStatus {
    /// Maps the version status to its corresponding diagnostic severity, if any.
    ///
    /// - `Current` -> `None` (OK)
    /// - `SupportedLegacy` -> `Some(Severity::Warning)`
    /// - `DeprecatedLegacy` -> `Some(Severity::Warning)`
    /// - `Unsupported` -> `Some(Severity::Error)`
    pub fn severity_of(self) -> Option<Severity> {
        match self {
            VersionStatus::Current => None,
            VersionStatus::SupportedLegacy => Some(Severity::Warning),
            VersionStatus::DeprecatedLegacy => Some(Severity::Warning),
            VersionStatus::Unsupported => Some(Severity::Error),
        }
    }
}

/// Source location for a diagnostic.
///
/// Contains a required project-relative path and optional line and column.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Location {
    pub path: PathBuf,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl Location {
    pub fn new(path: impl Into<PathBuf>, line: Option<usize>, column: Option<usize>) -> Self {
        Self {
            path: path.into(),
            line,
            column,
        }
    }

    pub fn path_only(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            line: None,
            column: None,
        }
    }
}

/// A diagnostic finding reported by the validation engine.
///
/// Note: Diagnostic derives no serialization traits to avoid exposing an unapproved JSON surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub location: Option<Location>,
    pub message: String,
    pub next_action: String,
    pub ordinal: usize,
}

impl Diagnostic {
    pub fn new(
        code: &'static str,
        severity: Severity,
        location: Option<Location>,
        message: impl Into<String>,
        next_action: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity,
            location,
            message: message.into(),
            next_action: next_action.into(),
            ordinal: 0,
        }
    }

    pub fn with_ordinal(mut self, ordinal: usize) -> Self {
        self.ordinal = ordinal;
        self
    }
}

/// Escapes newlines (`\n`, `\r`), tabs (`\t`), and control characters (e.g. `\u{7}`)
/// into safe printable escape sequences.
pub fn escape_control_chars(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

/// Normalizes a path to project-relative form with `/` separators,
/// stripping host root/drive prefixes and escaping control characters.
pub fn normalize_path(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    let p = Path::new(&s);

    let has_prefix_or_root = p
        .components()
        .any(|c| matches!(c, Component::Prefix(_) | Component::RootDir));
    let mut parts = Vec::new();

    for comp in p.components() {
        match comp {
            Component::Prefix(_) | Component::RootDir => {}
            Component::CurDir => {}
            Component::ParentDir => parts.push(".."),
            Component::Normal(c) => {
                parts.push(c.to_str().unwrap_or(""));
            }
        }
    }

    let starts_with_drive = if let Some(first) = parts.first() {
        first.len() == 2
            && first.as_bytes()[0].is_ascii_alphabetic()
            && first.as_bytes()[1] == b':'
    } else {
        false
    };

    if starts_with_drive {
        parts.remove(0);
    }

    let is_absolute = path.is_absolute() || has_prefix_or_root || starts_with_drive;

    if is_absolute {
        if let Some(last) = parts.pop() {
            parts = vec![last];
        }
    }

    let result = parts.join("/");
    escape_control_chars(&result)
}

fn normalized_path_str(location: Option<&Location>) -> String {
    match location {
        None => String::new(),
        Some(loc) => normalize_path(&loc.path),
    }
}

impl Ord for Diagnostic {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_path = normalized_path_str(self.location.as_ref());
        let other_path = normalized_path_str(other.location.as_ref());

        let self_line = self.location.as_ref().and_then(|l| l.line);
        let other_line = other.location.as_ref().and_then(|l| l.line);

        let self_col = self.location.as_ref().and_then(|l| l.column);
        let other_col = other.location.as_ref().and_then(|l| l.column);

        self_path
            .cmp(&other_path)
            .then_with(|| self_line.cmp(&other_line))
            .then_with(|| self_col.cmp(&other_col))
            .then_with(|| self.code.cmp(other.code))
            .then_with(|| self.ordinal.cmp(&other.ordinal))
    }
}

impl PartialOrd for Diagnostic {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Validation report containing all collected diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationReport {
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn add(&mut self, mut diagnostic: Diagnostic) {
        if diagnostic.ordinal == 0 && !self.diagnostics.is_empty() {
            diagnostic.ordinal = self.diagnostics.len();
        }
        self.diagnostics.push(diagnostic);
    }

    /// Returns `true` if the report contains any diagnostic with `Severity::Error`.
    pub fn has_error(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }

    /// Sorts diagnostics in-place according to the diagnostic total order.
    pub fn sort(&mut self) {
        self.diagnostics.sort();
    }
}

/// Represents an internal tool failure (e.g. unreadable file, schema compilation error).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolFault {
    pub message: String,
    pub path: Option<PathBuf>,
}

impl ToolFault {
    pub fn new(message: impl Into<String>, path: Option<impl Into<PathBuf>>) -> Self {
        Self {
            message: message.into(),
            path: path.map(|p| p.into()),
        }
    }
}

impl std::fmt::Display for ToolFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref path) = self.path {
            write!(f, "{}: {}", path.display(), self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for ToolFault {}

/// Renders a `Diagnostic` as a single-line string with injection hardening.
pub fn render(diagnostic: &Diagnostic) -> String {
    let severity_str = match diagnostic.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    let code_str = escape_control_chars(diagnostic.code);
    let message_str = escape_control_chars(&diagnostic.message);
    let action_str = escape_control_chars(&diagnostic.next_action);

    let loc_str = match &diagnostic.location {
        Some(loc) => {
            let path_str = normalize_path(&loc.path);
            match (loc.line, loc.column) {
                (Some(l), Some(c)) => format!("{path_str}:{l}:{c}"),
                (Some(l), None) => format!("{path_str}:{l}"),
                (None, None) => path_str,
                (None, Some(c)) => format!("{path_str}::{c}"),
            }
        }
        None => String::new(),
    };

    if loc_str.is_empty() {
        format!("{severity_str}: [{code_str}] {message_str} ({action_str})")
    } else {
        format!("{severity_str}: [{code_str}] {loc_str}: {message_str} ({action_str})")
    }
}

/// Renders a `ToolFault` as a single-line string carrying code `wda.tool.fault`.
pub fn render_fault(fault: &ToolFault) -> String {
    let code_str = crate::codes::TOOL_FAULT;
    let message_str = escape_control_chars(&fault.message);
    let path_str = fault.path.as_ref().map(|p| normalize_path(p));

    match path_str {
        Some(ref p) if !p.is_empty() => format!("error: [{code_str}] {p}: {message_str}"),
        _ => format!("error: [{code_str}] {message_str}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes;

    #[test]
    fn test_validation_report_has_error() {
        let mut report = ValidationReport::new();

        assert!(!report.has_error(), "Empty report has_error must be false");

        // Add Warning diagnostic
        report.add(Diagnostic::new(
            codes::DOCS_SOURCE_REF_UNRESOLVABLE,
            Severity::Warning,
            Some(Location::path_only("docs/arch.md")),
            "Unresolvable source ref",
            "Fix link target",
        ));

        assert!(
            !report.has_error(),
            "Warning-only report has_error must be false"
        );

        // Add Error diagnostic
        report.add(Diagnostic::new(
            codes::CONTRACT_MISSING,
            Severity::Error,
            Some(Location::path_only("wda.json")),
            "Missing wda.json",
            "Run wda init to create wda.json",
        ));

        assert!(
            report.has_error(),
            "Mixed report with Error has_error must be true"
        );
    }

    #[test]
    fn test_diagnostic_total_order_sort() {
        let d0 = Diagnostic::new(
            codes::DOCS_REQUIRED_DOCUMENT_MISSING,
            Severity::Error,
            None,
            "Missing document",
            "Create document",
        )
        .with_ordinal(1);

        let d1 = Diagnostic::new(
            codes::DOCS_REQUIRED_DOCUMENT_MISSING,
            Severity::Error,
            Some(Location::path_only("a.md")),
            "Missing document",
            "Create document",
        )
        .with_ordinal(1);

        let d2 = Diagnostic::new(
            codes::DOCS_REQUIRED_DOCUMENT_MISSING,
            Severity::Error,
            Some(Location::new("a.md", Some(10), None)),
            "Missing document",
            "Create document",
        )
        .with_ordinal(1);

        let d3 = Diagnostic::new(
            codes::DOCS_REQUIRED_DOCUMENT_MISSING,
            Severity::Error,
            Some(Location::new("a.md", Some(10), Some(5))),
            "Missing document",
            "Create document",
        )
        .with_ordinal(1);

        let d4 = Diagnostic::new(
            codes::HTML_DUPLICATE_ID,
            Severity::Error,
            Some(Location::new("a.md", Some(10), Some(5))),
            "Duplicate ID",
            "Fix ID",
        )
        .with_ordinal(1);

        let d5 = Diagnostic::new(
            codes::HTML_DUPLICATE_ID,
            Severity::Error,
            Some(Location::new("a.md", Some(10), Some(5))),
            "Duplicate ID",
            "Fix ID",
        )
        .with_ordinal(2);

        let mut report = ValidationReport::new();
        // Insert six diagnostics differing at each tie-break level in reverse order
        report.add(d5.clone());
        report.add(d4.clone());
        report.add(d3.clone());
        report.add(d2.clone());
        report.add(d1.clone());
        report.add(d0.clone());

        report.sort();

        let expected = vec![d0, d1, d2, d3, d4, d5];
        assert_eq!(report.diagnostics, expected);
    }

    #[test]
    fn test_render_diagnostic_injection_hardening() {
        let host_prefix = if cfg!(windows) {
            "C:\\Users\\host\\project\\"
        } else {
            "/home/user/project/"
        };
        let path_with_ctrl = format!("{host_prefix}src\\main\n\r\u{7}.rs");
        let loc = Location::new(path_with_ctrl, Some(10), Some(5));

        let diagnostic = Diagnostic::new(
            codes::CONTRACT_MISSING,
            Severity::Error,
            Some(loc),
            "Message with \n \r \u{7} control characters",
            "Action with \n \r \u{7} control characters",
        );

        let rendered = render(&diagnostic);

        // 1. Assert exactly one line (no literal \n or \r)
        assert_eq!(
            rendered.lines().count(),
            1,
            "rendered output must be exactly one line"
        );
        assert!(
            !rendered.contains('\n'),
            "rendered output must not contain unescaped newline \\n"
        );
        assert!(
            !rendered.contains('\r'),
            "rendered output must not contain unescaped carriage return \\r"
        );
        assert!(
            !rendered.contains('\u{7}'),
            "rendered output must not contain unescaped control char \\u{{7}}"
        );

        // 2. Assert control characters are escaped
        assert!(
            rendered.contains("\\n"),
            "rendered output must contain escaped \\\\n"
        );
        assert!(
            rendered.contains("\\r"),
            "rendered output must contain escaped \\\\r"
        );
        assert!(
            rendered.contains("\\u{7}"),
            "rendered output must contain escaped \\\\u{{7}}"
        );

        // 3. Assert no host absolute path or interior host directory name
        assert!(
            !rendered.contains("C:"),
            "rendered output must not contain host drive letter"
        );
        assert!(
            !rendered.contains("/home/user"),
            "rendered output must not contain host absolute path"
        );
        assert!(
            !rendered.contains("project"),
            "rendered output must not contain interior host directory name 'project'"
        );
        assert!(
            !rendered.contains("user"),
            "rendered output must not contain interior host directory name 'user'"
        );
        assert!(
            !rendered.contains("host"),
            "rendered output must not contain interior host directory name 'host'"
        );

        // 4. Assert required fields present (severity, code, path, position)
        assert!(rendered.contains("error"), "severity must be present");
        assert!(
            rendered.contains(codes::CONTRACT_MISSING),
            "code must be present"
        );
        assert!(rendered.contains("10:5"), "position must be present");
    }

    #[test]
    fn test_render_tool_fault() {
        let fault_with_path = ToolFault::new(
            "Schema compilation failed\nwith newline\r\u{7}",
            Some(PathBuf::from("schemas/wda.schema.json")),
        );
        let rendered_with_path = render_fault(&fault_with_path);
        assert_eq!(
            rendered_with_path.lines().count(),
            1,
            "ToolFault output must be exactly one line"
        );
        assert!(
            rendered_with_path.contains(codes::TOOL_FAULT),
            "ToolFault must carry wda.tool.fault code"
        );
        assert!(
            rendered_with_path.contains("schemas/wda.schema.json"),
            "ToolFault must carry path when known"
        );
        assert!(
            rendered_with_path.contains("\\n"),
            "ToolFault message newlines must be escaped"
        );

        let fault_no_path = ToolFault::new("Internal failure without path", None::<PathBuf>);
        let rendered_no_path = render_fault(&fault_no_path);
        assert_eq!(rendered_no_path.lines().count(), 1);
        assert!(rendered_no_path.contains(codes::TOOL_FAULT));
        assert!(rendered_no_path.contains("Internal failure without path"));
    }

    #[test]
    fn test_render_path_normalization() {
        let multi_component_path = PathBuf::from("src").join("validation").join("documents.rs");
        let loc = Location::new(multi_component_path, Some(42), Some(1));
        let diagnostic = Diagnostic::new(
            codes::DOCS_FRONT_MATTER_MALFORMED,
            Severity::Warning,
            Some(loc),
            "Malformed front matter",
            "Fix YAML syntax",
        );

        let rendered = render(&diagnostic);
        assert!(
            rendered.contains("src/validation/documents.rs"),
            "multi-component path must be formatted with / separators"
        );
        assert!(
            !rendered.contains('\\'),
            "rendered path must not contain backslashes on any OS"
        );
    }

    #[test]
    fn test_version_status_severity_mapping() {
        assert_eq!(VersionStatus::Current.severity_of(), None);
        assert_eq!(
            VersionStatus::SupportedLegacy.severity_of(),
            Some(Severity::Warning)
        );
        assert_eq!(
            VersionStatus::DeprecatedLegacy.severity_of(),
            Some(Severity::Warning)
        );
        assert_eq!(
            VersionStatus::Unsupported.severity_of(),
            Some(Severity::Error)
        );
    }

    #[test]
    fn test_no_serialization_surface_and_legacy_variants_unreachable() {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_string());
        let base_path = PathBuf::from(&manifest_dir);
        let src_dir = base_path.join("src");

        // Helper to recursively collect all .rs files under src/
        fn walk_src_dir(dir: &Path, files: &mut Vec<(PathBuf, String)>) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        walk_src_dir(&path, files);
                    } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            files.push((path, content));
                        }
                    }
                }
            }
        }

        let mut all_src_files = Vec::new();
        walk_src_dir(&src_dir, &mut all_src_files);

        // Find diagnostics.rs content specifically for part (a)
        let diag_content = all_src_files
            .iter()
            .find(|(p, _)| p.file_name().and_then(|n| n.to_str()) == Some("diagnostics.rs"))
            .map(|(_, content)| content.as_str())
            .expect("src/diagnostics.rs must exist");

        // (a) Diagnostic and ValidationReport carry no serialization derive
        fn has_serialization_derive(src: &str) -> bool {
            let ser = format!("{}ialize", "Ser");
            let de_ser = format!("{}erialize", "Des");

            let mut rest = src;
            while let Some(idx) = rest.find("derive(") {
                let after = &rest[idx + "derive(".len()..];
                if let Some(close_idx) = after.find(')') {
                    let inside = &after[..close_idx];
                    for trait_name in inside.split(',') {
                        let t = trait_name.trim();
                        if t == ser
                            || t == de_ser
                            || t.ends_with(&format!("::{ser}"))
                            || t.ends_with(&format!("::{de_ser}"))
                        {
                            return true;
                        }
                    }
                    rest = &after[close_idx..];
                } else {
                    break;
                }
            }
            false
        }

        assert!(
            !has_serialization_derive(diag_content),
            "diagnostics.rs must not contain serialization derives"
        );

        // (b) No serialization call appears in src/main.rs, src/cli.rs, or src/diagnostics.rs
        fn has_serialization_call(src: &str) -> bool {
            let ser_json = format!("{}json", "serde_");
            let to_j = format!("to_{}", "json");
            let ser_val = format!("serde::{ser}", ser = "Serialize");

            for line in src.lines() {
                if line.contains(&ser_json) || line.contains(&to_j) || line.contains(&ser_val) {
                    return true;
                }
            }
            false
        }

        let target_b_files = ["src/main.rs", "src/cli.rs", "src/diagnostics.rs"];
        for rel_path in target_b_files {
            let full_path = base_path.join(rel_path);
            if !full_path.exists() {
                continue;
            }
            let content = std::fs::read_to_string(&full_path)
                .unwrap_or_else(|_| panic!("Failed to read file {}", rel_path));
            assert!(
                !has_serialization_call(&content),
                "File {} must not contain serialization calls",
                rel_path
            );
        }

        // (c) VersionStatus legacy/unsupported constructors have no call site outside diagnostics.rs
        let legacy_variants = ["SupportedLegacy", "DeprecatedLegacy", "Unsupported"];
        for (path, content) in &all_src_files {
            let is_diagnostics_rs =
                path.file_name().and_then(|n| n.to_str()) == Some("diagnostics.rs");
            if !is_diagnostics_rs {
                for variant in &legacy_variants {
                    assert!(
                        !content.contains(variant),
                        "File {} must not construct or reference VersionStatus::{}",
                        path.display(),
                        variant
                    );
                }
            }
        }
    }
}
