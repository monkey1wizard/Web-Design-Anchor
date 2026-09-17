//! Generated project document and template rendering for project initialization.
//!
//! Provides front-matter stripping and authoring, template embedding,
//! and rendering for the four required project documents (`README.md`,
//! `docs/architecture.md`, `docs/design.md`, `docs/naming.md`) as well as
//! starter page, tokens, and gitignore assets.

use chrono::{DateTime, TimeZone};

use crate::builtins::wda_minimal::{self, render_criteria};

/// Embedded template content for `README.md`.
const README_TEMPLATE: &str =
    include_str!("../../docs/templates/generated-project/readme.md");

/// Embedded template content for `docs/architecture.md`.
const ARCHITECTURE_TEMPLATE: &str =
    include_str!("../../docs/templates/generated-project/architecture.md");

/// Embedded template content for `docs/naming.md`.
const NAMING_TEMPLATE: &str =
    include_str!("../../docs/templates/generated-project/naming.md");

/// Canonical display form for the default accessibility baseline.
pub const ACCESSIBILITY_BASELINE_DISPLAY: &str = "WCAG 2.2 AA";

/// Canonical gitignore content for newly initialized projects.
pub const GITIGNORE_CONTENT: &str = "/dist/\n";

/// Strips the leading YAML front-matter block (`---` ... `---`) and any
/// immediately following single newline from markdown content.
pub fn strip_front_matter(content: &str) -> &str {
    let mut lines = content.split_inclusive('\n');
    let first = match lines.next() {
        Some(line) => line,
        None => return content,
    };
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return content;
    }
    let mut consumed_bytes = first.len();
    for line in lines {
        consumed_bytes += line.len();
        if line.trim_end_matches(['\r', '\n']) == "---" {
            let rest = &content[consumed_bytes..];
            return rest
                .strip_prefix("\r\n")
                .or_else(|| rest.strip_prefix('\n'))
                .unwrap_or(rest);
        }
    }
    content
}

/// Authors a YAML front-matter block with the given `title`, `status`, `updated` date,
/// and appends the document `body`.
pub fn author_front_matter(title: &str, status: &str, updated: &str, body: &str) -> String {
    format!("---\ntitle: {title}\nstatus: {status}\nupdated: {updated}\n---\n\n{body}")
}

/// Formats a DateTime into `YYYY-MM-DD` representation for document front matter.
pub fn format_document_date<Tz: TimeZone>(dt: &DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    dt.format("%Y-%m-%d").to_string()
}

/// Renders the generated `README.md` with front matter stripped.
pub fn render_readme() -> String {
    strip_front_matter(README_TEMPLATE).to_string()
}

/// Renders the generated `docs/architecture.md` with authored front matter and stripped template body.
pub fn render_architecture_doc(date: &str) -> String {
    let body = strip_front_matter(ARCHITECTURE_TEMPLATE);
    author_front_matter("Architecture", "active", date, body)
}

/// Renders the generated `docs/design.md` with authored front matter and stripped criteria body.
pub fn render_design_doc(date: &str, design_system: &str) -> String {
    let rendered_criteria = render_criteria(design_system, ACCESSIBILITY_BASELINE_DISPLAY);
    let body = strip_front_matter(&rendered_criteria);
    author_front_matter(
        "Design System & Visual Review Criteria",
        "active",
        date,
        body,
    )
}

/// Renders the generated `docs/naming.md` with authored front matter and stripped template body.
pub fn render_naming_doc(date: &str) -> String {
    let body = strip_front_matter(NAMING_TEMPLATE);
    author_front_matter("Naming Conventions", "active", date, body)
}

/// Renders the starter `pages/index.html` page with `<title>` replaced by `project_name`.
pub fn render_starter_page(project_name: &str) -> String {
    let template = wda_minimal::starter_page();
    template.replace(
        "<title>WDA Minimal Starter</title>",
        &format!("<title>{project_name}</title>"),
    )
}

/// Returns the embedded `tokens/tokens.json` content unchanged.
pub fn tokens_json_content() -> &'static str {
    wda_minimal::tokens_json()
}

/// Returns the canonical `.gitignore` content.
pub fn gitignore_content() -> &'static str {
    GITIGNORE_CONTENT
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, NaiveDate, NaiveTime};

    fn count_front_matter_blocks(content: &str) -> usize {
        let mut count = 0;
        let mut in_block = false;
        for (idx, line) in content.lines().enumerate() {
            if line.trim_end() == "---" {
                if !in_block && idx == 0 {
                    in_block = true;
                } else if in_block {
                    count += 1;
                    in_block = false;
                }
            }
        }
        count
    }

    #[test]
    fn test_format_document_date() {
        let offset = FixedOffset::east_opt(8 * 3600).unwrap();
        let naive_date = NaiveDate::from_ymd_opt(2026, 8, 27).unwrap();
        let naive_time = NaiveTime::from_hms_opt(14, 3, 5).unwrap();
        let naive_dt = naive_date.and_time(naive_time);
        let dt = offset.from_local_datetime(&naive_dt).unwrap();

        assert_eq!(format_document_date(&dt), "2026-08-27");
    }

    #[test]
    fn test_strip_front_matter() {
        let input_with_fm = "---\ntitle: Sample\nstatus: active\nupdated: 2026-08-24\n---\n\n# Body\nContent";
        let stripped = strip_front_matter(input_with_fm);
        assert_eq!(stripped, "# Body\nContent");

        let input_without_fm = "# Direct Heading\nContent";
        let unchanged = strip_front_matter(input_without_fm);
        assert_eq!(unchanged, input_without_fm);
    }

    #[test]
    fn test_author_front_matter_format() {
        let authored = author_front_matter("Test Title", "active", "2026-09-02", "# Body Content\n");
        let expected = "---\ntitle: Test Title\nstatus: active\nupdated: 2026-09-02\n---\n\n# Body Content\n";
        assert_eq!(authored, expected);
        assert_eq!(count_front_matter_blocks(&authored), 1);
    }

    #[test]
    fn test_render_readme_has_no_front_matter_and_states_the_repository_is_already_created() {
        let readme = render_readme();
        assert!(
            !readme.starts_with("---"),
            "README.md must not have front matter"
        );
        assert_eq!(
            count_front_matter_blocks(&readme),
            0,
            "README.md must contain 0 front-matter blocks"
        );
        assert!(
            readme.contains("wda init"),
            "README.md must mention wda init"
        );
        assert!(
            !readme.contains("git init"),
            "README.md must not instruct the Designer to run a separate 'git init' step, since wda init already creates the local repository"
        );
        assert!(
            readme.contains("already a local git repository"),
            "README.md must state the project is already a local git repository after wda init succeeds"
        );
        assert!(
            !readme.contains("sourceRefs"),
            "README.md must not contain sourceRefs"
        );
    }

    #[test]
    fn test_render_architecture_doc_conforms_to_spec() {
        let fixed_date = "2026-09-02";
        let doc = render_architecture_doc(fixed_date);

        assert_eq!(
            count_front_matter_blocks(&doc),
            1,
            "architecture.md must have exactly one front-matter block"
        );
        assert!(doc.contains("title: Architecture\n"));
        assert!(doc.contains("status: active\n"));
        assert!(doc.contains(&format!("updated: {fixed_date}\n")));
        assert!(
            !doc.contains("sourceRefs"),
            "architecture.md must not contain sourceRefs"
        );
        assert!(
            doc.contains("# Architecture"),
            "architecture.md must contain body title"
        );
        assert!(
            !doc.contains("Generated Project Architecture Template"),
            "architecture.md must not retain template title"
        );
    }

    #[test]
    fn test_render_design_doc_conforms_to_spec() {
        let fixed_date = "2026-09-02";
        let doc = render_design_doc(fixed_date, "WDA Minimal");

        assert_eq!(
            count_front_matter_blocks(&doc),
            1,
            "design.md must have exactly one front-matter block"
        );
        assert!(doc.contains("title: Design System & Visual Review Criteria\n"));
        assert!(doc.contains("status: active\n"));
        assert!(doc.contains(&format!("updated: {fixed_date}\n")));
        assert!(
            !doc.contains("sourceRefs"),
            "design.md must not contain sourceRefs"
        );
        assert!(
            !doc.contains("{{"),
            "design.md must not contain unrendered {{ template markers"
        );
        assert!(
            !doc.contains("}}"),
            "design.md must not contain unrendered }} template markers"
        );
        assert!(
            doc.contains("Design System**: WDA Minimal"),
            "design.md must contain resolved design system name"
        );
        assert!(
            doc.contains("Accessibility Baseline**: WCAG 2.2 AA"),
            "design.md must contain display form accessibility baseline"
        );
        assert!(
            !doc.contains("updated: 2026-08-27"),
            "design.md must not contain stale frozen updated date"
        );
    }

    #[test]
    fn test_render_naming_doc_conforms_to_spec() {
        let fixed_date = "2026-09-02";
        let doc = render_naming_doc(fixed_date);

        assert_eq!(
            count_front_matter_blocks(&doc),
            1,
            "naming.md must have exactly one front-matter block"
        );
        assert!(doc.contains("title: Naming Conventions\n"));
        assert!(doc.contains("status: active\n"));
        assert!(doc.contains(&format!("updated: {fixed_date}\n")));
        assert!(
            !doc.contains("sourceRefs"),
            "naming.md must not contain sourceRefs"
        );
        assert!(
            doc.contains("MPA"),
            "naming.md must define or contain MPA"
        );
        assert!(
            doc.contains("SPA"),
            "naming.md must define or contain SPA"
        );
        assert!(
            doc.contains("a11y"),
            "naming.md must define or contain a11y"
        );
        assert!(
            doc.contains("WDA Minimal"),
            "naming.md must record official design system name WDA Minimal"
        );
        assert!(
            !doc.contains("Generated Project Naming Template"),
            "naming.md must not retain template title"
        );
    }

    #[test]
    fn test_no_generated_document_contains_source_refs() {
        let date = "2026-09-02";
        let readme = render_readme();
        let arch = render_architecture_doc(date);
        let design = render_design_doc(date, "WDA Minimal");
        let naming = render_naming_doc(date);

        assert!(!readme.contains("sourceRefs"));
        assert!(!arch.contains("sourceRefs"));
        assert!(!design.contains("sourceRefs"));
        assert!(!naming.contains("sourceRefs"));
    }

    #[test]
    fn test_rendered_documents_reproducibility() {
        let date = "2026-09-02";
        let readme1 = render_readme();
        let readme2 = render_readme();
        assert_eq!(readme1, readme2);

        let arch1 = render_architecture_doc(date);
        let arch2 = render_architecture_doc(date);
        assert_eq!(arch1, arch2);

        let design1 = render_design_doc(date, "WDA Minimal");
        let design2 = render_design_doc(date, "WDA Minimal");
        assert_eq!(design1, design2);

        let naming1 = render_naming_doc(date);
        let naming2 = render_naming_doc(date);
        assert_eq!(naming1, naming2);
    }

    #[test]
    fn test_render_starter_page_replaces_title_with_project_name() {
        let project_name = "Web Design Anchor Project";
        let starter = render_starter_page(project_name);
        let built_in = wda_minimal::starter_page();

        assert!(
            starter.contains(&format!("<title>{project_name}</title>")),
            "rendered starter page must contain the substituted project name title"
        );
        assert!(
            !starter.contains("<title>WDA Minimal Starter</title>"),
            "rendered starter page must not retain the built-in starter title"
        );

        let expected_replaced = built_in.replace(
            "<title>WDA Minimal Starter</title>",
            &format!("<title>{project_name}</title>"),
        );
        assert_eq!(
            starter, expected_replaced,
            "rendered starter page must match built-in template with only title tag substituted"
        );
    }

    #[test]
    fn test_tokens_json_content_and_gitignore_content() {
        let tokens = tokens_json_content();
        let expected_tokens = wda_minimal::tokens_json();
        assert_eq!(
            tokens, expected_tokens,
            "tokens json content must be byte-equal to the built-in accessor"
        );
        assert!(!tokens.is_empty(), "tokens json content must not be empty");

        let gitignore = gitignore_content();
        assert_eq!(
            gitignore, GITIGNORE_CONTENT,
            "gitignore content must equal canonical constant"
        );
        assert_eq!(
            gitignore, "/dist/\n",
            "gitignore content must carry canonical /dist/ entry"
        );
    }

    #[test]
    fn test_rendered_documents_pass_document_validation() {
        let temp_dir = std::env::temp_dir()
            .join(format!("wda_test_init_rendered_docs_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("docs")).unwrap();

        let date = "2026-09-02";
        std::fs::write(temp_dir.join("README.md"), render_readme()).unwrap();
        std::fs::write(
            temp_dir.join("docs/architecture.md"),
            render_architecture_doc(date),
        )
        .unwrap();
        std::fs::write(
            temp_dir.join("docs/design.md"),
            render_design_doc(date, "WDA Minimal"),
        )
        .unwrap();
        std::fs::write(
            temp_dir.join("docs/naming.md"),
            render_naming_doc(date),
        )
        .unwrap();

        let report = crate::validation::documents::validate_documents(&temp_dir)
            .expect("validate_documents must succeed");
        assert!(
            report.diagnostics.is_empty(),
            "rendered documents must produce zero document diagnostics: {:?}",
            report.diagnostics
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
