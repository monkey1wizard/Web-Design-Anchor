use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

fn get_bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_wda")
}

fn create_temp_dir(prefix: &str) -> PathBuf {
    let mut temp_dir = env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp_dir.push(format!("{}_{}_{}", prefix, std::process::id(), timestamp));
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
    temp_dir
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
/// `wda init`'s own identity-fallback path (`WDA <wda@localhost>`) takes over
/// when no identity resolves, so init still succeeds.
fn isolated_home_dir() -> PathBuf {
    create_temp_dir("wda_test_isolated_home")
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
fn test_init_empty_directory_success_e2e() {
    let temp_dir = create_temp_dir("wda_test_e2e_init_success");

    // Execute wda init in empty directory. `deno` is excluded from PATH so
    // online Spectrum 2 resolution deterministically falls back to WDA Minimal
    // instead of reaching the real npm registry under the bare `cargo test`,
    // while `git` and its supporting toolchain stay reachable. HOME/USERPROFILE
    // and the global git config paths are isolated so this machine's own git
    // identity and hooks never leak into the test.
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
        Some(0),
        "init in empty directory must exit 0"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The isolated HOME (see `isolated_home_dir`) has no git identity
    // configured, so init's `wda.init.git-identity-fallback` warning is
    // expected on stderr — this is a real, successful exercise of the
    // fallback-identity contract, not a failure.
    assert!(
        stderr.is_empty() || stderr.contains("wda.init.git-identity-fallback"),
        "stderr must be empty, or contain only the expected git-identity-fallback warning, got: {}",
        stderr
    );

    // G11 report assertions on stdout
    assert!(
        stdout.contains("Project initialized."),
        "stdout must contain 'Project initialized.', got: {}",
        stdout
    );
    assert!(
        stdout.contains("Design System: WDA Minimal"),
        "stdout must contain 'Design System: WDA Minimal', got: {}",
        stdout
    );
    assert!(
        stdout.contains("Online Design System resolution requires 'deno', which was not found on PATH"),
        "stdout must contain fallback reason, got: {}",
        stdout
    );

    // Verify all 8 expected output paths exist
    let expected_files = [
        "wda.json",
        "README.md",
        "docs/architecture.md",
        "docs/design.md",
        "docs/naming.md",
        "tokens/tokens.json",
        "pages/index.html",
        ".gitignore",
    ];

    for file_rel in &expected_files {
        let path = temp_dir.join(file_rel);
        assert!(
            path.exists(),
            "expected output file '{}' must exist",
            file_rel
        );
        assert!(
            path.is_file(),
            "expected output path '{}' must be a regular file",
            file_rel
        );
    }

    // Verify forbidden files are absent
    let forbidden_files = [
        "docs/development.md",
        "docs/handoff.md",
        "package.json",
        "package-lock.json",
        "yarn.lock",
        "pnpm-lock.yaml",
        "deno.lock",
        "bun.lockb",
        "Cargo.lock",
    ];

    for file_rel in &forbidden_files {
        let path = temp_dir.join(file_rel);
        assert!(
            !path.exists(),
            "forbidden file '{}' must not exist after init",
            file_rel
        );
    }

    // Verify no lock files exist anywhere in the generated project
    let dir_state = get_dir_state(&temp_dir);
    for (rel_path, _) in &dir_state {
        let file_name = rel_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        assert!(
            !file_name.contains("lock"),
            "no lock file should be generated, found: {}",
            rel_path.display()
        );
    }

    // Verify docs/ contains exactly 3 Markdown files
    let docs_dir = temp_dir.join("docs");
    let docs_entries = fs::read_dir(&docs_dir).expect("failed to read docs dir");
    let mut md_count = 0;
    for entry in docs_entries.flatten() {
        let p = entry.path();
        if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("md") {
            md_count += 1;
        }
    }
    assert_eq!(
        md_count, 3,
        "docs/ must contain exactly 3 Markdown files, found {}",
        md_count
    );

    // Verify wda.json has 7 fields and valid structure
    let wda_json_content =
        fs::read_to_string(temp_dir.join("wda.json")).expect("failed to read wda.json");
    let val: serde_json::Value =
        serde_json::from_str(&wda_json_content).expect("wda.json must be valid JSON");
    let obj = val.as_object().expect("wda.json must be an object");
    assert_eq!(
        obj.len(),
        7,
        "wda.json must have exactly 7 fields, found: {:?}",
        obj.keys()
    );
    assert_eq!(obj.get("projectName").and_then(|v| v.as_str()), Some("Web Design Anchor Project"));
    assert_eq!(obj.get("pageArchitecture").and_then(|v| v.as_str()), Some("mpa"));
    assert_eq!(
        obj.get("browserBaseline").and_then(|v| v.as_str()),
        Some("baseline-widely-available")
    );
    assert_eq!(
        obj.get("accessibilityBaseline").and_then(|v| v.as_str()),
        Some("wcag-2.2-aa")
    );
    assert_eq!(
        obj.get("wdaVersion").and_then(|v| v.as_str()),
        Some(env!("CARGO_PKG_VERSION"))
    );

    let ds = obj.get("designSystem").and_then(|v| v.as_object()).expect("designSystem must be object");
    assert_eq!(ds.get("name").and_then(|v| v.as_str()), Some("WDA Minimal"));
    assert_eq!(ds.get("version").and_then(|v| v.as_str()), Some(env!("CARGO_PKG_VERSION")));

    // Verify README.md has no front matter
    let readme_content =
        fs::read_to_string(temp_dir.join("README.md")).expect("failed to read README.md");
    assert!(
        !readme_content.starts_with("---"),
        "README.md must not contain front matter"
    );
    for doc in &[
        "README.md",
        "docs/architecture.md",
        "docs/design.md",
        "docs/naming.md",
    ] {
        let content = if *doc == "README.md" {
            readme_content.clone()
        } else {
            fs::read_to_string(temp_dir.join(doc)).expect("failed to read doc")
        };
        for key in &["type:", "description:", "tags:"] {
            assert!(
                !content.contains(key),
                "{} must not contain template front matter key {}",
                doc,
                key
            );
        }
    }

    // Verify docs/ files have front matter with title, status, updated, and NO sourceRefs
    for doc in &["docs/architecture.md", "docs/design.md", "docs/naming.md"] {
        let content = fs::read_to_string(temp_dir.join(doc)).expect("failed to read doc");
        assert!(
            content.starts_with("---\n") || content.starts_with("---\r\n"),
            "{} must start with front matter",
            doc
        );
        assert!(
            content.contains("title:"),
            "{} must contain title in front matter",
            doc
        );
        assert!(
            content.contains("status: active"),
            "{} must contain status: active in front matter",
            doc
        );
        assert!(
            content.contains("updated:"),
            "{} must contain updated in front matter",
            doc
        );
        assert!(
            !content.contains("sourceRefs"),
            "{} must not contain sourceRefs in front matter or body",
            doc
        );
    }

    // Verify pages/index.html title
    let index_html =
        fs::read_to_string(temp_dir.join("pages/index.html")).expect("failed to read index.html");
    assert!(
        index_html.contains("<title>Web Design Anchor Project</title>"),
        "index.html <title> must equal projectName, got: {}",
        index_html
    );
    assert!(
        !index_html.contains("WDA Minimal Starter"),
        "index.html must not contain 'WDA Minimal Starter'"
    );

    // Verify .gitignore contains /dist/
    let gitignore =
        fs::read_to_string(temp_dir.join(".gitignore")).expect("failed to read .gitignore");
    assert!(
        gitignore.contains("/dist/"),
        ".gitignore must contain /dist/"
    );

    // Verify wda check passes on initialized project
    let check_output = Command::new(get_bin_path())
        .arg("check")
        .current_dir(&temp_dir)
        .output()
        .expect("failed to execute wda check");

    assert_eq!(
        check_output.status.code(),
        Some(0),
        "wda check on initialized project must exit 0"
    );

    let check_stdout = String::from_utf8_lossy(&check_output.stdout);
    let check_stderr = String::from_utf8_lossy(&check_output.stderr);

    assert!(
        check_stderr.is_empty(),
        "wda check stderr must be empty (0 errors, 0 warnings), got: {}",
        check_stderr
    );
    assert!(
        check_stdout.contains("Validation succeeded."),
        "wda check stdout must contain 'Validation succeeded.'"
    );

    // Repository assertions
    let git_dir = temp_dir.join(".git");
    if git_dir.exists() {
        let log_output = Command::new("git")
            .args(["log", "--oneline"])
            .current_dir(&temp_dir)
            .output()
            .expect("failed to execute git log");
        assert_eq!(log_output.status.code(), Some(0));
        let log_stdout = String::from_utf8_lossy(&log_output.stdout);
        let commit_lines: Vec<&str> = log_stdout.lines().collect();
        assert_eq!(
            commit_lines.len(),
            1,
            "repository must have exactly one commit, got: {:?}",
            commit_lines
        );

        let tree_output = Command::new("git")
            .args(["ls-tree", "-r", "--name-only", "HEAD"])
            .current_dir(&temp_dir)
            .output()
            .expect("failed to execute git ls-tree");
        assert_eq!(tree_output.status.code(), Some(0));
        let tree_stdout = String::from_utf8_lossy(&tree_output.stdout);
        assert!(
            !tree_stdout.lines().any(|l| l.trim().starts_with("dist/")),
            "initial commit must not contain dist/, got: {}",
            tree_stdout
        );

        let remote_output = Command::new("git")
            .args(["remote", "-v"])
            .current_dir(&temp_dir)
            .output()
            .expect("failed to execute git remote");
        assert_eq!(remote_output.status.code(), Some(0));
        let remote_stdout = String::from_utf8_lossy(&remote_output.stdout);
        assert!(
            remote_stdout.trim().is_empty(),
            "git remote -v must be empty, got: {}",
            remote_stdout
        );
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_init_allowed_preexisting_files_success_e2e() {
    let temp_dir = create_temp_dir("wda_test_e2e_init_allowed_files");

    // Seed allowed pre-existing entries: notes.txt, logo.png, assets/hero.png,
    // .DS_Store, components/old.html
    let files_to_seed = [
        ("notes.txt", "project notes\n".as_bytes()),
        ("logo.png", b"\x89PNG\r\n\x1a\npre-existing-logo"),
        ("assets/hero.png", b"\x89PNG\r\n\x1a\npre-existing-hero"),
        (".DS_Store", b"\x00\x00\x00\x01Bud1fake-ds-store"),
        ("components/old.html", b"<div>legacy component</div>"),
    ];

    for (rel_path, content) in &files_to_seed {
        let full_path = temp_dir.join(rel_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("failed to create parent dir");
        }
        fs::write(&full_path, content).expect("failed to write seeded file");
    }

    let before_state = get_dir_state(&temp_dir);
    assert_eq!(
        before_state.len(),
        7, // 5 files + 2 directories (assets, components)
        "precondition: directory must contain seeded entries"
    );

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
        Some(0),
        "init with allowed pre-existing files must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The isolated HOME (see `isolated_home_dir`) has no git identity
    // configured, so init's `wda.init.git-identity-fallback` warning is
    // expected on stderr — this is a real, successful exercise of the
    // fallback-identity contract, not a failure.
    assert!(
        stderr.is_empty() || stderr.contains("wda.init.git-identity-fallback"),
        "stderr must be empty, or contain only the expected git-identity-fallback warning, got: {}",
        stderr
    );
    assert!(
        stdout.contains("Project initialized."),
        "stdout must contain 'Project initialized.', got: {}",
        stdout
    );

    // Verify all 5 pre-existing files are untouched and byte-identical
    for (rel_path, expected_content) in &files_to_seed {
        let full_path = temp_dir.join(rel_path);
        assert!(
            full_path.is_file(),
            "pre-existing file '{}' must still exist as a regular file",
            rel_path
        );
        let actual_content = fs::read(&full_path).expect("failed to read file");
        assert_eq!(
            &actual_content[..],
            *expected_content,
            "pre-existing file '{}' content must remain untouched",
            rel_path
        );
    }

    // Verify expected output files exist
    let expected_files = [
        "wda.json",
        "README.md",
        "docs/architecture.md",
        "docs/design.md",
        "docs/naming.md",
        "tokens/tokens.json",
        "pages/index.html",
        ".gitignore",
    ];

    for file_rel in &expected_files {
        let path = temp_dir.join(file_rel);
        assert!(
            path.is_file(),
            "expected output file '{}' must exist",
            file_rel
        );
    }

    // Verify wda check passes on initialized project
    let check_output = Command::new(get_bin_path())
        .arg("check")
        .current_dir(&temp_dir)
        .output()
        .expect("failed to execute wda check");

    assert_eq!(
        check_output.status.code(),
        Some(0),
        "wda check on initialized project must exit 0, stderr: {}",
        String::from_utf8_lossy(&check_output.stderr)
    );

    // Repository assertions
    let git_dir = temp_dir.join(".git");
    if git_dir.exists() {
        let log_output = Command::new("git")
            .args(["log", "--oneline"])
            .current_dir(&temp_dir)
            .output()
            .expect("failed to execute git log");
        assert_eq!(log_output.status.code(), Some(0));
        let log_stdout = String::from_utf8_lossy(&log_output.stdout);
        let commit_lines: Vec<&str> = log_stdout.lines().collect();
        assert_eq!(
            commit_lines.len(),
            1,
            "repository must have exactly one commit, got: {:?}",
            commit_lines
        );

        let tree_output = Command::new("git")
            .args(["ls-tree", "-r", "--name-only", "HEAD"])
            .current_dir(&temp_dir)
            .output()
            .expect("failed to execute git ls-tree");
        assert_eq!(tree_output.status.code(), Some(0));
        let tree_stdout = String::from_utf8_lossy(&tree_output.stdout);
        let committed_files: Vec<&str> = tree_stdout.lines().map(|l| l.trim()).collect();
        for (rel_path, _) in &files_to_seed {
            let normalized = rel_path.replace('\\', "/");
            assert!(
                committed_files.iter().any(|f| f == &normalized),
                "pre-existing file '{}' must be committed in initial commit, git ls-tree:\n{}",
                normalized,
                tree_stdout
            );
        }
        assert!(
            !committed_files.iter().any(|f| f.starts_with("dist/")),
            "committed tree must not contain dist/, got: {:?}",
            committed_files
        );

        let remote_output = Command::new("git")
            .args(["remote", "-v"])
            .current_dir(&temp_dir)
            .output()
            .expect("failed to execute git remote");
        assert_eq!(remote_output.status.code(), Some(0));
        let remote_stdout = String::from_utf8_lossy(&remote_output.stdout);
        assert!(
            remote_stdout.trim().is_empty(),
            "git remote -v must be empty, got: {}",
            remote_stdout
        );
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
#[ignore]
fn test_spectrum_two_init_creates_dependency_files_matching_lockfile_and_builds() {
    // Real-binary probe: online design-system resolution picks Spectrum 2 only
    // when `deno` is on PATH and the real npm registry is reachable, so this is
    // the one path that actually reaches the real `crate::deps::add` call chain.
    // Matches the real-registry probe tier in `src/deps/mod.rs`,
    // `src/deps/resolve.rs`, and `tests/build.rs`; run separately via
    // `cargo test -- --ignored` (the CI probe job installs `deno`).
    let temp_dir = create_temp_dir("wda_test_spectrum_two_init_success");

    let output = Command::new(get_bin_path())
        .arg("init")
        .current_dir(&temp_dir)
        .output()
        .expect("failed to execute binary");

    assert_eq!(
        output.status.code(),
        Some(0),
        "init must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Design System: Spectrum 2"),
        "stdout must report Spectrum 2 resolution when deno and the registry are reachable, got: {}",
        stdout
    );

    // scripts/main.ts, package.json, and the lockfile all exist.
    for file_rel in ["scripts/main.ts", "package.json", "deno.lock"] {
        assert!(
            temp_dir.join(file_rel).is_file(),
            "expected Spectrum 2 output file '{}' must exist",
            file_rel
        );
    }

    // designSystem.version in wda.json equals the version every direct
    // dependency is locked to in the lockfile.
    let wda_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(temp_dir.join("wda.json")).expect("failed to read wda.json"),
    )
    .expect("wda.json must be valid JSON");
    let resolved_version = wda_json["designSystem"]["version"]
        .as_str()
        .expect("designSystem.version must be a string")
        .to_string();

    let package_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(temp_dir.join("package.json")).expect("failed to read package.json"),
    )
    .expect("package.json must be valid JSON");
    let deps = package_json["dependencies"]
        .as_object()
        .expect("package.json must declare a dependencies object");

    // package.json must declare exactly the six direct
    // dependencies the Spectrum 2 starter script (scripts/main.ts) imports —
    // theme, button, textfield, picker, card, dialog — per the corrected paper
    // adapter's direct-dependency set. No fewer (an unbuildable project) and no
    // more (an unpinned transitive dependency).
    let expected_packages: std::collections::BTreeSet<&str> = [
        "@spectrum-web-components/theme",
        "@spectrum-web-components/button",
        "@spectrum-web-components/textfield",
        "@spectrum-web-components/picker",
        "@spectrum-web-components/card",
        "@spectrum-web-components/dialog",
    ]
    .into_iter()
    .collect();
    let actual_packages: std::collections::BTreeSet<&str> =
        deps.keys().map(|k| k.as_str()).collect();
    assert_eq!(
        actual_packages, expected_packages,
        "package.json dependencies must be exactly the six starter-script imports, got: {:?}",
        actual_packages
    );

    for (name, spec) in deps {
        let spec = spec.as_str().expect("dependency spec must be a string");
        assert_eq!(
            spec, resolved_version,
            "package.json dependency '{}' must be pinned to designSystem.version ({}), got '{}'",
            name, resolved_version, spec
        );
    }

    let lock_content =
        fs::read_to_string(temp_dir.join("deno.lock")).expect("failed to read deno.lock");
    for name in deps.keys() {
        let needle = format!("npm:{name}@{resolved_version}");
        assert!(
            lock_content.contains(&needle),
            "deno.lock must lock '{}' to designSystem.version ({}), expected to find '{}'",
            name, resolved_version, needle
        );
    }

    // check reports zero Errors.
    let check_output = Command::new(get_bin_path())
        .arg("check")
        .current_dir(&temp_dir)
        .output()
        .expect("failed to execute wda check");
    assert_eq!(
        check_output.status.code(),
        Some(0),
        "wda check on Spectrum 2 project must exit 0, stderr: {}",
        String::from_utf8_lossy(&check_output.stderr)
    );

    // build succeeds with no intervening command.
    let build_output = Command::new(get_bin_path())
        .arg("build")
        .current_dir(&temp_dir)
        .output()
        .expect("failed to execute wda build");
    assert_eq!(
        build_output.status.code(),
        Some(0),
        "wda build on Spectrum 2 project must exit 0, stderr: {}",
        String::from_utf8_lossy(&build_output.stderr)
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_init_refusal_for_each_entry_class() {
    enum EntryKind {
        Directory(&'static str),
        RegularFile(&'static str),
    }

    let cases: Vec<(&'static str, EntryKind, &'static str)> = vec![
        ("dot_git_dir", EntryKind::Directory(".git"), ".git"),
        ("dot_gitignore_file", EntryKind::RegularFile(".gitignore"), ".gitignore"),
        ("docs_dir", EntryKind::Directory("docs"), "docs"),
        ("pages_dir", EntryKind::Directory("pages"), "pages"),
        ("docs_file", EntryKind::RegularFile("docs"), "docs"),
    ];

    for (case_name, entry_kind, expected_path) in cases {
        let temp_dir = create_temp_dir(&format!("wda_test_init_refusal_{}", case_name));

        match entry_kind {
            EntryKind::RegularFile(name) => {
                let file_path = temp_dir.join(name);
                fs::write(&file_path, "pre-existing content").expect("failed to write file");
            }
            EntryKind::Directory(name) => {
                let dir_path = temp_dir.join(name);
                fs::create_dir_all(&dir_path).expect("failed to create directory");
            }
        }

        let before_state = get_dir_state(&temp_dir);
        assert!(
            !before_state.is_empty(),
            "pre-condition: directory must have entries"
        );

        // `deno` is excluded from PATH so `init`'s online resolution
        // deterministically falls back offline instead of reaching npm, while
        // `git` and its supporting toolchain stay reachable for the hard
        // `git`-presence gate. HOME/USERPROFILE and the global git config paths
        // are isolated so this machine's own git identity and hooks never leak
        // into the test.
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
            "init in colliding directory for case '{}' must exit 1",
            case_name
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            stdout.is_empty(),
            "stdout must be empty on refusal for case '{}', got: {}",
            case_name,
            stdout
        );
        assert!(
            stderr.contains("wda.init.path-conflict"),
            "stderr must contain wda.init.path-conflict for case '{}', got: {}",
            case_name,
            stderr
        );
        assert!(
            stderr.contains(expected_path),
            "stderr must name colliding path '{}' for case '{}', got: {}",
            expected_path,
            case_name,
            stderr
        );

        let after_state = get_dir_state(&temp_dir);
        assert_eq!(
            before_state, after_state,
            "filesystem state must remain unchanged after refusal for case '{}'",
            case_name
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
