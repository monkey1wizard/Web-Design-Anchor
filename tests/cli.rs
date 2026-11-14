use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

fn project_stderr(stderr: &str) -> String {
    stderr
        .lines()
        .filter(|line| {
            !(line.starts_with("warning: [wda.toolchain.")
                || line.starts_with("warning: [wda.tool.fault]"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn get_bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_wda")
}

fn get_dir_state(dir: &Path) -> BTreeMap<PathBuf, u64> {
    let mut state = BTreeMap::new();
    fn visit(base: &Path, current: &Path, state: &mut BTreeMap<PathBuf, u64>) {
        if let Ok(entries) = fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                let rel_path = path.strip_prefix(base).unwrap().to_path_buf();
                if path.is_dir() {
                    state.insert(rel_path.clone(), 0);
                    visit(base, &path, state);
                } else if path.is_file() {
                    let contents = fs::read(&path).unwrap_or_default();
                    let mut hasher = DefaultHasher::new();
                    contents.hash(&mut hasher);
                    state.insert(rel_path, hasher.finish());
                }
            }
        }
    }
    visit(dir, dir, &mut state);
    state
}

#[test]
fn test_cli_version() {
    let output = Command::new(get_bin_path())
        .arg("--version")
        .output()
        .expect("failed to execute binary");

    assert!(output.status.success());
    assert_eq!(output.status.code(), Some(0));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(stdout.trim(), format!("wda {}", env!("CARGO_PKG_VERSION")));
    assert!(
        stderr.is_empty(),
        "stderr must be empty for --version, got: {}",
        stderr
    );
}

#[test]
fn test_cli_bare_invocation_prints_usage() {
    let output = Command::new(get_bin_path())
        .output()
        .expect("failed to execute binary");

    assert_eq!(output.status.code(), Some(0));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    for token in ["init", "check", "build", "tools fetch", "--version"] {
        assert!(
            stdout.contains(token),
            "usage must mention {token}, got: {stdout}"
        );
    }
    assert!(
        !stdout.contains(get_bin_path()),
        "usage must not echo the executable path, got: {stdout}"
    );
    assert!(
        stderr.is_empty(),
        "stderr must be empty for a bare invocation, got: {}",
        stderr
    );
}

#[test]
fn test_cli_reject_json() {
    let cases = vec![
        vec!["--json"],
        vec!["--version", "--json"],
        vec!["init", "--json"],
    ];

    for case in cases {
        let output = Command::new(get_bin_path())
            .args(&case)
            .output()
            .expect("failed to execute binary");

        assert_eq!(
            output.status.code(),
            Some(2),
            "exit status must be 2 for --json rejection in case {:?}",
            case
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            stdout.is_empty(),
            "stdout must be empty for --json rejection"
        );
        assert!(
            stderr.contains("wda.cli.json-not-supported"),
            "stderr must contain wda.cli.json-not-supported for {:?}, got: {}",
            case,
            stderr
        );
    }
}

#[test]
fn author_json_boundary() {
    let root = tempfile::tempdir().unwrap();
    for (args, exit, status) in [
        (vec!["check", "--json"], 1, "project-error"),
        (vec!["build", "--json"], 1, "project-error"),
        (vec!["check", "--json", "--bad"], 2, "usage-error"),
        (vec!["build", "--json", "--json"], 2, "usage-error"),
    ] {
        let output = Command::new(get_bin_path())
            .current_dir(root.path())
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(exit));
        assert!(output.stderr.is_empty());
        assert!(output.stdout.ends_with(b"\n"));
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["status"], status);
        assert!(value["summary"]["dist_path"].is_null());
        assert!(value["summary"]["needs_http"].is_null());
    }
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/clean_project");
    copy_dir(&fixture, root.path());
    for command in ["check", "build"] {
        let output = Command::new(get_bin_path())
            .current_dir(root.path())
            .args([command, "--json"])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(output.stderr.is_empty());
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["status"], "success");
        if command == "build" {
            assert_eq!(value["summary"]["dist_path"], "dist");
            assert!(value["summary"]["needs_http"].is_boolean());
        }
    }
    let warning = Command::new(get_bin_path())
        .current_dir(root.path())
        .env("PATH", "")
        .args(["check", "--json"])
        .output()
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&warning.stdout).unwrap();
    assert_eq!(warning.status.code(), Some(0));
    assert_eq!(value["status"], "success");
    assert!(value["summary"]["warnings"].as_u64().unwrap() > 0);
    let tool_project = tempfile::tempdir().unwrap();
    let mapping = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/build/mapping");
    copy_dir(&mapping, tool_project.path());
    let cache = tempfile::tempdir().unwrap();
    let descriptor: serde_json::Value =
        serde_json::from_str(include_str!("../packaging/toolchain.json")).unwrap();
    let platform = format!("{}-{}", match env::consts::OS {
        "windows" => "win32", "macos" => "darwin", _ => "linux",
    }, if env::consts::ARCH == "aarch64" { "arm64" } else { "x64" });
    let integrity = descriptor["platforms"].as_array().unwrap().iter()
        .find(|row| row["platform"] == platform).unwrap()["integrity"].as_str().unwrap();
    use base64::Engine;
    let digest = base64::engine::general_purpose::STANDARD
        .decode(integrity.strip_prefix("sha512-").unwrap()).unwrap();
    let key: String = digest.iter().take(16).map(|byte| format!("{byte:02x}")).collect();
    let tool_dir = cache.path().join("wda/tools").join(key).join("package");
    fs::create_dir_all(tool_dir.join("lib")).unwrap();
    fs::write(tool_dir.join(if cfg!(windows) { "lib/tsc.exe" } else { "lib/tsc" }), "not a program").unwrap();
    fs::write(tool_dir.join("LICENSE"), "license").unwrap();
    fs::write(tool_dir.join("lib/lib.d.ts"), "").unwrap();
    let fault = Command::new(get_bin_path()).current_dir(tool_project.path())
        .env("LOCALAPPDATA", cache.path()).env("XDG_CACHE_HOME", cache.path())
        .env("HOME", cache.path()).env("PATH", "")
        .args(["build", "--json"]).output().unwrap();
    let value: serde_json::Value = serde_json::from_slice(&fault.stdout).unwrap();
    assert_eq!(fault.status.code(), Some(2), "{}", String::from_utf8_lossy(&fault.stdout));
    assert_eq!(value["status"], "tool-fault");
    assert!(value["summary"]["dist_path"].is_null());
    assert!(fault.stderr.is_empty());

    let parity = tempfile::tempdir().unwrap();
    copy_dir(&fixture, parity.path());
    fs::remove_file(parity.path().join("docs/design.md")).unwrap();
    fs::write(parity.path().join("tokens/tokens.json"), "{\n invalid\n}").unwrap();
    fs::write(parity.path().join("docs/architecture.md"),
        "---\ntitle: Architecture\nstatus: active\nupdated: 2026-08-25\nsourceRefs:\n  - \"missing\\nfile.md\"\n---\n").unwrap();
    let run = |json: bool| {
        let mut command = Command::new(get_bin_path());
        command.current_dir(parity.path()).env("PATH", "").arg("check");
        if json { command.arg("--json"); }
        command.output().unwrap()
    };
    let human = run(false);
    let json = run(true);
    assert_eq!(human.status.code(), Some(1));
    assert_eq!(json.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    let diagnostics = value["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().filter(|d| d["severity"] == "error").count() >= 2);
    assert!(diagnostics.iter().any(|d| d["severity"] == "warning"));
    assert!(diagnostics.iter().any(|d| d["message"].as_str().unwrap().contains('\n')));
    assert!(diagnostics.iter().any(|d| d["line"].is_number() && d["column"].is_number()));
    let rendered: Vec<String> = diagnostics.iter().map(|d| {
        let path = d["path"].as_str().map(|path| {
            assert!(!path.contains('\\'), "path must use forward slashes: {path}");
            let mut location = wda_core::diagnostics::escape_control_chars(path);
            if let Some(line) = d["line"].as_u64() { location.push_str(&format!(":{line}")); }
            if let Some(column) = d["column"].as_u64() {
                if d["line"].is_null() { location.push(':'); }
                location.push_str(&format!(":{column}"));
            }
            format!("{location}: ")
        }).unwrap_or_default();
        format!("{}: [{}] {}{} ({})", d["severity"].as_str().unwrap(),
            d["code"].as_str().unwrap(), path,
            wda_core::diagnostics::escape_control_chars(d["message"].as_str().unwrap()),
            wda_core::diagnostics::escape_control_chars(d["next_action"].as_str().unwrap()))
    }).collect();
    let human_stderr = String::from_utf8(human.stderr).unwrap();
    assert_eq!(rendered, human_stderr.lines().collect::<Vec<_>>());
    fs::remove_file(root.path().join("anchor.json")).unwrap();
    let output = Command::new(get_bin_path())
        .current_dir(root.path())
        .args(["build", "--json"])
        .output()
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(root.path().join("dist").exists());
    assert!(value["summary"]["dist_path"].is_null());
}

#[test]
fn test_cli_unknown_command() {
    let output = Command::new(get_bin_path())
        .arg("unknown_command_foo")
        .output()
        .expect("failed to execute binary");

    assert_eq!(output.status.code(), Some(2));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stdout.is_empty());
    assert!(
        stderr.contains("wda.cli.usage"),
        "stderr must contain wda.cli.usage, got: {}",
        stderr
    );
}

#[test]
fn test_cli_invalid_unicode_arg() {
    #[cfg(windows)]
    use std::os::windows::ffi::OsStringExt;
    #[cfg(unix)]
    use std::os::unix::ffi::OsStringExt;

    #[cfg(windows)]
    let bad_arg = std::ffi::OsString::from_wide(&[0xD800]);
    #[cfg(unix)]
    let bad_arg = std::ffi::OsString::from_vec(vec![0xFF]);

    let output = Command::new(get_bin_path())
        .arg(bad_arg)
        .output()
        .expect("failed to execute binary");

    assert_eq!(output.status.code(), Some(2));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stdout.is_empty());
    assert!(
        stderr.contains("wda.cli.usage"),
        "stderr must contain wda.cli.usage for invalid unicode arg, got: {}",
        stderr
    );
}

#[test]
fn native_check_preflight_cli_deps_is_usage_error_with_manager_hint() {
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("package.json"), "{\"dependencies\":{}}").unwrap();
    let before = get_dir_state(project.path());
    for args in [
        vec!["deps"],
        vec!["deps", "add", "is-number@7.0.0"],
        vec!["deps", "remove", "is-number"],
        vec!["deps", "update"],
        vec!["deps", "sync"],
        vec!["deps", "refresh"],
    ] {
        let output = Command::new(get_bin_path())
            .args(&args)
            .current_dir(project.path())
            .env("PATH", "")
            .output()
            .expect("failed to execute binary");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {stderr}");
        assert!(stdout.is_empty(), "{args:?}: {stdout}");
        assert!(stderr.contains("wda.cli.usage"), "{args:?}: {stderr}");
        assert!(stderr.contains("npm install"), "{args:?}: {stderr}");
        assert!(stderr.contains("bun add"), "{args:?}: {stderr}");
    }
    assert_eq!(before, get_dir_state(project.path()), "deps must not touch the project");
}


#[test]
fn native_remove_deps_cli_old_commands_and_aliases_are_usage_errors() {
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("package.json"), "{\"dependencies\":{}}").unwrap();
    let before = get_dir_state(project.path());
    for args in [
        vec!["deps", "add", "is-number@7.0.0"],
        vec!["deps", "remove", "is-number"],
        vec!["deps", "update"],
        vec!["add", "is-number@7.0.0"],
        vec!["remove", "is-number"],
        vec!["update"],
        vec!["sync"],
        vec!["refresh"],
    ] {
        let output = Command::new(get_bin_path())
            .args(&args)
            .current_dir(project.path())
            .env("PATH", "")
            .output()
            .expect("failed to execute binary");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {stderr}");
        assert!(stdout.is_empty(), "{args:?}: {stdout}");
        assert!(stderr.contains("wda.cli.usage"), "{args:?}: {stderr}");
    }
    assert_eq!(before, get_dir_state(project.path()), "old commands must not touch the project");
}

#[test]
fn native_remove_deps_integration_check_and_build_work_without_deps_state() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/clean_project");
    let project = tempfile::tempdir().unwrap();
    copy_dir(&fixture, project.path());
    let package_before = fs::read(project.path().join("package.json")).unwrap();
    assert!(!project.path().join("deno.lock").exists());
    assert!(!project.path().join("deno.jsonc").exists());

    let check = Command::new(get_bin_path())
        .arg("check")
        .current_dir(project.path())
        .env("PATH", "")
        .output()
        .expect("failed to execute binary");
    assert_eq!(check.status.code(), Some(0), "{}", String::from_utf8_lossy(&check.stderr));
    assert!(!String::from_utf8_lossy(&check.stderr).contains("wda deps"));
    assert_eq!(package_before, fs::read(project.path().join("package.json")).unwrap(), "check must not change the manifest");
    assert!(!project.path().join("deno.lock").exists(), "check must not create a lock");

    let build = Command::new(get_bin_path())
        .arg("build")
        .current_dir(project.path())
        .env("PATH", "")
        .output()
        .expect("failed to execute binary");
    assert_eq!(build.status.code(), Some(0), "{}", String::from_utf8_lossy(&build.stderr));
    assert!(project.path().join("dist").is_dir(), "build must create dist/");
    assert_eq!(package_before, fs::read(project.path().join("package.json")).unwrap(), "build must not change the manifest");
    assert!(!project.path().join("deno.lock").exists(), "build must not create a lock");
}
#[test]
fn native_check_preflight_cli_usage_names_tools_fetch_without_arguments() {
    let output = Command::new(get_bin_path()).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout.contains("tools fetch"), "{stdout}");
    assert!(!stdout.contains("deps"), "usage must not offer the removed command: {stdout}");
    for args in [vec!["tools"], vec!["tools", "7.0.2"], vec!["tools", "fetch", "7.0.2"]] {
        let output = Command::new(get_bin_path()).args(&args).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("wda.cli.usage"), "{args:?}");
    }
}

fn native_check_project(package_json: &str, with_script: bool) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("clean_project");
    copy_dir(&source, project.path());
    fs::write(project.path().join("package.json"), package_json).unwrap();
    if with_script {
        fs::create_dir_all(project.path().join("scripts")).unwrap();
        fs::write(project.path().join("scripts/main.ts"), "export {};\n").unwrap();
        let page = project.path().join("pages/index.html");
        let html = fs::read_to_string(&page).unwrap();
        fs::write(
            &page,
            html.replace("</head>", "<script type=\"module\" src=\"../scripts/main.ts\"></script></head>"),
        )
        .unwrap();
    }
    project
}

fn run_native_check(project: &Path, cache: &Path) -> std::process::Output {
    Command::new(get_bin_path())
        .arg("check")
        .current_dir(project)
        .env("PATH", "")
        .env("LOCALAPPDATA", cache)
        .env("XDG_CACHE_HOME", cache)
        .env("HOME", cache)
        .output()
        .expect("failed to execute binary")
}

#[test]
fn native_check_preflight_cli_projects_get_named_diagnostics_without_a_manager() {
    let cache = tempfile::tempdir().unwrap();

    let clean = native_check_project("{\"dependencies\":{}}", false);
    let before = get_dir_state(clean.path());
    let output = run_native_check(clean.path(), cache.path());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("wda.toolchain.git-missing"), "{stderr}");
    assert!(!stderr.contains("deno"), "{stderr}");
    assert!(!stderr.contains("wda.build."), "a clean project needs neither packages nor cache: {stderr}");
    assert_eq!(before, get_dir_state(clean.path()));

    let missing = native_check_project(
        "{\"dependencies\":{\"lit\":\"3.3.0\"},\"packageManager\":\"npm@10.2.3\"}",
        true,
    );
    let before = get_dir_state(missing.path());
    let output = run_native_check(missing.path(), cache.path());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("wda.build.dependency-unlocked"), "{stderr}");
    assert!(stderr.contains("'lit'"), "{stderr}");
    assert!(stderr.contains("wda.build.tool-unavailable"), "{stderr}");
    assert!(stderr.contains("wda tools fetch"), "{stderr}");
    assert!(!stderr.contains("deno"), "{stderr}");
    assert_eq!(before, get_dir_state(missing.path()), "check never installs or writes");
    assert!(!missing.path().join("node_modules").exists());
    assert!(fs::read_dir(cache.path()).unwrap().next().is_none(), "check never fills the cache");

    fs::create_dir_all(missing.path().join("node_modules/lit")).unwrap();
    let output = run_native_check(missing.path(), cache.path());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(
        !stderr.contains("wda.build.dependency-unlocked"),
        "an installed project is not misjudged when the manager is absent from PATH: {stderr}"
    );
}


/// Returns the current `PATH` with any directory holding `deno`, `npm`, `bun`, `pnpm`
/// or `yarn` removed, so an ordinary init never reaches a package manager or the
/// registry, keeping everything else — including git's own supporting toolchain
/// (its bundled `sh`/`bash` used to run hooks like `core.hooksPath`) — intact.
/// `init`'s hard `git`-presence gate and git's own commit/hook execution both
/// need more of `PATH` than just the single directory holding `git`, so this
/// excludes only what must stay hidden instead of narrowing to one directory.
fn deno_free_path() -> std::ffi::OsString {
    let path = env::var_os("PATH").unwrap_or_default();
    let hidden = ["deno", "npm", "bun", "pnpm", "yarn"];
    let filtered: Vec<PathBuf> = env::split_paths(&path)
        .filter(|dir| {
            !hidden.iter().any(|name| {
                [
                    name.to_string(),
                    format!("{name}.exe"),
                    format!("{name}.cmd"),
                    format!("{name}.bat"),
                ]
                .iter()
                .any(|file| dir.join(file).is_file())
            })
        })
        .collect();
    env::join_paths(filtered).unwrap_or_default()
}

/// Creates a fresh, empty directory to use as `HOME`/`USERPROFILE` for an
/// isolated `wda init` child process, so this machine's real global git
/// configuration (identity, `core.hooksPath`, etc.) never leaks into the test.
fn isolated_home_dir() -> PathBuf {
    let dir = env::temp_dir().join(format!(
        "wda_cli_isolated_home_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock must be available")
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("isolated HOME directory must be created");
    dir
}

#[test]
fn test_cli_init_non_empty_refusal() {
    let mut temp_dir = env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp_dir.push(format!("wda_test_init_non_empty_{}_{}", std::process::id(), timestamp));

    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
    let file_path = temp_dir.join("wda.json");
    fs::write(&file_path, "pre-existing content").expect("failed to write wda.json");

    let before_state = get_dir_state(&temp_dir);

    // PATH omits only `deno` so `init`'s hard `git`-presence gate (and any
    // commit/hook execution) still works, while online resolution (which
    // still runs ahead of the collision gate) deterministically falls back
    // offline instead of reaching the real npm registry under the bare
    // `cargo test`. HOME/USERPROFILE and the global git config paths are
    // isolated so this machine's own git identity and hooks never leak into
    // the test.
    let isolated_home = isolated_home_dir();
    let output = Command::new(get_bin_path())
        .arg("init")
        .current_dir(&temp_dir)
        .env("PATH", deno_free_path())
        .env("HOME", &isolated_home)
        .env("USERPROFILE", &isolated_home)
        .env("GIT_CONFIG_GLOBAL", isolated_home.join("nonexistent-gitconfig"))
        .env("GIT_CONFIG_SYSTEM", isolated_home.join("nonexistent-gitconfig-system"))
        .output()
        .expect("failed to execute binary");

    assert_eq!(
        output.status.code(),
        Some(1),
        "init in non-empty directory must exit with code 1"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.is_empty(),
        "stdout must be empty on init refusal, got: {}",
        stdout
    );
    assert!(
        stderr.contains("wda.init.path-conflict"),
        "stderr must contain wda.init.path-conflict, got: {}",
        stderr
    );

    let after_state = get_dir_state(&temp_dir);
    assert_eq!(
        before_state, after_state,
        "entry count and filesystem state must remain unchanged on refusal"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_cli_init_without_package_manager_uses_minimal_fallback() {
    let temp_dir = env::temp_dir().join(format!("wda_init_no_manager_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp project directory must be created");
    let isolated_home = isolated_home_dir();

    let output = Command::new(get_bin_path())
        .arg("init")
        .current_dir(&temp_dir)
        .env("PATH", deno_free_path())
        .env("HOME", &isolated_home)
        .env("USERPROFILE", &isolated_home)
        .env("GIT_CONFIG_GLOBAL", isolated_home.join("nonexistent-gitconfig"))
        .env("GIT_CONFIG_SYSTEM", isolated_home.join("nonexistent-gitconfig-system"))
        .output()
        .expect("failed to execute binary");

    assert_eq!(output.status.code(), Some(0), "no package manager must allow init fallback");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("WDA Minimal"), "stdout must describe fallback, got: {stdout}");
    assert!(stdout.contains("No supported package manager"), "stdout must give the reason, got: {stdout}");
    assert!(!stdout.contains("wda.toolchain."), "stdout must not contain diagnostics, got: {stdout}");
    assert!(!stderr.contains("deno-missing"), "Deno is no longer needed, got: {stderr}");
    assert!(!stderr.contains("wda.init.tool-unavailable"), "Git must remain available, got: {stderr}");

    let _ = fs::remove_dir_all(&temp_dir);
    let _ = fs::remove_dir_all(&isolated_home);
}

#[test]
fn test_cli_init_without_git_warns_and_creates_project() {
    let temp_dir = env::temp_dir().join(format!("wda_init_no_git_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp project directory must be created");

    let output = Command::new(get_bin_path())
        .arg("init")
        .current_dir(&temp_dir)
        .env("PATH", "")
        .output()
        .expect("failed to execute binary");

    assert_eq!(output.status.code(), Some(0), "missing Git is only a Warning");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("Project initialized."), "got: {stdout}");
    assert!(stderr.contains("wda.toolchain.git-missing"), "stderr must warn about missing Git, got: {stderr}");
    assert!(!temp_dir.join(".git").exists(), "no repository without git");
    let gitignore = fs::read_to_string(temp_dir.join(".gitignore")).expect(".gitignore must be written");
    assert!(gitignore.contains("/dist/") && gitignore.contains("/node_modules/"));
    assert!(temp_dir.join("anchor.json").is_file() && temp_dir.join("package.json").is_file());

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn native_init_transaction_cli_package_manager_usage_errors_exit_2() {
    let cases: [(&[&str], &str); 4] = [
        (&["init", "--package-manager"], "Missing package manager name"),
        (&["init", "--package-manager", "pnpm"], "Unknown package manager 'pnpm'"),
        (&["init", "--package-manager", "npm", "extra"], "Unexpected argument: extra"),
        (&["init", "--bogus"], "Unexpected argument: --bogus"),
    ];
    for (args, expected) in cases {
        let temp_dir = env::temp_dir().join(format!(
            "wda_cli_init_usage_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        let output = Command::new(get_bin_path())
            .args(args)
            .current_dir(&temp_dir)
            .output()
            .expect("failed to execute binary");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {stderr}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(stderr.contains("wda.cli.usage"), "{args:?}: {stderr}");
        assert!(stderr.contains(expected), "{args:?}: {stderr}");
        assert_eq!(fs::read_dir(&temp_dir).unwrap().count(), 0, "{args:?} writes nothing");
        let _ = fs::remove_dir_all(&temp_dir);
    }
}

#[test]
fn native_init_transaction_cli_named_manager_missing_creates_minimal_with_reason() {
    for argument in [vec!["init", "--package-manager", "bun"], vec!["init", "--package-manager=bun"]] {
        let temp_dir = env::temp_dir().join(format!(
            "wda_cli_init_named_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        let isolated_home = isolated_home_dir();
        let output = Command::new(get_bin_path())
            .args(&argument)
            .current_dir(&temp_dir)
            .env("PATH", deno_free_path())
            .env("HOME", &isolated_home)
            .env("USERPROFILE", &isolated_home)
            .env("GIT_CONFIG_GLOBAL", isolated_home.join("nonexistent-gitconfig"))
            .env("GIT_CONFIG_SYSTEM", isolated_home.join("nonexistent-gitconfig-system"))
            .output()
            .expect("failed to execute binary");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(0), "{argument:?}");
        assert!(stdout.contains("Design System: WDA Minimal"), "{stdout}");
        assert!(stdout.contains("bun") && stdout.contains("not available"), "{stdout}");
        assert!(temp_dir.join("anchor.json").is_file());
        assert!(temp_dir.join("package.json").is_file());
        assert!(!temp_dir.join("node_modules").exists());
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::remove_dir_all(&isolated_home);
    }
}

#[test]
fn test_cli_init_with_argument_is_usage_error() {
    let output = Command::new(get_bin_path())
        .args(["init", "some_dir"])
        .output()
        .expect("failed to execute binary");

    assert_eq!(
        output.status.code(),
        Some(2),
        "init with directory argument must exit with code 2"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stdout.is_empty());
    assert!(
        stderr.contains("wda.cli.usage"),
        "stderr must contain wda.cli.usage for init <dir>, got: {}",
        stderr
    );
}

#[test]
fn test_cli_check_clean_fixture() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("clean_project");

    let output = Command::new(get_bin_path())
        .arg("check")
        .current_dir(&fixture_dir)
        .output()
        .expect("failed to execute binary");

    assert_eq!(
        output.status.code(),
        Some(0),
        "clean fixture must exit 0"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        project_stderr(&stderr).is_empty(),
        "stderr must be empty for clean check run, got: {}",
        stderr
    );
    assert!(
        stdout.contains("WCAG"),
        "stdout must contain accessibility capability-boundary scope note on clean run, got: {}",
        stdout
    );
    assert!(
        stdout.contains(wda_core::cli::A11Y_SCOPE_NOTE),
        "stdout must contain exact A11Y_SCOPE_NOTE on clean run, got: {}",
        stdout
    );
}

#[test]
fn test_cli_check_conformant_fixture() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("conformant");

    let output = Command::new(get_bin_path())
        .arg("check")
        .current_dir(&fixture_dir)
        .output()
        .expect("failed to execute binary");

    assert_eq!(
        output.status.code(),
        Some(0),
        "conformant fixture must exit 0"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        project_stderr(&stderr).is_empty(),
        "stderr must be empty for conformant check run, got: {}",
        stderr
    );
    assert!(
        stdout.contains("WCAG"),
        "stdout must contain accessibility capability-boundary scope note on conformant run, got: {}",
        stdout
    );
    assert!(
        stdout.contains(wda_core::cli::A11Y_SCOPE_NOTE),
        "stdout must contain exact A11Y_SCOPE_NOTE on conformant run, got: {}",
        stdout
    );
}

#[test]
fn test_cli_check_error_fixture() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("probe_all_families");

    let output = Command::new(get_bin_path())
        .arg("check")
        .current_dir(&fixture_dir)
        .output()
        .expect("failed to execute binary");

    assert_eq!(
        output.status.code(),
        Some(1),
        "fixture with errors must exit 1"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.is_empty(),
        "stderr must contain diagnostics when errors exist"
    );
    assert!(
        !stdout.contains("WCAG"),
        "stdout must not contain success scope note when errors exist, got: {}",
        stdout
    );
}

fn copy_dir(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("destination directory must be created");
    for entry in fs::read_dir(source).expect("source directory must be readable") {
        let entry = entry.expect("directory entry must be readable");
        let target = destination.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("fixture file must be copied");
        }
    }
}

#[test]
fn test_cli_check_missing_toolchain_is_warning_only_and_read_only() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("clean_project");
    let temp_dir = env::temp_dir().join(format!("wda_check_toolchain_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    copy_dir(&source, &temp_dir);
    let before = get_dir_state(&temp_dir);

    let output = Command::new(get_bin_path())
        .arg("check")
        .current_dir(&temp_dir)
        .env("PATH", "")
        .output()
        .expect("failed to execute binary");

    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("wda.toolchain.git-missing"), "{stderr}");
    assert!(!stderr.contains("wda.toolchain.deno-missing"), "Deno is no longer probed: {stderr}");
    let after = get_dir_state(&temp_dir);
    assert_eq!(before, after, "check must not write project files");
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_cli_check_unstartable_toolchain_is_warning_only_and_read_only() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("clean_project");
    let temp_dir = env::temp_dir().join(format!("wda_check_toolchain_fault_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    copy_dir(&source, &temp_dir);

    let path_dir = temp_dir.join("path");
    fs::create_dir_all(&path_dir).expect("PATH directory must be created");
    #[cfg(windows)]
    let broken_git = path_dir.join("git.exe");
    #[cfg(not(windows))]
    let broken_git = path_dir.join("git");
    #[cfg(windows)]
    fs::write(&broken_git, b"MZ")
        .expect("broken Git candidate must be written");
    #[cfg(not(windows))]
    fs::write(&broken_git, b"not a valid executable format\n")
        .expect("broken Git candidate must be written");
    #[cfg(windows)]
    let locked_file = {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&broken_git)
            .expect("broken Git candidate must be locked against execution")
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&broken_git, fs::Permissions::from_mode(0o755))
            .expect("broken Git candidate must be executable");
    }
    let before = get_dir_state(&temp_dir);
    let output = Command::new(get_bin_path())
        .arg("check")
        .current_dir(&temp_dir)
        .env("PATH", &path_dir)
        .output()
        .expect("failed to execute binary");

    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("wda.tool.fault"), "{stderr}");
    assert!(!stderr.contains("wda.toolchain.git-missing"), "{stderr}");
    assert!(!stderr.contains("wda.toolchain.deno-missing"), "Deno is no longer probed: {stderr}");
    let after = get_dir_state(&temp_dir);
    assert_eq!(before, after, "check must not write project files");
    #[cfg(windows)]
    drop(locked_file);
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_cli_build_does_not_emit_check_toolchain_warnings() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("clean_project");
    let fixture_dir = env::temp_dir().join(format!("wda_build_toolchain_{}", std::process::id()));
    let _ = fs::remove_dir_all(&fixture_dir);
    copy_dir(&source, &fixture_dir);
    let output = Command::new(get_bin_path())
        .arg("build")
        .current_dir(&fixture_dir)
        .env("PATH", "")
        .output()
        .expect("failed to execute binary");
    assert_eq!(output.status.code(), Some(0), "Native build must succeed without any external tool on PATH");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("wda.toolchain.deno-missing"), "Build must not emit the Deno toolchain warning: {stderr}");
    assert!(!stderr.contains("wda.toolchain.git-missing"), "Build must not emit the Git toolchain warning: {stderr}");
    assert!(!stderr.contains("wda.build.tool-unavailable"), "Build must not report a tool-unavailable error: {stderr}");
    assert!(fixture_dir.join("dist").is_dir(), "Native build must produce dist/");
    let _ = fs::remove_dir_all(fixture_dir);
}

#[test]
fn test_cli_check_warning_only_fixture() {
    let mut temp_dir = env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp_dir.push(format!("wda_test_warning_only_{}_{}", std::process::id(), timestamp));

    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
    fs::create_dir_all(temp_dir.join("docs")).expect("failed to create docs dir");
    fs::create_dir_all(temp_dir.join("pages")).expect("failed to create pages dir");

    let wda_json = r#"{
        "projectName": "warning-demo",
        "projectVersion": "1.0.0",
        "wdaVersion": "0.1.0",
        "pageArchitecture": "mpa",
        "browserBaseline": "baseline-widely-available",
        "accessibilityBaseline": "wcag-2.2-aa",
        "designSystem": {
            "name": "ds",
            "version": "1.0.0"
        }
    }"#;
    fs::write(temp_dir.join("anchor.json"), wda_json).expect("failed to write anchor.json");
    fs::write(temp_dir.join("package.json"), "{\"dependencies\":{}}").expect("failed to write package.json");
    fs::write(temp_dir.join("README.md"), "# Warning Demo\n").expect("failed to write README.md");

    let doc_header = "---\ntitle: Doc\nstatus: active\nupdated: 2026-08-25\nsourceRefs:\n  - nonexistent_target_ref.md\n---\n";
    fs::write(temp_dir.join("docs").join("architecture.md"), doc_header).expect("failed to write architecture.md");
    fs::write(
        temp_dir.join("docs").join("design.md"),
        "---\ntitle: Design\nstatus: active\nupdated: 2026-08-25\n---\n",
    )
    .expect("failed to write design.md");
    fs::write(
        temp_dir.join("docs").join("glossary.md"),
        "---\ntitle: Glossary\nstatus: active\nupdated: 2026-08-25\n---\n",
    )
    .expect("failed to write glossary.md");
    fs::write(
        temp_dir.join("pages").join("index.html"),
        "<!DOCTYPE html><html lang=\"en\"><head><title>Index</title></head><body></body></html>",
    )
    .expect("failed to write index.html");

    let state_before = get_dir_state(&temp_dir);

    // Run 1
    let output1 = Command::new(get_bin_path())
        .arg("check")
        .current_dir(&temp_dir)
        .output()
        .expect("failed to execute binary");

    assert_eq!(
        output1.status.code(),
        Some(0),
        "warning-only fixture must exit 0"
    );

    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    let stderr1 = String::from_utf8_lossy(&output1.stderr);

    assert!(
        stderr1.contains("wda.docs.source-ref-unresolvable"),
        "stderr must contain Warning diagnostic wda.docs.source-ref-unresolvable, got: {}",
        stderr1
    );
    assert!(
        stdout1.contains("WCAG"),
        "stdout must contain success scope note on Warning-only run, got: {}",
        stdout1
    );

    // Run 2 (Unchanged rerun)
    let output2 = Command::new(get_bin_path())
        .arg("check")
        .current_dir(&temp_dir)
        .output()
        .expect("failed to execute binary");

    assert_eq!(
        output2.status.code(),
        Some(0),
        "warning-only fixture second run must exit 0"
    );

    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    let stderr2 = String::from_utf8_lossy(&output2.stderr);

    assert_eq!(
        stderr1, stderr2,
        "stderr on second run must be identical to first run"
    );
    assert_eq!(
        stdout1, stdout2,
        "stdout on second run must be identical to first run"
    );

    let state_after = get_dir_state(&temp_dir);
    assert_eq!(
        state_before, state_after,
        "filesystem state must remain unchanged after check runs"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
#[test]
fn native_tools_fetch_rejects_extra_arguments() {
    let output = std::process::Command::new(get_bin_path())
        .args(["tools", "fetch", "7.0.2"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("wda.cli.usage"));
}

#[test]
#[ignore = "needs registry.npmjs.org"]
fn native_tools_fetch_restores_runnable_package_concurrently() {
    let cache = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let manifest = project.path().join("anchor.json");
    std::fs::write(&manifest, "unchanged").unwrap();
    let spawn = || {
        let mut command = std::process::Command::new(get_bin_path());
        command.args(["tools", "fetch"]).current_dir(project.path());
        if cfg!(windows) { command.env("LOCALAPPDATA", cache.path()); }
        else { command.env("XDG_CACHE_HOME", cache.path()); }
        command.spawn().unwrap()
    };
    let mut first = spawn();
    let mut second = spawn();
    assert!(first.wait().unwrap().success());
    assert!(second.wait().unwrap().success());
    assert_eq!(std::fs::read_to_string(manifest).unwrap(), "unchanged");
    let root = cache.path().join("wda/tools");
    let entries: Vec<_> = std::fs::read_dir(root).unwrap().collect();
    assert_eq!(entries.len(), 1);
    let package = entries[0].as_ref().unwrap().path().join("package");
    assert!(package.join("LICENSE").is_file());
    assert!(package.join("lib/lib.d.ts").is_file());
    let executable = package.join(if cfg!(windows) { "lib/tsc.exe" } else { "lib/tsc" });
    let output = std::process::Command::new(executable).arg("--version").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("7.0.2"));
}
