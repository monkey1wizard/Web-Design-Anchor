/// Returns the embedded `tokens.json` source content for WDA Minimal.
pub(crate) fn tokens_json() -> &'static str {
    include_str!("wda_minimal/tokens/tokens.json")
}

/// Returns the embedded `index.html` starter page content for WDA Minimal.
pub(crate) fn starter_page() -> &'static str {
    include_str!("wda_minimal/pages/index.html")
}

/// Returns the embedded `design.md` criteria source content for WDA Minimal.
pub(crate) fn criteria_source() -> &'static str {
    include_str!("wda_minimal/docs/design.md")
}

/// Renders the parameterized `design.md` criteria source for a given design system
/// name and accessibility baseline specification.
pub(crate) fn render_criteria(design_system: &str, accessibility_baseline: &str) -> String {
    let source = criteria_source();
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokens_json_accessor_returns_non_empty_valid_content() {
        let tokens = tokens_json();
        assert!(!tokens.is_empty(), "tokens_json must not be empty");
        let parsed: serde_json::Value =
            serde_json::from_str(tokens).expect("tokens_json must be valid JSON");
        assert!(parsed.is_object(), "tokens_json must be a JSON object");
    }

    #[test]
    fn test_starter_page_accessor_returns_non_empty_html() {
        let html = starter_page();
        assert!(!html.is_empty(), "starter_page must not be empty");
        assert!(
            html.contains("<!DOCTYPE html>"),
            "starter_page must contain doctype"
        );
        assert!(
            html.contains("<html lang=\"en\">"),
            "starter_page must contain lang tag"
        );
    }

    #[test]
    fn test_criteria_source_accessor_returns_non_empty_markdown() {
        let criteria = criteria_source();
        assert!(!criteria.is_empty(), "criteria_source must not be empty");
        assert!(
            criteria.contains("# Design System & Visual Review Criteria"),
            "criteria_source must contain top-level title"
        );
    }

    #[test]
    fn test_render_criteria_wda_minimal() {
        let rendered = render_criteria("WDA Minimal", "WCAG 2.2 AA");
        assert!(rendered.contains("Design System**: WDA Minimal"));
        assert!(rendered.contains("Accessibility Baseline**: WCAG 2.2 AA"));
        assert!(rendered.contains("2D flat layout model"));
        assert!(!rendered.contains("Adobe Spectrum's 5-tier elevation system"));
        assert!(!rendered.contains("{{#if_design_system"));
        assert!(!rendered.contains("{{/if_design_system}}"));
    }

    #[test]
    fn test_render_criteria_spectrum() {
        let rendered = render_criteria("Spectrum 2", "WCAG 2.1 AA");
        assert!(rendered.contains("Design System**: Spectrum 2"));
        assert!(rendered.contains("Accessibility Baseline**: WCAG 2.1 AA"));
        assert!(rendered.contains("reduced-elevation surface model"));
        assert!(!rendered.contains("2D flat layout model"));
        assert!(!rendered.contains("{{#if_design_system"));
        assert!(!rendered.contains("{{/if_design_system}}"));
    }

    #[test]
    fn test_render_criteria_substantive_difference() {
        let wda = render_criteria("WDA Minimal", "WCAG 2.2 AA");
        let spectrum = render_criteria("Spectrum 2", "WCAG 2.1 AA");
        assert_ne!(wda, spectrum);
        assert!(wda.contains("WDA Minimal's font-size scale"));
        assert!(spectrum.contains("Spectrum 2's refreshed Adobe Clean type scale"));
    }
}
