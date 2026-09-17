//! Diagnostic code registry for Web Design Anchor (WDA).
//!
//! Diagnostic codes are the only stable machine-facing identifier in V1.
//! All diagnostic codes match the scheme `wda.<area>.<condition>`.
//!
//! No module outside `src/codes.rs` may declare a diagnostic code literal.

pub const COMMAND_NOT_IMPLEMENTED: &'static str = "wda.command.not-implemented";
pub const CLI_JSON_NOT_SUPPORTED: &'static str = "wda.cli.json-not-supported";
pub const CLI_USAGE: &'static str = "wda.cli.usage";

pub const CONTRACT_MISSING: &'static str = "wda.contract.missing";
pub const CONTRACT_MALFORMED_JSON: &'static str = "wda.contract.malformed-json";
pub const CONTRACT_UNKNOWN_FIELD: &'static str = "wda.contract.unknown-field";
pub const CONTRACT_SCHEMA_VIOLATION: &'static str = "wda.contract.schema-violation";

pub const DOCS_REQUIRED_DOCUMENT_MISSING: &'static str = "wda.docs.required-document-missing";
pub const DOCS_FRONT_MATTER_MISSING: &'static str = "wda.docs.front-matter-missing";
pub const DOCS_FRONT_MATTER_MALFORMED: &'static str = "wda.docs.front-matter-malformed";
pub const DOCS_FRONT_MATTER_FIELD_INVALID: &'static str = "wda.docs.front-matter-field-invalid";
pub const DOCS_SOURCE_REF_UNRESOLVABLE: &'static str = "wda.docs.source-ref-unresolvable";
pub const DOCS_SOURCE_REF_ESCAPES_ROOT: &'static str = "wda.docs.source-ref-escapes-root";

pub const HTML_LANG_MISSING: &'static str = "wda.html.lang-missing";
pub const HTML_IMG_ALT_MISSING: &'static str = "wda.html.img-alt-missing";
pub const HTML_DUPLICATE_ID: &'static str = "wda.html.duplicate-id";

pub const TOKENS_MALFORMED_JSON: &'static str = "wda.tokens.malformed-json";
pub const TOKENS_VALUE_MISSING: &'static str = "wda.tokens.value-missing";
pub const TOKENS_TYPE_UNRESOLVABLE: &'static str = "wda.tokens.type-unresolvable";
pub const TOKENS_ALIAS_UNRESOLVABLE: &'static str = "wda.tokens.alias-unresolvable";

pub const TOOL_FAULT: &'static str = "wda.tool.fault";

pub const INIT_TOOL_UNAVAILABLE: &'static str = "wda.init.tool-unavailable";
pub const INIT_PATH_CONFLICT: &'static str = "wda.init.path-conflict";
pub const INIT_GIT_IDENTITY_FALLBACK: &'static str = "wda.init.git-identity-fallback";

pub const BUILD_TOKENS_CSS_IN_SOURCE: &'static str = "wda.build.tokens-css-in-source";
pub const BUILD_INCLUDE_UNRESOLVABLE: &'static str = "wda.build.include-unresolvable";
pub const BUILD_INCLUDE_ESCAPES_ROOT: &'static str = "wda.build.include-escapes-root";
pub const BUILD_INCLUDE_CYCLE: &'static str = "wda.build.include-cycle";
pub const BUILD_TYPE_ERROR: &'static str = "wda.build.type-error";
pub const BUILD_TOOL_UNAVAILABLE: &'static str = "wda.build.tool-unavailable";
pub const BUILD_SECRET_PATH_REFERENCED: &'static str = "wda.build.secret-path-referenced";
pub const BUILD_REFERENCE_UNRESOLVABLE: &'static str = "wda.build.reference-unresolvable";
pub const BUILD_UNREFERENCED_ASSET: &'static str = "wda.build.unreferenced-asset";
pub const BUILD_OUTPUT_PATH_COLLISION: &'static str = "wda.build.output-path-collision";
pub const BUILD_DEPENDENCY_UNLOCKED: &'static str = "wda.build.dependency-unlocked";

/// Slice enumerating all diagnostic code constants in the registry.
pub const ALL_CODES: &[&'static str] = &[
    COMMAND_NOT_IMPLEMENTED,
    CLI_JSON_NOT_SUPPORTED,
    CLI_USAGE,
    CONTRACT_MISSING,
    CONTRACT_MALFORMED_JSON,
    CONTRACT_UNKNOWN_FIELD,
    CONTRACT_SCHEMA_VIOLATION,
    DOCS_REQUIRED_DOCUMENT_MISSING,
    DOCS_FRONT_MATTER_MISSING,
    DOCS_FRONT_MATTER_MALFORMED,
    DOCS_FRONT_MATTER_FIELD_INVALID,
    DOCS_SOURCE_REF_UNRESOLVABLE,
    DOCS_SOURCE_REF_ESCAPES_ROOT,
    HTML_LANG_MISSING,
    HTML_IMG_ALT_MISSING,
    HTML_DUPLICATE_ID,
    TOKENS_MALFORMED_JSON,
    TOKENS_VALUE_MISSING,
    TOKENS_TYPE_UNRESOLVABLE,
    TOKENS_ALIAS_UNRESOLVABLE,
    TOOL_FAULT,
    INIT_TOOL_UNAVAILABLE,
    INIT_PATH_CONFLICT,
    INIT_GIT_IDENTITY_FALLBACK,
    BUILD_TOKENS_CSS_IN_SOURCE,
    BUILD_INCLUDE_UNRESOLVABLE,
    BUILD_INCLUDE_ESCAPES_ROOT,
    BUILD_INCLUDE_CYCLE,
    BUILD_TYPE_ERROR,
    BUILD_TOOL_UNAVAILABLE,
    BUILD_SECRET_PATH_REFERENCED,
    BUILD_REFERENCE_UNRESOLVABLE,
    BUILD_UNREFERENCED_ASSET,
    BUILD_OUTPUT_PATH_COLLISION,
    BUILD_DEPENDENCY_UNLOCKED,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_code_registry() {
        let expected_codes = [
            "wda.command.not-implemented",
            "wda.cli.json-not-supported",
            "wda.cli.usage",
            "wda.contract.missing",
            "wda.contract.malformed-json",
            "wda.contract.unknown-field",
            "wda.contract.schema-violation",
            "wda.docs.required-document-missing",
            "wda.docs.front-matter-missing",
            "wda.docs.front-matter-malformed",
            "wda.docs.front-matter-field-invalid",
            "wda.docs.source-ref-unresolvable",
            "wda.docs.source-ref-escapes-root",
            "wda.html.lang-missing",
            "wda.html.img-alt-missing",
            "wda.html.duplicate-id",
            "wda.tokens.malformed-json",
            "wda.tokens.value-missing",
            "wda.tokens.type-unresolvable",
            "wda.tokens.alias-unresolvable",
            "wda.tool.fault",
            "wda.init.tool-unavailable",
            "wda.init.path-conflict",
            "wda.init.git-identity-fallback",
            "wda.build.tokens-css-in-source",
            "wda.build.include-unresolvable",
            "wda.build.include-escapes-root",
            "wda.build.include-cycle",
            "wda.build.type-error",
            "wda.build.tool-unavailable",
            "wda.build.secret-path-referenced",
            "wda.build.reference-unresolvable",
            "wda.build.unreferenced-asset",
            "wda.build.output-path-collision",
            "wda.build.dependency-unlocked",
        ];

        assert_eq!(
            ALL_CODES.len(),
            35,
            "slice must contain exactly 35 registry codes"
        );
        assert_eq!(
            ALL_CODES.len(),
            expected_codes.len(),
            "slice length must match expected codes length"
        );

        for (i, &code) in ALL_CODES.iter().enumerate() {
            assert_eq!(
                code, expected_codes[i],
                "code at index {} must match expected code",
                i
            );

            // Assert every value matches wda.<area>.<condition>
            let parts: Vec<&str> = code.split('.').collect();
            assert_eq!(
                parts.len(),
                3,
                "code '{}' must have exactly 3 dot-separated parts",
                code
            );
            assert_eq!(parts[0], "wda", "code '{}' must start with 'wda'", code);
            assert!(!parts[1].is_empty(), "area in '{}' must not be empty", code);
            assert!(
                !parts[2].is_empty(),
                "condition in '{}' must not be empty",
                code
            );
        }
    }
}
