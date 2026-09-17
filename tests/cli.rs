use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

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
fn test_cli_reject_json() {
    let cases = vec![
        vec!["--json"],
        vec!["--version", "--json"],
        vec!["check", "--json"],
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
fn test_cli_deps_dispatch() {
    let mut temp_dir = env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp_dir.push(format!("wda_test_deps_cli_{}_{}", std::process::id(), timestamp));
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    // 1. Bare deps without subcommand yields CLI_USAGE and exit 2
    let output = Command::new(get_bin_path())
        .arg("deps")
        .current_dir(&temp_dir)
        .output()
        .expect("failed to execute binary");

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.is_empty());
    assert!(stderr.contains("wda.cli.usage"));
    assert!(!stderr.contains("wda.command.not-implemented"));

    // 2. deps add without spec yields CLI_USAGE and exit 2
    let output = Command::new(get_bin_path())
        .args(["deps", "add"])
        .current_dir(&temp_dir)
        .output()
        .expect("failed to execute binary");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("wda.cli.usage"));
    assert!(!stderr.contains("wda.command.not-implemented"));

    // 3. deps remove without name yields CLI_USAGE and exit 2
    let output = Command::new(get_bin_path())
        .args(["deps", "remove"])
        .current_dir(&temp_dir)
        .output()
        .expect("failed to execute binary");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("wda.cli.usage"));
    assert!(!stderr.contains("wda.command.not-implemented"));

    // 4. deps remove on non-existent dependency in project with no package.json yields CLI_USAGE, exit 2
    let output = Command::new(get_bin_path())
        .args(["deps", "remove", "not-exist"])
        .current_dir(&temp_dir)
        .output()
        .expect("failed to execute binary");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("wda.cli.usage"));
    assert!(!stderr.contains("wda.command.not-implemented"));

    // 5. deps --json rejection
    let output = Command::new(get_bin_path())
        .args(["deps", "--json", "add", "is-number"])
        .current_dir(&temp_dir)
        .output()
        .expect("failed to execute binary");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("wda.cli.json-not-supported"));

    // 6. deps update on zero-dependency empty project succeeds with exit 0 and summary
    let output = Command::new(get_bin_path())
        .args(["deps", "update"])
        .current_dir(&temp_dir)
        .output()
        .expect("failed to execute binary");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dependencies updated"));
    assert!(!stderr.contains("wda.command.not-implemented"));

    let _ = fs::remove_dir_all(&temp_dir);
}

/// Returns the current `PATH` with any directory holding `deno`/`deno.exe`
/// removed, keeping everything else — including git's own supporting toolchain
/// (its bundled `sh`/`bash` used to run hooks like `core.hooksPath`) — intact.
/// `init`'s hard `git`-presence gate and git's own commit/hook execution both
/// need more of `PATH` than just the single directory holding `git`, so this
/// excludes only what must stay hidden instead of narrowing to one directory.
fn deno_free_path() -> std::ffi::OsString {
    let path = env::var_os("PATH").unwrap_or_default();
    let exe_name = if cfg!(windows) { "deno.exe" } else { "deno" };
    let filtered: Vec<PathBuf> = env::split_paths(&path)
        .filter(|dir| !dir.join(exe_name).is_file())
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
        stderr.is_empty(),
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
        stderr.is_empty(),
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
    fs::write(temp_dir.join("wda.json"), wda_json).expect("failed to write wda.json");
    fs::write(temp_dir.join("README.md"), "# Warning Demo\n").expect("failed to write README.md");

    let doc_header = "---\ntitle: Doc\nstatus: active\nupdated: 2026-08-25\nsourceRefs:\n  - nonexistent_target_ref.md\n---\n";
    fs::write(temp_dir.join("docs").join("architecture.md"), doc_header).expect("failed to write architecture.md");
    fs::write(
        temp_dir.join("docs").join("design.md"),
        "---\ntitle: Design\nstatus: active\nupdated: 2026-08-25\n---\n",
    )
    .expect("failed to write design.md");
    fs::write(
        temp_dir.join("docs").join("naming.md"),
        "---\ntitle: Naming\nstatus: active\nupdated: 2026-08-25\n---\n",
    )
    .expect("failed to write naming.md");
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

