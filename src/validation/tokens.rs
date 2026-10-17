use std::fs;
use std::path::Path;

use crate::codes;
use crate::diagnostics::{Diagnostic, Location, Severity, ToolFault, ValidationReport};
use crate::validation::project_contract::{compute_max_json_depth, escape_json_pointer_token, MAX_JSON_DEPTH};


fn get_sourcemap_location(
    sourcemap: &json_sourcemap::ParseResult,
    pointer: &str,
    preferred_sub_key: Option<&str>,
    rel_path: &str,
) -> Location {
    if let Some(sub) = preferred_sub_key {
        let sub_pointer = if pointer.is_empty() {
            format!("/{escaped_sub}", escaped_sub = escape_json_pointer_token(sub))
        } else {
            format!("{pointer}/{escaped_sub}", escaped_sub = escape_json_pointer_token(sub))
        };
        if let Some(pos) = sourcemap.pointers.get(&sub_pointer) {
            return Location::new(rel_path, Some(pos.key().line + 1), Some(pos.key().column + 1));
        }
    }
    if let Some(pos) = sourcemap.pointers.get(pointer) {
        let loc = if pointer.is_empty() {
            pos.value()
        } else {
            pos.key()
        };
        Location::new(rel_path, Some(loc.line + 1), Some(loc.column + 1))
    } else {
        Location::path_only(rel_path)
    }
}

pub(crate) fn extract_aliases(s: &str) -> Vec<&str> {
    let mut aliases = Vec::new();
    let mut start = 0;
    while let Some(open) = s[start..].find('{') {
        let open_idx = start + open;
        if let Some(close) = s[open_idx..].find('}') {
            let close_idx = open_idx + close;
            let alias_ref = &s[open_idx + 1..close_idx];
            if !alias_ref.is_empty() && !alias_ref.contains('{') {
                aliases.push(alias_ref.trim());
            }
            start = close_idx + 1;
        } else {
            break;
        }
    }
    aliases
}

pub(crate) fn resolve_alias<'a>(
    root_val: &'a serde_json::Value,
    alias_ref: &str,
) -> Option<&'a serde_json::Value> {
    let clean_ref = alias_ref.trim();
    let clean_ref = if clean_ref.starts_with('{') && clean_ref.ends_with('}') {
        clean_ref[1..clean_ref.len() - 1].trim()
    } else {
        clean_ref
    };
    resolve_alias_depth(root_val, clean_ref, 0)
}

fn resolve_alias_depth<'a>(
    root_val: &'a serde_json::Value,
    alias_ref: &str,
    depth: usize,
) -> Option<&'a serde_json::Value> {
    if depth > MAX_JSON_DEPTH {
        return None;
    }
    let parts: Vec<&str> = alias_ref.split('.').map(|s| s.trim()).collect();
    if parts.is_empty() || parts.iter().any(|p| p.is_empty()) {
        return None;
    }
    let mut curr = root_val;
    for part in parts {
        match curr {
            serde_json::Value::Object(map) => {
                if let Some(next) = map.get(part) {
                    curr = next;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
    let val = match curr {
        serde_json::Value::Object(map) => map.get("$value")?,
        other => other,
    };
    if let serde_json::Value::String(s) = val {
        let trimmed = s.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') && !trimmed[1..trimmed.len() - 1].contains('{') {
            let inner = trimmed[1..trimmed.len() - 1].trim();
            return resolve_alias_depth(root_val, inner, depth + 1);
        }
    }
    Some(val)
}

fn collect_string_values<'a>(val: &'a serde_json::Value, strings: &mut Vec<&'a str>) {
    match val {
        serde_json::Value::String(s) => strings.push(s.as_str()),
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_string_values(item, strings);
            }
        }
        serde_json::Value::Object(map) => {
            for v in map.values() {
                collect_string_values(v, strings);
            }
        }
        _ => {}
    }
}

fn validate_token_node(
    val: &serde_json::Value,
    pointer: &str,
    token_path: &str,
    inherited_type: Option<&str>,
    sourcemap: &json_sourcemap::ParseResult,
    root_val: &serde_json::Value,
    rel_path: &str,
    report: &mut ValidationReport,
    depth: usize,
) {
    if depth > MAX_JSON_DEPTH {
        return;
    }
    match val {
        serde_json::Value::Object(map) => {
            let node_type = map
                .get("$type")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty());
            let current_type = node_type.or(inherited_type);

            let has_dollar_value = map.contains_key("$value");
            let has_child_keys = map.keys().any(|k| !k.starts_with('$'));

            let is_leaf = has_dollar_value || (!has_child_keys && !pointer.is_empty());

            if is_leaf {
                // Rule 2: $value missing check
                if let Some(val_node) = map.get("$value") {
                    // Rule 4: Alias resolution check
                    let mut string_values = Vec::new();
                    collect_string_values(val_node, &mut string_values);
                    for s in string_values {
                        for alias_ref in extract_aliases(s) {
                            if resolve_alias(root_val, alias_ref).is_none() {
                                let loc = get_sourcemap_location(
                                    sourcemap,
                                    pointer,
                                    Some("$value"),
                                    rel_path,
                                );
                                report.add(Diagnostic::new(
                                    codes::TOKENS_ALIAS_UNRESOLVABLE,
                                    Severity::Error,
                                    Some(loc),
                                    format!(
                                        "Token alias '{{{alias_ref}}}' in '{token_path}' cannot be resolved within tokens/tokens.json"
                                    ),
                                    format!("Ensure alias target '{{{alias_ref}}}' exists in tokens/tokens.json"),
                                ));
                            }
                        }
                    }
                } else {
                    let loc = get_sourcemap_location(sourcemap, pointer, None, rel_path);
                    report.add(Diagnostic::new(
                        codes::TOKENS_VALUE_MISSING,
                        Severity::Error,
                        Some(loc),
                        format!("Leaf token '{token_path}' is missing a $value property"),
                        format!("Add a $value property to token '{token_path}'"),
                    ));
                }

                // Rule 3: $type unresolvable check
                if current_type.is_none() {
                    let loc = get_sourcemap_location(sourcemap, pointer, None, rel_path);
                    report.add(Diagnostic::new(
                        codes::TOKENS_TYPE_UNRESOLVABLE,
                        Severity::Error,
                        Some(loc),
                        format!("Leaf token '{token_path}' has no resolvable $type on itself or an ancestor group"),
                        format!("Specify a $type property on token '{token_path}' or an ancestor group"),
                    ));
                }
            } else {
                // Group node: recurse into child keys that do not start with '$'
                for (k, v) in map {
                    if k.starts_with('$') {
                        continue;
                    }
                    let escaped_k = escape_json_pointer_token(k);
                    let child_pointer = if pointer.is_empty() {
                        format!("/{escaped_k}")
                    } else {
                        format!("{pointer}/{escaped_k}")
                    };
                    let child_path = if token_path.is_empty() {
                        k.clone()
                    } else {
                        format!("{token_path}.{k}")
                    };
                    validate_token_node(
                        v,
                        &child_pointer,
                        &child_path,
                        current_type,
                        sourcemap,
                        root_val,
                        rel_path,
                        report,
                        depth + 1,
                    );
                }
            }
        }
        _ => {
            // Primitive value under a token key (e.g. "key": "#fff" instead of object with $value)
            let loc = get_sourcemap_location(sourcemap, pointer, None, rel_path);

            // Rule 2: missing $value
            report.add(Diagnostic::new(
                codes::TOKENS_VALUE_MISSING,
                Severity::Error,
                Some(loc.clone()),
                format!("Leaf token '{token_path}' is missing a $value property"),
                format!("Add a $value property to token '{token_path}'"),
            ));

            // Rule 3: $type unresolvable check
            if inherited_type.is_none() {
                report.add(Diagnostic::new(
                    codes::TOKENS_TYPE_UNRESOLVABLE,
                    Severity::Error,
                    Some(loc.clone()),
                    format!("Leaf token '{token_path}' has no resolvable $type on itself or an ancestor group"),
                    format!("Specify a $type property on token '{token_path}' or an ancestor group"),
                ));
            }

            // Rule 4: check aliases if primitive is string
            if let Some(s) = val.as_str() {
                for alias_ref in extract_aliases(s) {
                    if resolve_alias(root_val, alias_ref).is_none() {
                        report.add(Diagnostic::new(
                            codes::TOKENS_ALIAS_UNRESOLVABLE,
                            Severity::Error,
                            Some(loc.clone()),
                            format!(
                                "Token alias '{{{alias_ref}}}' in '{token_path}' cannot be resolved within tokens/tokens.json"
                            ),
                            format!("Ensure alias target '{{{alias_ref}}}' exists in tokens/tokens.json"),
                        ));
                    }
                }
            }
        }
    }
}

/// Validates DTCG tokens file `tokens/tokens.json` under `root` if present.
pub fn validate_tokens(root: &Path) -> Result<ValidationReport, ToolFault> {
    let mut report = ValidationReport::new();
    let rel_path = "tokens/tokens.json";
    let tokens_json_path = root.join("tokens").join("tokens.json");

    if !tokens_json_path.exists() {
        return Ok(report);
    }

    let content = match fs::read_to_string(&tokens_json_path) {
        Ok(c) => c,
        Err(err) => {
            report.add(Diagnostic::new(
                codes::TOKENS_MALFORMED_JSON,
                Severity::Error,
                Some(Location::path_only(rel_path)),
                format!("Failed to read tokens file '{rel_path}': {err}"),
                "Ensure tokens/tokens.json is readable and valid UTF-8 JSON",
            ));
            return Ok(report);
        }
    };

    if compute_max_json_depth(&content) > MAX_JSON_DEPTH {
        report.add(Diagnostic::new(
            codes::TOKENS_MALFORMED_JSON,
            Severity::Error,
            Some(Location::path_only(rel_path)),
            format!("Malformed JSON in tokens/tokens.json: nesting depth exceeds maximum allowed limit of {MAX_JSON_DEPTH}"),
            "Fix JSON structure or reduce nesting depth in tokens/tokens.json",
        ));
        return Ok(report);
    }

    let parsed_sourcemap = match json_sourcemap::parse(&content, json_sourcemap::Options::default()) {
        Ok(res) => res,
        Err(_) => {
            let loc = match serde_json::from_str::<serde_json::Value>(&content) {
                Err(json_err) => {
                    if json_err.line() > 0 {
                        Location::new(rel_path, Some(json_err.line()), Some(json_err.column()))
                    } else {
                        Location::path_only(rel_path)
                    }
                }
                Ok(_) => Location::path_only(rel_path),
            };
            report.add(Diagnostic::new(
                codes::TOKENS_MALFORMED_JSON,
                Severity::Error,
                Some(loc),
                "Malformed JSON in tokens/tokens.json",
                "Fix JSON syntax errors in tokens/tokens.json",
            ));
            return Ok(report);
        }
    };

    let root_val = &parsed_sourcemap.value;
    if !root_val.is_object() {
        report.add(Diagnostic::new(
            codes::TOKENS_MALFORMED_JSON,
            Severity::Error,
            Some(Location::path_only(rel_path)),
            "Root value in tokens/tokens.json must be a JSON object",
            "Ensure the root of tokens/tokens.json is a JSON object",
        ));
        return Ok(report);
    }

    validate_token_node(
        root_val,
        "",
        "",
        None,
        &parsed_sourcemap,
        root_val,
        rel_path,
        &mut report,
        0,
    );

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_temp_dir(suffix: &str) -> std::path::PathBuf {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_tokens_{}_{}", suffix, std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        temp_dir
    }

    #[test]
    fn test_absent_tokens_file_yields_zero_diagnostics() {
        let temp = create_temp_dir("absent");
        let report = validate_tokens(&temp).expect("validation report");
        assert!(report.diagnostics.is_empty());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_non_json_tokens_content_yields_malformed_json_error() {
        let temp = create_temp_dir("malformed");
        let tokens_dir = temp.join("tokens");
        fs::create_dir_all(&tokens_dir).unwrap();
        fs::write(tokens_dir.join("tokens.json"), "{ invalid json }").unwrap();

        let report = validate_tokens(&temp).expect("validation report");
        assert_eq!(report.diagnostics.len(), 1);
        let diag = &report.diagnostics[0];
        assert_eq!(diag.code, codes::TOKENS_MALFORMED_JSON);
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.location.as_ref().unwrap().path, Path::new("tokens/tokens.json"));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_non_object_root_yields_malformed_json_error() {
        let temp = create_temp_dir("non_object_root");
        let tokens_dir = temp.join("tokens");
        fs::create_dir_all(&tokens_dir).unwrap();
        fs::write(tokens_dir.join("tokens.json"), "[1, 2, 3]").unwrap();

        let report = validate_tokens(&temp).expect("validation report");
        assert_eq!(report.diagnostics.len(), 1);
        let diag = &report.diagnostics[0];
        assert_eq!(diag.code, codes::TOKENS_MALFORMED_JSON);
        assert_eq!(diag.severity, Severity::Error);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_leaf_token_missing_value_property_yields_value_missing_error() {
        let temp = create_temp_dir("value_missing");
        let tokens_dir = temp.join("tokens");
        fs::create_dir_all(&tokens_dir).unwrap();
        let content = r##"{
  "color": {
    "primary": {
      "$type": "color"
    }
  }
}"##;
        fs::write(tokens_dir.join("tokens.json"), content).unwrap();

        let report = validate_tokens(&temp).expect("validation report");
        assert_eq!(report.diagnostics.len(), 1);
        let diag = &report.diagnostics[0];
        assert_eq!(diag.code, codes::TOKENS_VALUE_MISSING);
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("color.primary"));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_leaf_token_unresolvable_type_yields_type_unresolvable_error() {
        let temp = create_temp_dir("type_unresolvable");
        let tokens_dir = temp.join("tokens");
        fs::create_dir_all(&tokens_dir).unwrap();
        let content = r##"{
  "color": {
    "primary": {
      "$value": "#ffffff"
    }
  }
}"##;
        fs::write(tokens_dir.join("tokens.json"), content).unwrap();

        let report = validate_tokens(&temp).expect("validation report");
        assert_eq!(report.diagnostics.len(), 1);
        let diag = &report.diagnostics[0];
        assert_eq!(diag.code, codes::TOKENS_TYPE_UNRESOLVABLE);
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("color.primary"));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_unresolvable_token_alias_yields_alias_unresolvable_error() {
        let temp = create_temp_dir("alias_unresolvable");
        let tokens_dir = temp.join("tokens");
        fs::create_dir_all(&tokens_dir).unwrap();
        let content = r##"{
  "color": {
    "$type": "color",
    "accent": {
      "$value": "{color.missing}"
    }
  }
}"##;
        fs::write(tokens_dir.join("tokens.json"), content).unwrap();

        let report = validate_tokens(&temp).expect("validation report");
        assert_eq!(report.diagnostics.len(), 1);
        let diag = &report.diagnostics[0];
        assert_eq!(diag.code, codes::TOKENS_ALIAS_UNRESOLVABLE);
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("color.missing"));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_leaf_inheriting_type_from_ancestor_group_yields_zero_diagnostics() {
        let temp = create_temp_dir("inheriting_type");
        let tokens_dir = temp.join("tokens");
        fs::create_dir_all(&tokens_dir).unwrap();
        let content = r##"{
  "color": {
    "$type": "color",
    "primary": {
      "$value": "#ffffff"
    }
  }
}"##;
        fs::write(tokens_dir.join("tokens.json"), content).unwrap();

        let report = validate_tokens(&temp).expect("validation report");
        assert!(report.diagnostics.is_empty(), "leaf inheriting $type from ancestor group should yield zero diagnostics");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_valid_tokens_file_with_resolvable_alias_yields_zero_diagnostics() {
        let temp = create_temp_dir("valid_alias");
        let tokens_dir = temp.join("tokens");
        fs::create_dir_all(&tokens_dir).unwrap();
        let content = r##"{
  "color": {
    "$type": "color",
    "brand": {
      "primary": {
        "$value": "#ff0000"
      }
    },
    "accent": {
      "$value": "{color.brand.primary}"
    }
  }
}"##;
        fs::write(tokens_dir.join("tokens.json"), content).unwrap();

        let report = validate_tokens(&temp).expect("validation report");
        assert!(report.diagnostics.is_empty());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_exact_line_and_column_positions_for_token_diagnostics() {
        let temp = create_temp_dir("positions");
        let tokens_dir = temp.join("tokens");
        fs::create_dir_all(&tokens_dir).unwrap();
        let content = r##"{
  "color": {
    "primary": {
      "$value": "#ffffff"
    }
  }
}"##;
        fs::write(tokens_dir.join("tokens.json"), content).unwrap();

        let report = validate_tokens(&temp).expect("validation report");
        assert_eq!(report.diagnostics.len(), 1);
        let loc = report.diagnostics[0].location.as_ref().unwrap();
        assert_eq!(loc.line, Some(3));
        assert_eq!(loc.column, Some(5));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_tokens_json_exceeding_max_depth_yields_malformed_json_without_panic() {
        let temp = create_temp_dir("deep_nesting");
        let tokens_dir = temp.join("tokens");
        fs::create_dir_all(&tokens_dir).unwrap();

        let mut deep_json = String::new();
        for _ in 0..(MAX_JSON_DEPTH + 10) {
            deep_json.push_str("{\"nested\":");
        }
        deep_json.push_str("1");
        for _ in 0..(MAX_JSON_DEPTH + 10) {
            deep_json.push('}');
        }

        fs::write(tokens_dir.join("tokens.json"), deep_json).unwrap();

        let report = validate_tokens(&temp).expect("validation report");
        assert_eq!(report.diagnostics.len(), 1);
        let diag = &report.diagnostics[0];
        assert_eq!(diag.code, codes::TOKENS_MALFORMED_JSON);
        assert_eq!(diag.severity, Severity::Error);
        let loc = diag.location.as_ref().expect("has location");
        assert_eq!(loc.path, Path::new("tokens/tokens.json"));
        assert_eq!(loc.line, None, "deep depth guard location must be path only");
        assert_eq!(loc.column, None, "deep depth guard location must be path only");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_resolve_alias_returns_resolved_value_and_chains() {
        let content = r##"{
  "color": {
    "base": {
      "blue": {
        "$value": "#2563eb"
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
        let root_val: serde_json::Value = serde_json::from_str(content).unwrap();

        let direct = resolve_alias(&root_val, "color.base.blue");
        assert_eq!(direct, Some(&serde_json::Value::String("#2563eb".to_string())));

        let chained1 = resolve_alias(&root_val, "color.brand.primary");
        assert_eq!(chained1, Some(&serde_json::Value::String("#2563eb".to_string())));

        let chained2 = resolve_alias(&root_val, "color.accent");
        assert_eq!(chained2, Some(&serde_json::Value::String("#2563eb".to_string())));

        let with_braces = resolve_alias(&root_val, "{color.accent}");
        assert_eq!(with_braces, Some(&serde_json::Value::String("#2563eb".to_string())));

        let missing = resolve_alias(&root_val, "color.missing");
        assert_eq!(missing, None);
    }
}
