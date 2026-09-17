//! Design System resolution seam for project initialization.
//!
//! Resolves the design system to use for project initialization online, targeting
//! Spectrum 2, by reusing the deno-backed resolver in [`crate::deps::resolve`] to
//! query the npm registry for the Spectrum Web Components bundle package's latest
//! distribution tag. Falls back to the built-in WDA Minimal design system, carrying
//! a fallback reason naming exactly which of the seven failure branches triggered,
//! whenever online resolution cannot complete.

use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::deps::resolve::resolve_deno_with_path;

/// Resolved design system metadata resulting from resolution.
///
/// Fields are owned strings rather than `&'static str` because an online-resolved
/// version (e.g. an exact semver pulled from the npm registry) cannot be static.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDesignSystem {
    pub name: String,
    pub version: String,
    pub fallback_reason: Option<String>,
}

/// Built-in design system name for WDA Minimal.
pub const BUILTIN_DESIGN_SYSTEM_NAME: &str = "WDA Minimal";

/// Design system name resolved online, targeting Adobe's Spectrum 2.
pub const SPECTRUM_TWO_DESIGN_SYSTEM_NAME: &str = "Spectrum 2";

/// The npm package queried for the Spectrum Web Components bundle's latest
/// distribution tag, used only for version discovery. It is not itself added
/// as a project dependency: `crate::init::SPECTRUM_TWO_DIRECT_DEPENDENCY_PACKAGES`
/// names the direct-dependency set actually pinned into a generated project.
pub(crate) const SPECTRUM_NPM_PACKAGE: &str = "@spectrum-web-components/bundle";

/// Fixed cap on the online resolution subprocess. This is new mechanism, not reuse:
/// neither `src/deps/resolve.rs` nor `src/deps/mod.rs` carries any timeout or
/// `Duration` usage, and `invoke_staged_deno` waits blockingly.
const RESOLUTION_TIMEOUT: Duration = Duration::from_secs(10);

/// Fallback reason: `deno` could not be resolved on PATH.
pub(crate) const FALLBACK_REASON_DENO_ABSENT: &str =
    "Online Design System resolution requires 'deno', which was not found on PATH";

/// Fallback reason: the deno subprocess exited with a non-zero status.
pub(crate) const FALLBACK_REASON_NON_ZERO_EXIT: &str =
    "Online Design System resolution failed because the deno command exited with a non-zero status";

/// Fallback reason: the deno subprocess did not complete within the ten-second cap.
pub(crate) const FALLBACK_REASON_TIMEOUT: &str =
    "Online Design System resolution timed out after ten seconds";

/// Fallback reason: the npm registry response could not be parsed as JSON.
pub(crate) const FALLBACK_REASON_UNPARSEABLE_OUTPUT: &str =
    "Online Design System resolution failed because the npm registry response could not be parsed as JSON";

/// Fallback reason: the npm registry response omitted the `latest` distribution tag.
pub(crate) const FALLBACK_REASON_MISSING_FIELD: &str =
    "Online Design System resolution failed because the npm registry response did not include a 'latest' distribution tag";

/// Fallback reason: the resolved `latest` distribution tag is a pre-release version.
pub(crate) const FALLBACK_REASON_NON_RELEASE_VERSION: &str =
    "Online Design System resolution failed because the latest distribution tag is a pre-release version";

/// Fallback reason: the resolved `latest` distribution tag is below `1.0.0`.
pub(crate) const FALLBACK_REASON_VERSION_BELOW_1_0_0: &str =
    "Online Design System resolution failed because the latest distribution tag is below 1.0.0";

/// The raw outcome of the timed deno subprocess invocation used for online
/// resolution, standing in for a real process wait/timeout/output capture so tests
/// can drive each of the six post-deno-presence failure branches deterministically.
#[derive(Debug, Clone)]
pub(crate) enum RawInvocation {
    /// The process exited within the ten-second cap, carrying its success flag and
    /// captured standard output.
    Completed { success: bool, stdout: Vec<u8> },
    /// The process did not exit within the ten-second cap.
    TimedOut,
}

/// Resolves the design system to use for project initialization.
///
/// Attempts online resolution targeting Spectrum 2 by querying the npm registry for
/// the Spectrum Web Components bundle package's latest distribution tag through the
/// existing deno-backed resolver in [`crate::deps::resolve`]. On any of the seven
/// failure branches, returns the built-in "WDA Minimal" design system at the running
/// package version, carrying a fallback reason naming which branch triggered.
pub fn resolve_design_system() -> ResolvedDesignSystem {
    resolve_design_system_with(std::env::var_os("PATH").as_deref(), None)
}

/// Resolves the design system with an explicit PATH and an optional injected
/// subprocess result, shaped like [`crate::deps::add_with_path`]'s PATH-injection
/// seam at `src/deps/mod.rs:215`: the production entry above always passes `None`
/// here and reaches the real timed `deno` subprocess; tests pass
/// `Some(RawInvocation)` to drive each of the six post-deno-presence failure
/// branches deterministically. The seventh branch (deno absent) is driven by
/// `path_env` alone, with `injected` left `None`, so `resolve_deno_with_path` still
/// runs and fails on a PATH lacking `deno`.
pub(crate) fn resolve_design_system_with(
    path_env: Option<&OsStr>,
    injected: Option<RawInvocation>,
) -> ResolvedDesignSystem {
    let outcome: Result<String, &'static str> = if let Some(raw) = injected {
        resolve_version_from_raw(raw)
    } else {
        match resolve_deno_with_path(path_env) {
            Ok(deno_binary) => resolve_version_from_raw(invoke_online_resolution(&deno_binary)),
            Err(_) => Err(FALLBACK_REASON_DENO_ABSENT),
        }
    };

    match outcome {
        Ok(version) => ResolvedDesignSystem {
            name: SPECTRUM_TWO_DESIGN_SYSTEM_NAME.to_string(),
            version,
            fallback_reason: None,
        },
        Err(reason) => builtin_fallback(reason),
    }
}

/// Builds the built-in WDA Minimal fallback result carrying `reason`.
fn builtin_fallback(reason: &str) -> ResolvedDesignSystem {
    ResolvedDesignSystem {
        name: BUILTIN_DESIGN_SYSTEM_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        fallback_reason: Some(reason.to_string()),
    }
}

/// Maps a raw subprocess outcome to a resolved Spectrum 2 version, or to the failure
/// reason naming which of the six post-deno-presence branches triggered.
fn resolve_version_from_raw(raw: RawInvocation) -> Result<String, &'static str> {
    let stdout = match raw {
        RawInvocation::TimedOut => return Err(FALLBACK_REASON_TIMEOUT),
        RawInvocation::Completed {
            success: false,
            stdout: _,
        } => return Err(FALLBACK_REASON_NON_ZERO_EXIT),
        RawInvocation::Completed {
            success: true,
            stdout,
        } => stdout,
    };

    let text = String::from_utf8_lossy(&stdout);
    let value: serde_json::Value =
        serde_json::from_str(text.trim()).map_err(|_| FALLBACK_REASON_UNPARSEABLE_OUTPUT)?;
    let latest = value
        .get("latest")
        .and_then(|v| v.as_str())
        .ok_or(FALLBACK_REASON_MISSING_FIELD)?;

    if is_prerelease(latest) {
        return Err(FALLBACK_REASON_NON_RELEASE_VERSION);
    }
    if major_version(latest).unwrap_or(0) < 1 {
        return Err(FALLBACK_REASON_VERSION_BELOW_1_0_0);
    }

    Ok(latest.to_string())
}

/// Returns `true` if `version` carries a semver pre-release identifier (a `-` before
/// any build-metadata `+`).
fn is_prerelease(version: &str) -> bool {
    version.split('+').next().unwrap_or(version).contains('-')
}

/// Extracts the leading major version component, if `version` starts with a valid
/// non-negative integer segment.
fn major_version(version: &str) -> Option<u64> {
    version.split(['.', '-', '+']).next()?.parse().ok()
}

/// Invokes `deno` to query the npm registry for [`SPECTRUM_NPM_PACKAGE`]'s
/// distribution tags, polling with `try_wait` under a fixed ten-second cap rather
/// than blocking indefinitely (unlike `invoke_staged_deno`, which waits blockingly).
fn invoke_online_resolution(deno_binary: &Path) -> RawInvocation {
    let script = format!(
        "const r = await fetch('https://registry.npmjs.org/{SPECTRUM_NPM_PACKAGE}'); \
         const j = await r.json(); \
         console.log(JSON.stringify(j['dist-tags'] ?? {{}}));"
    );

    let mut child = match Command::new(deno_binary)
        .args(["eval", "--allow-net=registry.npmjs.org", &script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            return RawInvocation::Completed {
                success: false,
                stdout: Vec::new(),
            };
        }
    };

    let deadline = Instant::now() + RESOLUTION_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    let _ = out.read_to_end(&mut stdout);
                }
                return RawInvocation::Completed {
                    success: status.success(),
                    stdout,
                };
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return RawInvocation::TimedOut;
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => {
                return RawInvocation::Completed {
                    success: false,
                    stdout: Vec::new(),
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_design_system_falls_back_to_wda_minimal_when_deno_absent() {
        let resolved = resolve_design_system_with(Some(OsStr::new("")), None);
        assert_eq!(resolved.name, BUILTIN_DESIGN_SYSTEM_NAME);
        assert_eq!(resolved.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(
            resolved.fallback_reason.as_deref(),
            Some(FALLBACK_REASON_DENO_ABSENT)
        );
    }

    #[test]
    fn test_resolve_design_system_online_success_returns_spectrum_two() {
        let raw = RawInvocation::Completed {
            success: true,
            stdout: br#"{"latest":"1.2.3"}"#.to_vec(),
        };
        let resolved = resolve_design_system_with(None, Some(raw));
        assert_eq!(resolved.name, SPECTRUM_TWO_DESIGN_SYSTEM_NAME);
        assert_eq!(resolved.version, "1.2.3");
        assert!(resolved.fallback_reason.is_none());
    }

    /// Per-branch honesty: this replaces the prior single fallback-reason-wording
    /// test (which asserted no reason could ever mention network/timeout/offline
    /// wording — a rule this change makes false, since online resolution can now
    /// genuinely time out or fail to connect). Each of the seven failure branches is
    /// driven by its own fixture and asserted to fall back to WDA Minimal with
    /// exactly its own reason string, with no branch's reason bleeding into another.
    #[test]
    fn test_each_failure_branch_falls_back_with_its_own_honest_reason() {
        let deno_absent = resolve_design_system_with(Some(OsStr::new("")), None);
        let non_zero_exit = resolve_design_system_with(
            None,
            Some(RawInvocation::Completed {
                success: false,
                stdout: Vec::new(),
            }),
        );
        let timed_out = resolve_design_system_with(None, Some(RawInvocation::TimedOut));
        let unparseable = resolve_design_system_with(
            None,
            Some(RawInvocation::Completed {
                success: true,
                stdout: b"not json".to_vec(),
            }),
        );
        let missing_field = resolve_design_system_with(
            None,
            Some(RawInvocation::Completed {
                success: true,
                stdout: br#"{"beta":"1.0.0"}"#.to_vec(),
            }),
        );
        let non_release = resolve_design_system_with(
            None,
            Some(RawInvocation::Completed {
                success: true,
                stdout: br#"{"latest":"2.0.0-beta.1"}"#.to_vec(),
            }),
        );
        let below_1_0_0 = resolve_design_system_with(
            None,
            Some(RawInvocation::Completed {
                success: true,
                stdout: br#"{"latest":"0.9.5"}"#.to_vec(),
            }),
        );

        let branches = [
            ("deno absent", &deno_absent, FALLBACK_REASON_DENO_ABSENT),
            (
                "non-zero exit",
                &non_zero_exit,
                FALLBACK_REASON_NON_ZERO_EXIT,
            ),
            ("timeout", &timed_out, FALLBACK_REASON_TIMEOUT),
            (
                "unparseable output",
                &unparseable,
                FALLBACK_REASON_UNPARSEABLE_OUTPUT,
            ),
            (
                "missing field",
                &missing_field,
                FALLBACK_REASON_MISSING_FIELD,
            ),
            (
                "non-release version",
                &non_release,
                FALLBACK_REASON_NON_RELEASE_VERSION,
            ),
            (
                "version below 1.0.0",
                &below_1_0_0,
                FALLBACK_REASON_VERSION_BELOW_1_0_0,
            ),
        ];

        for (label, resolved, expected_reason) in branches {
            assert_eq!(
                resolved.name, BUILTIN_DESIGN_SYSTEM_NAME,
                "{label} must fall back to the Minimal system"
            );
            assert_eq!(
                resolved.version,
                env!("CARGO_PKG_VERSION"),
                "{label} must carry the running package version"
            );
            assert_eq!(
                resolved.fallback_reason.as_deref(),
                Some(expected_reason),
                "{label} must carry exactly its own reason string"
            );
        }

        // Honesty cross-checks: a branch's reason must not smuggle in another
        // branch's wording.
        let non_zero_reason = non_zero_exit.fallback_reason.as_deref().unwrap().to_lowercase();
        assert!(!non_zero_reason.contains("timeout"));
        assert!(!non_zero_reason.contains("json"));

        let timeout_reason = timed_out.fallback_reason.as_deref().unwrap().to_lowercase();
        assert!(!timeout_reason.contains("non-zero"));
        assert!(!timeout_reason.contains("json"));

        let deno_absent_reason = deno_absent.fallback_reason.as_deref().unwrap().to_lowercase();
        assert!(!deno_absent_reason.contains("timeout"));
        assert!(!deno_absent_reason.contains("non-zero"));
        assert!(deno_absent_reason.contains("deno"));
    }
}
