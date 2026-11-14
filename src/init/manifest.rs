//! Project manifest (`anchor.json` and `package.json`) generation for project initialization.
//!
//! Provides formatting for `projectVersion` and manifest writing for both manifests.

use chrono::{DateTime, TimeZone};
use std::fs;
use std::path::Path;

use serde_json::json;

use crate::diagnostics::ToolFault;

use super::resolve::ResolvedDesignSystem;

/// Canonical default project name for newly initialized projects.
pub const PROJECT_NAME: &str = "Web Design Anchor Project";

/// Canonical default page architecture (`mpa`).
pub const PAGE_ARCHITECTURE: &str = "mpa";

/// Canonical default browser baseline (`baseline-widely-available`).
pub const BROWSER_BASELINE: &str = "baseline-widely-available";

/// Canonical default accessibility baseline (`wcag-2.2-aa`).
pub const ACCESSIBILITY_BASELINE: &str = "wcag-2.2-aa";

/// Formats a date-time into RFC 3339 format with seconds and numeric offset,
/// without fractional seconds.
///
/// Example: `2026-08-27T14:03:05+08:00`
pub fn format_project_version<Tz: TimeZone>(dt: &DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    dt.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

/// Generates the canonical `anchor.json` manifest content for project initialization.
///
/// Takes a date-time parameter representing the creation instant (production supplies
/// the current local time; tests supply a fixed instant).
///
/// Serializes the seven contract fields, recording the actual design system's name and
/// version under `designSystem`. Only when the resolution fell back does it add
/// `initialization.fallback` with `requested_design_system` (the system init tried
/// first) and the reason.
pub fn generate_manifest<Tz: TimeZone>(
    dt: &DateTime<Tz>,
    resolved_ds: &ResolvedDesignSystem,
    requested_design_system: &str,
) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let mut manifest = json!({
        "projectName": PROJECT_NAME,
        "projectVersion": format_project_version(dt),
        "wdaVersion": env!("CARGO_PKG_VERSION"),
        "pageArchitecture": PAGE_ARCHITECTURE,
        "browserBaseline": BROWSER_BASELINE,
        "accessibilityBaseline": ACCESSIBILITY_BASELINE,
        "designSystem": {
            "name": resolved_ds.name,
            "version": resolved_ds.version
        }
    });
    if let Some(reason) = resolved_ds.fallback_reason.as_deref() {
        manifest["initialization"] = json!({
            "fallback": {
                "requestedDesignSystem": requested_design_system,
                "reason": reason
            }
        });
    }

    serialize_manifest(&manifest)
}

/// Generates the `package.json` content for a new project.
///
/// `dependencies` are exact versions keyed by package name; `package_manager` is the
/// standard `packageManager` field value (`name@exact-version`). Minimal passes an empty
/// dependency list and `None`.
pub fn generate_package_manifest(
    dependencies: &[(String, String)],
    package_manager: Option<&str>,
) -> String {
    let deps: std::collections::BTreeMap<&str, &str> = dependencies
        .iter()
        .map(|(name, version)| (name.as_str(), version.as_str()))
        .collect();
    let mut manifest = json!({
        "name": "web-design-anchor-project",
        "private": true,
        "type": "module",
        "dependencies": deps,
    });
    if let Some(manager) = package_manager {
        manifest["packageManager"] = json!(manager);
    }
    serialize_manifest(&manifest)
}

fn serialize_manifest(manifest: &serde_json::Value) -> String {
    let mut rendered =
        serde_json::to_string_pretty(manifest).expect("manifest serialization must not fail");
    rendered.push('\n');
    rendered
}

/// Updates the running WDA version in a project's manifest.
pub(crate) fn update_wda_version(root: &Path) -> Result<(), ToolFault> {
    let name = "anchor.json";
    let manifest_path = root.join(name);
    let contents = fs::read_to_string(&manifest_path).map_err(|error| {
        ToolFault::new(
            format!("Failed to read {name}: {error}"),
            Some(manifest_path.clone()),
        )
    })?;
    let mut manifest: serde_json::Value = serde_json::from_str(&contents).map_err(|error| {
        ToolFault::new(
            format!("Failed to parse {name}: {error}"),
            Some(manifest_path.clone()),
        )
    })?;

    let object = manifest.as_object_mut().ok_or_else(|| {
        ToolFault::new(
            format!("Failed to update {name}: manifest root must be an object"),
            Some(manifest_path.clone()),
        )
    })?;
    object.insert(
        "wdaVersion".to_owned(),
        serde_json::Value::String(env!("CARGO_PKG_VERSION").to_owned()),
    );

    fs::write(&manifest_path, serialize_manifest(&manifest)).map_err(|error| {
        ToolFault::new(
            format!("Failed to write {name}: {error}"),
            Some(manifest_path),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::super::resolve::BUILTIN_DESIGN_SYSTEM_NAME;
    use super::*;
    use chrono::{FixedOffset, NaiveDate, NaiveTime};

    #[test]
    fn native_manifest_unit_version_writeback_uses_anchor() {
        let dir = std::env::temp_dir().join(format!("wda_anchor_version_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("anchor.json"), "{\"wdaVersion\":\"old\"}").unwrap();
        fs::write(dir.join("wda.json"), "legacy").unwrap();
        update_wda_version(&dir).unwrap();
        assert_eq!(fs::read_to_string(dir.join("wda.json")).unwrap(), "legacy");
        let anchor: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join("anchor.json")).unwrap()).unwrap();
        assert_eq!(anchor["wdaVersion"], env!("CARGO_PKG_VERSION"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn native_strict_manifest_cutover_unit_writeback_refuses_legacy_only_project() {
        let dir = std::env::temp_dir().join(format!("wda_strict_writeback_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("wda.json"), "{\"wdaVersion\":\"old\"}").unwrap();
        assert!(update_wda_version(&dir).is_err());
        assert!(!dir.join("anchor.json").exists());
        assert_eq!(
            fs::read_to_string(dir.join("wda.json")).unwrap(),
            "{\"wdaVersion\":\"old\"}"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    /// Fixture standing in for offline, injected design-system resolution so manifest
    /// tests never reach the real online resolver's network call.
    fn offline_wda_minimal() -> ResolvedDesignSystem {
        ResolvedDesignSystem {
            name: BUILTIN_DESIGN_SYSTEM_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            fallback_reason: Some(super::super::resolve::FALLBACK_REASON_DENO_ABSENT.to_string()),
        }
    }

    fn no_fallback_minimal() -> ResolvedDesignSystem {
        ResolvedDesignSystem {
            name: BUILTIN_DESIGN_SYSTEM_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            fallback_reason: None,
        }
    }

    #[test]
    fn native_init_transaction_unit_manifest_records_fallback_only_on_fallback() {
        let dt = chrono::Utc.with_ymd_and_hms(2026, 9, 8, 12, 0, 0).unwrap();
        let plain: serde_json::Value = serde_json::from_str(&generate_manifest(
            &dt,
            &no_fallback_minimal(),
            "Spectrum 2",
        ))
        .unwrap();
        assert!(plain.get("initialization").is_none());
        let fell_back: serde_json::Value = serde_json::from_str(&generate_manifest(
            &dt,
            &offline_wda_minimal(),
            "Spectrum 2",
        ))
        .unwrap();
        assert_eq!(
            fell_back["initialization"]["fallback"]["requestedDesignSystem"],
            "Spectrum 2"
        );
        assert!(fell_back["initialization"]["fallback"]["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()));
    }

    #[test]
    fn native_init_transaction_unit_package_manifest_has_dependencies_only_for_spectrum() {
        let minimal: serde_json::Value =
            serde_json::from_str(&generate_package_manifest(&[], None)).unwrap();
        assert_eq!(minimal["dependencies"], serde_json::json!({}));
        assert!(minimal.get("packageManager").is_none());
        let deps = vec![("lit".to_string(), "3.0.0".to_string())];
        let full: serde_json::Value =
            serde_json::from_str(&generate_package_manifest(&deps, Some("npm@10.9.2"))).unwrap();
        assert_eq!(full["dependencies"]["lit"], "3.0.0");
        assert_eq!(full["packageManager"], "npm@10.9.2");
    }

    #[test]
    fn test_format_project_version_fixed_instant() {
        let offset = FixedOffset::east_opt(8 * 3600).unwrap();
        let naive_date = NaiveDate::from_ymd_opt(2026, 8, 27).unwrap();
        let naive_time = NaiveTime::from_hms_opt(14, 3, 5).unwrap();
        let naive_dt = naive_date.and_time(naive_time);
        let dt = offset.from_local_datetime(&naive_dt).unwrap();

        let formatted = format_project_version(&dt);
        assert_eq!(formatted, "2026-08-27T14:03:05+08:00");
    }

    #[test]
    fn test_format_project_version_zero_numeric_offset() {
        let offset = FixedOffset::east_opt(0).unwrap();
        let naive_date = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let naive_time = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
        let naive_dt = naive_date.and_time(naive_time);
        let dt = offset.from_local_datetime(&naive_dt).unwrap();

        let formatted = format_project_version(&dt);
        assert_eq!(formatted, "2026-01-15T09:30:00+00:00");
    }

    #[test]
    fn test_format_project_version_negative_offset() {
        let offset = FixedOffset::west_opt(5 * 3600).unwrap();
        let naive_date = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let naive_time = NaiveTime::from_hms_opt(23, 59, 59).unwrap();
        let naive_dt = naive_date.and_time(naive_time);
        let dt = offset.from_local_datetime(&naive_dt).unwrap();

        let formatted = format_project_version(&dt);
        assert_eq!(formatted, "2026-12-31T23:59:59-05:00");
    }

    #[test]
    fn test_format_project_version_omits_fractional_seconds() {
        let offset = FixedOffset::east_opt(8 * 3600).unwrap();
        let naive_date = NaiveDate::from_ymd_opt(2026, 8, 27).unwrap();
        let naive_time = NaiveTime::from_hms_nano_opt(14, 3, 5, 987_654_321).unwrap();
        let naive_dt = naive_date.and_time(naive_time);
        let dt = offset.from_local_datetime(&naive_dt).unwrap();

        let formatted = format_project_version(&dt);
        assert_eq!(formatted, "2026-08-27T14:03:05+08:00");
        assert!(!formatted.contains('.'));
    }

    #[test]
    fn test_generate_manifest_has_seven_exact_contract_fields() {
        let offset = FixedOffset::east_opt(8 * 3600).unwrap();
        let naive_date = NaiveDate::from_ymd_opt(2026, 8, 27).unwrap();
        let naive_time = NaiveTime::from_hms_opt(14, 3, 5).unwrap();
        let naive_dt = naive_date.and_time(naive_time);
        let dt = offset.from_local_datetime(&naive_dt).unwrap();

        let json_str = generate_manifest(&dt, &no_fallback_minimal(), "Spectrum 2");
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("generated manifest must be valid JSON");

        let map = parsed
            .as_object()
            .expect("manifest root must be a JSON object");

        // Exactly seven keys
        assert_eq!(
            map.len(),
            7,
            "manifest must contain exactly 7 top-level keys"
        );

        // Verify four fixed literal fields
        assert_eq!(
            parsed["projectName"], "Web Design Anchor Project",
            "projectName must match Requirements literal"
        );
        assert_eq!(
            parsed["pageArchitecture"], "mpa",
            "pageArchitecture must match Requirements literal"
        );
        assert_eq!(
            parsed["browserBaseline"], "baseline-widely-available",
            "browserBaseline must match Requirements literal"
        );
        assert_eq!(
            parsed["accessibilityBaseline"], "wcag-2.2-aa",
            "accessibilityBaseline must match Requirements literal"
        );

        // Verify projectVersion timestamp format
        assert_eq!(
            parsed["projectVersion"], "2026-08-27T14:03:05+08:00",
            "projectVersion must match formatted timestamp"
        );

        // Verify two running version fields
        let running_version = env!("CARGO_PKG_VERSION");
        assert_eq!(
            parsed["wdaVersion"], running_version,
            "wdaVersion must equal running package version"
        );
        assert_eq!(
            parsed["designSystem"]["version"], running_version,
            "designSystem.version must equal running package version"
        );

        // Verify designSystem object
        let ds_map = parsed["designSystem"]
            .as_object()
            .expect("designSystem must be a JSON object");
        assert_eq!(
            ds_map.len(),
            2,
            "designSystem object must contain exactly 2 keys"
        );
        assert_eq!(
            parsed["designSystem"]["name"], "WDA Minimal",
            "designSystem.name must equal WDA Minimal"
        );
    }

    #[test]
    fn test_generate_manifest_records_spectrum_two_resolution() {
        let offset = FixedOffset::east_opt(8 * 3600).unwrap();
        let naive_date = NaiveDate::from_ymd_opt(2026, 9, 6).unwrap();
        let naive_time = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let naive_dt = naive_date.and_time(naive_time);
        let dt = offset.from_local_datetime(&naive_dt).unwrap();

        let resolved_spectrum_two = ResolvedDesignSystem {
            name: super::super::resolve::SPECTRUM_TWO_DESIGN_SYSTEM_NAME.to_string(),
            version: "1.2.3".to_string(),
            fallback_reason: None,
        };

        let json_str = generate_manifest(&dt, &resolved_spectrum_two, "Spectrum 2");
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("generated manifest must be valid JSON");

        assert_eq!(
            parsed["designSystem"]["name"], "Spectrum 2",
            "designSystem.name must record the resolved Spectrum 2 name, not the built-in Minimal name"
        );
        assert_eq!(
            parsed["designSystem"]["version"], "1.2.3",
            "designSystem.version must record the resolved Spectrum 2 version, not the running package version"
        );
    }

    #[test]
    fn test_generate_manifest_conforms_to_canonical_schema() {
        let offset = FixedOffset::east_opt(8 * 3600).unwrap();
        let naive_date = NaiveDate::from_ymd_opt(2026, 8, 27).unwrap();
        let naive_time = NaiveTime::from_hms_opt(14, 3, 5).unwrap();
        let naive_dt = naive_date.and_time(naive_time);
        let dt = offset.from_local_datetime(&naive_dt).unwrap();

        let json_str = generate_manifest(&dt, &offline_wda_minimal(), "Spectrum 2");
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("generated manifest must be valid JSON");

        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_manifest_schema_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("anchor.json"), &json_str).unwrap();
        fs::write(
            temp_dir.join("package.json"),
            generate_package_manifest(&[], None),
        )
        .unwrap();
        let report = crate::validation::project_contract::validate_project_contract(&temp_dir)
            .expect("contract validation must run");
        assert!(
            report.diagnostics.is_empty(),
            "generated manifests must pass contract validation: {:?}",
            report.diagnostics
        );
        assert!(parsed.is_object());
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_update_wda_version_changes_only_version_line() {
        let temp_dir = std::env::temp_dir().join(format!(
            "wda_test_manifest_update_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();

        let offset = FixedOffset::east_opt(8 * 3600).unwrap();
        let naive_date = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        let naive_time = NaiveTime::from_hms_opt(15, 0, 0).unwrap();
        let dt = offset
            .from_local_datetime(&naive_date.and_time(naive_time))
            .unwrap();
        crate::init::init_project_with_time_and_resolution(
            &temp_dir,
            &dt,
            Some(offline_wda_minimal()),
        )
        .unwrap();
        let before = fs::read_to_string(temp_dir.join("anchor.json")).unwrap();

        update_wda_version(&temp_dir).unwrap();

        let after = fs::read_to_string(temp_dir.join("anchor.json")).unwrap();
        let before_without_version = before
            .lines()
            .filter(|line| !line.contains("\"wdaVersion\""))
            .collect::<Vec<_>>();
        let after_without_version = after
            .lines()
            .filter(|line| !line.contains("\"wdaVersion\""))
            .collect::<Vec<_>>();
        assert_eq!(before_without_version, after_without_version);
        assert!(after.contains(&format!(
            "\"wdaVersion\": \"{}\"",
            env!("CARGO_PKG_VERSION")
        )));

        fs::remove_dir_all(temp_dir).unwrap();
    }
}
