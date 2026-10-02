//! Smoke tests for pinned parsing dependencies and span composition.
//!
//! Pinned dependency capabilities:
//! 1. JSON Schema validation: `jsonschema` (0.51.0)
//! 2. JSON parsing preserving byte offsets: `json-sourcemap` (0.2.0)
//! 3. YAML front-matter parsing preserving byte offsets: `marked-yaml` (0.8.0)
//! 4. Tolerant HTML5 parsing exposing element source positions: `html5gum` (0.8.4)
//! 5. Schema violation to span composition: `jsonschema` instance pointer joined with `json-sourcemap` pointer map.

#[test]
fn test_json_schema_validation_capability() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "projectName": { "type": "string" }
        },
        "required": ["projectName"]
    });
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    let instance = serde_json::json!({});
    let errors: Vec<_> = validator.iter_errors(&instance).collect();
    assert!(!errors.is_empty(), "expected validation errors for missing required property");
    assert_eq!(errors[0].instance_path().to_string(), "");
}

#[test]
fn test_json_parsing_spans_capability() {
    let json_str = r#"{
  "projectName": "test"
}"#;
    let res = json_sourcemap::parse(json_str, json_sourcemap::Options::default())
        .expect("valid JSON");
    let loc = res
        .pointers
        .get("/projectName")
        .expect("pointer exists")
        .key();
    assert!(loc.line > 0, "line must be non-zero");
    assert!(loc.column > 0, "column must be non-zero");
    assert!(loc.pos > 0, "pos byte offset must be non-zero");
}

#[test]
fn test_yaml_parsing_spans_capability() {
    let yaml_str = "title: Test Document\nstatus: active\n";
    let node = marked_yaml::parse_yaml(0, yaml_str).expect("valid YAML");
    let span = node.span();
    let start = span.start().expect("has start marker");
    assert!(start.line() > 0, "line must be non-zero");
    assert!(start.column() > 0, "column must be non-zero");
}

#[test]
fn test_html5_parsing_spans_capability() {
    let html_str = "\n\n<html lang=\"en\">\n<body>\n<h1>Hello</h1>\n</body>\n</html>";
    let emitter = html5gum::DefaultEmitter::<usize>::new_with_span();
    let tokenizer = html5gum::Tokenizer::new_with_emitter(html_str, emitter);
    let mut found_html_tag = false;
    let mut start_pos = 0;

    for token in tokenizer {
        if let Ok(html5gum::Token::StartTag(tag)) = token {
            if tag.name.as_slice() == b"html" {
                found_html_tag = true;
                start_pos = tag.span.start;
                break;
            }
        }
    }

    assert!(found_html_tag, "should find html start tag");
    assert!(start_pos > 0, "start position byte offset must be non-zero");
}

#[test]
fn test_schema_violation_span_composition() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "designSystem": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "additionalProperties": false
            }
        },
        "required": ["designSystem"]
    });

    let json_str = r#"{
  "designSystem": {
    "name": 123,
    "unknownField": "bad"
  }
}"#;

    let instance: serde_json::Value =
        serde_json::from_str(json_str).expect("instance parses into Value");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");

    let parsed_sourcemap = json_sourcemap::parse(json_str, json_sourcemap::Options::default())
        .expect("json sourcemap parses");

    let errors: Vec<_> = validator.iter_errors(&instance).collect();
    assert!(!errors.is_empty(), "expected validation errors");

    // 1. Check type violation on /designSystem/name
    let type_err = errors
        .iter()
        .find(|e| e.instance_path().to_string() == "/designSystem/name")
        .expect("type violation on /designSystem/name should exist");

    let type_pointer = type_err.instance_path().to_string();
    let type_loc = parsed_sourcemap
        .pointers
        .get(&type_pointer)
        .expect("type violation pointer resolves in sourcemap")
        .key();

    assert!(type_loc.line > 0, "type error line must be non-zero");
    assert!(type_loc.column > 0, "type error column must be non-zero");

    // 2. Check additionalProperties violation on /designSystem/unknownField
    let add_err = errors
        .iter()
        .find(|e| matches!(e.kind(), jsonschema::error::ValidationErrorKind::AdditionalProperties { .. }))
        .expect("additionalProperties violation should exist");

    if let jsonschema::error::ValidationErrorKind::AdditionalProperties { unexpected } = add_err.kind() {
        let base_path = add_err.instance_path().to_string();
        let first_unexpected = &unexpected[0];
        let full_pointer = format!("{}/{}", base_path, first_unexpected);

        let add_loc = parsed_sourcemap
            .pointers
            .get(&full_pointer)
            .expect("unknownField pointer resolves in sourcemap")
            .key();

        assert!(add_loc.line > 0, "unknown field error line must be non-zero");
        assert!(add_loc.column > 0, "unknown field error column must be non-zero");
    } else {
        panic!("expected AdditionalProperties error kind");
    }
}
