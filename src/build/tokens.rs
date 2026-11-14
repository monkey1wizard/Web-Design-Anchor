//! Deterministic CSS custom properties from validated tokens.

use crate::validation::tokens::token_properties;
use serde_json::Value;

pub(crate) fn emit_token_stylesheet(root: &Value) -> Result<String, String> {
    let properties =
        token_properties(root).map_err(|errors| format!("Invalid token profile: {errors:?}"))?;
    let mut css = String::from(":root {\n");
    for (name, value) in properties {
        css.push_str(&format!("  {name}: {value};\n"));
    }
    css.push_str("}\n");
    Ok(css)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn author_token_profile_unit() {
        let value: Value = serde_json::from_str(r##"{"color":{"$type":"color","base":{"$value":"#fff"},"alias":{"$value":"{color.base}"}},"size":{"$type":"number","large":{"$value":2}}}"##).unwrap();
        let css = emit_token_stylesheet(&value).unwrap();
        assert_eq!(
            css,
            ":root {\n  --color-alias: #fff;\n  --color-base: #fff;\n  --size-large: 2;\n}\n"
        );
        assert_eq!(css, emit_token_stylesheet(&value).unwrap());
        let escaped = serde_json::json!({"text": {"$type": "string", "$value": "'a; b!'"}});
        assert!(emit_token_stylesheet(&escaped).unwrap().contains("'a; b!'"));
    }

    #[test]
    fn test_empty_tokens_returns_empty_root_block() {
        let val: Value = serde_json::from_str("{}").unwrap();
        let css = emit_token_stylesheet(&val).unwrap();
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
        let css1 = emit_token_stylesheet(&val).unwrap();
        let css2 = emit_token_stylesheet(&val).unwrap();

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
    "$type": "color",
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
        let css = emit_token_stylesheet(&val).unwrap();

        assert!(css.contains("  --color-base-blue: #0000ff;\n"));
        assert!(css.contains("  --color-brand-primary: #0000ff;\n"));
        assert!(css.contains("  --color-accent: #0000ff;\n"));
    }

    #[test]
    fn test_emitter_on_starter_tokens_fixture() {
        let content = include_str!("../builtins/wda_minimal/tokens/tokens.json");
        let val: Value = serde_json::from_str(content).unwrap();

        let css1 = emit_token_stylesheet(&val).unwrap();
        let css2 = emit_token_stylesheet(&val).unwrap();

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
