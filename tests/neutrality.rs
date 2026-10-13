//! Dual-system neutrality matrix.
//!
//! Runs both Design Systems `wda init` can resolve to — `WDA Minimal` (offline,
//! zero-npm) and Spectrum 2 (online, npm-backed) — through the full `init` then
//! `check` then `build` chain, and asserts the model-separation and determinism
//! claims that make Core's neutrality between the two systems a proven fact rather
//! than an assumption. See `docs/design-systems/wda-minimal.md` and
//! `docs/design-systems/spectrum-paper-adapter.md` for the claims this matrix backs.

use std::collections::BTreeMap;
use std::env;
use std::fs;
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
        "wda_neutrality_{name}_{}_{}",
        std::process::id(),
        suffix
    ));
    fs::create_dir_all(&path).expect("temporary project directory must be created");
    path
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
    let dir = env::temp_dir().join(format!(
        "wda_neutrality_isolated_home_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be available")
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("isolated HOME directory must be created");
    dir
}

/// Runs `command` against `project`. `init` has `deno` excluded from `PATH`
/// (everything else, including `git` and its supporting toolchain, stays
/// reachable) so online Spectrum 2 resolution deterministically falls back to
/// WDA Minimal instead of reaching the real npm registry under the bare
/// `cargo test`, matching the injected-offline convention `tests/init.rs` and
/// `tests/build.rs` already establish. `HOME`/`USERPROFILE` and the global git
/// config paths are isolated so this machine's own git identity and hooks
/// never leak into the test. Pass `online: true` to run `init` with the real,
/// unmodified environment instead, for the `#[ignore]`-gated Spectrum 2 cases.
fn run(command: &str, project: &Path, online: bool) -> Output {
    let mut cmd = Command::new(binary());
    cmd.arg(command).current_dir(project);
    if command == "init" && !online {
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
                        .expect("entry must be below root")
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

/// The Minimal path completes `init` then `check` then `build` under the default
/// `cargo test`, carrying none of the Spectrum 2 side's dependency or
/// component-registration artifacts. Matches the neutrality matrix's model-
/// separation claim: "The Minimal project has no `package.json`, no lockfile, and
/// no custom-element registration."
#[test]
fn neutrality_minimal_completes_init_check_build_with_no_dependency_or_component_model() {
    let project = temporary_project("minimal_chain");

    let init = run("init", &project, false);
    assert_eq!(
        init.status.code(),
        Some(0),
        "Minimal init must exit 0, stderr: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    assert!(
        String::from_utf8_lossy(&init.stdout).contains("Design System: WDA Minimal"),
        "offline resolution must report WDA Minimal, got: {}",
        String::from_utf8_lossy(&init.stdout)
    );

    // Model separation: no dependency model, no component model.
    for absent in ["package.json", "deno.lock", "scripts/main.ts"] {
        assert!(
            !project.join(absent).exists(),
            "WDA Minimal project must not carry '{}'",
            absent
        );
    }
    let index_html = fs::read_to_string(project.join("pages/index.html"))
        .expect("failed to read pages/index.html");
    assert!(
        !index_html.contains("<sp-"),
        "WDA Minimal starter page must register no vendor custom element, got: {}",
        index_html
    );

    let check = run("check", &project, false);
    assert_eq!(
        check.status.code(),
        Some(0),
        "Minimal check must exit 0, stderr: {}",
        String::from_utf8_lossy(&check.stderr)
    );

    let build = run("build", &project, false);
    assert_eq!(
        build.status.code(),
        Some(0),
        "Minimal build must exit 0, stderr: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(project.join("dist/index.html").is_file());

    let _ = fs::remove_dir_all(&project);
}

/// The Spectrum 2 path completes the same `init` then `check` then `build` chain
/// online, carrying exactly the dependency and component-model artifacts the
/// Minimal path lacks. Matches the neutrality matrix's model-separation claim: "the
/// Spectrum 2 project has all three [`package.json`, lockfile, custom-element
/// registration]."
/// Real-registry probe: online design-system resolution only picks Spectrum 2 when
/// `deno` is on PATH and the npm registry is reachable, so this is `#[ignore]`-gated
/// like `tests/init.rs`'s `test_spectrum_two_init_creates_dependency_files_matching_lockfile_and_builds`,
/// which already asserts the exact six-package dependency set and lockfile pinning
/// in full; this test focuses on the matrix's own model-separation and chain-
/// completion claims rather than re-deriving that detail. Run separately via
/// `cargo test -- --ignored` (the CI probe job installs `deno`).
#[test]
#[ignore]
fn neutrality_spectrum_two_completes_init_check_build_with_dependency_and_component_model() {
    let project = temporary_project("spectrum_two_chain");

    let init = run("init", &project, true);
    assert_eq!(
        init.status.code(),
        Some(0),
        "Spectrum 2 init must exit 0, stderr: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    assert!(
        String::from_utf8_lossy(&init.stdout).contains("Design System: Spectrum 2"),
        "online resolution must report Spectrum 2, got: {}",
        String::from_utf8_lossy(&init.stdout)
    );

    // Model separation: dependency model and component model both present.
    for present in ["package.json", "deno.lock", "scripts/main.ts"] {
        assert!(
            project.join(present).is_file(),
            "Spectrum 2 project must carry '{}'",
            present
        );
    }
    let index_html = fs::read_to_string(project.join("pages/index.html"))
        .expect("failed to read pages/index.html");
    assert!(
        index_html.contains("<sp-"),
        "Spectrum 2 starter page must register a vendor custom element, got: {}",
        index_html
    );

    let check = run("check", &project, true);
    assert_eq!(
        check.status.code(),
        Some(0),
        "Spectrum 2 check must exit 0, stderr: {}",
        String::from_utf8_lossy(&check.stderr)
    );

    let build = run("build", &project, true);
    assert_eq!(
        build.status.code(),
        Some(0),
        "Spectrum 2 build must exit 0, stderr: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(project.join("dist/index.html").is_file());

    let _ = fs::remove_dir_all(&project);
}

/// Two `init` runs at one (release, resolved-design-system-version) pair produce
/// byte-identical output, outside the one field that varies by design.
///
/// Deviation from the plan's literal instruction to run this "through
/// `init_project_with_time` with a fixed clock": that function is `pub fn` on
/// `src/init/mod.rs`, but the `init` module itself is declared `pub(crate) mod init`
/// in `src/lib.rs` and is not re-exported at the crate root, so it is unreachable
/// from `tests/neutrality.rs` (an external integration-test crate bound by normal
/// Rust module privacy) — and this task's affected-files allowlist is
/// `tests/neutrality.rs` only, so widening `src/lib.rs`'s exports is out of scope. An
/// equivalent fixed-clock, byte-identical-runs assertion already exists at the unit
/// level as `test_init_project_deterministic_byte_identical_runs` in
/// `src/init/mod.rs`, exercised through the crate-internal
/// `init_project_with_time_and_resolution` seam. This test instead proves the same
/// claim at the binary level, the level this matrix otherwise runs at: it excludes
/// only `wda.json`'s `projectVersion` field, which `src/init/manifest.rs`'s
/// `generate_manifest` derives from the real wall-clock instant `wda init` runs at
/// (RFC 3339 with second precision) and which two real CLI invocations therefore
/// cannot be relied on to match — exactly the reason the plan gives for wanting a
/// fixed clock in the first place.
#[test]
fn neutrality_two_minimal_init_runs_are_byte_identical_outside_the_clock_derived_field() {
    let project1 = temporary_project("determinism_1");
    let project2 = temporary_project("determinism_2");

    let init1 = run("init", &project1, false);
    assert_eq!(
        init1.status.code(),
        Some(0),
        "first init must exit 0, stderr: {}",
        String::from_utf8_lossy(&init1.stderr)
    );
    let init2 = run("init", &project2, false);
    assert_eq!(
        init2.status.code(),
        Some(0),
        "second init must exit 0, stderr: {}",
        String::from_utf8_lossy(&init2.stderr)
    );

    // `.git/` is excluded from this comparison: init's first commit
    // embeds the real wall-clock commit timestamp, so its tree/commit object
    // hashes — and therefore every loose-object filename under `.git/objects/`
    // — are non-deterministic across two separately-timed runs even though the
    // repository content they describe is identical. This is a second,
    // git-specific clock-derived field alongside `wda.json`'s `projectVersion`,
    // not a break in the neutrality claim this test backs (which is about the
    // Design System's own generated files, not git's internal object encoding).
    let is_git_internal = |p: &Path| p.starts_with(".git");

    let tree1 = snapshot(&project1);
    let tree2 = snapshot(&project2);
    let keys1: Vec<&PathBuf> = tree1.keys().filter(|p| !is_git_internal(p)).collect();
    let keys2: Vec<&PathBuf> = tree2.keys().filter(|p| !is_git_internal(p)).collect();
    assert_eq!(
        keys1, keys2,
        "both runs must produce the same set of output paths outside '.git/'"
    );

    for (rel_path, bytes1) in &tree1 {
        if is_git_internal(rel_path) {
            continue;
        }
        let bytes2 = tree2
            .get(rel_path)
            .unwrap_or_else(|| panic!("second run must also produce '{}'", rel_path.display()));
        if rel_path == Path::new("wda.json") {
            continue;
        }
        assert_eq!(
            bytes1, bytes2,
            "'{}' must be byte-identical across both runs",
            rel_path.display()
        );
    }

    let manifest1: serde_json::Value =
        serde_json::from_slice(&tree1[Path::new("wda.json")]).expect("wda.json must be valid JSON");
    let manifest2: serde_json::Value =
        serde_json::from_slice(&tree2[Path::new("wda.json")]).expect("wda.json must be valid JSON");
    let mut obj1 = manifest1.as_object().expect("wda.json must be an object").clone();
    let mut obj2 = manifest2.as_object().expect("wda.json must be an object").clone();
    obj1.remove("projectVersion");
    obj2.remove("projectVersion");
    assert_eq!(
        obj1, obj2,
        "wda.json must be identical across both runs outside 'projectVersion'"
    );

    let _ = fs::remove_dir_all(&project1);
    let _ = fs::remove_dir_all(&project2);
}

/// Source files that legitimately hold a production-code reference to a
/// resolved Design System's name — the resolution seam that first produces the
/// name (`src/init/resolve.rs`, which also defines the two name constants
/// themselves), and the asset seam that reads a resolved name back to select
/// which per-system starter assets, dependencies, and documentation content to
/// emit (`src/init/mod.rs`'s `build_write_plan` and Spectrum 2 dependency
/// pinning, `src/init/documents.rs`'s Minimal starter-page title
/// substitution, `src/builtins/wda_minimal.rs`'s per-system `design.md`
/// template renderer, and `src/builtins/spectrum_two.rs`'s Spectrum 2 asset
/// accessors). Paths are relative to the repository root, `/`-separated.
const IDENTITY_SEAM_FILES: &[&str] = &[
    "src/init/resolve.rs",
    "src/init/mod.rs",
    "src/init/documents.rs",
    "src/builtins/wda_minimal.rs",
    "src/builtins/spectrum_two.rs",
];

/// Substrings that identify a Design System name reference: the two design
/// system names themselves (as they appear inside a Rust string literal) and
/// the two `pub const` names `src/init/resolve.rs` exports for them. Comparing
/// or hard-coding either form has the same neutrality effect, so both are
/// audited identically.
const IDENTITY_MARKERS: &[&str] = &[
    "WDA Minimal",
    "Spectrum 2",
    "SPECTRUM_TWO_DESIGN_SYSTEM_NAME",
    "BUILTIN_DESIGN_SYSTEM_NAME",
];

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("src directory must be readable") {
        let entry = entry.expect("src directory entry must be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// If `indices[i..]` opens a Rust string literal (plain `"..."`, raw
/// `r#"..."#` with any number of `#`, or the `b`/`br` byte-string variants),
/// returns the index (into `indices`) just past its closing delimiter.
/// Otherwise returns `None`, leaving the caller to treat `indices[i]` as an
/// ordinary character. Needed so a brace character inside a string literal —
/// as in this codebase's `"{{DESIGN_SYSTEM_NAME}}"` template markers and
/// `br#"{"latest":"1.2.3"}"#` JSON test fixtures — never desynchronizes the
/// module-body brace count below.
fn try_skip_string_literal(indices: &[(usize, char)], i: usize) -> Option<usize> {
    let mut j = i;
    if indices.get(j).map(|&(_, c)| c) == Some('b') {
        j += 1;
    }
    match indices.get(j).map(|&(_, c)| c) {
        Some('"') => Some(skip_plain_string_body(indices, j + 1)),
        Some('r') => {
            let mut k = j + 1;
            let mut hashes = 0usize;
            while indices.get(k).map(|&(_, c)| c) == Some('#') {
                hashes += 1;
                k += 1;
            }
            if indices.get(k).map(|&(_, c)| c) == Some('"') {
                Some(skip_raw_string_body(indices, k + 1, hashes))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Skips a plain `"..."` string body starting right after the opening quote,
/// honouring `\"` and `\\` escapes, and returns the index just past the
/// closing quote.
fn skip_plain_string_body(indices: &[(usize, char)], mut i: usize) -> usize {
    while i < indices.len() {
        match indices[i].1 {
            '\\' => i += 2,
            '"' => return i + 1,
            _ => i += 1,
        }
    }
    i
}

/// Skips a raw string body starting right after its opening `"`, returning
/// the index just past the closing `"` followed by exactly `hashes` `#`s.
fn skip_raw_string_body(indices: &[(usize, char)], mut i: usize, hashes: usize) -> usize {
    while i < indices.len() {
        if indices[i].1 == '"' {
            let mut k = i + 1;
            let mut seen = 0usize;
            while seen < hashes && indices.get(k).map(|&(_, c)| c) == Some('#') {
                seen += 1;
                k += 1;
            }
            if seen == hashes {
                return k;
            }
        }
        i += 1;
    }
    i
}

/// If `indices[i]` opens a char literal (`'x'`, `'\''`, `'\"'`, `'\u{7B}'`,
/// `'\xNN'`, ...), returns the index just past its closing `'`. Returns
/// `None` for a lifetime (`'a`, `'static`) instead, which never closes with a
/// second `'`, so the caller falls back to treating `'` as an ordinary
/// character. Needed because a naive scan otherwise mistakes a char
/// literal's escaped quote — as in this codebase's
/// `matches!(character, '\"' | '\'' | ';')` — for the start of a new string
/// literal, then eats real code up to some unrelated later quote.
fn try_skip_char_literal(indices: &[(usize, char)], i: usize) -> Option<usize> {
    let mut j = i + 1;
    if indices.get(j).map(|&(_, c)| c) == Some('\\') {
        j += 1;
        match indices.get(j).map(|&(_, c)| c) {
            Some('u') => {
                j += 1;
                if indices.get(j).map(|&(_, c)| c) != Some('{') {
                    return None;
                }
                j += 1;
                loop {
                    match indices.get(j).map(|&(_, c)| c) {
                        Some('}') => {
                            j += 1;
                            break;
                        }
                        Some(_) => j += 1,
                        None => return None,
                    }
                }
            }
            Some('x') => j += 3,
            Some(_) => j += 1,
            None => return None,
        }
    } else {
        j += 1;
    }
    if indices.get(j).map(|&(_, c)| c) == Some('\'') {
        Some(j + 1)
    } else {
        None
    }
}

/// Finds the byte offset just past the closing brace matching the `{` at
/// `start_byte` in `source`, skipping over line comments, block comments,
/// char literals, and string/raw-string/byte-string literals so a brace
/// character inside one of those never miscounts as real module-body
/// nesting.
fn matching_brace_end(source: &str, start_byte: usize) -> Option<usize> {
    let indices: Vec<(usize, char)> = source.char_indices().collect();
    let mut i = indices.iter().position(|&(b, _)| b == start_byte)?;
    let mut depth = 0i32;

    while i < indices.len() {
        match indices[i].1 {
            '{' => {
                depth += 1;
                i += 1;
            }
            '}' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return Some(indices.get(i).map(|&(b, _)| b).unwrap_or(source.len()));
                }
            }
            '/' if indices.get(i + 1).map(|&(_, c)| c) == Some('/') => {
                i += 2;
                while i < indices.len() && indices[i].1 != '\n' {
                    i += 1;
                }
            }
            '/' if indices.get(i + 1).map(|&(_, c)| c) == Some('*') => {
                i += 2;
                let mut comment_depth = 1i32;
                while i < indices.len() && comment_depth > 0 {
                    if indices[i].1 == '/' && indices.get(i + 1).map(|&(_, c)| c) == Some('*') {
                        comment_depth += 1;
                        i += 2;
                    } else if indices[i].1 == '*' && indices.get(i + 1).map(|&(_, c)| c) == Some('/')
                    {
                        comment_depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }
            '\'' => {
                i = try_skip_char_literal(&indices, i).unwrap_or(i + 1);
            }
            '"' | 'b' | 'r' => {
                i = try_skip_string_literal(&indices, i).unwrap_or(i + 1);
            }
            _ => {
                i += 1;
            }
        }
    }
    None
}

/// Removes every `#[cfg(test)]`-gated module body from `source` by brace
/// matching, leaving only production code behind. A test module exercises a
/// seam's already-resolved output rather than branching production behaviour
/// on it, so a literal or constant living only inside one is not an identity
/// hard-coding violation.
fn strip_cfg_test_modules(source: &str) -> String {
    const MARKER: &str = "#[cfg(test)]";
    let mut result = String::with_capacity(source.len());
    let mut cursor = 0usize;
    while let Some(marker_pos) = source[cursor..].find(MARKER) {
        let marker_start = cursor + marker_pos;
        result.push_str(&source[cursor..marker_start]);
        let after_marker_start = marker_start + MARKER.len();
        let brace_start = source[after_marker_start..]
            .find('{')
            .map(|p| after_marker_start + p)
            .expect("#[cfg(test)] must be followed by a module body");
        cursor = matching_brace_end(source, brace_start)
            .expect("#[cfg(test)] module body must be well-braced");
    }
    result.push_str(&source[cursor..]);
    result
}

/// Strips whole-line `//`, `///`, and `//!` comments. This codebase's doc
/// comments narrate both Design System names in prose throughout, which is
/// not a code-level identity branch or hard-coding.
fn strip_comment_lines(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Returns `path`'s production code (test modules and comments stripped).
fn production_code(path: &Path) -> String {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    strip_comment_lines(&strip_cfg_test_modules(&raw))
}

/// Design-system identity hard-coding audit.
///
/// Rule 1: outside the resolution seam and the asset seam (`IDENTITY_SEAM_FILES`),
/// no production line may compare a Design System name against a literal (or
/// against the constant holding one) to select behaviour — a line carrying an
/// identity marker alongside `==`.
///
/// Rule 2: outside the same two seams, no production line may hard-code a
/// Design System name into generated project output — a line carrying an
/// identity marker without a comparison, since a non-seam file should only
/// ever receive a resolved name through a variable or parameter, never spell
/// it out itself.
///
/// Test-gated code is excluded via `strip_cfg_test_modules`: this is what lets
/// `src/cli.rs`, `src/lib.rs`, `src/init/manifest.rs`, and `src/build/mod.rs`
/// carry the design-system name literal or constant only inside their own
/// `#[cfg(test)]` fixtures without a naive whole-file scan flagging them.
#[test]
fn neutrality_audit_no_identity_branch_or_hard_coded_name_outside_the_seams() {
    let repo_root = repository_root();
    let src_root = repo_root.join("src");

    let mut files = Vec::new();
    collect_rs_files(&src_root, &mut files);
    assert!(
        !files.is_empty(),
        "src/ must contain .rs files for the identity audit to inspect"
    );

    let mut branch_violations = Vec::new();
    let mut hardcode_violations = Vec::new();

    for path in &files {
        let rel_path = path
            .strip_prefix(&repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        if IDENTITY_SEAM_FILES.contains(&rel_path.as_str()) {
            continue;
        }

        let code = production_code(path);
        for (line_no, line) in code.lines().enumerate() {
            let carries_identity_marker = IDENTITY_MARKERS.iter().any(|marker| line.contains(marker));
            if !carries_identity_marker {
                continue;
            }

            let location = format!("{}:{}: {}", rel_path, line_no + 1, line.trim());
            if line.contains("==") {
                branch_violations.push(location);
            } else {
                hardcode_violations.push(location);
            }
        }
    }

    assert!(
        branch_violations.is_empty(),
        "production code outside the resolution seam (src/init/resolve.rs) and \
         the asset seam ({}) must not compare a Design System name against a \
         literal to select behaviour, got: {:#?}",
        IDENTITY_SEAM_FILES[1..].join(", "),
        branch_violations
    );
    assert!(
        hardcode_violations.is_empty(),
        "production code outside the resolution seam (src/init/resolve.rs) and \
         the asset seam ({}) must not hard-code a Design System name into \
         generated project output, got: {:#?}",
        IDENTITY_SEAM_FILES[1..].join(", "),
        hardcode_violations
    );
}

/// Spectrum 1 residue markers this guard rejects, paired with the label used
/// in a failure report. Each is a substring that only ever appears as a
/// Spectrum 1 vendor illustration: `spectrum--light`/`spectrum--dark`/
/// `spectrum--medium` are Spectrum 1 theme classes superseded by Spectrum 2's
/// `<sp-theme>` attributes; `--spectrum-global-` and `--spectrum-alias-` are
/// Spectrum 1's global and alias custom-property tiers, which Spectrum 2
/// replaced with plain `--spectrum-*` token names; `@spectrum-css/vars` is
/// the Spectrum 1 CSS variables package, superseded by `@spectrum-css/tokens`.
/// See `docs/design-systems/adapter-boundary.md`'s Token Mapping, Styles, and
/// Theme facets, corrected to their Spectrum 2 equivalents.
const SPECTRUM_ONE_RESIDUE_MARKERS: &[(&str, &str)] = &[
    ("spectrum--light", "Spectrum 1 theme class"),
    ("spectrum--dark", "Spectrum 1 theme class"),
    ("spectrum--medium", "Spectrum 1 theme class"),
    ("--spectrum-global-", "Spectrum 1 global custom property"),
    ("--spectrum-alias-", "Spectrum 1 alias custom property"),
    ("@spectrum-css/vars", "Spectrum 1 CSS variables package"),
];

/// Recursively collects every file under `dir` into `out`, `/`-separated and
/// relative to nothing in particular (callers strip a repo-root prefix).
/// Unlike `collect_rs_files`, this walks every extension: the residue guard
/// below must catch a stray Spectrum 1 reference in Markdown docs just as
/// readily as in Rust source.
fn collect_all_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("directory must be readable") {
        let entry = entry.expect("directory entry must be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_all_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Spectrum 1 residue guard.
///
/// Neither `docs/` nor `src/` may contain a Spectrum 1 theme class, a
/// Spectrum 1 global or alias custom property, or a reference to the
/// Spectrum 1 CSS variables package: every Spectrum vendor illustration in
/// both trees must already have been corrected to its Spectrum 2 equivalent.
/// Fails on the tree as it stood before the paper-adapter, Minimal-design, and
/// boundary corrections that respectively fixed
/// `docs/design-systems/spectrum-paper-adapter.md`,
/// `docs/design-systems/wda-minimal.md`, and
/// `docs/design-systems/adapter-boundary.md` respectively, and passes once
/// all three have landed.
#[test]
fn neutrality_no_spectrum_one_residue_under_docs_or_source_trees() {
    let repo_root = repository_root();

    let mut files = Vec::new();
    for tree in ["docs", "src"] {
        let root = repo_root.join(tree);
        assert!(root.is_dir(), "'{}' must exist under the repository root", tree);
        collect_all_files(&root, &mut files);
    }
    assert!(
        !files.is_empty(),
        "docs/ and src/ together must contain files for the residue guard to inspect"
    );

    let mut violations = Vec::new();
    for path in &files {
        let Ok(contents) = fs::read_to_string(path) else {
            // Non-UTF-8 or binary files (fonts, images) carry no vendor text.
            continue;
        };
        let rel_path = path
            .strip_prefix(&repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        for (line_no, line) in contents.lines().enumerate() {
            for (marker, label) in SPECTRUM_ONE_RESIDUE_MARKERS {
                if line.contains(marker) {
                    violations.push(format!(
                        "{}:{}: {} ('{}'): {}",
                        rel_path,
                        line_no + 1,
                        label,
                        marker,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "docs/ and src/ must carry no Spectrum 1 residue, got: {:#?}",
        violations
    );
}

/// Positive-claim substrings this guard treats as a migration-claim marker:
/// each corresponds to one of the four forbidden claim categories (automatic
/// switching, zero-cost migration, lossless migration, and zero HTML
/// changes). Matched case-insensitively against the whole-file text
/// with line breaks collapsed to spaces, so a claim that wraps across a
/// soft-wrapped Markdown line boundary — as both of the repository's genuine
/// disclaimers currently do — is still found as one contiguous phrase.
const MIGRATION_CLAIM_MARKERS: &[&str] = &[
    "automatic switching",
    "automatically switches",
    "zero-cost migration",
    "zero cost migration",
    "lossless migration",
    "zero html change",
    "zero html modification",
    "automatic migration",
];

/// Disclaimer phrasings that make a nearby migration-claim marker a hedge
/// rather than a positive claim. The repository's two current hits —
/// `docs/design-systems/wda-minimal.md`'s "not a promise of automatic or
/// lossless migration" and "not a guarantee of zero HTML modification, ...
/// or automatic migration" — are both covered by this list, and both are
/// legitimate disclaimers this guard must not flag.
const DISCLAIMER_CUES: &[&str] = &[
    "not a promise of",
    "not a guarantee of",
    "no guarantee of",
    "not guaranteed",
    "is not automatic",
    "isn't automatic",
    "not automatically",
    "no automatic",
    "not lossless",
    "not zero-cost",
    "not zero cost",
];

/// Byte-distance window (in the flattened, lowercased text) searched backward
/// from a claim marker for a disclaimer cue. Comfortably covers both existing
/// disclaimers — `"...not a guarantee of zero HTML modification..."` puts the
/// cue immediately before the marker, and `"...not a promise of automatic or
/// lossless migration..."` puts it roughly thirty bytes before — while
/// staying short enough that an unrelated negation elsewhere in the same
/// document cannot suppress a real, separate positive claim.
const DISCLAIMER_LOOKBACK_BYTES: usize = 120;

/// Replaces every line break in `source` with a plain space and lowercases
/// the result, so a migration claim that a Markdown soft-wrap splits across
/// two lines still reads as one contiguous, case-insensitively matchable
/// phrase.
fn flatten_lowercase(source: &str) -> String {
    source
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect::<String>()
        .to_lowercase()
}

/// Returns the nearest valid `str` char boundary at or before `index`.
fn char_boundary_at_or_before(text: &str, mut index: usize) -> usize {
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// Returns the nearest valid `str` char boundary at or after `index`.
fn char_boundary_at_or_after(text: &str, mut index: usize) -> usize {
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

/// True if `text[..marker_start]` carries a `DISCLAIMER_CUES` entry within
/// the last `DISCLAIMER_LOOKBACK_BYTES` bytes, meaning the marker starting at
/// `marker_start` is a disclaimer rather than a positive claim.
fn has_disclaimer_cue_before(text: &str, marker_start: usize) -> bool {
    let window_start = char_boundary_at_or_before(
        text,
        marker_start.saturating_sub(DISCLAIMER_LOOKBACK_BYTES),
    );
    let window = &text[window_start..marker_start];
    DISCLAIMER_CUES.iter().any(|cue| window.contains(cue))
}

/// Migration-claim guard.
///
/// Neither `docs/` nor the repository-root `README.md` may assert a claim of
/// automatic switching, zero-cost migration, lossless migration, or zero HTML
/// changes between Design Systems. `docs/design-systems/wda-minimal.md`
/// explicitly disclaims exactly this ("not a promise of automatic or
/// lossless migration", "not a guarantee of zero HTML modification, ... or
/// automatic migration"), and this guard exists to keep that disclaimer
/// intact rather than let a positive version of the same claim slip back in.
/// Negated and disclaimer forms are excluded via `has_disclaimer_cue_before`.
#[test]
fn neutrality_no_migration_claim_under_docs_or_readme() {
    let repo_root = repository_root();

    let mut files = Vec::new();
    let docs_root = repo_root.join("docs");
    assert!(
        docs_root.is_dir(),
        "'docs' must exist under the repository root"
    );
    collect_all_files(&docs_root, &mut files);
    let readme = repo_root.join("README.md");
    assert!(
        readme.is_file(),
        "'README.md' must exist under the repository root"
    );
    files.push(readme);

    assert!(
        !files.is_empty(),
        "docs/ and README.md together must contain files for the migration-claim guard to inspect"
    );

    let mut violations = Vec::new();
    for path in &files {
        let Ok(contents) = fs::read_to_string(path) else {
            // Non-UTF-8 or binary files (fonts, images) carry no prose claims.
            continue;
        };
        let rel_path = path
            .strip_prefix(&repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let flattened = flatten_lowercase(&contents);

        for marker in MIGRATION_CLAIM_MARKERS {
            for (marker_start, _) in flattened.match_indices(marker) {
                if has_disclaimer_cue_before(&flattened, marker_start) {
                    continue;
                }
                let excerpt_start =
                    char_boundary_at_or_before(&flattened, marker_start.saturating_sub(40));
                let excerpt_end = char_boundary_at_or_after(
                    &flattened,
                    (marker_start + marker.len() + 40).min(flattened.len()),
                );
                violations.push(format!(
                    "{}: '{}' near: ...{}...",
                    rel_path,
                    marker,
                    flattened[excerpt_start..excerpt_end].trim()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "docs/ and README.md must carry no un-disclaimed migration claim \
         (automatic switching, zero-cost migration, lossless migration, or \
         zero HTML changes), got: {:#?}",
        violations
    );
}

/// Matcher-logic probe for the migration-claim guard above, run against
/// synthetic strings instead of the real `docs/`/`README.md` tree (the
/// allowlist for this task is `tests/neutrality.rs` only, so this test cannot
/// introduce a positive claim into a real doc file to watch the guard fail on
/// it). Exercises the same `flatten_lowercase` / `MIGRATION_CLAIM_MARKERS` /
/// `has_disclaimer_cue_before` pipeline the guard test uses, directly proving
/// both halves of its acceptance: a bare positive claim is flagged, and each
/// of the two disclaimer phrasings already living in
/// `docs/design-systems/wda-minimal.md` suppresses that same marker.
#[test]
fn neutrality_migration_claim_matcher_flags_positive_claims_and_spares_disclaimers() {
    let positive_claims = [
        "WDA now supports automatic switching between Spectrum 2 and Minimal.",
        "Migrating is a zero-cost migration with no follow-up work.",
        "This upgrade path is a lossless migration of every page.",
        "Switching design systems produces zero HTML changes in your project.",
    ];
    for claim in &positive_claims {
        let flattened = flatten_lowercase(claim);
        let found_unsuppressed_marker = MIGRATION_CLAIM_MARKERS.iter().any(|marker| {
            flattened
                .match_indices(marker)
                .any(|(marker_start, _)| !has_disclaimer_cue_before(&flattened, marker_start))
        });
        assert!(
            found_unsuppressed_marker,
            "matcher must flag the positive claim: {}",
            claim
        );
    }

    // The repository's two current, legitimate disclaimers from
    // `docs/design-systems/wda-minimal.md`: both must be fully suppressed,
    // i.e. every marker occurrence they contain has a disclaimer cue nearby.
    let disclaimers = [
        "This is not a promise of automatic or lossless migration.",
        "This is not a guarantee of zero HTML modification, styling parity, \
         or automatic migration.",
    ];
    for disclaimer in &disclaimers {
        let flattened = flatten_lowercase(disclaimer);
        let mut saw_marker = false;
        for marker in MIGRATION_CLAIM_MARKERS {
            for (marker_start, _) in flattened.match_indices(marker) {
                saw_marker = true;
                assert!(
                    has_disclaimer_cue_before(&flattened, marker_start),
                    "matcher must not flag the disclaimer '{}' at marker '{}'",
                    disclaimer,
                    marker
                );
            }
        }
        assert!(
            saw_marker,
            "disclaimer fixture must actually contain a migration-claim marker: {}",
            disclaimer
        );
    }
}
