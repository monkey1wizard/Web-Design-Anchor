//! DTCG token stylesheet emission into CSS Custom Properties.
//!
//! Renders DTCG tokens from `tokens/tokens.json` into CSS Custom Properties
//! formatted inside a `:root { ... }` rule block. Aliases are resolved using
//! the shared alias resolution engine in `crate::validation::tokens`.

#![allow(dead_code)]

use serde_json::Value;

use crate::validation::tokens::{extract_aliases, resolve_alias};

fn to_kebab_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    let mut prev_is_lower = false;
    for c in s.chars() {
        if c == '_' || c == ' ' {
            if !result.is_empty() && !result.ends_with('-') {
                result.push('-');
            }
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower && !result.ends_with('-') {
                result.push('-');
            }
            for lc in c.to_lowercase() {
                result.push(lc);
            }
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = c.is_lowercase() || c.is_numeric();
        }
    }
    result
}

fn format_css_custom_property(segments: &[&str]) -> String {
    let parts: Vec<String> = segments
        .iter()
        .map(|s| to_kebab_case(s))
        .filter(|s| !s.is_empty())
        .collect();
    format!("--{}", parts.join("-"))
}

fn resolve_string_value(root_val: &Value, s: &str) -> String {
    let aliases = extract_aliases(s);
    if aliases.is_empty() {
        return s.to_string();
    }
    let trimmed = s.trim();
    if trimmed.starts_with('{')
        && trimmed.ends_with('}')
        && !trimmed[1..trimmed.len() - 1].contains('{')
    {
        let alias_ref = trimmed[1..trimmed.len() - 1].trim();
        if let Some(resolved) = resolve_alias(root_val, alias_ref) {
            return value_to_css_string(root_val, resolved);
        }
    }

    let mut result = s.to_string();
    for alias_ref in aliases {
        let pattern = format!("{{{alias_ref}}}");
        if let Some(resolved) = resolve_alias(root_val, alias_ref) {
            let resolved_str = value_to_css_string(root_val, resolved);
            result = result.replace(&pattern, &resolved_str);
        }
    }
    result
}

fn value_to_css_string(root_val: &Value, val: &Value) -> String {
    match val {
        Value::String(s) => resolve_string_value(root_val, s),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(arr) => arr
            .iter()
            .map(|item| value_to_css_string(root_val, item))
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    }
}

fn collect_tokens<'a>(
    val: &'a Value,
    segments: &mut Vec<&'a str>,
    out: &mut Vec<(String, &'a Value)>,
) {
    match val {
        Value::Object(map) => {
            let has_dollar_value = map.contains_key("$value");
            let has_child_keys = map.keys().any(|k| !k.starts_with('$'));
            let is_leaf = has_dollar_value || (!has_child_keys && !segments.is_empty());

            if is_leaf {
                if let Some(v) = map.get("$value") {
                    let prop_name = format_css_custom_property(segments);
                    out.push((prop_name, v));
                }
            } else {
                for (k, v) in map {
                    if k.starts_with('$') {
                        continue;
                    }
                    segments.push(k.as_str());
                    collect_tokens(v, segments, out);
                    segments.pop();
                }
            }
        }
        _ => {
            if !segments.is_empty() {
                let prop_name = format_css_custom_property(segments);
                out.push((prop_name, val));
            }
        }
    }
}

/// Emits a deterministic CSS stylesheet containing CSS Custom Properties derived from DTCG tokens.
///
/// Returns a stylesheet string containing a `:root { ... }` block with all resolved tokens.
/// Output is deterministic: the same token structure always yields byte-identical CSS.
pub(crate) fn emit_token_stylesheet(root_val: &Value) -> String {
    let mut segments = Vec::new();
    let mut raw_tokens = Vec::new();
    collect_tokens(root_val, &mut segments, &mut raw_tokens);

    let mut properties: Vec<(String, String)> = raw_tokens
        .into_iter()
        .map(|(prop_name, val)| {
            let resolved_str = value_to_css_string(root_val, val);
            (prop_name, resolved_str)
        })
        .collect();

    properties.sort_by(|a, b| a.0.cmp(&b.0));

    let mut css = String::new();
    css.push_str(":root {\n");
    for (name, val) in properties {
        css.push_str(&format!("  {name}: {val};\n"));
    }
    css.push_str("}\n");

    css
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tokens_returns_empty_root_block() {
        let val: Value = serde_json::from_str("{}").unwrap();
        let css = emit_token_stylesheet(&val);
        assert_eq!(css, ":root {\n}\n");
    }

    #[test]
    fn test_emitter_renders_expected_properties_and_is_deterministic() {
        let content = r##"{
  "color": {
    "$type": "color",
    "background": {
      "page": {
        "$value": "#ffffff"
      },
      "surface": {
        "$value": "#f8fafc"
      }
    },
    "brand": {
      "primary": {
        "$value": "#2563eb"
      }
    }
  },
  "lineHeight": {
    "$type": "lineHeight",
    "tight": {
      "$value": 1.25
    }
  },
  "spacing": {
    "$type": "dimension",
    "md": {
      "$value": "1rem"
    }
  }
}"##;
        let val: Value = serde_json::from_str(content).unwrap();
        let css1 = emit_token_stylesheet(&val);
        let css2 = emit_token_stylesheet(&val);

        // Byte-identical check
        assert_eq!(css1, css2);
        assert_eq!(css1.as_bytes(), css2.as_bytes());

        // Expected properties check
        assert!(css1.contains("  --color-background-page: #ffffff;\n"));
        assert!(css1.contains("  --color-background-surface: #f8fafc;\n"));
        assert!(css1.contains("  --color-brand-primary: #2563eb;\n"));
        assert!(css1.contains("  --line-height-tight: 1.25;\n"));
        assert!(css1.contains("  --spacing-md: 1rem;\n"));
    }

    #[test]
    fn test_emitter_resolves_aliases_and_chained_aliases() {
        let content = r##"{
  "color": {
    "base": {
      "blue": {
        "$value": "#0000ff"
      }
    },
    "brand": {
      "primary": {
        "$value": "{color.base.blue}"
      }
    },
    "accent": {
      "$value": "{color.brand.primary}"
    }
  }
}"##;
        let val: Value = serde_json::from_str(content).unwrap();
        let css = emit_token_stylesheet(&val);

        assert!(css.contains("  --color-base-blue: #0000ff;\n"));
        assert!(css.contains("  --color-brand-primary: #0000ff;\n"));
        assert!(css.contains("  --color-accent: #0000ff;\n"));
    }

    #[test]
    fn test_emitter_on_starter_tokens_fixture() {
        let content = include_str!("../builtins/wda_minimal/tokens/tokens.json");
        let val: Value = serde_json::from_str(content).unwrap();

        let css1 = emit_token_stylesheet(&val);
        let css2 = emit_token_stylesheet(&val);

        // Determinism: byte-identical repeat
        assert_eq!(css1, css2);
        assert_eq!(css1.as_bytes(), css2.as_bytes());

        // Verify key properties from minimal starter
        assert!(css1.contains("  --color-background-page: #f8f8f8;\n"));
        assert!(css1.contains("  --color-brand-primary: #3b63fb;\n"));
        assert!(css1.contains("  --color-text-primary: #292929;\n"));
        assert!(css1.contains("  --font-family-base: 'Noto Sans', system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif;\n"));
        assert!(css1.contains("  --font-size-base: 1rem;\n"));
        assert!(css1.contains("  --font-weight-normal: 400;\n"));
        assert!(css1.contains("  --line-height-tight: 1.3;\n"));
        assert!(css1.contains("  --spacing-md: 1rem;\n"));
        assert!(css1.contains("  --radius-full: 9999px;\n"));
    }
}