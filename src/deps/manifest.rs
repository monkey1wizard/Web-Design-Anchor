//! Canonical `package.json` read/write for `wda deps`.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::diagnostics::ToolFault;

/// Resolves the path to `package.json` given a file or directory path.
fn resolve_manifest_path(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.join("package.json")
    } else if path.file_name() == Some(std::ffi::OsStr::new("package.json")) {
        path.to_path_buf()
    } else if path.is_file() {
        path.to_path_buf()
    } else if path.extension().is_none() {
        path.join("package.json")
    } else {
        path.to_path_buf()
    }
}

/// Reads the `dependencies` map from `package.json` at `path`.
///
/// If `package.json` is absent, returns an empty map with no error.
/// If `package.json` exists, parses the JSON and extracts the `dependencies` map.
pub fn read_manifest(path: &Path) -> Result<BTreeMap<String, String>, ToolFault> {
    let manifest_path = resolve_manifest_path(path);
    let contents = match fs::read_to_string(&manifest_path) {
        Ok(s) => s,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(err) => {
            return Err(ToolFault::new(
                format!("Failed to read package.json at {}: {}", manifest_path.display(), err),
                Some(manifest_path),
            ));
        }
    };

    let value: serde_json::Value = serde_json::from_str(&contents).map_err(|err| {
        ToolFault::new(
            format!("Failed to parse package.json at {}: {}", manifest_path.display(), err),
            Some(manifest_path.clone()),
        )
    })?;

    let root_obj = value.as_object().ok_or_else(|| {
        ToolFault::new(
            format!("Invalid package.json at {}: root must be an object", manifest_path.display()),
            Some(manifest_path.clone()),
        )
    })?;

    let Some(deps_val) = root_obj.get("dependencies") else {
        return Ok(BTreeMap::new());
    };

    let deps_obj = deps_val.as_object().ok_or_else(|| {
        ToolFault::new(
            format!("Invalid package.json at {}: 'dependencies' must be an object", manifest_path.display()),
            Some(manifest_path.clone()),
        )
    })?;

    let mut dependencies = BTreeMap::new();
    for (k, v) in deps_obj {
        let version = v.as_str().ok_or_else(|| {
            ToolFault::new(
                format!(
                    "Invalid dependency version for '{}' in {}: value must be a string",
                    k,
                    manifest_path.display()
                ),
                Some(manifest_path.clone()),
            )
        })?;
        dependencies.insert(k.clone(), version.to_string());
    }

    Ok(dependencies)
}

/// Reads dependencies from `package.json` at `path`.
///
/// Direct alias for [`read_manifest`].
pub fn read_dependencies(path: &Path) -> Result<BTreeMap<String, String>, ToolFault> {
    read_manifest(path)
}

/// Serializes only the `dependencies` key as UTF-8 JSON with deterministic sorted key order.
pub fn serialize_manifest(dependencies: &BTreeMap<String, String>) -> Result<String, ToolFault> {
    let mut manifest = serde_json::Map::new();
    let deps_value = serde_json::to_value(dependencies).map_err(|err| {
        ToolFault::new(
            format!("Failed to serialize dependencies: {err}"),
            Option::<PathBuf>::None,
        )
    })?;
    manifest.insert("dependencies".to_string(), deps_value);

    let mut rendered = serde_json::to_string_pretty(&manifest).map_err(|err| {
        ToolFault::new(
            format!("Failed to format manifest JSON: {err}"),
            Option::<PathBuf>::None,
        )
    })?;
    rendered.push('\n');
    Ok(rendered)
}

/// Serializes dependencies into canonical manifest JSON string.
///
/// Direct alias for [`serialize_manifest`].
pub fn serialize_dependencies(dependencies: &BTreeMap<String, String>) -> Result<String, ToolFault> {
    serialize_manifest(dependencies)
}

/// Writes `dependencies` to `package.json` at `path`.
///
/// Serializes only the `dependencies` key as UTF-8 JSON with deterministic sorted key order.
pub fn write_manifest(path: &Path, dependencies: &BTreeMap<String, String>) -> Result<(), ToolFault> {
    let manifest_path = resolve_manifest_path(path);
    let content = serialize_manifest(dependencies)?;

    if let Some(parent) = manifest_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(|err| {
                ToolFault::new(
                    format!("Failed to create parent directory for {}: {}", manifest_path.display(), err),
                    Some(manifest_path.clone()),
                )
            })?;
        }
    }

    fs::write(&manifest_path, content.as_bytes()).map_err(|err| {
        ToolFault::new(
            format!("Failed to write package.json at {}: {}", manifest_path.display(), err),
            Some(manifest_path),
        )
    })?;

    Ok(())
}

/// Writes dependencies to `package.json` at `path`.
///
/// Direct alias for [`write_manifest`].
pub fn write_dependencies(path: &Path, dependencies: &BTreeMap<String, String>) -> Result<(), ToolFault> {
    write_manifest(path, dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_manifest_absent_yields_empty_map() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_manifest_absent_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let deps = read_manifest(&temp_dir).expect("absent package.json must not error");
        assert!(deps.is_empty(), "expected empty map when package.json absent");

        let direct_path = temp_dir.join("package.json");
        let deps_direct = read_manifest(&direct_path).expect("absent direct path must not error");
        assert!(deps_direct.is_empty());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_read_manifest_with_dependencies() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_manifest_read_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let manifest_content = r#"{
  "name": "sample-project",
  "dependencies": {
    "zebra": "1.0.0",
    "alpha": "^2.1.0"
  }
}
"#;
        fs::write(temp_dir.join("package.json"), manifest_content).unwrap();

        let deps = read_manifest(&temp_dir).expect("reading existing package.json");
        assert_eq!(deps.len(), 2);
        assert_eq!(deps.get("zebra"), Some(&"1.0.0".to_string()));
        assert_eq!(deps.get("alpha"), Some(&"^2.1.0".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_read_manifest_without_dependencies_key() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_manifest_no_deps_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let manifest_content = r#"{
  "name": "sample-project"
}
"#;
        fs::write(temp_dir.join("package.json"), manifest_content).unwrap();

        let deps = read_manifest(&temp_dir).expect("reading package.json without dependencies");
        assert!(deps.is_empty());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_write_manifest_serializes_only_dependencies_key_deterministic_order() {
        let mut deps = BTreeMap::new();
        deps.insert("zebra".to_string(), "1.0.0".to_string());
        deps.insert("alpha".to_string(), "^2.0.0".to_string());
        deps.insert("middle".to_string(), "0.5.0".to_string());

        let serialized = serialize_manifest(&deps).expect("serialize manifest");
        let expected = "{\n  \"dependencies\": {\n    \"alpha\": \"^2.0.0\",\n    \"middle\": \"0.5.0\",\n    \"zebra\": \"1.0.0\"\n  }\n}\n";
        assert_eq!(serialized, expected);

        let value: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.len(), 1, "manifest must contain only dependencies key");
        assert!(obj.contains_key("dependencies"));
    }

    #[test]
    fn test_roundtrip_two_entry_map_and_identical_byte_sequences() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_manifest_roundtrip_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let mut original = BTreeMap::new();
        original.insert("pkg-b".to_string(), "2.0.0".to_string());
        original.insert("pkg-a".to_string(), "1.0.0".to_string());

        // Write first time
        write_manifest(&temp_dir, &original).expect("first write succeeds");
        let bytes_first = fs::read(temp_dir.join("package.json")).unwrap();

        // Read back and assert equality
        let read_back = read_manifest(&temp_dir).expect("read back succeeds");
        assert_eq!(read_back, original);

        // Write second time and assert identical byte sequences
        write_manifest(&temp_dir, &original).expect("second write succeeds");
        let bytes_second = fs::read(temp_dir.join("package.json")).unwrap();

        assert_eq!(bytes_first, bytes_second, "consecutive writes of same map must be byte-identical");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_malformed_json_returns_tool_fault() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_manifest_malformed_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        fs::write(temp_dir.join("package.json"), "{ invalid json }").unwrap();

        let err = read_manifest(&temp_dir).unwrap_err();
        assert!(err.message.contains("Failed to parse package.json"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_non_object_root_returns_tool_fault() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_manifest_non_obj_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        fs::write(temp_dir.join("package.json"), "[\"not\", \"an\", \"object\"]").unwrap();

        let err = read_manifest(&temp_dir).unwrap_err();
        assert!(err.message.contains("root must be an object"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_non_string_version_returns_tool_fault() {
        let temp_dir = std::env::temp_dir().join(format!("wda_test_manifest_non_str_ver_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        fs::write(temp_dir.join("package.json"), "{\"dependencies\": {\"pkg\": 123}}").unwrap();

        let err = read_manifest(&temp_dir).unwrap_err();
        assert!(err.message.contains("value must be a string"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
