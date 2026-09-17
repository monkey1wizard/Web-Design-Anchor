use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_wda")
}

fn deno_available() -> bool {
    Command::new("deno").arg("--version").output().is_ok()
}

fn temporary_project(name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be available")
        .as_nanos();
    let path = env::temp_dir().join(format!(
        "wda_build_{name}_{}_{}",
        std::process::id(),
        suffix
    ));
    fs::create_dir_all(&path).expect("temporary project directory must be created");
    path
}

fn copy_tree(source: &Path, destination: &Path) {
    for entry in fs::read_dir(source).expect("fixture must be readable") {
        let entry = entry.expect("fixture entry must be readable");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            fs::create_dir_all(&destination_path).expect("fixture directory must be copied");
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).expect("fixture file must be copied");
        }
    }
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/build")
        .join(name)
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
/// configuration (identity, `core.hooksPath`, etc.) never leaks into the test —
/// e2e tests must behave the same regardless of the developer machine's own
/// git setup. `wda init`'s own identity-fallback path (`WDA <wda@localhost>`)
/// takes over when no identity resolves, so init still succeeds.
fn isolated_home_dir() -> PathBuf {
    let dir = env::temp_dir().join(format!(
        "wda_test_isolated_home_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be available")
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("isolated HOME directory must be created");
    dir
}

fn run(command: &str, project: &Path, args: &[&str]) -> Output {
    let mut cmd = Command::new(binary());
    cmd.arg(command).args(args).current_dir(project);
    if command == "init" {
        // PATH is overridden to omit `deno` so `init`'s online Spectrum 2
        // resolution deterministically falls back to WDA Minimal instead of
        // reaching the real npm registry under the bare `cargo test`. The `deno`
        // these build tests actually exercise runs in a later `build` invocation,
        // spawned separately with the real, unmodified PATH. `git` and its own
        // supporting toolchain stay on PATH so `init`'s `git`-presence gate and
        // its commit/hook execution both still work. HOME/USERPROFILE and the
        // global git config paths are isolated so this machine's own git
        // identity and hooks never leak into the test.
        let isolated_home = isolated_home_dir();
        cmd.env("PATH", deno_free_path());
        cmd.env("HOME", &isolated_home);
        cmd.env("USERPROFILE", &isolated_home);
        cmd.env(
            "GIT_CONFIG_GLOBAL",
            isolated_home.join("nonexistent-gitconfig"),
        );
        cmd.env(
            "GIT_CONFIG_SYSTEM",
            isolated_home.join("nonexistent-gitconfig-system"),
        );
    }
    cmd.output().expect("wda must start")
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, current: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(current).expect("output tree must be readable") {
            let entry = entry.expect("output entry must be readable");
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root)
                        .expect("output must be below dist")
                        .to_path_buf(),
                    fs::read(path).expect("output file must be readable"),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}

fn count_residual_includes(root: &Path) -> usize {
    fn walk(current: &Path, count: &mut usize) {
        for entry in fs::read_dir(current).expect("output tree must be readable") {
            let entry = entry.expect("output entry must be readable");
            let path = entry.path();
            if path.is_dir() {
                walk(&path, count);
            } else {
                let contents = fs::read(path).expect("output file must be readable");
                *count += contents
                    .windows(b"<wda-include".len())
                    .filter(|window| *window == b"<wda-include")
                    .count();
            }
        }
    }

    let mut count = 0;
    walk(root, &mut count);
    count
}

fn digest(tree: &BTreeMap<PathBuf, Vec<u8>>) -> u64 {
    let mut hasher = DefaultHasher::new();
    tree.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn init_then_builds_the_untouched_generated_tree() {
    if !deno_available() {
        return;
    }
    let project = temporary_project("init");
    let init = run("init", &project, &[]);
    assert_eq!(
        init.status.code(),
        Some(0),
        "init stderr: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    let build = run("build", &project, &[]);
    assert_eq!(
        build.status.code(),
        Some(0),
        "build stderr: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(project.join("dist").is_dir());
    assert!(project.join("dist/index.html").is_file());
    assert!(project.join("dist/styles/tokens.css").is_file());
    assert!(
        String::from_utf8_lossy(&build.stdout).contains(
            project
                .join("dist")
                .to_str()
                .expect("temporary project path must be valid UTF-8")
        ),
        "successful build must report its absolute dist path: {}",
        String::from_utf8_lossy(&build.stdout)
    );
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn build_reports_the_three_exit_statuses() {
    let error_project = temporary_project("error");
    copy_tree(&fixture("mapping"), &error_project);
    fs::create_dir_all(error_project.join("styles")).unwrap();
    fs::write(
        error_project.join("styles/tokens.css"),
        "/* forbidden source output */",
    )
    .unwrap();
    let error = run("build", &error_project, &[]);
    assert_eq!(error.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&error.stderr).contains("wda.build.tokens-css-in-source"));
    assert!(!error_project.join("dist").exists());
    fs::remove_dir_all(error_project).unwrap();

    let json_project = temporary_project("json");
    let json = run("build", &json_project, &["--json"]);
    assert_eq!(json.status.code(), Some(2));
    assert!(json.stdout.is_empty());
    fs::remove_dir_all(json_project).unwrap();

    if !deno_available() {
        return;
    }
    let warning_project = temporary_project("warning");
    copy_tree(&fixture("mapping"), &warning_project);
    fs::create_dir_all(warning_project.join("assets")).unwrap();
    fs::write(warning_project.join("assets/unused.svg"), "unused").unwrap();
    let warning = run("build", &warning_project, &[]);
    assert_eq!(
        warning.status.code(),
        Some(0),
        "build stderr: {}",
        String::from_utf8_lossy(&warning.stderr)
    );
    assert!(String::from_utf8_lossy(&warning.stderr).contains("wda.build.unreferenced-asset"));
    fs::remove_dir_all(warning_project).unwrap();
}

#[test]
fn build_expands_includes_and_leaves_no_residue() {
    if !deno_available() {
        return;
    }
    let project = temporary_project("include");
    copy_tree(&fixture("mapping"), &project);
    fs::write(project.join("pages/index.html"), "<!DOCTYPE html><html lang=\"en\"><head><title>Include</title></head><body><wda-include src=\"../components/card.html\"></wda-include></body></html>").unwrap();
    fs::create_dir_all(project.join("components")).unwrap();
    fs::write(
        project.join("components/card.html"),
        "<article><slot>Card</slot></article>",
    )
    .unwrap();
    let output = run("build", &project, &[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "build stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let html = fs::read_to_string(project.join("dist/index.html")).unwrap();
    assert_eq!(count_residual_includes(&project.join("dist")), 0);
    assert!(html.contains("<slot>Card</slot>"));
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn build_cleans_stale_files_and_is_deterministic() {
    if !deno_available() {
        return;
    }
    let project = temporary_project("deterministic");
    copy_tree(&fixture("mapping"), &project);
    fs::create_dir_all(project.join("dist")).unwrap();
    fs::write(project.join("dist/stale.txt"), "stale").unwrap();
    let first = run("build", &project, &[]);
    assert_eq!(
        first.status.code(),
        Some(0),
        "first build stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(!project.join("dist/stale.txt").exists());
    let first_tree = snapshot(&project.join("dist"));
    let second = run("build", &project, &[]);
    assert_eq!(
        second.status.code(),
        Some(0),
        "second build stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    let second_tree = snapshot(&project.join("dist"));
    assert_eq!(digest(&first_tree), digest(&second_tree));
    assert_eq!(first_tree, second_tree);
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn build_enforces_output_mapping_and_locked_dependencies() {
    if deno_available() {
        let project = temporary_project("mapping");
        copy_tree(&fixture("mapping"), &project);
        let output = run("build", &project, &[]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "build stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(project.join("dist/styles/tokens.css").is_file());
        let tokens = fs::read_to_string(project.join("dist/styles/tokens.css")).unwrap();
        assert!(tokens.contains("--color-brand-primary"));
        assert!(project.join("dist/scripts/main.js").is_file());
        let html = fs::read_to_string(project.join("dist/index.html")).unwrap();
        assert!(html.contains("scripts/main.js"));
        assert!(!html.contains("scripts/main.ts"));
        fs::remove_dir_all(project).unwrap();
    }

    if !deno_available() {
        return;
    }
    let project = temporary_project("unlocked");
    copy_tree(&fixture("unlocked"), &project);
    let output = run("build", &project, &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("wda.build.dependency-unlocked"),
        "unlocked fixture stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!project.join("dist").exists());
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn build_reports_a_type_error_with_the_project_relative_path() {
    if !deno_available() {
        return;
    }
    let project = temporary_project("type-error");
    copy_tree(&fixture("mapping"), &project);
    let mut main_ts = fs::read_to_string(project.join("scripts/main.ts")).unwrap();
    main_ts.push_str("\nconst bad: number = \"s\";\n");
    fs::write(project.join("scripts/main.ts"), main_ts).unwrap();
    let output = run("build", &project, &[]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("wda.build.type-error"),
        "type-error stderr: {stderr}"
    );
    assert!(
        stderr.contains("scripts/main.ts") || stderr.contains("scripts\\main.ts"),
        "type-error stderr: {stderr}"
    );
    assert!(
        !stderr.contains("wda-build-ws"),
        "type-error stderr must not leak the build workspace path: {stderr}"
    );
    assert!(!project.join("dist").exists());
    fs::remove_dir_all(project).unwrap();
}

#[test]
#[ignore]
fn build_installs_a_locked_npm_import_and_rejects_a_stale_lock() {
    // Real-binary probe: builds a fixture with a locked npm dependency through
    // the production `deno install --frozen` / `deno check` / esbuild path, and
    // the stale-lock half reaches the real registry to detect the out-of-date
    // lock (see fix-build-npm-delivery Risks). `fix-deps-test-determinism`
    // landed before this plan's pipeline started, so per this plan's own
    // Deferred Follow-up ("owned by whichever plan lands second"), this test
    // joins the `#[ignore]` probe tier from the start rather than gating on
    // `deno_available()`, matching the five real-registry probes in
    // `src/deps/resolve.rs`. Run separately via `cargo test -- --ignored`
    // (`.gitlab-ci.yml`'s non-blocking `test:ignored-probes` job).
    let project = temporary_project("locked-import");
    copy_tree(&fixture("locked-import"), &project);
    let fixture_lock = fs::read(fixture("locked-import").join("deno.lock")).unwrap();

    let first = run("build", &project, &[]);
    assert_eq!(
        first.status.code(),
        Some(0),
        "first build stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    let bundle_path = project.join("dist/scripts/main.js");
    assert!(bundle_path.is_file());
    let bundle = fs::read_to_string(&bundle_path).unwrap();
    assert!(
        bundle.contains("preact"),
        "bundle must contain the package identifier: {bundle}"
    );
    assert_eq!(
        fs::read(project.join("deno.lock")).unwrap(),
        fixture_lock,
        "deno.lock must stay byte-identical after a successful build"
    );
    assert!(!project.join("node_modules").exists());
    let first_tree = snapshot(&project.join("dist"));

    let second = run("build", &project, &[]);
    assert_eq!(
        second.status.code(),
        Some(0),
        "second build stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    let second_tree = snapshot(&project.join("dist"));
    assert_eq!(first_tree, second_tree);
    fs::remove_dir_all(project).unwrap();

    let stale_project = temporary_project("locked-import-stale");
    copy_tree(&fixture("locked-import"), &stale_project);
    fs::write(
        stale_project.join("package.json"),
        r#"{"dependencies":{
    "preact": "^10.29.8",
    "is-number": "^7.0.0"
  }}"#,
    )
    .unwrap();
    let stale = run("build", &stale_project, &[]);
    assert_eq!(stale.status.code(), Some(1));
    let stale_stderr = String::from_utf8_lossy(&stale.stderr);
    assert!(
        stale_stderr.contains("wda.build.dependency-unlocked"),
        "stale-lock stderr: {stale_stderr}"
    );
    assert!(
        stale_stderr.contains("lockfile is out of date"),
        "stale-lock stderr: {stale_stderr}"
    );
    assert!(!stale_project.join("dist").exists());
    assert_eq!(
        fs::read(stale_project.join("deno.lock")).unwrap(),
        fixture_lock,
        "deno.lock must stay byte-identical after a rejected build"
    );
    fs::remove_dir_all(stale_project).unwrap();
}

#[test]
#[ignore]
fn build_ignores_a_node_modules_planted_above_the_project_root() {
    // Real-binary probe, gated the same way as the locked-import test above (see that
    // test's comment): a `node_modules/` planted directly above the temporary project
    // directory, not inside it, holding a fake `preact` with a marker string in place of
    // the real package's `preact/hooks` subpath. If the build ever resolved Node-style
    // imports by walking up from the project root instead of the isolated build
    // workspace, esbuild would pick up the fake package and the marker would land in the
    // bundle instead of the real package's identifier.
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be available")
        .as_nanos();
    let parent = env::temp_dir().join(format!(
        "wda_build_ancestor_isolation_{}_{}",
        std::process::id(),
        suffix
    ));
    let fake_package = parent.join("node_modules/preact");
    fs::create_dir_all(&fake_package).expect("fake ancestor package directory must be created");
    fs::write(
        fake_package.join("package.json"),
        r#"{"name":"preact","version":"0.0.0-fake"}"#,
    )
    .unwrap();
    fs::write(
        fake_package.join("hooks.js"),
        "export const useState = \"WDA_TEST_ANCESTOR_NODE_MODULES_MARKER\";\n",
    )
    .unwrap();

    let project = parent.join("project");
    fs::create_dir_all(&project).expect("project directory must be created");
    copy_tree(&fixture("locked-import"), &project);

    let output = run("build", &project, &[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "build stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bundle = fs::read_to_string(project.join("dist/scripts/main.js")).unwrap();
    assert!(
        !bundle.contains("WDA_TEST_ANCESTOR_NODE_MODULES_MARKER"),
        "bundle must not resolve node_modules from an ancestor of the project root: {bundle}"
    );
    assert!(
        bundle.contains("preact"),
        "bundle must contain the real package identifier: {bundle}"
    );
    fs::remove_dir_all(&parent).unwrap();
}
