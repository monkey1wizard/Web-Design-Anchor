//! Design System resolution seam for project initialization.
//!
//! Resolves the design system to use for project initialization online, targeting
//! Spectrum 2, by reusing the deno-backed resolver in [`crate::tool_probe`] to
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

use crate::tool_probe::{resolve_tool_candidate_with_path, ToolCandidateResolution};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagerError {
    NotFound,
    CannotStart(String),
    VersionFailure(String),
    InstallFailure(String),
    RegistryUnreachable(String),
    RegistryHttp(u16),
    RegistryTls(String),
    RegistryFormat(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManager {
    pub name: String,
    pub executable: std::path::PathBuf,
    pub version: String,
}

pub fn supported_package_managers() -> Vec<String> {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("../../packaging/toolchain.json"))
            .expect("embedded descriptor is valid JSON");
    value["package_managers"]
        .as_array()
        .expect("manager list")
        .iter()
        .map(|v| v.as_str().expect("manager name").to_owned())
        .collect()
}

impl PackageManager {
    pub fn probe(name: &str, executable: &Path, cwd: &Path) -> Result<Self, ManagerError> {
        if !supported_package_managers().iter().any(|n| n == name) {
            return Err(ManagerError::NotFound);
        }
        let output = manager_command(executable, cwd)
            .arg("--version")
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    ManagerError::NotFound
                } else {
                    ManagerError::CannotStart(e.to_string())
                }
            })?;
        let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let valid = version.split('.').count() == 3
            && version
                .split('.')
                .all(|part| !part.is_empty() && part.bytes().all(|c| c.is_ascii_digit()));
        if !output.status.success() || !valid {
            return Err(ManagerError::VersionFailure(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }
        Ok(Self {
            name: name.to_owned(),
            executable: executable.to_owned(),
            version,
        })
    }

    pub fn install(&self, cwd: &Path) -> Result<(), ManagerError> {
        let args: &[&str] = match self.name.as_str() {
            "npm" => &["install", "--ignore-scripts"],
            "bun" => &["install", "--ignore-scripts"],
            _ => return Err(ManagerError::NotFound),
        };
        let output = manager_command(&self.executable, cwd)
            .args(args)
            .output()
            .map_err(|e| ManagerError::CannotStart(e.to_string()))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(ManagerError::InstallFailure(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ))
        }
    }
}

fn manager_command(executable: &Path, cwd: &Path) -> Command {
    let mut command = Command::new(executable);
    command.current_dir(cwd).env_clear();
    for key in [
        "PATH",
        "SYSTEMROOT",
        "WINDIR",
        "TEMP",
        "TMP",
        "HOME",
        "USERPROFILE",
        "npm_config_registry",
        "NPM_CONFIG_REGISTRY",
        "BUN_CONFIG_REGISTRY",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
        .env("COREPACK_ENABLE_NETWORK", "0")
        .env("COREPACK_ENABLE_DOWNLOAD_PROMPT", "0");
    command
}

pub fn registry_metadata(registry: &str, package: &str) -> Result<serde_json::Value, ManagerError> {
    let valid = !package.is_empty()
        && package
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"@/._-".contains(&c))
        && !package.contains("..")
        && !package.starts_with('/');
    if !valid {
        return Err(ManagerError::RegistryFormat("invalid package name".into()));
    }
    let base =
        reqwest::Url::parse(registry).map_err(|e| ManagerError::RegistryFormat(e.to_string()))?;
    if !matches!(base.scheme(), "http" | "https") || base.cannot_be_a_base() {
        return Err(ManagerError::RegistryFormat("invalid registry URL".into()));
    }
    let mut url = base;
    url.path_segments_mut()
        .map_err(|_| ManagerError::RegistryFormat("invalid registry URL".into()))?
        .pop_if_empty()
        .push(package);
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| ManagerError::RegistryTls(e.to_string()))?;
    let response = client.get(url).send().map_err(|e| {
        let message = e.to_string();
        if e.is_connect() || e.is_timeout() {
            ManagerError::RegistryUnreachable(message)
        } else {
            ManagerError::RegistryTls(message)
        }
    })?;
    if !response.status().is_success() {
        return Err(ManagerError::RegistryHttp(response.status().as_u16()));
    }
    serde_json::from_reader(response).map_err(|e| ManagerError::RegistryFormat(e.to_string()))
}

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

impl ResolvedDesignSystem {
    /// Composes an additional fallback reason into this resolved design system,
    /// maintaining deterministic ordering and suppressing duplicates.
    pub fn compose_fallback_reason(&mut self, additional_reason: &str) {
        self.fallback_reason =
            compose_fallback_reasons(self.fallback_reason.as_deref(), Some(additional_reason));
    }
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
/// the tool probe in `src/tool_probe.rs` carries no timeout or
/// `Duration` usage, and `invoke_staged_deno` waits blockingly.
const RESOLUTION_TIMEOUT: Duration = Duration::from_secs(10);

/// Fallback reason: `deno` could not be resolved on PATH.
pub(crate) const FALLBACK_REASON_DENO_ABSENT: &str =
    "Online Design System resolution requires 'deno', which was not found on PATH";

/// Fallback reason: Deno was found on PATH but could not be started.
pub(crate) const FALLBACK_REASON_DENO_UNSTARTABLE: &str =
    "Online Design System resolution found 'deno' on PATH but could not start it";

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

/// Fallback reason: installing a default direct dependency failed (such as Lit on
/// WDA Minimal or pinned packages on Spectrum 2). Generalized from the original
/// Spectrum-pinned-only wording so it can name absent Lit and the `wda deps add lit`
/// recovery command when online.
pub(crate) const FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED: &str =
    "Installing a default direct dependency failed; Lit was not installed. Run 'wda deps add lit' when online";

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
/// existing deno-backed resolver in [`crate::tool_probe`]. On any of the seven
/// failure branches, returns the built-in "WDA Minimal" design system at the running
/// package version, carrying a fallback reason naming which branch triggered.
pub fn resolve_design_system() -> ResolvedDesignSystem {
    resolve_design_system_with(std::env::var_os("PATH").as_deref(), None)
}

/// Resolves the design system with an explicit PATH and an optional injected
/// subprocess result, shaped like `resolve_tool_candidate_with_path`'s PATH-injection
/// seam: the production entry above always passes `None`
/// here and reaches the real timed `deno` subprocess; tests pass
/// `Some(RawInvocation)` to drive each of the six post-deno-presence failure
/// branches deterministically. The seventh branch (deno absent) is driven by
/// `path_env` alone, with `injected` left `None`, so `resolve_deno_with_path` still
/// runs and fails on a PATH lacking `deno`.
pub(crate) fn resolve_design_system_with(
    path_env: Option<&OsStr>,
    injected: Option<RawInvocation>,
) -> ResolvedDesignSystem {
    let outcome: Result<String, String> = if let Some(raw) = injected {
        resolve_version_from_raw(raw).map_err(str::to_string)
    } else {
        match resolve_tool_candidate_with_path("deno", path_env) {
            ToolCandidateResolution::Startable(deno_binary) => {
                resolve_version_from_raw(invoke_online_resolution(&deno_binary))
                    .map_err(str::to_string)
            }
            ToolCandidateResolution::NotFound => Err(FALLBACK_REASON_DENO_ABSENT.to_string()),
            ToolCandidateResolution::FoundButUnstartable(diagnostic) => Err(format!(
                "{FALLBACK_REASON_DENO_UNSTARTABLE}; {}",
                diagnostic.message
            )),
            ToolCandidateResolution::InvalidName(diagnostic) => Err(diagnostic.message),
        }
    };

    match outcome {
        Ok(version) => ResolvedDesignSystem {
            name: SPECTRUM_TWO_DESIGN_SYSTEM_NAME.to_string(),
            version,
            fallback_reason: None,
        },
        Err(reason) => ResolvedDesignSystem {
            name: BUILTIN_DESIGN_SYSTEM_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            fallback_reason: Some(reason),
        },
    }
}

/// Builds the built-in WDA Minimal fallback result carrying `reason`.
pub(crate) fn builtin_fallback(reason: &str) -> ResolvedDesignSystem {
    ResolvedDesignSystem {
        name: BUILTIN_DESIGN_SYSTEM_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        fallback_reason: Some(reason.to_string()),
    }
}

/// Combines an optional existing design system fallback reason with an optional
/// dependency install fallback reason in deterministic order, suppressing duplicates.
///
/// When both reasons are present and distinct, the design system resolution reason
/// appears first, followed by the dependency install reason separated by `; `.
/// If either reason is absent or empty, the other reason is returned.
/// If both reasons are identical or one already contains the other, duplicate entries are suppressed.
pub(crate) fn compose_fallback_reasons(
    design_system_reason: Option<&str>,
    dependency_reason: Option<&str>,
) -> Option<String> {
    let ds_opt = design_system_reason
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let dep_opt = dependency_reason.map(str::trim).filter(|s| !s.is_empty());

    match (ds_opt, dep_opt) {
        (None, None) => None,
        (Some(ds), None) => Some(ds.to_string()),
        (None, Some(dep)) => Some(dep.to_string()),
        (Some(ds), Some(dep)) => {
            if ds == dep || ds.contains(dep) {
                Some(ds.to_string())
            } else if dep.contains(ds) {
                Some(dep.to_string())
            } else {
                Some(format!("{ds}; {dep}"))
            }
        }
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

    fn manager_fixture() -> tempfile::TempDir {
        let dir = tempfile::Builder::new()
            .prefix("manager fixture ")
            .tempdir()
            .unwrap();
        let source = dir.path().join("fixture.rs");
        std::fs::write(&source, r###"
use std::{env, fs};
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let record = format!("{}\n{}\n{}\n{}", env::current_dir().unwrap().display(), args.join("|"), env::var("COREPACK_ENABLE_NETWORK").unwrap_or_default(), env::var("COREPACK_ENABLE_DOWNLOAD_PROMPT").unwrap_or_default());
    fs::write(if args == ["--version"] { "probe.txt" } else { "install.txt" }, record).unwrap();
    if args == ["--version"] {
        if std::path::Path::new("bad-version").exists() { println!("v1.2.3"); } else { println!("1.2.3"); }
    } else if std::path::Path::new("fail-install").exists() {
        eprintln!("install failed");
        std::process::exit(7);
    }
}
"###).unwrap();
        let executable = dir.path().join(if cfg!(windows) {
            "manager fixture.exe"
        } else {
            "manager fixture"
        });
        assert!(Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(executable)
            .status()
            .unwrap()
            .success());
        dir
    }

    #[test]
    fn native_package_manager_unit_commands_and_failures() {
        let dir = manager_fixture();
        let executable = dir.path().join(if cfg!(windows) {
            "manager fixture.exe"
        } else {
            "manager fixture"
        });
        let cwd = dir.path().join("project with spaces");
        std::fs::create_dir(&cwd).unwrap();
        for (name, args) in [
            ("npm", "install|--ignore-scripts"),
            ("bun", "install|--ignore-scripts"),
        ] {
            let manager = PackageManager::probe(name, &executable, &cwd).unwrap();
            assert_eq!(manager.version, "1.2.3");
            assert_eq!(
                std::fs::read_to_string(cwd.join("probe.txt")).unwrap(),
                format!("{}\n--version\n0\n0", cwd.display())
            );
            manager.install(&cwd).unwrap();
            assert_eq!(
                std::fs::read_to_string(cwd.join("install.txt")).unwrap(),
                format!("{}\n{}\n0\n0", cwd.display(), args)
            );
            std::fs::write(cwd.join("fail-install"), "").unwrap();
            assert!(
                matches!(manager.install(&cwd), Err(ManagerError::InstallFailure(message)) if message.contains("install failed"))
            );
            std::fs::remove_file(cwd.join("fail-install")).unwrap();
        }
        std::fs::write(cwd.join("bad-version"), "").unwrap();
        assert!(matches!(
            PackageManager::probe("npm", &executable, &cwd),
            Err(ManagerError::VersionFailure(_))
        ));
        assert!(matches!(
            PackageManager::probe("npm", &dir.path().join("missing"), &cwd),
            Err(ManagerError::NotFound)
        ));
        assert!(matches!(
            PackageManager::probe("npm", &cwd, &cwd),
            Err(ManagerError::CannotStart(_))
        ));
        assert!(matches!(
            PackageManager::probe("unsupported", &executable, &cwd),
            Err(ManagerError::NotFound)
        ));
    }

    #[test]
    fn native_package_manager_unit_registry_env_passes_through_and_others_are_cleared() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("envdump.rs");
        std::fs::write(
            &source,
            "fn main() { for (k, v) in std::env::vars() { println!(\"{k}={v}\"); } }",
        )
        .unwrap();
        let executable = dir
            .path()
            .join(if cfg!(windows) { "envdump.exe" } else { "envdump" });
        assert!(Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .status()
            .unwrap()
            .success());
        std::env::set_var("npm_config_registry", "http://127.0.0.1:9/reg/");
        std::env::set_var("BUN_CONFIG_REGISTRY", "http://127.0.0.1:9/bun/");
        std::env::set_var("WDA_TEST_UNRELATED", "leak");
        let output = manager_command(&executable, dir.path()).output().unwrap();
        std::env::remove_var("npm_config_registry");
        std::env::remove_var("BUN_CONFIG_REGISTRY");
        std::env::remove_var("WDA_TEST_UNRELATED");
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(
            text.contains("npm_config_registry=http://127.0.0.1:9/reg/"),
            "{text}"
        );
        assert!(
            text.contains("BUN_CONFIG_REGISTRY=http://127.0.0.1:9/bun/"),
            "{text}"
        );
        assert!(!text.contains("WDA_TEST_UNRELATED"), "{text}");
    }

    fn registry_fixture(status: &str, body: &str) -> (String, std::thread::JoinHandle<String>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let thread = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = [0; 4096];
            let count = stream.read(&mut bytes).unwrap();
            stream.write_all(response.as_bytes()).unwrap();
            String::from_utf8(bytes[..count].to_vec()).unwrap()
        });
        (format!("http://{address}/registry/"), thread)
    }

    #[test]
    fn native_package_manager_unit_registry_metadata_and_errors() {
        let (url, server) = registry_fixture("200 OK", r#"{"version":"1.2.3"}"#);
        let metadata = registry_metadata(&url, "@scope/package").unwrap();
        assert_eq!(metadata["version"], "1.2.3");
        let request = server.join().unwrap();
        assert!(
            request.starts_with("GET /registry/@scope%2Fpackage HTTP/1.1\r\n"),
            "{request}"
        );
        let (url, server) = registry_fixture("404 Not Found", "missing");
        assert_eq!(
            registry_metadata(&url, "package"),
            Err(ManagerError::RegistryHttp(404))
        );
        server.join().unwrap();
        let (url, server) = registry_fixture("200 OK", "not json");
        assert!(matches!(
            registry_metadata(&url, "package"),
            Err(ManagerError::RegistryFormat(_))
        ));
        server.join().unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        assert!(matches!(
            registry_metadata(&format!("http://{address}"), "package"),
            Err(ManagerError::RegistryUnreachable(_))
        ));
        for package in ["../escape", "pkg;touch pwned", "pkg?x=1", "/absolute", ""] {
            assert!(
                matches!(
                    registry_metadata("http://127.0.0.1:1", package),
                    Err(ManagerError::RegistryFormat(_))
                ),
                "{package}"
            );
        }
    }

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
    fn test_resolve_design_system_distinguishes_found_but_unstartable_deno() {
        let dir = std::env::temp_dir().join(format!("wda_unstartable_deno_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let deno = dir.join(if cfg!(windows) { "deno.exe" } else { "deno" });
        std::fs::write(&deno, "not a runnable executable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&deno, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let resolved = resolve_design_system_with(Some(dir.as_os_str()), None);

        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(resolved.name, BUILTIN_DESIGN_SYSTEM_NAME);
        assert!(resolved
            .fallback_reason
            .as_deref()
            .is_some_and(|reason| reason.starts_with(FALLBACK_REASON_DENO_UNSTARTABLE)));
        assert!(!resolved
            .fallback_reason
            .as_deref()
            .is_some_and(|reason| reason.starts_with(FALLBACK_REASON_DENO_ABSENT)));
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
        let non_zero_reason = non_zero_exit
            .fallback_reason
            .as_deref()
            .unwrap()
            .to_lowercase();
        assert!(!non_zero_reason.contains("timeout"));
        assert!(!non_zero_reason.contains("json"));

        let timeout_reason = timed_out.fallback_reason.as_deref().unwrap().to_lowercase();
        assert!(!timeout_reason.contains("non-zero"));
        assert!(!timeout_reason.contains("json"));

        let deno_absent_reason = deno_absent
            .fallback_reason
            .as_deref()
            .unwrap()
            .to_lowercase();
        assert!(!deno_absent_reason.contains("timeout"));
        assert!(!deno_absent_reason.contains("non-zero"));
        assert!(deno_absent_reason.contains("deno"));
    }

    #[test]
    fn test_generalized_dependency_install_fallback_reason_names_lit_and_recovery() {
        assert!(
            FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED
                .contains("Installing a default direct dependency failed"),
            "fallback reason must generalize from pinned to default direct dependency"
        );
        assert!(
            FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED.contains("Lit was not installed"),
            "fallback reason must name absent Lit"
        );
        assert!(
            FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED.contains("wda deps add lit"),
            "fallback reason must name the 'wda deps add lit' recovery command"
        );

        let fallback_ds = builtin_fallback(FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED);
        assert_eq!(fallback_ds.name, BUILTIN_DESIGN_SYSTEM_NAME);
        assert_eq!(
            fallback_ds.fallback_reason.as_deref(),
            Some(FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED)
        );
    }

    #[test]
    fn test_compose_fallback_reasons_preserves_deterministic_two_reason_ordering() {
        let original_reasons = [
            FALLBACK_REASON_DENO_ABSENT,
            FALLBACK_REASON_NON_ZERO_EXIT,
            FALLBACK_REASON_TIMEOUT,
            FALLBACK_REASON_UNPARSEABLE_OUTPUT,
            FALLBACK_REASON_MISSING_FIELD,
            FALLBACK_REASON_NON_RELEASE_VERSION,
            FALLBACK_REASON_VERSION_BELOW_1_0_0,
        ];

        for original in original_reasons {
            let composed = compose_fallback_reasons(
                Some(original),
                Some(FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED),
            )
            .expect("composed reasons must be present");

            // Verbatim presence of both reasons
            assert!(
                composed.contains(original),
                "composed reason must observe the original reason verbatim: {original}"
            );
            assert!(
                composed.contains(FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED),
                "composed reason must observe the generalized install reason verbatim"
            );

            // Stable two-reason ordering: design system reason first, install reason second
            assert!(
                composed.starts_with(original),
                "design-system resolution reason must appear first in stable ordering"
            );
            assert!(
                composed.ends_with(FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED),
                "dependency install reason must appear second in stable ordering"
            );
            assert_eq!(
                composed,
                format!("{original}; {FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED}")
            );
        }
    }

    #[test]
    fn test_compose_fallback_reasons_suppresses_duplicates_and_handles_none() {
        // None combinations
        assert_eq!(compose_fallback_reasons(None, None), None);
        assert_eq!(
            compose_fallback_reasons(Some("Reason A"), None).as_deref(),
            Some("Reason A")
        );
        assert_eq!(
            compose_fallback_reasons(None, Some("Reason B")).as_deref(),
            Some("Reason B")
        );

        // Empty/whitespace-only handling
        assert_eq!(compose_fallback_reasons(Some(""), None), None);
        assert_eq!(
            compose_fallback_reasons(Some("  "), Some("Reason B")).as_deref(),
            Some("Reason B")
        );

        // Duplicate suppression: exact match
        assert_eq!(
            compose_fallback_reasons(
                Some(FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED),
                Some(FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED)
            )
            .as_deref(),
            Some(FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED)
        );

        // Duplicate suppression: already part of compound reason
        let compound = format!("Reason A; {FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED}");
        assert_eq!(
            compose_fallback_reasons(
                Some(&compound),
                Some(FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED)
            )
            .as_deref(),
            Some(compound.as_str())
        );
        assert_eq!(
            compose_fallback_reasons(Some(&compound), Some("Reason A")).as_deref(),
            Some(compound.as_str())
        );
    }

    #[test]
    fn test_resolved_design_system_compose_fallback_reason_method() {
        let mut ds = ResolvedDesignSystem {
            name: BUILTIN_DESIGN_SYSTEM_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            fallback_reason: None,
        };

        // Composing onto None
        ds.compose_fallback_reason(FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED);
        assert_eq!(
            ds.fallback_reason.as_deref(),
            Some(FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED)
        );

        // Composing onto existing reason (from resolution)
        let mut ds_with_res = ResolvedDesignSystem {
            name: BUILTIN_DESIGN_SYSTEM_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            fallback_reason: Some(FALLBACK_REASON_DENO_ABSENT.to_string()),
        };
        ds_with_res.compose_fallback_reason(FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED);
        let expected =
            format!("{FALLBACK_REASON_DENO_ABSENT}; {FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED}");
        assert_eq!(
            ds_with_res.fallback_reason.as_deref(),
            Some(expected.as_str())
        );

        // Duplicate suppression via method
        ds_with_res.compose_fallback_reason(FALLBACK_REASON_DEPENDENCY_INSTALL_FAILED);
        assert_eq!(
            ds_with_res.fallback_reason.as_deref(),
            Some(expected.as_str())
        );
    }
}
