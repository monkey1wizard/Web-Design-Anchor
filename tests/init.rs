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

/// Hides Deno and package managers while retaining other PATH directories for git.
fn offline_init_path() -> std::ffi::OsString {
    filter_init_path(&env::var_os("PATH").unwrap_or_default())
}

fn filter_init_path(path: &std::ffi::OsStr) -> std::ffi::OsString {
    let descriptor: serde_json::Value =
        serde_json::from_str(include_str!("../packaging/toolchain.json")).unwrap();
    let mut names: Vec<&str> = descriptor["package_managers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    names.extend(["deno", "npm", "bun", "pnpm", "yarn"]);
    let filtered: Vec<PathBuf> = env::split_paths(path)
        .filter(|dir| {
            !names.iter().any(|name| {
                [
                    name.to_string(),
                    format!("{name}.exe"),
                    format!("{name}.cmd"),
                    format!("{name}.bat"),
                    format!("{name}.com"),
                    format!("{name}.ps1"),
                ]
                .iter()
                .any(|file| dir.join(file).is_file())
            })
        })
        .collect();
    env::join_paths(filtered).unwrap_or_default()
}

#[test]
fn native_package_manager_integration_registry_and_name_validation() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use wda_core::{registry_metadata, supported_package_managers, ManagerError, PackageManager};

    assert_eq!(supported_package_managers(), ["npm", "bun"]);
    let cwd = create_temp_dir("wda manager space");
    assert!(matches!(
        PackageManager::probe("unknown", &cwd.join("missing"), &cwd),
        Err(ManagerError::NotFound)
    ));
    assert!(matches!(
        PackageManager::probe("npm", &cwd.join("missing"), &cwd),
        Err(ManagerError::NotFound)
    ));
    assert!(matches!(
        registry_metadata("http://127.0.0.1:1/", "package;touch injected"),
        Err(ManagerError::RegistryFormat(_))
    ));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 4096];
        let count = stream.read(&mut request).unwrap();
        let request = String::from_utf8_lossy(&request[..count]);
        assert!(request.starts_with("GET /@scope%2Fpackage "), "{request}");
        let body = r#"{"dist-tags":{"latest":"1.2.3"}}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });
    let metadata = registry_metadata(&format!("http://{address}/"), "@scope/package").unwrap();
    assert_eq!(metadata["dist-tags"]["latest"], "1.2.3");
    server.join().unwrap();
    fs::remove_dir_all(cwd).unwrap();
}

#[test]
fn offline_init_path_excludes_package_managers() {
    let root = tempfile::tempdir().unwrap();
    let safe = root.path().join("safe");
    fs::create_dir(&safe).unwrap();
    let mut paths = vec![safe.clone()];
    let mut names = vec![
        "deno".to_string(),
        "npm".into(),
        "bun".into(),
        "pnpm".into(),
        "yarn".into(),
    ];
    names.extend(wda_core::supported_package_managers());
    for (index, name) in names.iter().enumerate() {
        for (suffix_index, suffix) in ["", ".exe", ".cmd", ".bat", ".com", ".ps1"]
            .iter()
            .enumerate()
        {
            let dir = root.path().join(format!("{index}-{suffix_index}"));
            fs::create_dir(&dir).unwrap();
            fs::write(dir.join(format!("{name}{suffix}")), "").unwrap();
            paths.push(dir);
        }
    }
    let path = env::join_paths(paths).unwrap();
    assert_eq!(
        env::split_paths(&filter_init_path(&path)).collect::<Vec<_>>(),
        vec![safe]
    );
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

    // Execute wda init in empty directory. Deno and package managers are excluded from PATH so
    // online Spectrum 2 resolution deterministically falls back to WDA Minimal
    // instead of reaching the real npm registry under the bare `cargo test`,
    // while `git` and its supporting toolchain stay reachable. HOME/USERPROFILE
    // and the global git config paths are isolated so this machine's own git
    // identity and hooks never leak into the test.
    let isolated_home = isolated_home_dir();
    let output = Command::new(get_bin_path())
        .arg("init")
        .current_dir(&temp_dir)
        .env("PATH", offline_init_path())
        .env("HOME", &isolated_home)
        .env("USERPROFILE", &isolated_home)
        .env(
            "GIT_CONFIG_GLOBAL",
            isolated_home.join("nonexistent-gitconfig"),
        )
        .env(
            "GIT_CONFIG_SYSTEM",
            isolated_home.join("nonexistent-gitconfig-system"),
        )
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
        stdout.contains("No supported package manager is available"),
        "stdout must contain fallback reason, got: {}",
        stdout
    );
    assert!(
        stdout.contains("WDA Skill"),
        "stdout must hint that the Skill can add the defaults later, got: {}",
        stdout
    );

    // Verify all 9 expected output paths exist
    let expected_files = [
        "anchor.json",
        "package.json",
        "README.md",
        "docs/architecture.md",
        "docs/design.md",
        "docs/glossary.md",
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
        "node_modules",
        "package-lock.json",
        "wda.json",
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

    // Verify anchor.json has the 7 contract fields plus the fallback record
    let wda_json_content =
        fs::read_to_string(temp_dir.join("anchor.json")).expect("failed to read anchor.json");
    let val: serde_json::Value =
        serde_json::from_str(&wda_json_content).expect("anchor.json must be valid JSON");
    let obj = val.as_object().expect("anchor.json must be an object");
    assert_eq!(
        obj.len(),
        8,
        "anchor.json must have 7 contract fields plus initialization, found: {:?}",
        obj.keys()
    );
    assert_eq!(
        val["initialization"]["fallback"]["requestedDesignSystem"],
        "Spectrum 2"
    );
    let package_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(temp_dir.join("package.json")).expect("failed to read package.json"),
    )
    .expect("package.json must be valid JSON");
    assert_eq!(package_json["dependencies"], serde_json::json!({}));
    assert!(package_json.get("packageManager").is_none());
    assert_eq!(
        obj.get("projectName").and_then(|v| v.as_str()),
        Some("Web Design Anchor Project")
    );
    assert_eq!(
        obj.get("pageArchitecture").and_then(|v| v.as_str()),
        Some("mpa")
    );
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

    let ds = obj
        .get("designSystem")
        .and_then(|v| v.as_object())
        .expect("designSystem must be object");
    assert_eq!(ds.get("name").and_then(|v| v.as_str()), Some("WDA Minimal"));
    assert_eq!(
        ds.get("version").and_then(|v| v.as_str()),
        Some(env!("CARGO_PKG_VERSION"))
    );

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
        "docs/glossary.md",
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
    for doc in &["docs/architecture.md", "docs/design.md", "docs/glossary.md"] {
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
        gitignore.contains("/dist/") && gitignore.contains("/node_modules/"),
        ".gitignore must contain /dist/ and /node_modules/"
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
        project_stderr(&check_stderr).is_empty(),
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
        .env("PATH", offline_init_path())
        .env("HOME", &isolated_home)
        .env("USERPROFILE", &isolated_home)
        .env(
            "GIT_CONFIG_GLOBAL",
            isolated_home.join("nonexistent-gitconfig"),
        )
        .env(
            "GIT_CONFIG_SYSTEM",
            isolated_home.join("nonexistent-gitconfig-system"),
        )
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
        "anchor.json",
        "package.json",
        "README.md",
        "docs/architecture.md",
        "docs/design.md",
        "docs/glossary.md",
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
fn test_spectrum_two_init_with_real_manager_installs_pinned_defaults() {
    // Real-binary probe: needs a real npm or Bun and the real registry, so it is
    // outside the ordinary suite; run separately via `cargo test -- --ignored`.
    // Either outcome must leave a complete project: Spectrum 2 with pinned
    // dependencies, or WDA Minimal with the manager-reported reason.
    let temp_dir = create_temp_dir("wda_test_spectrum_two_init_real_manager");

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
    for file_rel in ["anchor.json", "package.json", "pages/index.html"] {
        assert!(temp_dir.join(file_rel).is_file(), "{file_rel} must exist");
    }
    if stdout.contains("Design System: Spectrum 2") {
        let anchor: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(temp_dir.join("anchor.json")).expect("read anchor.json"),
        )
        .unwrap();
        let package: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(temp_dir.join("package.json")).expect("read package.json"),
        )
        .unwrap();
        let version = anchor["designSystem"]["version"].as_str().unwrap();
        assert_eq!(package["dependencies"]["@spectrum-web-components/button"], version);
        assert!(package["packageManager"].as_str().unwrap().contains('@'));
        assert!(temp_dir.join("node_modules").is_dir());
        assert!(temp_dir.join("scripts/main.ts").is_file());
    } else {
        assert!(stdout.contains("Design System: WDA Minimal"), "{stdout}");
        assert!(stdout.contains("Note:"), "the actual reason must be printed: {stdout}");
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

// ---- native init transaction: fake managers on PATH, a local registry, a seeded tool cache ----

const FAKE_MANAGER_VERSION: &str = "9.8.7";

/// Writes an executable stand-in for a package manager into `dir`. `--version` prints
/// a fixed version; `ci` / `install` records that it ran in `<dir>/<name>.ran`, then
/// either fails with "boom" or creates `node_modules` and the manager's lockfile in
/// its working directory. Nothing here talks to a registry.
fn write_fake_manager(dir: &Path, name: &str, fail_install: bool) {
    let lock = if name == "bun" { "bun.lock" } else { "package-lock.json" };
    let marker = dir.join(format!("{name}.ran"));
    #[cfg(windows)]
    {
        let body = if fail_install {
            "echo boom 1>&2\r\nexit /b 1\r\n".to_string()
        } else {
            format!("mkdir node_modules\\lit\r\necho {{}}> {lock}\r\nexit /b 0\r\n")
        };
        let script = format!(
            "@echo off\r\nif \"%1\"==\"--version\" (\r\n  echo {FAKE_MANAGER_VERSION}\r\n  exit /b 0\r\n)\r\necho ran> \"{}\"\r\n{body}",
            marker.display()
        );
        fs::write(dir.join(format!("{name}.cmd")), script).unwrap();
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let body = if fail_install {
            "echo boom >&2\nexit 1\n".to_string()
        } else {
            format!("/bin/mkdir -p node_modules/lit\necho '{{}}' > {lock}\nexit 0\n")
        };
        let script = format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo {FAKE_MANAGER_VERSION}; exit 0; fi\necho ran > '{}'\n{body}",
            marker.display()
        );
        let path = dir.join(name);
        fs::write(&path, script).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// Serves `{"dist-tags":{"latest":...}}` for any package on a local port: `lit` gets
/// the Lit version, every other package the Spectrum version.
fn start_fake_registry(spectrum: &'static str, lit: &'static str) -> String {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buffer = [0u8; 2048];
            let read = stream.read(&mut buffer).unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..read]).into_owned();
            let first_line = request.lines().next().unwrap_or_default().to_string();
            let version = if first_line.contains("/lit ") { lit } else { spectrum };
            let body = format!("{{\"dist-tags\":{{\"latest\":\"{version}\"}}}}");
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
        }
    });
    format!("http://{address}/")
}

/// A registry URL nothing listens on, so a connection is refused.
fn unreachable_registry() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    format!("http://{address}/")
}

/// Seeds a complete tool cache for this platform under `cache_base`, so init's tool
/// restore finds it and never reaches a network. Returns the base to export as the
/// platform cache variable.
fn seed_tool_cache(cache_base: &Path) {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let os = match env::consts::OS {
        "windows" => "win32",
        "macos" => "darwin",
        _ => "linux",
    };
    let arch = if env::consts::ARCH == "aarch64" { "arm64" } else { "x64" };
    let platforms =
        wda_core::tools::validate_descriptor(include_str!("../packaging/toolchain.json")).unwrap();
    let platform = platforms
        .into_iter()
        .find(|p| p.platform == format!("{os}-{arch}"))
        .expect("descriptor row for this platform");
    let digest = STANDARD
        .decode(platform.integrity.strip_prefix("sha512-").unwrap())
        .unwrap();
    let key: String = digest.iter().take(16).map(|b| format!("{b:02x}")).collect();
    let root = cache_base.join("wda").join("tools").join(key);
    fs::create_dir_all(root.join("package/lib")).unwrap();
    fs::write(root.join(&platform.executable), "").unwrap();
    fs::write(root.join("package/LICENSE"), "").unwrap();
    fs::write(root.join("package/lib/lib.d.ts"), "").unwrap();
}

struct NativeInit {
    project: PathBuf,
    tools: PathBuf,
    fakes: PathBuf,
}

impl NativeInit {
    fn new(tag: &str) -> Self {
        let tools = create_temp_dir(&format!("wda_native_init_cache_{tag}"));
        seed_tool_cache(&tools);
        Self {
            project: create_temp_dir(&format!("wda_native_init_{tag}")),
            tools,
            fakes: create_temp_dir(&format!("wda_native_init_fakes_{tag}")),
        }
    }

    fn marker(&self, name: &str) -> PathBuf {
        self.fakes.join(format!("{name}.ran"))
    }

    /// Runs `wda init <args>` with the fake manager directory first on PATH. `with_git`
    /// keeps the host's git reachable; otherwise PATH holds only the fakes.
    fn run(&self, registry: &str, with_git: bool, args: &[&str]) -> std::process::Output {
        let mut dirs = vec![self.fakes.clone()];
        if with_git {
            dirs.extend(env::split_paths(&filter_init_path(
                &env::var_os("PATH").unwrap_or_default(),
            )));
        }
        let isolated_home = isolated_home_dir();
        let mut command = Command::new(get_bin_path());
        command
            .arg("init")
            .args(args)
            .current_dir(&self.project)
            .env("PATH", env::join_paths(dirs).unwrap())
            .env("npm_config_registry", registry)
            .env("HOME", &isolated_home)
            .env("USERPROFILE", &isolated_home)
            .env("LOCALAPPDATA", &self.tools)
            .env("XDG_CACHE_HOME", &self.tools)
            .env("GIT_CONFIG_GLOBAL", isolated_home.join("nonexistent-gitconfig"))
            .env("GIT_CONFIG_SYSTEM", isolated_home.join("nonexistent-gitconfig-system"));
        command.output().expect("failed to execute binary")
    }

    fn json(&self, name: &str) -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(self.project.join(name)).unwrap()).unwrap()
    }

    fn top_level(&self) -> std::collections::BTreeSet<String> {
        fs::read_dir(&self.project)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect()
    }
}

impl Drop for NativeInit {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.project);
        let _ = fs::remove_dir_all(&self.tools);
        let _ = fs::remove_dir_all(&self.fakes);
    }
}

fn set_of(items: &[&str]) -> std::collections::BTreeSet<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn native_init_transaction_integration_offline_with_cache_creates_minimal() {
    let t = NativeInit::new("offline");
    write_fake_manager(&t.fakes, "npm", false);
    let out = t.run(&unreachable_registry(), true, &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert_eq!(out.status.code(), Some(0), "stderr: {stderr}");
    assert!(stdout.contains("Design System: WDA Minimal"), "{stdout}");
    assert!(stdout.contains("registry unreachable"), "{stdout}");
    assert!(stdout.contains("WDA Skill"), "{stdout}");
    assert!(
        !t.marker("npm").exists(),
        "a seeded tool cache and an installed manager must not switch init to Spectrum"
    );
    assert!(!t.project.join("node_modules").exists());
    assert!(!t.project.join("scripts").exists());
    assert_eq!(t.json("package.json")["dependencies"], serde_json::json!({}));
    assert!(t.json("package.json").get("packageManager").is_none());
    assert_eq!(
        t.json("anchor.json")["initialization"]["fallback"]["requestedDesignSystem"],
        "Spectrum 2"
    );
}

#[test]
fn native_init_transaction_integration_one_manager_online_installs_default() {
    let t = NativeInit::new("one");
    write_fake_manager(&t.fakes, "npm", false);
    let registry = start_fake_registry("1.2.3", "3.4.5");
    let out = t.run(&registry, true, &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert_eq!(out.status.code(), Some(0), "stderr: {stderr}");
    assert!(stdout.contains("Design System: Spectrum 2"), "{stdout}");
    assert!(t.marker("npm").is_file(), "the manager ran the stage install");
    assert_eq!(
        t.top_level(),
        set_of(&[
            ".git", ".gitignore", "README.md", "anchor.json", "docs", "node_modules",
            "package-lock.json", "package.json", "pages", "scripts", "tokens",
        ]),
        "stage directory is gone and install output was committed"
    );
    let package = t.json("package.json");
    assert_eq!(package["packageManager"], format!("npm@{FAKE_MANAGER_VERSION}"));
    assert_eq!(package["dependencies"]["lit"], "3.4.5");
    assert_eq!(package["dependencies"]["@spectrum-web-components/button"], "1.2.3");
    assert_eq!(t.json("anchor.json")["designSystem"]["name"], "Spectrum 2");
    assert!(t.json("anchor.json").get("initialization").is_none());
    let log = Command::new("git")
        .args(["log", "--oneline"])
        .current_dir(&t.project)
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&log.stdout).contains("wda init"));
}

#[test]
fn native_init_transaction_integration_two_managers_without_terminal_exits_usage_2() {
    let t = NativeInit::new("two");
    write_fake_manager(&t.fakes, "npm", false);
    write_fake_manager(&t.fakes, "bun", false);
    let registry = start_fake_registry("1.2.3", "3.4.5");
    let out = t.run(&registry, true, &[]);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert_eq!(out.status.code(), Some(2), "stderr: {stderr}");
    assert!(stderr.contains("--package-manager"), "{stderr}");
    assert!(stderr.contains("bun, npm"), "{stderr}");
    assert!(out.stdout.is_empty());
    assert!(t.top_level().is_empty(), "nothing is written when the choice is ambiguous");
    assert!(!t.marker("npm").exists() && !t.marker("bun").exists());

    let named = t.run(&registry, true, &["--package-manager", "bun"]);
    assert_eq!(
        named.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&named.stderr)
    );
    assert!(t.marker("bun").is_file() && !t.marker("npm").exists());
    assert!(t.project.join("bun.lock").is_file());
    assert_eq!(
        t.json("package.json")["packageManager"],
        format!("bun@{FAKE_MANAGER_VERSION}")
    );
}

#[test]
fn native_init_transaction_integration_named_manager_missing_never_switches() {
    let t = NativeInit::new("named_missing");
    write_fake_manager(&t.fakes, "npm", false);
    let registry = start_fake_registry("1.2.3", "3.4.5");
    let out = t.run(&registry, true, &["--package-manager", "bun"]);
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert_eq!(out.status.code(), Some(0));
    assert!(stdout.contains("Design System: WDA Minimal"), "{stdout}");
    assert!(stdout.contains("bun") && stdout.contains("not available"), "{stdout}");
    assert!(!t.marker("npm").exists(), "an unavailable named manager never switches");
    assert!(!t.project.join("package-lock.json").exists());
}

#[test]
fn native_init_transaction_integration_install_failure_leaves_complete_minimal() {
    let t = NativeInit::new("install_fail");
    write_fake_manager(&t.fakes, "npm", true);
    let registry = start_fake_registry("1.2.3", "3.4.5");
    let out = t.run(&registry, true, &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert_eq!(out.status.code(), Some(0));
    assert!(stdout.contains("Design System: WDA Minimal"), "{stdout}");
    assert!(stdout.contains("boom"), "the manager-reported reason is printed: {stdout}");
    assert!(stdout.contains("WDA Skill"), "{stdout}");
    assert!(t.marker("npm").is_file());
    assert_eq!(
        t.top_level(),
        set_of(&[
            ".git", ".gitignore", "README.md", "anchor.json", "docs", "package.json", "pages",
            "tokens",
        ]),
        "no stage, no node_modules, no Spectrum-only content"
    );
}

#[test]
fn native_init_transaction_integration_collision_is_refused_before_any_install() {
    let t = NativeInit::new("collision");
    write_fake_manager(&t.fakes, "npm", false);
    fs::write(t.project.join("package.json"), "{\"name\":\"mine\"}").unwrap();
    let before = get_dir_state(&t.project);
    let registry = start_fake_registry("1.2.3", "3.4.5");
    let out = t.run(&registry, true, &[]);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert_eq!(out.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("wda.init.path-conflict"), "{stderr}");
    assert!(stderr.contains("package.json"), "{stderr}");
    assert!(out.stdout.is_empty());
    assert!(!t.marker("npm").exists());
    assert_eq!(before, get_dir_state(&t.project));
}

#[test]
fn native_init_transaction_integration_missing_git_warns_and_keeps_gitignore() {
    let t = NativeInit::new("no_git");
    write_fake_manager(&t.fakes, "npm", false);
    let registry = start_fake_registry("1.2.3", "3.4.5");
    let out = t.run(&registry, false, &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert_eq!(out.status.code(), Some(0), "stderr: {stderr}");
    assert!(stderr.contains("wda.toolchain.git-missing"), "{stderr}");
    assert!(stdout.contains("Design System: Spectrum 2"), "{stdout}");
    assert!(!t.project.join(".git").exists());
    let gitignore = fs::read_to_string(t.project.join(".gitignore")).unwrap();
    assert!(gitignore.contains("/dist/") && gitignore.contains("/node_modules/"));
    assert!(t.project.join("package.json").is_file() && t.project.join("anchor.json").is_file());
}

#[test]
fn test_init_refusal_for_each_entry_class() {
    enum EntryKind {
        Directory(&'static str),
        RegularFile(&'static str),
    }

    let cases: Vec<(&'static str, EntryKind, &'static str)> = vec![
        ("dot_git_dir", EntryKind::Directory(".git"), ".git"),
        (
            "dot_gitignore_file",
            EntryKind::RegularFile(".gitignore"),
            ".gitignore",
        ),
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

        // Deno and package managers are excluded from PATH so `init`'s online resolution
        // deterministically falls back offline instead of reaching npm, while
        // `git` and its supporting toolchain stay reachable for the hard
        // `git`-presence gate. HOME/USERPROFILE and the global git config paths
        // are isolated so this machine's own git identity and hooks never leak
        // into the test.
        let isolated_home = isolated_home_dir();
        let output = Command::new(get_bin_path())
            .arg("init")
            .current_dir(&temp_dir)
            .env("PATH", offline_init_path())
            .env("HOME", &isolated_home)
            .env("USERPROFILE", &isolated_home)
            .env(
                "GIT_CONFIG_GLOBAL",
                isolated_home.join("nonexistent-gitconfig"),
            )
            .env(
                "GIT_CONFIG_SYSTEM",
                isolated_home.join("nonexistent-gitconfig-system"),
            )
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

fn compat_external_init_with(manager: &str) {
    assert!(wda_core::supported_package_managers()
        .iter()
        .any(|name| name == manager));
    let temp_dir = create_temp_dir(&format!("compat_init_{manager}"));
    let (output, anchor) = (1..=3).find_map(|attempt| {
        let output = Command::new(get_bin_path())
            .args(["init", "--package-manager", manager])
            .current_dir(&temp_dir)
            .output()
            .expect("wda init must start");
        assert!(
            output.status.success(),
            "init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let anchor: serde_json::Value =
            serde_json::from_slice(&fs::read(temp_dir.join("anchor.json")).unwrap()).unwrap();
        let reason = anchor["initialization"]["fallback"]["reason"]
            .as_str()
            .unwrap_or_default();
        let registry_unreachable = anchor["designSystem"]["name"] == "WDA Minimal"
            && (reason.starts_with("Default Spectrum 2 and Lit were not resolved:")
                || reason.starts_with("Lit version was not resolved:"))
            && reason.contains("registry unreachable");
        if registry_unreachable {
            assert!(
                attempt < 3,
                "registry unreachable environment failure after {attempt} init attempts for {manager}: {reason}"
            );
            fs::remove_dir_all(&temp_dir).unwrap();
            fs::create_dir(&temp_dir).unwrap();
            std::thread::sleep(std::time::Duration::from_secs(1));
            None
        } else {
            Some((output, anchor))
        }
    }).unwrap();
    assert_eq!(
        anchor["designSystem"]["name"],
        "Spectrum 2",
        "init must install rather than fall back: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let package: serde_json::Value =
        serde_json::from_slice(&fs::read(temp_dir.join("package.json")).unwrap()).unwrap();
    assert!(package["packageManager"]
        .as_str()
        .unwrap()
        .starts_with(&format!("{manager}@")));
    assert!(temp_dir.join("node_modules/lit/package.json").is_file());
    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
#[ignore]
fn compat_external_init_real_npm() {
    compat_external_init_with("npm");
}

#[test]
#[ignore]
fn compat_external_init_real_bun() {
    compat_external_init_with("bun");
}
