use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn plugin_manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugin.json")
}

#[test]
fn plugin_manifest_has_exact_shape() {
    let path = plugin_manifest_path();
    let content = fs::read_to_string(&path).expect("plugin.json must be readable");
    let value: serde_json::Value =
        serde_json::from_str(&content).expect("plugin.json must be valid JSON");
    let object = value
        .as_object()
        .expect("plugin.json must contain a JSON object");

    let expected_keys: BTreeSet<&str> = ["$schema", "name"].into_iter().collect();
    let actual_keys: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected_keys.difference(&actual_keys).copied().collect();
    let extra: Vec<_> = actual_keys.difference(&expected_keys).copied().collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "plugin.json keys differ; missing: {missing:?}; extra: {extra:?}"
    );

    assert_eq!(
        object.get("$schema").and_then(serde_json::Value::as_str),
        Some("https://agent-plugins.org/schemas/1.0.0/plugin.schema.json")
    );
    assert_eq!(object.get("name").and_then(serde_json::Value::as_str), Some("wda"));
}
