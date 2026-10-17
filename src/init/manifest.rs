//! Project manifest (`wda.json`) generation and formatting for project initialization.
//!
//! Provides formatting for `projectVersion` and manifest writing for `wda.json`.

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

/// Generates the canonical `wda.json` manifest content for project initialization.
///
/// Takes a date-time parameter representing the creation instant (production supplies
/// the current local time; tests supply a fixed instant).
///
/// Serializes exactly the seven contract fields defined in the schema, recording the
/// resolved design system's name and version under `designSystem`.
pub fn generate_manifest<Tz: TimeZone>(dt: &DateTime<Tz>, resolved_ds: &ResolvedDesignSystem) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let manifest = json!({
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
    let manifest_path = root.join("wda.json");
    let contents = fs::read_to_string(&manifest_path).map_err(|error| {
        ToolFault::new(
            format!("Failed to read wda.json: {error}"),
            Some(manifest_path.clone()),
        )
    })?;
    let mut manifest: serde_json::Value = serde_json::from_str(&contents).map_err(|error| {
        ToolFault::new(
            format!("Failed to parse wda.json: {error}"),
            Some(manifest_path.clone()),
        )
    })?;

    let object = manifest.as_object_mut().ok_or_else(|| {
        ToolFault::new(
            "Failed to update wda.json: manifest root must be an object",
            Some(manifest_path.clone()),
        )
    })?;
    object.insert(
        "wdaVersion".to_owned(),
        serde_json::Value::String(env!("CARGO_PKG_VERSION").to_owned()),
    );

    fs::write(&manifest_path, serialize_manifest(&manifest)).map_err(|error| {
        ToolFault::new(
            format!("Failed to write wda.json: {error}"),
            Some(manifest_path),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::resolve::BUILTIN_DESIGN_SYSTEM_NAME;
    use chrono::{FixedOffset, NaiveDate, NaiveTime};

    /// Fixture standing in for offline, injected design-system resolution so manifest
    /// tests never reach the real online resolver's network call.
    fn offline_wda_minimal() -> ResolvedDesignSystem {
        ResolvedDesignSystem {
            name: BUILTIN_DESIGN_SYSTEM_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            fallback_reason: Some(
                super::super::resolve::FALLBACK_REASON_DENO_ABSENT.to_string(),
            ),
        }
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

        let json_str = generate_manifest(&dt, &offline_wda_minimal());
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

        let json_str = generate_manifest(&dt, &resolved_spectrum_two);
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

        let json_str = generate_manifest(&dt, &offline_wda_minimal());
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("generated manifest must be valid JSON");

        let validator = crate::validation::project_contract::get_validator()
            .expect("canonical schema validator must compile");
        assert!(
            validator.is_valid(&parsed),
            "generated manifest must pass canonical schema validation"
        );
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
        let before = fs::read_to_string(temp_dir.join("wda.json")).unwrap();

        update_wda_version(&temp_dir).unwrap();

        let after = fs::read_to_string(temp_dir.join("wda.json")).unwrap();
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
