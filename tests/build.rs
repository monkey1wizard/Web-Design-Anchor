use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_wda")
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
        // Keep init offline while retaining git and its supporting tools. Build
        // invocations use the unmodified PATH to exercise the real Deno binary.
        let isolated_home = isolated_home_dir();
        cmd.env("PATH", offline_init_path());
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

// ---------------------------------------------------------------------------
// Native build helpers
// ---------------------------------------------------------------------------

/// PATH with every JavaScript runtime and package manager hidden, so a passing build proves
/// the native pipeline needs none of them.
fn tool_free_path() -> OsString {
    let hidden = ["node", "bun", "deno", "npm", "npx", "pnpm", "yarn"];
    let kept: Vec<PathBuf> = env::split_paths(&env::var_os("PATH").unwrap_or_default())
        .filter(|dir| {
            !hidden.iter().any(|name| {
                ["", ".exe", ".cmd", ".bat", ".com", ".ps1"]
                    .iter()
                    .any(|suffix| dir.join(format!("{name}{suffix}")).is_file())
            })
        })
        .collect();
    env::join_paths(kept).unwrap_or_default()
}

fn tool_executable_relative() -> &'static str {
    if cfg!(windows) {
        "package/lib/tsc.exe"
    } else {
        "package/lib/tsc"
    }
}

/// Hex form of the embedded SRI for this platform: the name of the cache directory.
fn tool_cache_key() -> String {
    use base64::Engine;
    let descriptor: serde_json::Value =
        serde_json::from_str(include_str!("../packaging/toolchain.json")).unwrap();
    let os = match env::consts::OS {
        "windows" => "win32",
        "macos" => "darwin",
        _ => "linux",
    };
    let arch = if env::consts::ARCH == "aarch64" {
        "arm64"
    } else {
        "x64"
    };
    let wanted = format!("{os}-{arch}");
    let row = descriptor["platforms"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["platform"] == wanted)
        .expect("the descriptor must list this platform");
    let integrity = row["integrity"].as_str().unwrap();
    let digest = base64::engine::general_purpose::STANDARD
        .decode(integrity.strip_prefix("sha512-").unwrap())
        .unwrap();
    digest
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn tool_cache_dir(base: &Path) -> PathBuf {
    base.join("wda").join("tools").join(tool_cache_key())
}

fn real_cache_base() -> Option<PathBuf> {
    if cfg!(windows) {
        env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else {
        env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
    }
}

/// True when this machine already holds the verified native type-check tool. Tests that must
/// run the real checker skip otherwise, because ordinary tests never use the network.
fn real_tool_cache_present() -> bool {
    real_cache_base().is_some_and(|base| {
        let dir = tool_cache_dir(&base);
        dir.join(tool_executable_relative()).is_file()
            && dir.join("package/LICENSE").is_file()
            && dir.join("package/lib/lib.d.ts").is_file()
    })
}

macro_rules! require_tool_cache {
    () => {
        if !real_tool_cache_present() {
            eprintln!("skipped: the native TypeScript tool cache is not present on this machine");
            return;
        }
    };
}

#[derive(Clone)]
enum Cache {
    /// The machine's own cache.
    Real,
    /// A cache base directory used instead of the machine's own.
    Isolated(PathBuf),
}

fn run_build(project: &Path, cache: &Cache) -> Output {
    let mut command = Command::new(binary());
    command
        .arg("build")
        .current_dir(project)
        .env("PATH", tool_free_path());
    if let Cache::Isolated(base) = cache {
        command
            .env("LOCALAPPDATA", base)
            .env("XDG_CACHE_HOME", base)
            .env("HOME", base);
    }
    command.output().expect("wda must start")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A cache whose tool file is not a runnable program.
fn broken_cache() -> Cache {
    let base = temporary_project("broken_cache");
    let dir = tool_cache_dir(&base);
    fs::create_dir_all(dir.join("package/lib")).unwrap();
    fs::write(dir.join(tool_executable_relative()), "not a program").unwrap();
    fs::write(dir.join("package/LICENSE"), "license").unwrap();
    fs::write(dir.join("package/lib/lib.d.ts"), "").unwrap();
    Cache::Isolated(base)
}

fn empty_cache() -> Cache {
    Cache::Isolated(temporary_project("empty_cache"))
}

fn write_file(project: &Path, relative: &str, contents: &str) {
    let path = project.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn page_html(title: &str, script: Option<&str>) -> String {
    let script = script
        .map(|src| format!("<script type=\"module\" src=\"{src}\"></script>"))
        .unwrap_or_default();
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>{title}</title>{script}</head><body><h1>{title}</h1></body></html>\n"
    )
}

/// A project from the mapping fixture whose only page references `scripts/main.ts`.
fn mapping_project(name: &str) -> PathBuf {
    let project = temporary_project(name);
    copy_tree(&fixture("mapping"), &project);
    project
}

#[test]
fn author_token_profile_integration() {
    let project = mapping_project("author_token_profile");
    write_file(&project, "pages/index.html", &page_html("Tokens", None));
    let source = r##"{"color":{"$type":"color","base":{"$value":"#fff"},"alias":{"$value":"{color.base}"}},"dimension":{"$type":"dimension","a":{"$value":"1rem"}},"fontFamily":{"$type":"fontFamily","a":{"$value":"Arial, sans-serif"}},"fontWeight":{"$type":"fontWeight","a":{"$value":400}},"lineHeight":{"$type":"lineHeight","a":{"$value":1.5}},"number":{"$type":"number","a":{"$value":2}},"string":{"$type":"string","a":{"$value":"solid"}}}"##;
    write_file(&project, "tokens/tokens.json", source);
    let build = wda_core::build_project(&project).unwrap();
    assert!(
        !build.validation.has_error(),
        "{:?}",
        build.validation.diagnostics
    );
    let css = fs::read_to_string(project.join("dist/styles/tokens.css")).unwrap();
    for expected in [
        "--color-alias: #fff;",
        "--dimension-a: 1rem;",
        "--font-family-a: Arial, sans-serif;",
        "--font-weight-a: 400;",
        "--line-height-a: 1.5;",
        "--number-a: 2;",
        "--string-a: solid;",
    ] {
        assert!(css.contains(expected), "{css}");
    }
    let before = snapshot(&project.join("dist"));
    for invalid in [
        r#"{"x":{"$type":"color","$value":"red; color: blue"}}"#,
        r#"{"x":{"$type":"color","$value":"before {x}"}}"#,
        r#"{"x":{"$type":"color","$value":{"x":1}}}"#,
        r#"{"x":{"$type":"shadow","$value":"red"}}"#,
        r#"{"x":{"$type":"color","$value":"{y}"},"y":{"$type":"color","$value":"{x}"}}"#,
        r#"{"fooBar":{"$type":"string","$value":"a"},"foo-bar":{"$type":"string","$value":"b"}}"#,
    ] {
        write_file(&project, "tokens/tokens.json", invalid);
        let build = wda_core::build_project(&project).unwrap();
        assert!(build.validation.has_error(), "{invalid}");
        assert!(build
            .validation
            .diagnostics
            .iter()
            .any(|d| d.code.starts_with("wda.tokens.")
                && d.location
                    .as_ref()
                    .is_some_and(|l| l.path == Path::new("tokens/tokens.json"))));
        assert_eq!(snapshot(&project.join("dist")), before);
    }
    let mut chain = serde_json::Map::new();
    for i in 0..130 {
        let value = if i == 129 {
            "red".to_owned()
        } else {
            format!("{{t{}}}", i + 1)
        };
        chain.insert(
            format!("t{i}"),
            serde_json::json!({"$type":"color", "$value":value}),
        );
    }
    write_file(
        &project,
        "tokens/tokens.json",
        &serde_json::Value::Object(chain).to_string(),
    );
    fs::remove_dir_all(project.join("dist")).unwrap();
    let build = wda_core::build_project(&project).unwrap();
    assert!(build
        .validation
        .diagnostics
        .iter()
        .any(|d| d.code.starts_with("wda.tokens.")));
    assert!(!project.join("dist").exists());
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn author_source_analysis_shared_locations_and_guards() {
    let project = mapping_project("author_source_analysis");
    write_file(&project, "pages/index.html", "<html lang=\"en\"><body><div id=\"same\"></div>\r\n<wda-include src=\"../components/a.html\"></wda-include></body></html>");
    write_file(
        &project,
        "components/a.html",
        "<wda-include src=\"b.html\"></wda-include>",
    );
    write_file(
        &project,
        "components/b.html",
        "first\r\néé<span id=\"same\"></span>",
    );
    let check = wda_core::validate_project(&project).unwrap();
    let build = wda_core::build_project(&project).unwrap();
    let find = |report: &wda_core::diagnostics::ValidationReport| {
        report
            .diagnostics
            .iter()
            .find(|d| d.code == "wda.html.duplicate-id")
            .unwrap()
            .clone()
    };
    let duplicate = find(&check);
    assert_eq!(duplicate.location, find(&build.validation).location);
    let location = duplicate.location.unwrap();
    assert_eq!(location.path, PathBuf::from("components/b.html"));
    assert_eq!((location.line, location.column), (Some(2), Some(5)));
    assert!(!project.join("dist").exists());

    write_file(&project, "pages/index.html", "<html lang=\"en\"><body><wda-include src=\"../components/a.html\"></wda-include>\n<wda-include src=\"../components/a.html\"></wda-include></body></html>");
    let repeated = wda_core::validate_project(&project).unwrap();
    let duplicate = repeated
        .diagnostics
        .iter()
        .find(|d| d.code == "wda.html.duplicate-id")
        .unwrap();
    assert!(
        duplicate.message.contains("pages/index.html:2:1"),
        "{}",
        duplicate.message
    );
    write_file(&project, "pages/index.html", "<html lang=\"en\"><body><wda-include src=\"../components/a.html\"></wda-include></body></html>");

    for (source, code, chain) in [
        (
            "<wda-include src=\"a.html\"></wda-include>",
            "wda.build.include-cycle",
            "components/a.html -> components/a.html",
        ),
        (
            "<wda-include src=\"b.html\"></wda-include>",
            "wda.build.include-cycle",
            "components/a.html -> components/b.html -> components/a.html",
        ),
        (
            "<wda-include src=\"../../outside.html\"></wda-include>",
            "wda.build.include-escapes-root",
            "",
        ),
        (
            "<wda-include src=\"missing.html\"></wda-include>",
            "wda.build.include-unresolvable",
            "",
        ),
    ] {
        write_file(&project, "components/a.html", source);
        write_file(
            &project,
            "components/b.html",
            "<wda-include src=\"a.html\"></wda-include>",
        );
        let check = wda_core::validate_project(&project).unwrap();
        let build = wda_core::build_project(&project).unwrap();
        let expected = check.diagnostics.iter().find(|d| d.code == code).unwrap();
        let actual = build
            .validation
            .diagnostics
            .iter()
            .find(|d| d.code == code)
            .unwrap();
        assert_eq!(
            (expected.location.clone(), &expected.message),
            (actual.location.clone(), &actual.message)
        );
        assert!(expected.message.contains(chain), "{}", expected.message);
        assert!(!project.join("dist").exists());
    }
    let outside = temporary_project("author_source_outside");
    write_file(&outside, "fragment.html", "<p>outside</p>");
    if link_dir(&project.join("components/outside"), &outside) {
        write_file(
            &project,
            "components/a.html",
            "<wda-include src=\"outside/fragment.html\"></wda-include>",
        );
        let check = wda_core::validate_project(&project).unwrap();
        let build = wda_core::build_project(&project).unwrap();
        let find = |report: &wda_core::diagnostics::ValidationReport| {
            report
                .diagnostics
                .iter()
                .find(|d| d.code == "wda.build.include-escapes-root")
                .unwrap()
                .clone()
        };
        assert_eq!(find(&check).location, find(&build.validation).location);
    }
    fs::remove_dir_all(outside).unwrap();
    fs::remove_dir_all(project).unwrap();
}

/// Puts a recognizable previous output in place and returns its snapshot.
fn plant_old_dist(project: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    write_file(project, "dist/index.html", "<!-- previous build -->");
    write_file(project, "dist/previous.txt", "previous");
    snapshot(&project.join("dist"))
}

fn assert_old_dist_intact(project: &Path, before: &BTreeMap<PathBuf, Vec<u8>>) {
    assert_eq!(
        &snapshot(&project.join("dist")),
        before,
        "a rejected build must leave the previous dist/ unchanged"
    );
    let residue: Vec<String> = fs::read_dir(project.parent().unwrap())
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            name.contains(".wda-stage-")
                && name.contains(project.file_name().unwrap().to_str().unwrap())
        })
        .collect();
    assert!(
        residue.is_empty(),
        "stage directories left behind: {residue:?}"
    );
}

/// Creates a directory link; returns false when the platform refuses.
fn link_dir(link: &Path, target: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
    #[cfg(windows)]
    {
        if std::os::windows::fs::symlink_dir(target, link).is_ok() {
            return true;
        }
        Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link.to_string_lossy().replace('/', "\\"))
            .arg(target.to_string_lossy().replace('/', "\\"))
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

/// Module specifiers a built script still imports; every one must be a local relative path.
fn output_specifiers(code: &str) -> Vec<String> {
    let mut found = Vec::new();
    for opener in [
        "from \"",
        "from '",
        "import(\"",
        "import('",
        "import \"",
        "import '",
    ] {
        let quote = opener.chars().last().unwrap();
        let mut rest = code;
        while let Some(start) = rest.find(opener) {
            rest = &rest[start + opener.len()..];
            if let Some(end) = rest.find(quote) {
                found.push(rest[..end].to_string());
            }
        }
    }
    found
}

fn script_files(dist: &Path) -> Vec<(PathBuf, String)> {
    snapshot(dist)
        .into_iter()
        .filter(|(path, _)| path.extension().is_some_and(|ext| ext == "js"))
        .map(|(path, bytes)| (path, String::from_utf8(bytes).unwrap()))
        .collect()
}

fn assert_dist_is_local_and_clean(project: &Path) {
    let dist = project.join("dist");
    let host = project.to_string_lossy().replace('\\', "/");
    let tree = snapshot(&dist);
    assert!(!tree.is_empty());
    for (path, bytes) in &tree {
        let name = path.to_string_lossy().replace('\\', "/");
        assert!(
            !name.split('/').any(|part| part.starts_with('.')),
            "dotfile in dist: {name}"
        );
        assert!(
            !name.contains("node_modules"),
            "node_modules in dist: {name}"
        );
        assert!(
            !String::from_utf8_lossy(bytes)
                .replace('\\', "/")
                .contains(&host),
            "host path in dist file {name}"
        );
    }
    for (path, code) in script_files(&dist) {
        for specifier in output_specifiers(&code) {
            assert!(
                specifier.starts_with("./") || specifier.starts_with("../"),
                "{} keeps the non-local module '{specifier}'",
                path.display()
            );
            let target = dist.join(path.parent().unwrap()).join(&specifier);
            assert!(
                target.is_file(),
                "{} imports '{specifier}' which is not in dist/",
                path.display()
            );
        }
    }
}

fn assert_page_loads_local_script(project: &Path, page: &str, script: &str) {
    let html = fs::read_to_string(project.join("dist").join(page)).unwrap();
    assert!(
        html.contains(&format!("src=\"{script}\"")),
        "{page} must point at the emitted entry {script}: {html}"
    );
    assert!(project.join("dist").join(script).is_file());
    assert!(
        !html.contains(".ts\""),
        "{page} must not reference a .ts file"
    );
}

// ---------------------------------------------------------------------------
// Existing behavior, now on the native pipeline
// ---------------------------------------------------------------------------

#[test]
fn init_then_builds_the_untouched_generated_tree() {
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
    assert_eq!(json.status.code(), Some(1));
    let stdout = std::str::from_utf8(&json.stdout).unwrap();
    assert!(stdout.ends_with('\n'));
    let document: serde_json::Value = serde_json::from_str(stdout).unwrap();
    assert_eq!(document["command"], "build");
    assert_eq!(document["status"], "project-error");
    assert!(document["summary"]["errors"].as_u64().unwrap() > 0);
    assert!(!json_project.join("dist").exists());
    fs::remove_dir_all(json_project).unwrap();

    require_tool_cache!();
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
    require_tool_cache!();
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
    require_tool_cache!();
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
fn build_enforces_output_mapping_and_rejects_uninstalled_packages() {
    require_tool_cache!();
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

    // A package that is not installed fails the build; nothing is installed or fetched.
    let project = temporary_project("unlocked");
    copy_tree(&fixture("unlocked"), &project);
    let output = run("build", &project, &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr_of(&output).contains("wda.build.dependency-unlocked"),
        "uninstalled package stderr: {}",
        stderr_of(&output)
    );
    assert!(!project.join("dist").exists());
    assert!(!project.join("node_modules").exists());
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn build_reports_a_type_error_with_the_project_relative_path() {
    require_tool_cache!();
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
        !stderr.contains("wda-tsc"),
        "type-error stderr must not leak the temporary configuration path: {stderr}"
    );
    assert!(!project.join("dist").exists());
    fs::remove_dir_all(project).unwrap();
}

// ---------------------------------------------------------------------------
// native_build_pipeline
// ---------------------------------------------------------------------------

#[test]
fn native_build_pipeline_pure_html_builds_without_tools_or_tool_cache() {
    let project = mapping_project("native_pure_html");
    fs::remove_dir_all(project.join("scripts")).unwrap();
    write_file(&project, "pages/index.html", &page_html("Pure", None));
    let output = run_build(&project, &empty_cache());
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        stderr_of(&output)
    );
    assert!(project.join("dist/index.html").is_file());
    assert!(!stderr_of(&output).contains("tool"));
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn native_build_pipeline_correct_typescript_builds_with_no_runtime_tool_on_path() {
    require_tool_cache!();
    let project = mapping_project("native_ts_ok");
    let output = run_build(&project, &Cache::Real);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        stderr_of(&output)
    );
    assert_page_loads_local_script(&project, "index.html", "scripts/main.js");
    assert_dist_is_local_and_clean(&project);
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn native_build_pipeline_type_error_exits_1_and_project_settings_cannot_weaken_it() {
    require_tool_cache!();
    let project = mapping_project("native_ts_error");
    write_file(
        &project,
        "scripts/main.ts",
        "export function add(a, b) {\n  return a + b;\n}\n",
    );
    write_file(
        &project,
        "tsconfig.json",
        r#"{ "compilerOptions": { "strict": false, "noImplicitAny": false, "skipLibCheck": false } }"#,
    );
    let before = plant_old_dist(&project);
    let output = run_build(&project, &Cache::Real);
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        stderr_of(&output)
    );
    let stderr = stderr_of(&output);
    assert!(stderr.contains("wda.build.type-error"), "{stderr}");
    assert!(stderr.contains("scripts/main.ts"), "{stderr}");
    assert!(
        !stderr
            .replace('\\', "/")
            .contains(&project.to_string_lossy().replace('\\', "/")),
        "diagnostic leaked the host path: {stderr}"
    );
    assert_old_dist_intact(&project, &before);
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn native_build_pipeline_tool_fault_exits_2_and_keeps_the_old_dist() {
    let project = mapping_project("native_tool_fault");
    let before = plant_old_dist(&project);
    let output = run_build(&project, &broken_cache());
    assert_eq!(
        output.status.code(),
        Some(2),
        "stderr: {}",
        stderr_of(&output)
    );
    assert!(output.stdout.is_empty());
    assert_old_dist_intact(&project, &before);
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn native_build_pipeline_missing_cache_exits_1_and_hints_fetch() {
    let project = mapping_project("native_missing_cache");
    let before = plant_old_dist(&project);
    let output = run_build(&project, &empty_cache());
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        stderr_of(&output)
    );
    let stderr = stderr_of(&output);
    assert!(stderr.contains("wda tools fetch"), "{stderr}");
    assert!(stderr.contains("wda.build.tool-unavailable"), "{stderr}");
    assert_old_dist_intact(&project, &before);
    fs::remove_dir_all(project).unwrap();
}

/// Writes the package `lib` the way a manager leaves it under `node_modules/lib`.
fn write_lib_package(directory: &Path) {
    let package = |relative: &str, contents: &str| {
        let path = directory.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    };
    package(
        "package.json",
        r#"{"name":"lib","version":"1.0.0","type":"module","exports":{".":{"types":"./dist/index.d.ts","browser":"./dist/browser.js","default":"./dist/node.js"},"./extra":{"types":"./dist/extra.d.ts","default":"./dist/extra.js"}}}"#,
    );
    package("LICENSE", "MIT");
    package("dist/index.d.ts", "export declare const flavor: string;\n");
    package(
        "dist/browser.js",
        "export const flavor = 'browser-flavor';\n",
    );
    package("dist/node.js", "export const flavor = 'node-flavor';\n");
    package("dist/extra.d.ts", "export declare const extra: string;\n");
    package("dist/extra.js", "export const extra = 'extra-value';\n");
}

/// A new-format project (anchor.json and package.json from `wda init`) with two pages that
/// share modules, import each other circularly, and load one module lazily.
fn package_project(name: &str) -> PathBuf {
    let project = temporary_project(name);
    let init = run("init", &project, &[]);
    assert_eq!(
        init.status.code(),
        Some(0),
        "init stderr: {}",
        stderr_of(&init)
    );
    write_file(
        &project,
        "pages/index.html",
        &page_html("Home", Some("../scripts/a.ts")),
    );
    write_file(
        &project,
        "pages/about.html",
        &page_html("About", Some("../scripts/b.ts")),
    );
    write_file(
        &project,
        "scripts/shared.ts",
        "import { flavor } from 'lib';\nimport { extra } from 'lib/extra';\nimport { ping } from './ping.ts';\nexport const shared: string = flavor + extra + ping();\n",
    );
    write_file(
        &project,
        "scripts/ping.ts",
        "import { shared } from './shared.ts';\nexport function ping(): string {\n  return typeof shared;\n}\n",
    );
    write_file(
        &project,
        "scripts/lazy.ts",
        "export const lazy: string = 'lazy-value';\n",
    );
    write_file(
        &project,
        "scripts/a.ts",
        "import { shared } from './shared.ts';\ndocument.title = shared;\nvoid import('./lazy.ts').then((m) => console.log(m.lazy));\n",
    );
    write_file(
        &project,
        "scripts/b.ts",
        "import { shared } from './shared.ts';\ndocument.title = shared + '!';\n",
    );
    write_file(
        &project,
        "package.json",
        r#"{
  "name": "native-fixture",
  "private": true,
  "dependencies": { "lib": "1.0.0" }
}
"#,
    );
    write_file(
        &project,
        "package-lock.json",
        "{\"lockfileVersion\":3,\"packages\":{}}\n",
    );
    project
}

#[test]
fn native_build_pipeline_npm_and_bun_layouts_build_the_same_local_output() {
    require_tool_cache!();

    let npm = package_project("native_npm_layout");
    write_lib_package(&npm.join("node_modules/lib"));
    let npm_inputs = (
        fs::read(npm.join("package.json")).unwrap(),
        fs::read(npm.join("package-lock.json")).unwrap(),
    );
    let npm_output = run_build(&npm, &Cache::Real);
    assert_eq!(
        npm_output.status.code(),
        Some(0),
        "stderr: {}",
        stderr_of(&npm_output)
    );
    assert_eq!(
        npm_inputs,
        (
            fs::read(npm.join("package.json")).unwrap(),
            fs::read(npm.join("package-lock.json")).unwrap()
        ),
        "the build must not change package.json or the lockfile"
    );
    assert!(!npm.join("node_modules/.package-lock.json").exists());
    assert_page_loads_local_script(&npm, "index.html", "scripts/a.js");
    assert_page_loads_local_script(&npm, "about.html", "scripts/b.js");
    assert_dist_is_local_and_clean(&npm);
    let scripts = script_files(&npm.join("dist"));
    assert!(
        scripts
            .iter()
            .any(|(path, _)| path.starts_with("scripts/chunks")),
        "shared and lazy chunks expected: {:?}",
        scripts.iter().map(|(path, _)| path).collect::<Vec<_>>()
    );
    let all: String = scripts.iter().map(|(_, code)| code.as_str()).collect();
    assert!(all.contains("browser-flavor") && all.contains("extra-value"));
    assert!(all.contains("lazy-value"));
    assert!(
        !all.contains("node-flavor"),
        "the browser condition must win"
    );
    let first = snapshot(&npm.join("dist"));

    // The same content behind a Bun-style link into its package store.
    let bun = package_project("native_bun_layout");
    write_lib_package(&bun.join("node_modules/.bun/lib@1.0.0/node_modules/lib"));
    let linked = link_dir(
        &bun.join("node_modules/lib"),
        &bun.join("node_modules/.bun/lib@1.0.0/node_modules/lib"),
    );
    if !linked {
        eprintln!("skipped the link layout: this machine refuses directory links");
    } else {
        let bun_output = run_build(&bun, &Cache::Real);
        assert_eq!(
            bun_output.status.code(),
            Some(0),
            "stderr: {}",
            stderr_of(&bun_output)
        );
        assert_dist_is_local_and_clean(&bun);
        assert_eq!(
            snapshot(&bun.join("dist")),
            first,
            "both layouts must produce byte-identical output"
        );
    }
    fs::remove_dir_all(npm).unwrap();
    fs::remove_dir_all(bun).unwrap();
}

#[test]
fn native_build_pipeline_rejects_node_only_remote_and_unanalyzable_imports() {
    require_tool_cache!();
    let cases: [(&str, &str, &str); 3] = [
        (
            "node",
            "import fs from 'node:fs';\nconsole.log(fs);\n",
            "wda.build.reference-unresolvable",
        ),
        (
            "remote",
            "import 'https://example.com/lib.js';\n",
            "wda.build.reference-unresolvable",
        ),
        (
            "dynamic",
            "export function load(name: string) {\n  return import(name);\n}\n",
            "wda.build.reference-unresolvable",
        ),
    ];
    for (label, source, code) in cases {
        let project = mapping_project(&format!("native_reject_{label}"));
        write_file(&project, "scripts/main.ts", source);
        let before = plant_old_dist(&project);
        let output = run_build(&project, &Cache::Real);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{label} must be rejected, stderr: {}",
            stderr_of(&output)
        );
        assert!(
            stderr_of(&output).contains(code),
            "{label}: {}",
            stderr_of(&output)
        );
        assert_old_dist_intact(&project, &before);
        fs::remove_dir_all(project).unwrap();
    }
}

#[test]
fn native_build_pipeline_rejects_secret_paths_and_link_escapes() {
    require_tool_cache!();

    let secret = mapping_project("native_reject_secret");
    write_file(&secret, "scripts/.hidden.ts", "export const hidden = 1;\n");
    write_file(
        &secret,
        "scripts/main.ts",
        "import { hidden } from './.hidden.ts';\nconsole.log(hidden);\n",
    );
    let before = plant_old_dist(&secret);
    let output = run_build(&secret, &Cache::Real);
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        stderr_of(&output)
    );
    assert!(stderr_of(&output).contains("wda.build.secret-path-referenced"));
    assert_old_dist_intact(&secret, &before);
    fs::remove_dir_all(secret).unwrap();

    let escape = mapping_project("native_reject_escape");
    let outside = temporary_project("native_escape_target");
    write_file(&outside, "escaped.ts", "export const escaped = 1;\n");
    if !link_dir(&escape.join("scripts/outside"), &outside) {
        eprintln!("skipped the link escape case: this machine refuses directory links");
    } else {
        write_file(
            &escape,
            "scripts/main.ts",
            "import { escaped } from './outside/escaped.ts';\nconsole.log(escaped);\n",
        );
        let before = plant_old_dist(&escape);
        let output = run_build(&escape, &Cache::Real);
        assert_eq!(
            output.status.code(),
            Some(1),
            "stderr: {}",
            stderr_of(&output)
        );
        assert!(
            stderr_of(&output).contains("outside the project root"),
            "{}",
            stderr_of(&output)
        );
        assert_old_dist_intact(&escape, &before);
    }
    fs::remove_dir_all(escape).unwrap();
    fs::remove_dir_all(outside).unwrap();

    let package_escape = mapping_project("native_reject_package_escape");
    let outside_package = temporary_project("native_package_escape_target");
    write_lib_package(&outside_package);
    fs::create_dir_all(package_escape.join("node_modules")).unwrap();
    if !link_dir(&package_escape.join("node_modules/lib"), &outside_package) {
        eprintln!("skipped the package link escape case: this machine refuses directory links");
    } else {
        write_file(
            &package_escape,
            "scripts/main.ts",
            "import { flavor } from 'lib';\nconsole.log(flavor);\n",
        );
        let before = plant_old_dist(&package_escape);
        let output = run_build(&package_escape, &Cache::Real);
        assert_eq!(
            output.status.code(),
            Some(1),
            "stderr: {}",
            stderr_of(&output)
        );
        assert!(
            stderr_of(&output).contains("outside the project root"),
            "{}",
            stderr_of(&output)
        );
        assert_old_dist_intact(&package_escape, &before);
    }
    fs::remove_dir_all(package_escape).unwrap();
    fs::remove_dir_all(outside_package).unwrap();
}

#[test]
fn native_build_pipeline_rejects_packages_resolved_above_the_project_root() {
    require_tool_cache!();
    let parent = temporary_project("native_ancestor_isolation");
    let ancestor_package = parent.join("node_modules/lib");
    fs::create_dir_all(&ancestor_package).unwrap();
    fs::write(
        ancestor_package.join("package.json"),
        r#"{"name":"lib","version":"0.0.0-fake","main":"index.js"}"#,
    )
    .unwrap();
    fs::write(
        ancestor_package.join("index.js"),
        "export const flavor = 'WDA_TEST_ANCESTOR_NODE_MODULES_MARKER';\n",
    )
    .unwrap();
    fs::write(
        ancestor_package.join("index.d.ts"),
        "export declare const flavor: string;\n",
    )
    .unwrap();

    let project = parent.join("project");
    fs::create_dir_all(&project).unwrap();
    copy_tree(&fixture("mapping"), &project);
    write_file(
        &project,
        "scripts/main.ts",
        "import { flavor } from 'lib';\nconsole.log(flavor);\n",
    );
    let output = run_build(&project, &Cache::Real);
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        stderr_of(&output)
    );
    assert!(
        stderr_of(&output).contains("outside the project root"),
        "{}",
        stderr_of(&output)
    );
    assert!(!project.join("dist").exists());
    fs::remove_dir_all(&parent).unwrap();
}

// ---------------------------------------------------------------------------
// Spectrum 2 starter: installed packages and the native build
// ---------------------------------------------------------------------------

/// The six Spectrum packages `wda init` pins to one version. `lit` is a separate direct
/// dependency with its own version.
const SPECTRUM_STARTER_PACKAGES: &[&str] = &[
    "@spectrum-web-components/theme",
    "@spectrum-web-components/button",
    "@spectrum-web-components/textfield",
    "@spectrum-web-components/picker",
    "@spectrum-web-components/card",
    "@spectrum-web-components/dialog",
];

/// Every subpath the starter script imports, as (package, file inside the package).
const SPECTRUM_STARTER_MODULES: &[(&str, &str)] = &[
    ("@spectrum-web-components/theme", "sp-theme.js"),
    (
        "@spectrum-web-components/theme",
        "spectrum-two/scale-medium.js",
    ),
    (
        "@spectrum-web-components/theme",
        "spectrum-two/scale-large.js",
    ),
    (
        "@spectrum-web-components/theme",
        "spectrum-two/theme-light.js",
    ),
    (
        "@spectrum-web-components/theme",
        "spectrum-two/theme-dark.js",
    ),
    ("@spectrum-web-components/button", "sp-button.js"),
    ("@spectrum-web-components/textfield", "sp-textfield.js"),
    ("@spectrum-web-components/picker", "sp-picker.js"),
    ("@spectrum-web-components/card", "sp-card.js"),
    ("@spectrum-web-components/dialog", "sp-dialog.js"),
    ("@spectrum-web-components/dialog", "sp-dialog-wrapper.js"),
];

const SPECTRUM_STARTER_VERSION: &str = "1.2.3";
const SPECTRUM_STARTER_LIT_VERSION: &str = "3.3.0";

/// Writes the six Spectrum packages under `node_modules` as a manager would leave them:
/// one exact version each, with the module files the starter imports. Each module records
/// its own specifier when it runs, so the bundle can be checked for every import.
fn write_spectrum_packages(project: &Path) {
    for package in SPECTRUM_STARTER_PACKAGES {
        let root = project.join("node_modules").join(package);
        let mut exports = serde_json::Map::new();
        for (owner, file) in SPECTRUM_STARTER_MODULES {
            if owner != package {
                continue;
            }
            let stem = file.trim_end_matches(".js");
            exports.insert(
                format!("./{file}"),
                serde_json::json!({
                    "types": format!("./{stem}.d.ts"),
                    "default": format!("./{file}"),
                }),
            );
            write_file(
                &root,
                file,
                &format!(
                    "globalThis.__starterLoaded = (globalThis.__starterLoaded || []).concat([\"{owner}/{file}\"]);\n"
                ),
            );
            write_file(&root, &format!("{stem}.d.ts"), "export {};\n");
        }
        let manifest = serde_json::json!({
            "name": package,
            "version": SPECTRUM_STARTER_VERSION,
            "type": "module",
            "sideEffects": true,
            "exports": exports,
        });
        write_file(&root, "package.json", &manifest.to_string());
    }
}

/// A project with the Spectrum starter files as `wda init` writes them, the six Spectrum
/// packages and `lit` pinned in package.json, and the packages installed offline.
fn spectrum_starter_project(name: &str) -> PathBuf {
    let project = temporary_project(name);
    let init = run("init", &project, &[]);
    assert_eq!(
        init.status.code(),
        Some(0),
        "init stderr: {}",
        stderr_of(&init)
    );
    write_file(
        &project,
        "pages/index.html",
        include_str!("../src/builtins/spectrum_two/pages/index.html"),
    );
    write_file(
        &project,
        "scripts/main.ts",
        include_str!("../src/builtins/spectrum_two/scripts/main.ts"),
    );
    let mut dependencies = serde_json::Map::new();
    for package in SPECTRUM_STARTER_PACKAGES {
        dependencies.insert(
            (*package).to_string(),
            serde_json::Value::from(SPECTRUM_STARTER_VERSION),
        );
    }
    dependencies.insert(
        "lit".to_string(),
        serde_json::Value::from(SPECTRUM_STARTER_LIT_VERSION),
    );
    let manifest = serde_json::json!({
        "name": "spectrum-starter-fixture",
        "private": true,
        "dependencies": dependencies,
    });
    write_file(&project, "package.json", &manifest.to_string());
    write_file(
        &project,
        "package-lock.json",
        "{\"lockfileVersion\":3,\"packages\":{}}\n",
    );
    write_spectrum_packages(&project);
    project
}

#[test]
fn native_spectrum_starter_offline_build_uses_installed_packages_only() {
    require_tool_cache!();

    let project = spectrum_starter_project("native_spectrum_starter_offline");
    let output = run_build(&project, &Cache::Real);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        stderr_of(&output)
    );
    assert_dist_is_local_and_clean(&project);
    let scripts = script_files(&project.join("dist"));
    let all: String = scripts.iter().map(|(_, code)| code.as_str()).collect();
    for (owner, file) in SPECTRUM_STARTER_MODULES {
        assert!(
            all.contains(&format!("{owner}/{file}")),
            "the bundle must contain the installed module {owner}/{file}"
        );
    }
    let first = snapshot(&project.join("dist"));
    let again = run_build(&project, &Cache::Real);
    assert_eq!(
        again.status.code(),
        Some(0),
        "stderr: {}",
        stderr_of(&again)
    );
    assert_eq!(
        snapshot(&project.join("dist")),
        first,
        "the same input must produce the same output"
    );
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn native_spectrum_starter_offline_missing_package_is_refused_with_its_source() {
    require_tool_cache!();

    let project = spectrum_starter_project("native_spectrum_starter_missing");
    write_file(
        &project,
        "scripts/main.ts",
        "import '@spectrum-web-components/not-installed/sp-missing.js';\n",
    );
    let before = plant_old_dist(&project);
    let output = run_build(&project, &Cache::Real);
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        stderr_of(&output)
    );
    assert!(
        stderr_of(&output).contains("scripts/main.ts"),
        "the error must name the source file: {}",
        stderr_of(&output)
    );
    assert_old_dist_intact(&project, &before);
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn native_spectrum_starter_offline_remote_import_is_refused() {
    require_tool_cache!();

    let project = spectrum_starter_project("native_spectrum_starter_remote");
    write_file(
        &project,
        "scripts/main.ts",
        "import 'https://cdn.example.com/sp-button.js';\n",
    );
    let before = plant_old_dist(&project);
    let output = run_build(&project, &Cache::Real);
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        stderr_of(&output)
    );
    assert!(
        stderr_of(&output).contains("wda.build.reference-unresolvable"),
        "stderr: {}",
        stderr_of(&output)
    );
    assert_old_dist_intact(&project, &before);
    fs::remove_dir_all(project).unwrap();
}

/// Real install and build with npm and registry access.
#[test]
#[ignore]
fn compat_external_probes_real_npm() {
    real_spectrum_starter_builds_with("npm");
}

/// Real install and build with Bun and registry access.
#[test]
#[ignore]
fn compat_external_probes_real_bun() {
    real_spectrum_starter_builds_with("bun");
}

/// Real install and build of the Spectrum starter through one package manager.
fn real_spectrum_starter_builds_with(manager: &str) {
    assert!(wda_core::supported_package_managers()
        .iter()
        .any(|name| name == manager));
    assert!(
        real_tool_cache_present(),
        "native TypeScript cache is required; run wda tools fetch"
    );
    let executable = if cfg!(windows) && manager == "npm" {
        "npm.cmd"
    } else {
        manager
    };
    let manager_version = Command::new(executable)
        .arg("--version")
        .output()
        .expect("manager must be available");
    assert!(
        manager_version.status.success(),
        "manager version probe failed"
    );
    eprintln!(
        "{manager}: {}",
        String::from_utf8_lossy(&manager_version.stdout).trim()
    );
    let project = temporary_project(&format!("native_spectrum_starter_real_{manager}"));
    let anchor = (1..=3).find_map(|attempt| {
        let init = Command::new(binary())
            .args(["init", "--package-manager", manager])
            .current_dir(&project)
            .output()
            .expect("wda must start");
        assert_eq!(
            init.status.code(),
            Some(0),
            "init stderr: {}",
            stderr_of(&init)
        );
        let anchor: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(project.join("anchor.json")).unwrap()).unwrap();
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
            fs::remove_dir_all(&project).unwrap();
            fs::create_dir(&project).unwrap();
            std::thread::sleep(std::time::Duration::from_secs(1));
            None
        } else {
            Some(anchor)
        }
    }).unwrap();
    assert_eq!(anchor["designSystem"]["name"], "Spectrum 2", "{anchor}");
    let package: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(project.join("package.json")).unwrap()).unwrap();
    let version = package["dependencies"]["@spectrum-web-components/theme"]
        .as_str()
        .unwrap()
        .to_string();
    for name in SPECTRUM_STARTER_PACKAGES {
        assert_eq!(package["dependencies"][*name], version.as_str(), "{name}");
        let installed = project.join("node_modules").join(name);
        assert!(
            installed
                .canonicalize()
                .unwrap()
                .starts_with(project.canonicalize().unwrap()),
            "{name} must resolve inside the project"
        );
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(installed.join("package.json")).unwrap()).unwrap();
        assert_eq!(manifest["version"], version, "installed version for {name}");
    }
    assert!(package["dependencies"]["lit"].is_string(), "{package}");
    for (owner, file) in SPECTRUM_STARTER_MODULES {
        assert!(
            project
                .join("node_modules")
                .join(owner)
                .join(file)
                .is_file(),
            "the manager must install {owner}/{file}"
        );
    }
    let lock = ["package-lock.json", "bun.lock", "bun.lockb"]
        .iter()
        .find_map(|name| fs::read(project.join(name)).ok())
        .expect("the manager must write a lockfile");
    assert!(project.join("node_modules/lit/package.json").is_file());
    let empty_path = tempfile::tempdir().unwrap();
    let check = Command::new(binary())
        .arg("check")
        .current_dir(&project)
        .env("PATH", empty_path.path())
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "check failed: {}",
        stderr_of(&check)
    );
    let build = |project: &Path| {
        Command::new(binary())
            .arg("build")
            .current_dir(project)
            .env("PATH", empty_path.path())
            .env("HTTP_PROXY", "http://127.0.0.1:9")
            .env("HTTPS_PROXY", "http://127.0.0.1:9")
            .env("NO_PROXY", "")
            .output()
            .unwrap()
    };
    let output = build(&project);
    assert!(
        output.status.success(),
        "build failed: {}",
        stderr_of(&output)
    );
    assert_dist_is_local_and_clean(&project);
    let dist_files = snapshot(&project.join("dist")).len();
    fs::remove_dir_all(project.join("dist")).unwrap();
    let script = project.join("scripts/main.ts");
    let original = fs::read_to_string(&script).unwrap();
    fs::write(
        &script,
        format!("{original}\nimport '@spectrum-web-components/button/no-such-subpath.js';\n"),
    )
    .unwrap();
    let missing = build(&project);
    assert!(
        !missing.status.success(),
        "a missing package subpath must fail the build"
    );
    let missing_diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&missing.stdout),
        stderr_of(&missing)
    );
    assert!(
        missing_diagnostic.contains("no-such-subpath"),
        "unexpected diagnostic: {missing_diagnostic}"
    );
    fs::write(
        &script,
        format!("{original}\nconst compat_type_error: string = 123;\n"),
    )
    .unwrap();
    let negative = build(&project);
    assert!(
        !negative.status.success(),
        "a TypeScript error must fail the build"
    );
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&negative.stdout),
        stderr_of(&negative)
    );
    assert!(
        diagnostic.contains("TS2322"),
        "unexpected diagnostic: {diagnostic}"
    );
    eprintln!(
        "spectrum starter: versions {version}, lock bytes {}, dist files {}",
        lock.len(),
        dist_files
    );
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn compat_external_probes_manager_contract() {
    let managers = wda_core::supported_package_managers();
    assert_eq!(managers, ["npm", "bun"]);
    assert!(SPECTRUM_STARTER_PACKAGES.len() > 1);
    assert!(SPECTRUM_STARTER_MODULES
        .iter()
        .any(|(_, path)| path.contains('/')));
    let root = tempfile::tempdir().unwrap();
    assert!(
        env::split_paths(&env::join_paths([root.path()]).unwrap()).all(|path| path == root.path())
    );
}

#[cfg(test)]
mod token_profile_regression {
    use std::fs;
    use std::path::{Path, PathBuf};

    fn project_root(label: &str) -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().expect("temporary root must be created");
        let root = temp.path().join(label);
        fs::create_dir_all(&root).expect("project root must be created");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/build/mapping");
        copy_tree(&fixture, &root);
        fs::write(
        root.join("pages/index.html"),
        "<!doctype html><html lang=\"en\"><head><title>Token profile</title></head><body></body></html>\n",
    )
    .expect("page must be written");
        (temp, root)
    }

    fn copy_tree(source: &Path, destination: &Path) {
        for entry in fs::read_dir(source).expect("project fixture must be readable") {
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

    fn set_tokens(root: &Path, tokens: &serde_json::Value) {
        fs::create_dir_all(root.join("tokens")).expect("tokens directory must exist");
        fs::write(
            root.join("tokens/tokens.json"),
            serde_json::to_vec_pretty(tokens).expect("token fixture must be serializable"),
        )
        .expect("token source must be written");
    }

    fn rejected_profile_has_source_diagnostic(tokens: serde_json::Value) -> Vec<String> {
        let mut failures = Vec::new();
        let (_temp, root) = project_root("rejected");
        set_tokens(&root, &tokens);

        match wda_core::validate_project(&root) {
            Ok(validation) => {
                if !validation.has_error() {
                    failures.push(format!("no validation error for {tokens}"));
                }
                if !validation.diagnostics.iter().any(|diagnostic| {
                    diagnostic.severity == wda_core::diagnostics::Severity::Error
                        && diagnostic.location.as_ref().is_some_and(|location| {
                            location.path == Path::new("tokens/tokens.json") && location.line.is_some()
                        })
                }) {
                    failures.push(format!("no token source location for {tokens}"));
                }
            }
            Err(error) => failures.push(format!("validation failed for {tokens}: {error}")),
        }

        match wda_core::build_project(&root) {
            Ok(build) if !build.validation.has_error() => {
                failures.push(format!("build accepted rejected tokens: {tokens}"));
            }
            Err(error) => failures.push(format!(
                "build failed before reporting tokens: {tokens}: {error}"
            )),
            _ => {}
        }
        if root.join("dist").exists() {
            failures.push(format!("rejected tokens left a dist directory: {tokens}"));
        }
        failures
    }
    #[test]
    fn author_token_profile() {
        let (_temp, root) = project_root("supported");
        let supported = serde_json::json!({
            "palette": {
                "$type": "color",
                "primary": { "$value": "#112233" },
                "primaryAlias": { "$value": "{palette.primary}" }
            },
            "spacing": {
                "$type": "dimension",
                "medium": { "$value": "2rem" }
            },
            "fontFamily": {
                "$type": "fontFamily",
                "body": { "$value": "Inter, sans-serif" }
            },
            "fontWeight": {
                "$type": "fontWeight",
                "strong": { "$value": 600 }
            },
            "lineHeight": {
                "$type": "lineHeight",
                "body": { "$value": 1.5 }
            },
            "scale": {
                "$type": "number",
                "unitless": { "$value": 2 }
            },
            "borderStyle": {
                "$type": "string",
                "solid": { "$value": "solid" }
            }
        });
        set_tokens(&root, &supported);

        for _ in 0..2 {
            let build = wda_core::build_project(&root).expect("supported profile must build");
            assert!(
                !build.validation.has_error(),
                "supported values and aliases must pass validation: {:?}",
                build.validation.diagnostics
            );
            let css = fs::read_to_string(build.dist_path.join("styles/tokens.css"))
                .expect("token stylesheet must be emitted");
            assert!(!css.trim().is_empty(), "supported values must emit CSS");
            for value in [
                "#112233",
                "2rem",
                "Inter, sans-serif",
                "600",
                "1.5",
                "2",
                "solid",
            ] {
                assert!(
                    css.contains(value),
                    "emitted CSS must contain {value:?}: {css}"
                );
            }
            let declarations: Vec<_> = css
                .lines()
                .filter_map(|line| line.split_once(':'))
                .filter(|(name, _)| name.trim().starts_with("--"))
                .collect();
            assert!(
                declarations.len() >= 7,
                "each supported token must emit: {css}"
            );
            assert!(
                declarations
                    .iter()
                    .all(|(_, value)| { !value.trim().trim_end_matches(';').trim().is_empty() }),
                "emitted custom properties must not have empty values: {css}"
            );
        }

        let first = fs::read(root.join("dist/styles/tokens.css")).unwrap();
        let build = wda_core::build_project(&root).expect("repeated build must complete");
        let second = fs::read(build.dist_path.join("styles/tokens.css")).unwrap();
        assert_eq!(first, second, "token CSS must be deterministic");

        let mut failures = Vec::new();
        for bad in [
            serde_json::json!({"sample": {"$type": "color", "$value": null}}),
            serde_json::json!({"sample": {"$type": "color", "$value": true}}),
            serde_json::json!({"sample": {"$type": "color", "$value": []}}),
            serde_json::json!({"sample": {"$type": "color", "$value": {"nested": "value"}}}),
            serde_json::json!({"sample": {"$type": "unknown", "$value": "value"}}),
            serde_json::json!({"sample": {"$type": "color", "$value": "prefix {palette.primary}"}}),
            serde_json::json!({"sample": {"$type": "color", "$value": "red; --injected: yes"}}),
            serde_json::json!({"palette": {"$type": "color", "a": {"$value": "{palette.b}"}, "b": {"$value": "{palette.a}"}}}),
            serde_json::json!({"fooBar": {"$type": "string", "$value": "one"}, "foo-bar": {"$type": "string", "$value": "two"}}),
            serde_json::json!({"sample": {"$ref": "other.json"}}),
            serde_json::json!({"sample": {"$extends": "base"}}),
        ] {
            failures.extend(rejected_profile_has_source_diagnostic(bad));
        }

        let mut deep = serde_json::Map::new();
        for index in 0..130 {
            let value = if index == 129 {
                serde_json::json!("#112233")
            } else {
                serde_json::json!(format!("{{depth.token{}}}", index + 1))
            };
            deep.insert(
                format!("token{index}"),
                serde_json::json!({"$type": "color", "$value": value}),
            );
        }
        failures.extend(rejected_profile_has_source_diagnostic(
            serde_json::json!({"depth": deep}),
        ));
        assert!(failures.is_empty(), "rejected token cases must report source diagnostics and leave dist clean: {failures:#?}");
    }
}
