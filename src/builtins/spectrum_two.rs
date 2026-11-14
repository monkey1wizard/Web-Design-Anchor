/// Returns the embedded `tokens.json` source content for Spectrum 2.
pub(crate) fn tokens_json() -> &'static str {
    include_str!("spectrum_two/tokens/tokens.json")
}

/// Returns the embedded `index.html` starter page content for Spectrum 2.
pub(crate) fn starter_page() -> &'static str {
    include_str!("spectrum_two/pages/index.html")
}

/// Returns the embedded `main.ts` starter script content for Spectrum 2.
pub(crate) fn starter_script() -> &'static str {
    include_str!("spectrum_two/scripts/main.ts")
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
    }

    #[test]
    fn test_starter_script_accessor_returns_non_empty_content() {
        let script = starter_script();
        assert!(!script.is_empty(), "starter_script must not be empty");
    }

    #[test]
    fn native_spectrum_starter_unit_imports_name_only_spectrum_packages() {
        let script = starter_script();
        let mut imports = 0usize;
        for line in script.lines() {
            let Some(rest) = line.trim_start().strip_prefix("import ") else {
                continue;
            };
            imports += 1;
            let specifier = rest
                .split(['"', '\''])
                .nth(1)
                .expect("an import line must carry a quoted specifier");
            assert!(
                !specifier.contains("://"),
                "{specifier} must not be a remote URL"
            );
            assert!(
                specifier.starts_with("@spectrum-web-components/"),
                "{specifier} must name an installed Spectrum package"
            );
        }
        assert!(imports > 0, "the starter must import its components");
        assert!(
            !script.to_ascii_lowercase().contains("deno"),
            "the starter must not name Deno"
        );
    }
}
