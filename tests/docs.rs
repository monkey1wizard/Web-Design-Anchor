use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// The six front matter keys every Markdown file under `docs/` must carry,
/// independent of their `type`. `docs/architecture.md` "Documentation Rules for This Repository" owns
/// the key set; this scan is the enforcing side.
const REQUIRED_FRONT_MATTER_KEYS: &[&str] =
    &["title", "status", "updated", "type", "description", "tags"];

/// The exact five h2 sections every ADR must carry, in this order
/// (`docs/architecture.md` "Documentation Rules for This Repository", ADR format).
const EXPECTED_ADR_SECTIONS: &[&str] = &[
    "## Context",
    "## Options",
    "## Decision",
    "## Consequences",
    "## Status History",
];

/// Exact ADR count under `docs/adr/`. The 26 atomic ADRs have been
/// consolidated into 6 thematic ADRs (ADR-01 through ADR-06).
const EXPECTED_ADR_FILES: usize = 6;

/// The exact Markdown file set under `docs/` (R1:「docs/ 文件集等於 Approach
/// 的目標樹」). Freezing the set here means a missing or renamed directory
/// cannot pass silently (R18), and an unexpected file is caught the same
/// way the forbidden-strings scan catches unexpected text. Counts: 6
/// contract files + 6 thematic ADR files + 2 design-system files + 3 generated-
/// project templates + 10 zh-TW translations + 10 Japanese translations = 37.
const EXPECTED_DOCS_MD_FILES: &[&str] = &[
    "docs/architecture.md",
    "docs/commands.md",
    "docs/design-system.md",
    "docs/generated-project.md",
    "docs/glossary.md",
    "docs/i18n/ja/glossary.ja.md",
    "docs/i18n/ja/architecture.ja.md",
    "docs/i18n/ja/commands.ja.md",
    "docs/i18n/ja/design-system.ja.md",
    "docs/i18n/ja/generated-project.ja.md",
    "docs/i18n/ja/skill-spec.ja.md",
    "docs/i18n/ja/spectrum-adapter.ja.md",
    "docs/i18n/ja/wda-minimal.ja.md",
    "docs/i18n/ja/README.ja.md",
    "docs/i18n/ja/SECURITY.ja.md",
    "docs/i18n/zh-TW/architecture.zh-TW.md",
    "docs/i18n/zh-TW/commands.zh-TW.md",
    "docs/i18n/zh-TW/design-system.zh-TW.md",
    "docs/i18n/zh-TW/generated-project.zh-TW.md",
    "docs/i18n/zh-TW/glossary.zh-TW.md",
    "docs/i18n/zh-TW/README.zh-TW.md",
    "docs/i18n/zh-TW/SECURITY.zh-TW.md",
    "docs/i18n/zh-TW/skill-spec.zh-TW.md",
    "docs/i18n/zh-TW/spectrum-adapter.zh-TW.md",
    "docs/i18n/zh-TW/wda-minimal.zh-TW.md",
    "docs/skill-spec.md",
    "docs/adr/01-product-scope-and-output-boundaries.md",
    "docs/adr/02-authoring-dependencies-and-toolchain.md",
    "docs/adr/03-build-validation-and-diagnostics.md",
    "docs/adr/04-design-systems-tokens-and-neutrality.md",
    "docs/adr/05-generated-project-lifecycle-and-reversibility.md",
    "docs/adr/06-ai-skill-boundaries-and-evaluation.md",
    "docs/design-systems/spectrum-adapter.md",
    "docs/design-systems/wda-minimal.md",
    "docs/templates/generated-project/architecture.md",
    "docs/templates/generated-project/glossary.md",
    "docs/templates/generated-project/readme.md",
];

/// Retired document names and section-numbering markers that must never
/// reappear in tracked text (R18). Each entry is built from two literal
/// halves via `concat!` so this file's own source text never contains the
/// contiguous forbidden string, keeping the scan self-consistent even
/// where `tests/docs.rs` is not the excluded file.
const FORBIDDEN_STRINGS: &[&str] = &[
    concat!("§", "2."),
    concat!("docs/manifest", ".md"),
    concat!("docs/conventions", ".md"),
    concat!("docs/skill-boundary", ".md"),
    concat!("docs/validation", ".md"),
    concat!("docs/install", ".md"),
];

/// Repository roots walked by [`no_forbidden_strings`], relative to the
/// repository root. A bare file name (e.g. `README.md`) is scanned as a
/// single file; anything else is walked recursively.
const FORBIDDEN_SCAN_ROOTS: &[&str] = &[
    "docs",
    "README.md",
    "SECURITY.md",
    "AGENTS.md",
    "skills",
    "src",
    "tests",
    ".github",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn docs_dir() -> PathBuf {
    repo_root().join("docs")
}

/// Every regular file under `root`, walked recursively; `root` itself is
/// returned as a single-element result when it is already a file.
fn collect_files(root: &Path, files: &mut Vec<PathBuf>) {
    if root.is_file() {
        files.push(root.to_path_buf());
        return;
    }
    for entry in
        fs::read_dir(root).unwrap_or_else(|e| panic!("{} must be readable: {e}", root.display()))
    {
        let path = entry.expect("directory entry must be readable").path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

/// Repository-relative path of `path`, with `/` separators so assertion
/// output is stable across platforms.
fn rel_repo_path(path: &Path) -> String {
    path.strip_prefix(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .expect("path must live under the repository")
        .to_string_lossy()
        .replace('\\', "/")
}

/// Every Markdown file under `docs/`, walked recursively and sorted.
fn docs_md_files() -> Vec<PathBuf> {
    let mut result = Vec::new();
    let mut pending = vec![docs_dir()];
    while let Some(dir) = pending.pop() {
        for entry in
            fs::read_dir(&dir).unwrap_or_else(|e| panic!("{} must be readable: {e}", dir.display()))
        {
            let entry = entry.expect("directory entry must be readable");
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|e| e == "md") {
                result.push(path);
            }
        }
    }
    result.sort();
    result
}

/// The front matter YAML slice of `content` (between the leading `---`
/// delimiter and its closing line), or `None` when there is no front matter.
/// Splits on `\n` alone and trims the trailing `\r` on each line so the
/// byte-length bookkeeping in `slice_end` stays sound under CRLF checkouts.
fn front_matter_yaml_slice(content: &str) -> Option<&str> {
    let lines: Vec<&str> = content.split('\n').collect();
    if lines.first().map(|l| l.trim_end()) != Some("---") {
        return None;
    }
    let close_idx = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| line.trim_end() == "---")
        .map(|(idx, _)| idx)?;
    let slice_start = content.find('\n').map(|i| i + 1).unwrap_or(content.len());
    let slice_end = lines[..close_idx]
        .iter()
        .map(|l| l.len() + 1)
        .sum::<usize>();
    Some(&content[slice_start..slice_end])
}

/// The front matter keys present in `content`, as a set. A file without any
/// front matter yields an empty set, so every required key reports missing.
fn front_matter_keys(content: &str) -> BTreeSet<String> {
    let Some(slice) = front_matter_yaml_slice(content) else {
        return BTreeSet::new();
    };
    let node = marked_yaml::parse_yaml(0, slice).expect("front matter must be valid YAML");
    let mut keys = BTreeSet::new();
    if let Some(map) = node.as_mapping() {
        for (k, _) in map.iter() {
            keys.insert(k.as_str().to_string());
        }
    }
    keys
}

/// True when `text` contains `ADR-` immediately followed by an ASCII digit,
/// the regex `ADR-\d` the ADR body rule forbids (the ADR's own h1 is the
/// exception and is not part of the body this scans).
fn has_adr_number(text: &str) -> bool {
    let bytes = text.as_bytes();
    (0..bytes.len().saturating_sub(4))
        .any(|i| bytes[i..i + 4] == *b"ADR-" && bytes[i + 4].is_ascii_digit())
}

/// Diagnostic-code coverage guardrail (R14): `docs/commands.md` is the
/// lookup table for `src/codes.rs`, the single declaration site, so every
/// code in `ALL_CODES` must appear there at least once.
#[test]
fn codes_are_documented() {
    let path = docs_dir().join("commands.md");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));

    let missing: Vec<&str> = wda_core::codes::ALL_CODES
        .iter()
        .copied()
        .filter(|code| !content.contains(code))
        .collect();

    assert!(
        missing.is_empty(),
        "docs/commands.md does not document these diagnostic codes: {:?}",
        missing
    );
}

fn readme() -> String {
    let path = repo_root().join("README.md");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()))
}

/// README distinguishes computer-wide PATH tools from project-local Lit.
#[test]
fn readme_distinguishes_computer_and_project_tools() {
    let content = readme();
    assert!(
        content.contains("Git is optional"),
        "README must identify Git as an optional computer-level tool"
    );
    assert!(
        content.contains("Lit is a website package that the Spectrum default declares in `package.json`. It is not a global tool"),
        "README must identify Lit as a project package, not a global tool"
    );
}

/// README's no-Skill path requires a readable source before giving guidance.
#[test]
fn readme_no_skill_first_contact_requires_source() {
    let content = readme();
    assert!(
        content.contains("If the AI does not have the WDA Skill, first give it this README or another readable official WDA source"),
        "README must direct first contact without the Skill to an official readable source"
    );
    assert!(
        content.contains("it must ask you to provide an accessible source and then stop"),
        "README must require the AI to stop when it cannot read the source"
    );
    assert!(
        content.contains("The AI should not guess installation commands"),
        "README must forbid guessing installation commands"
    );
}

/// README requires read-only probes and distinguishes absent from unstartable.
#[test]
fn readme_documents_tool_probes_and_launch_failures() {
    let content = readme();
    for probe in ["`git --version`", "`wda --version`"] {
        assert!(
            content.contains(probe),
            "README must document the {probe} probe"
        );
    }
    assert!(
        content.contains("in a read-only way"),
        "README probes must be read-only"
    );
    assert!(
        content.contains("is not found on PATH"),
        "README must distinguish a tool absent from PATH"
    );
    assert!(
        content.contains("is found but cannot start"),
        "README must distinguish a tool that cannot start"
    );
    assert!(
        content.contains("launch error") && content.contains("must not invent an exit status"),
        "README must record launch errors without inventing an exit status"
    );
}

/// Command reference keeps Check's tool probes warning-only and project rules intact.
#[test]
fn commands_describe_warning_only_probes_and_shared_project_rules() {
    let path = docs_dir().join("commands.md");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
    let check_section = content
        .split("## wda check")
        .nth(1)
        .expect("docs/commands.md must describe wda check");
    let check_section = check_section
        .split("## wda build")
        .next()
        .expect("wda check section must end before wda build");
    assert!(
        check_section.contains("Check and Build both run the four project validation rule modules"),
        "Check and Build must be documented as running the same four project rule modules"
    );
    assert!(
        check_section.contains("These Warnings do not change the success status"),
        "missing or unstartable optional tools must not change Check status"
    );
    assert!(
        check_section.contains("Build does not run the PATH tool checks"),
        "Build must be documented as skipping the optional PATH probe"
    );
}

/// The command reference keeps the Git fallback, the external-manager boundary and the
/// retired Deno wording accurate.
#[test]
fn commands_document_git_warning_and_external_manager_fallback() {
    let path = docs_dir().join("commands.md");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
    assert!(
        content.contains("it still creates the project and `.gitignore`, skips `.git` and the first commit, and emits a `wda.toolchain.git-missing` Warning"),
        "missing Git must only warn and skip the repository"
    );
    assert!(
        content.contains("`wda init` creates a complete `WDA Minimal` project")
            && content.contains("never launch a package manager, never install or update packages, and never use the network"),
        "missing manager must fall back to Minimal and check/build must never install"
    );
    assert!(
        content.contains("`wda.tool.fault` | Error or Warning"),
        "command reference must document wda.tool.fault severity by impact"
    );
    assert!(
        content.contains("`wda.toolchain.git-missing` | Warning")
            && content.contains("`wda.toolchain.deno-missing` | Retired"),
        "command reference must list the Git warning and mark the Deno code retired"
    );
}

/// Every language of the command contract names the no-argument `tools fetch`, the removed
/// `deps` command with its external-manager hint, and the read-only check boundary.
#[test]
fn native_check_preflight_docs_three_language_contract_matches_cli() {
    let sources = [
        docs_dir().join("commands.md"),
        docs_dir().join("i18n/zh-TW/commands.zh-TW.md"),
        docs_dir().join("i18n/ja/commands.ja.md"),
    ];
    for path in &sources {
        let content = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
        for required in [
            "wda tools fetch",
            "wda deps",
            "npm install <package>",
            "bun add <package>",
            "wda.toolchain.git-missing",
            "wda.build.dependency-unlocked",
            "wda.build.tool-unavailable",
            "packageManager",
            "anchor.json",
        ] {
            assert!(
                content.contains(required),
                "{} must mention `{required}`",
                path.display()
            );
        }
        assert!(
            !content.contains("deno run") && !content.contains("deno install"),
            "{} must not describe Deno invocations",
            path.display()
        );
        assert!(
            !content.contains("wda deps add") && !content.contains("wda deps update"),
            "{} must not document deps subcommands",
            path.display()
        );
    }
    let usage = wda_core::cli::USAGE;
    assert!(usage.contains("tools fetch") && !usage.contains("deps"));
}

/// Every code in `ALL_CODES` stays queryable in the English and both translated tables, and a
/// retired code is marked as such instead of describing a tool requirement.
#[test]
fn native_check_preflight_docs_all_codes_are_queryable_and_retired_is_marked() {
    for path in [
        docs_dir().join("commands.md"),
        docs_dir().join("i18n/zh-TW/commands.zh-TW.md"),
        docs_dir().join("i18n/ja/commands.ja.md"),
    ] {
        let content = fs::read_to_string(&path).unwrap();
        for code in wda_core::codes::ALL_CODES {
            assert!(
                content.contains(code),
                "{} must list {code}",
                path.display()
            );
        }
        let retired_row = content
            .lines()
            .find(|line| line.starts_with("| `wda.toolchain.deno-missing`"))
            .unwrap_or_else(|| panic!("{} must keep a deno-missing row", path.display()));
        assert!(
            retired_row.contains("Retired") || retired_row.contains("已淘汰"),
            "{}: {retired_row}",
            path.display()
        );
        assert!(
            !retired_row.contains("Install Deno") && !retired_row.contains("check / init"),
            "{}: {retired_row}",
            path.display()
        );
    }
}

/// Front matter guardrail: every Markdown file under `docs/` carries the
/// six `REQUIRED_FRONT_MATTER_KEYS`, and the walked set equals the frozen
/// `EXPECTED_DOCS_MD_FILES` tree so a missing or renamed directory cannot
/// pass silently.
#[test]
fn docs_front_matter_has_six_keys() {
    let walked = docs_md_files();
    let walked_set: BTreeSet<String> = walked.iter().map(|p| rel_repo_path(p)).collect();
    let expected_set: BTreeSet<String> = EXPECTED_DOCS_MD_FILES
        .iter()
        .map(|s| s.to_string())
        .collect();

    let unexpected: Vec<&String> = walked_set.difference(&expected_set).collect();
    let missing: Vec<&String> = expected_set.difference(&walked_set).collect();
    assert_eq!(
        walked_set.len(),
        EXPECTED_DOCS_MD_FILES.len(),
        "docs/ walk found {} Markdown files, expected exact target tree of {}",
        walked_set.len(),
        EXPECTED_DOCS_MD_FILES.len()
    );
    assert!(
        unexpected.is_empty(),
        "docs/ contains Markdown files outside the target tree: {:?}",
        unexpected
    );
    assert!(
        missing.is_empty(),
        "docs/ is missing files from the target tree: {:?}",
        missing
    );

    for path in &walked {
        let content = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
        let present = front_matter_keys(&content);
        let absent: Vec<&str> = REQUIRED_FRONT_MATTER_KEYS
            .iter()
            .copied()
            .filter(|key| !present.contains(*key))
            .collect();
        assert!(
            absent.is_empty(),
            "{} is missing front matter key(s): {:?}",
            rel_repo_path(path),
            absent
        );
    }
}

/// ADR structure guardrail: each `docs/adr/*.md` carries exactly the five
/// `EXPECTED_ADR_SECTIONS` in order, and its body (everything after the h1)
/// carries no `ADR-<digit>` cross-reference, no `docs/` path and no
/// `README`. The failure list below is the work list for the ADR audit.
#[test]
fn adr_has_five_sections() {
    let adr_dir = docs_dir().join("adr");
    let mut files: Vec<PathBuf> = fs::read_dir(&adr_dir)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", adr_dir.display()))
        .map(|entry| entry.expect("directory entry must be readable").path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
        .collect();
    files.sort();

    assert_eq!(
        files.len(),
        EXPECTED_ADR_FILES,
        "docs/adr must contain exactly {} ADR files, found {}",
        EXPECTED_ADR_FILES,
        files.len()
    );

    let mut offenders: Vec<(String, Vec<String>)> = Vec::new();
    for path in &files {
        let rel = rel_repo_path(path);
        let content = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
        let mut violations: Vec<String> = Vec::new();
        let filename = path
            .file_name()
            .and_then(|f| f.to_str())
            .expect("valid utf-8 filename");
        let valid_filename = filename.len() > 3
            && filename[0..2].chars().all(|c| c.is_ascii_digit())
            && &filename[2..3] == "-"
            && filename.ends_with(".md");
        if !valid_filename {
            violations.push(format!("filename '{filename}' does not match 'NN-*.md'"));
        }

        let h1_idx = content
            .lines()
            .position(|line| {
                let t = line.trim_end();
                t.starts_with("# ") && !t.starts_with("## ")
            })
            .expect("every ADR must carry an h1 heading");

        let h1_line = content.lines().nth(h1_idx).unwrap().trim_end();
        let valid_h1 = h1_line.starts_with("# ADR-")
            && h1_line.len() >= 9
            && h1_line[6..8].chars().all(|c| c.is_ascii_digit())
            && &h1_line[8..9] == " ";
        if !valid_h1 {
            violations.push(format!(
                "h1 heading '{h1_line}' does not match '# ADR-NN <title>'"
            ));
        }

        let body_lines: Vec<&str> = content
            .lines()
            .skip(h1_idx + 1)
            .map(|l| l.trim_end_matches('\r'))
            .collect();

        let h2: Vec<String> = body_lines
            .iter()
            .filter(|l| l.starts_with("## "))
            .map(|l| l.to_string())
            .collect();
        if h2 != EXPECTED_ADR_SECTIONS {
            violations.push(format!(
                "h2 sections {:?} != expected {:?} in order",
                h2, EXPECTED_ADR_SECTIONS
            ));
        }

        let body = body_lines.join("\n");
        if has_adr_number(&body) {
            violations.push("body contains 'ADR-<digit>'".to_string());
        }
        if body.contains("docs/") {
            violations.push("body contains 'docs/'".to_string());
        }
        if body.contains("README") {
            violations.push("body contains 'README'".to_string());
        }

        if !violations.is_empty() {
            offenders.push((rel, violations));
        }
    }

    assert!(
        offenders.is_empty(),
        "ADRs violating the five-section rule ({}):\n{}",
        offenders.len(),
        offenders
            .iter()
            .map(|(rel, v)| format!("  {}: {}", rel, v.join("; ")))
            .collect::<Vec<String>>()
            .join("\n")
    );
}

/// Forbidden-strings guardrail (R18): retired document names and the old
/// `§2.` section-numbering marker must never reappear under the tracked
/// surfaces this repository ships to readers or CI. `tests/docs.rs` is
/// excluded from its own scan, since [`FORBIDDEN_STRINGS`] intentionally
/// names the strings it forbids.
#[test]
fn no_forbidden_strings() {
    let root = repo_root();
    let excluded = root.join("tests").join("docs.rs");

    let mut files = Vec::new();
    for rel in FORBIDDEN_SCAN_ROOTS {
        let path = root.join(rel);
        if rel == &"AGENTS.md" && !path.exists() && fully_stripped_export(&root) {
            continue;
        }
        assert!(
            path.exists(),
            "required forbidden-strings scan root must exist: {rel}"
        );
        collect_files(&path, &mut files);
    }
    files.retain(|p| p != &excluded);

    let mut offenders: Vec<String> = Vec::new();
    let mut scanned = 0usize;
    for path in &files {
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        scanned += 1;
        for needle in FORBIDDEN_STRINGS {
            if content.contains(needle) {
                offenders.push(format!("{}: {:?}", rel_repo_path(path), needle));
            }
        }
    }

    assert!(
        scanned >= 60,
        "forbidden-strings scan covered only {} files, expected at least 60",
        scanned
    );
    assert!(
        offenders.is_empty(),
        "forbidden strings found ({}):\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

/// A public snapshot omits every private-source marker and the private publish controller.
fn fully_stripped_export(root: &Path) -> bool {
    [
        "AGENTS.md",
        "CLAUDE.md",
        ".dev",
        ".claude",
        ".github/instructions",
    ]
    .iter()
    .all(|path| !root.join(path).exists())
        && !root.join(".github/workflows/publish-public.yml").exists()
}

#[test]
fn export_scan_exempts_only_fully_stripped_public_tree() {
    let root = std::env::temp_dir().join(format!("wda-doc-scan-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    assert!(fully_stripped_export(&root));
    for marker in [
        "AGENTS.md",
        "CLAUDE.md",
        ".dev",
        ".claude",
        ".github/instructions",
    ] {
        let path = root.join(marker);
        if marker.contains('.') && !marker.ends_with(".md") {
            fs::create_dir_all(&path).unwrap();
        } else {
            fs::write(&path, "private marker").unwrap();
        }
        assert!(
            !fully_stripped_export(&root),
            "partial marker {marker} was accepted"
        );
        if path.is_dir() {
            fs::remove_dir_all(path).unwrap();
        } else {
            fs::remove_file(path).unwrap();
        }
    }
    let controller = root.join(".github/workflows/publish-public.yml");
    fs::create_dir_all(controller.parent().unwrap()).unwrap();
    fs::write(&controller, "controller").unwrap();
    assert!(
        !fully_stripped_export(&root),
        "controller presence was accepted as public export"
    );
    fs::remove_dir_all(root).unwrap();
}

/// Thematic ADR mapping table: target thematic file to absorbed 3-digit ADR numbers.
const THEMATIC_ADR_MAPPING: &[(&str, &[&str])] = &[
    (
        "01-product-scope-and-output-boundaries.md",
        &["001", "002", "016", "017", "028"],
    ),
    (
        "02-authoring-dependencies-and-toolchain.md",
        &["006", "011", "013"],
    ),
    (
        "03-build-validation-and-diagnostics.md",
        &["007", "010", "012", "015", "018", "027"],
    ),
    (
        "04-design-systems-tokens-and-neutrality.md",
        &["008", "014", "019", "024", "025", "031"],
    ),
    (
        "05-generated-project-lifecycle-and-reversibility.md",
        &["023", "029", "030"],
    ),
    (
        "06-ai-skill-boundaries-and-evaluation.md",
        &["003", "022", "026"],
    ),
];

/// Front-matter `absorbs` values are quoted strings, leading-zero
/// members round-trip byte-for-byte, and the 26-to-6 mapping is complete with
/// no duplicate or orphaned decision.
#[test]
fn adr_absorbs_quoted_strings_roundtrip_and_mapping() {
    let adr_dir = docs_dir().join("adr");
    let mut all_absorbed = BTreeSet::new();

    for (filename, expected_absorbs) in THEMATIC_ADR_MAPPING {
        let path = adr_dir.join(filename);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));

        let yaml_slice = front_matter_yaml_slice(&content)
            .unwrap_or_else(|| panic!("{} must carry front matter", filename));

        // 1. Verify raw text has absorbs key and values are quoted strings
        let absorbs_line = yaml_slice
            .lines()
            .find(|l| l.trim_start().starts_with("absorbs:"))
            .unwrap_or_else(|| panic!("{} front matter must contain 'absorbs' key", filename));

        for src in *expected_absorbs {
            let quoted = format!("\"{src}\"");
            assert!(
                absorbs_line.contains(&quoted),
                "{} front matter 'absorbs' must contain quoted string {}",
                filename,
                quoted
            );
        }

        // 2. Parse YAML and verify absorbs is a sequence of strings
        let node = marked_yaml::parse_yaml(0, yaml_slice).expect("front matter must be valid YAML");
        let map = node.as_mapping().expect("front matter must be a mapping");
        let absorbs_node = map
            .get("absorbs")
            .unwrap_or_else(|| panic!("{} must have 'absorbs' node", filename));
        let seq = absorbs_node
            .as_sequence()
            .unwrap_or_else(|| panic!("{} 'absorbs' must be a YAML sequence", filename));

        let actual_absorbs: Vec<String> = seq
            .iter()
            .map(|item| {
                item.as_scalar()
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_default()
            })
            .collect();

        assert_eq!(
            actual_absorbs, *expected_absorbs,
            "{} absorbs mismatch",
            filename
        );

        for src in *expected_absorbs {
            assert!(
                all_absorbed.insert(*src),
                "Duplicate absorbed decision {} found in {}",
                src,
                filename
            );
        }
    }

    // 3. Verify exactly 26 source numbers absorbed with no orphans
    assert_eq!(
        all_absorbed.len(),
        26,
        "Total absorbed decisions must be exactly 26, found {}",
        all_absorbed.len()
    );

    // 4. Verify leading-zero member e.g. "008" round-trips byte-for-byte through jsonc_parser / serde
    let adr04_content =
        fs::read_to_string(adr_dir.join("04-design-systems-tokens-and-neutrality.md"))
            .expect("ADR-04 must be readable");
    assert!(
        adr04_content.contains("\"008\""),
        "ADR-04 front matter must retain quoted leading-zero \"008\""
    );
}

/// Expected status-history entry dates per ADR, in file order. Taken from the
/// files before the English translation, the translation entry, and later entries.
const ADR_HISTORY_DATES: &[(&str, &[&str])] = &[
    (
        "01-product-scope-and-output-boundaries.md",
        &[
            "2026-08-24",
            "2026-09-18",
            "2026-08-24",
            "2026-09-18",
            "2026-08-24",
            "2026-08-24",
            "2026-09-18",
            "2026-08-24",
            "2026-08-24",
            "2026-09-18",
            "2026-08-24",
            "2026-09-18",
            "2026-09-21",
            "2026-10-23",
        ],
    ),
    (
        "02-authoring-dependencies-and-toolchain.md",
        &[
            "2026-08-24",
            "2026-09-02",
            "2026-09-08",
            "2026-09-18",
            "2026-08-24",
            "2026-08-24",
            "2026-09-02",
            "2026-09-18",
            "2026-08-24",
            "2026-08-24",
            "2026-09-02",
            "2026-09-18",
            "2026-10-14",
            "2026-09-21",
            "2026-10-23",
            "2026-11-06",
        ],
    ),
    (
        "03-build-validation-and-diagnostics.md",
        &[
            "2026-08-24",
            "2026-09-18",
            "2026-08-24",
            "2026-08-24",
            "2026-09-18",
            "2026-08-24",
            "2026-08-24",
            "2026-09-18",
            "2026-08-24",
            "2026-08-24",
            "2026-09-18",
            "2026-08-24",
            "2026-08-24",
            "2026-09-18",
            "2026-08-24",
            "2026-08-26",
            "2026-09-18",
            "2026-09-21",
            "2026-10-23",
        ],
    ),
    (
        "04-design-systems-tokens-and-neutrality.md",
        &[
            "2026-08-24",
            "2026-08-27",
            "2026-09-18",
            "2026-08-24",
            "2026-08-24",
            "2026-09-18",
            "2026-08-24",
            "2026-08-24",
            "2026-08-27",
            "2026-09-07",
            "2026-09-18",
            "2026-09-05",
            "2026-09-18",
            "2026-09-05",
            "2026-09-07",
            "2026-09-18",
            "2026-08-24",
            "2026-08-27",
            "2026-09-18",
            "2026-09-21",
            "2026-10-23",
        ],
    ),
    (
        "05-generated-project-lifecycle-and-reversibility.md",
        &[
            "2026-09-02",
            "2026-09-08",
            "2026-09-09",
            "2026-09-18",
            "2026-08-24",
            "2026-09-08",
            "2026-09-18",
            "2026-08-24",
            "2026-09-18",
            "2026-10-14",
            "2026-09-21",
            "2026-10-23",
            "2026-11-06",
        ],
    ),
    (
        "06-ai-skill-boundaries-and-evaluation.md",
        &[
            "2026-08-24",
            "2026-09-18",
            "2026-08-24",
            "2026-08-26",
            "2026-09-08",
            "2026-09-18",
            "2026-09-05",
            "2026-09-09",
            "2026-09-18",
            "2026-09-21",
            "2026-10-23",
        ],
    ),
];

/// Every replacement ADR keeps every status-history entry in order: the
/// entry dates match `ADR_HISTORY_DATES`, and the consolidation entry immediately
/// precedes the translation entry.
#[test]
fn adr_history_preserved_in_mapped_order() {
    let adr_dir = docs_dir().join("adr");

    for (filename, _) in THEMATIC_ADR_MAPPING {
        let path = adr_dir.join(filename);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));

        let history_start = content
            .find("## Status History")
            .expect("must contain ## Status History");
        let history_section = &content[history_start..];
        let history_lines: Vec<&str> = history_section
            .lines()
            .map(|l| l.trim_end_matches('\r'))
            .filter(|l| l.starts_with("- "))
            .collect();

        let count = history_lines.len();
        assert!(
            count >= 2,
            "{filename} must have at least two history entries"
        );
        let translation_idx = history_lines
            .iter()
            .position(|l| *l == "- 2026-10-23: Translated to English. The decision is unchanged.")
            .unwrap_or_else(|| panic!("{filename} must contain the translation notice"));
        assert!(
            translation_idx > 0,
            "{filename} translation notice must have a preceding entry"
        );
        assert_eq!(
            history_lines[translation_idx - 1],
            "- 2026-09-21: Consolidated from existing atomic decisions. The sources are listed in the front matter `absorbs`.",
            "{filename} entry preceding translation must be the consolidation notice"
        );
        for line in &history_lines {
            assert!(
                !has_adr_number(line),
                "{filename} history entry must not contain literal ADR-<digits>: {line}"
            );
        }

        let dates: Vec<&str> = history_lines
            .iter()
            .map(|l| l.trim_start_matches("- ").split(':').next().unwrap_or(""))
            .collect();
        let expected = ADR_HISTORY_DATES
            .iter()
            .find(|(name, _)| name == filename)
            .map(|(_, dates)| *dates)
            .unwrap_or_else(|| panic!("{filename} must have an expected date list"));
        // Entries recorded after the baseline list are appended in order, so the
        // recorded list must remain an exact prefix of the history.
        assert!(
            dates.len() >= expected.len() && dates[..expected.len()] == expected[..],
            "{filename} history entry dates must keep the recorded list as an ordered prefix, got {dates:?}"
        );
    }
}

/// Human-readable 26-to-6 mapping table exists in `docs/architecture.md`.
#[test]
fn architecture_contains_mapping_table() {
    let arch_path = docs_dir().join("architecture.md");
    let content = fs::read_to_string(&arch_path).expect("architecture.md must be readable");

    for (filename, absorbs) in THEMATIC_ADR_MAPPING {
        assert!(
            content.contains(filename),
            "architecture.md table must contain filename {}",
            filename
        );
        for src in *absorbs {
            assert!(
                content.contains(src),
                "architecture.md table must mention absorbed source {}",
                src
            );
        }
    }
}

/// Tracked search restricted to the live-contract surface returns zero
/// stale three-digit label or path (retired numbers 004, 005, 009, 020, 021
/// in docs/architecture.md's retired-numbers table stay verbatim).
#[test]
fn live_contract_has_no_stale_three_digit_adr_references() {
    let root = repo_root();
    let live_contract_files: &[&str] = &[
        "docs/commands.md",
        "docs/design-system.md",
        "docs/generated-project.md",
        "docs/glossary.md",
        "docs/skill-spec.md",
        "docs/design-systems/spectrum-adapter.md",
        "docs/design-systems/wda-minimal.md",
        "docs/templates/generated-project/architecture.md",
        "docs/templates/generated-project/glossary.md",
        "docs/templates/generated-project/readme.md",
        "AGENTS.md",
        "skills/wda/references/reversibility.md",
        "tests/skill.rs",
    ];

    let mut offenders = Vec::new();

    for rel in live_contract_files {
        let path = root.join(rel);
        if let Ok(content) = fs::read_to_string(&path) {
            for (idx, line) in content.lines().enumerate() {
                let bytes = line.as_bytes();
                for i in 0..bytes.len().saturating_sub(6) {
                    if &bytes[i..i + 4] == b"ADR-"
                        && bytes[i + 4].is_ascii_digit()
                        && bytes[i + 5].is_ascii_digit()
                        && bytes[i + 6].is_ascii_digit()
                    {
                        offenders.push(format!("{}:{}: {}", rel, idx + 1, line.trim()));
                    }
                }
                if line.contains("docs/adr/0") {
                    offenders.push(format!("{}:{}: {}", rel, idx + 1, line.trim()));
                }
            }
        }
    }

    // Also check docs/architecture.md up to "## Retired ADR Numbers"
    let arch_content = fs::read_to_string(root.join("docs/architecture.md"))
        .expect("architecture.md must be readable");
    let active_arch = arch_content
        .split("## Retired ADR Numbers")
        .next()
        .unwrap_or(&arch_content);
    for (idx, line) in active_arch.lines().enumerate() {
        let bytes = line.as_bytes();
        for i in 0..bytes.len().saturating_sub(6) {
            if &bytes[i..i + 4] == b"ADR-"
                && bytes[i + 4].is_ascii_digit()
                && bytes[i + 5].is_ascii_digit()
                && bytes[i + 6].is_ascii_digit()
            {
                offenders.push(format!("docs/architecture.md:{}: {}", idx + 1, line.trim()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "Found stale 3-digit ADR references in live-contract surface ({}):\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

/// Architecture contract for the native tool and package duties.
///
/// The English architecture document, its zh-TW and ja translations, and the
/// code seams it names must agree. Each translation carries the SHA-256 of the
/// LF-normalized English file in its `source_sha256` front matter key, and each
/// decision ADR keeps its current text free of the retired dependency model.
#[test]
fn native_architecture_contract() {
    let root = repo_root();
    let english = fs::read_to_string(root.join("docs/architecture.md"))
        .expect("architecture.md must be readable")
        .replace("\r\n", "\n");

    for required in [
        "`anchor.json`",
        "`package.json`",
        "Rolldown 1.3.1",
        "`packaging/toolchain.json`",
        "npm or Bun",
    ] {
        assert!(
            english.contains(required),
            "architecture.md must mention {required}"
        );
    }
    for retired in [
        "esm.sh",
        "import map",
        "vendor/import_map",
        "wda deps",
        "tsgo",
    ] {
        assert!(
            !english.contains(retired),
            "architecture.md must not keep the retired term {retired}"
        );
    }

    let toolchain_text = fs::read_to_string(root.join("packaging/toolchain.json"))
        .expect("packaging/toolchain.json must be readable");
    let toolchain: serde_json::Value =
        serde_json::from_str(&toolchain_text).expect("packaging/toolchain.json must be JSON");
    assert_eq!(toolchain["schema_version"], 1);
    assert_eq!(
        toolchain["package_managers"],
        serde_json::json!(["npm", "bun"])
    );
    assert_eq!(toolchain["name"], "typescript");
    assert_eq!(toolchain["platforms"].as_array().map(Vec::len), Some(6));

    let cargo = fs::read_to_string(root.join("Cargo.toml")).expect("Cargo.toml must be readable");
    assert!(
        cargo.contains("rolldown = \"=1.3.1\""),
        "Cargo.toml must pin Rolldown 1.3.1"
    );
    for seam in [
        "src/tools.rs",
        "src/tool_probe.rs",
        "src/init/resolve.rs",
        "src/validation/project_contract.rs",
        "schemas/anchor.schema.json",
    ] {
        assert!(
            root.join(seam).is_file(),
            "{seam} must exist as a named code seam"
        );
    }

    let digest: String = Sha256::digest(english.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    for translation in [
        "docs/i18n/zh-TW/architecture.zh-TW.md",
        "docs/i18n/ja/architecture.ja.md",
    ] {
        let content = fs::read_to_string(root.join(translation))
            .unwrap_or_else(|e| panic!("{translation} must be readable: {e}"))
            .replace("\r\n", "\n");
        let front_matter = front_matter_yaml_slice(&content)
            .unwrap_or_else(|| panic!("{translation} must have front matter"));
        assert!(
            front_matter
                .lines()
                .any(|line| line == "source: docs/architecture.md"),
            "{translation} must name docs/architecture.md as its source"
        );
        assert!(
            front_matter
                .lines()
                .any(|line| line == format!("source_sha256: {digest}")),
            "{translation} source_sha256 must equal the SHA-256 of the LF-normalized English file"
        );
        for required in ["anchor.json", "packaging/toolchain.json", "Rolldown 1.3.1"] {
            assert!(
                content.contains(required),
                "{translation} must carry {required}"
            );
        }
    }

    for adr in [
        "docs/adr/02-authoring-dependencies-and-toolchain.md",
        "docs/adr/05-generated-project-lifecycle-and-reversibility.md",
    ] {
        let text = fs::read_to_string(root.join(adr))
            .unwrap_or_else(|e| panic!("{adr} must be readable: {e}"))
            .replace("\r\n", "\n");
        let decision = text
            .split("## Decision")
            .nth(1)
            .and_then(|rest| rest.split("## Status History").next())
            .unwrap_or_else(|| panic!("{adr} must have Decision and Status History sections"));
        for retired in ["esm.sh", "import map", "vendor/", "wda deps add"] {
            assert!(
                !decision.contains(retired),
                "{adr} current decision must not keep the retired term {retired}"
            );
        }
        let last_entry = text
            .lines()
            .filter(|line| line.starts_with("- "))
            .last()
            .unwrap_or_default();
        if adr.contains("02-authoring") {
            let original = "- 2026-11-12: Removed the esm.sh, vendor directory, import map, type-declaration mirror, cross-version tool switching, and Deno-necessity statements from the current decision and consequences.";
            let appended = "- 2026-11-14: Aligned manifest, version writeback, manager, and embedded tool guidance with the native CLI.";
            assert!(text.contains(&format!("{original}\n{appended}")), "{adr} must preserve the original entry immediately before the new entry");
            assert_eq!(last_entry, appended);
        } else {
            assert!(
                last_entry.starts_with("- 2026-11-12:"),
                "{adr} must end its Status History with the 2026-11-12 entry, got {last_entry}"
            );
        }
    }
}

fn read_lf_document(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel))
        .unwrap_or_else(|e| panic!("{rel} must be readable: {e}"))
        .replace("\r\n", "\n")
}

fn lf_sha256_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Design System and glossary contract in three languages.
///
/// The English `design-system` and `glossary` documents and their zh-TW and ja
/// translations must name the same manifest, lockfile, tool cache and fallback
/// terms, must not keep the retired `wda.json`, Deno or deps entry points, and
/// must carry the SHA-256 of the LF-normalized English source.
#[test]
fn native_design_contract() {
    let root = repo_root();
    let retired = [
        "wda.json",
        "deno.jsonc",
        "deno.lock",
        "wda deps",
        "esm.sh",
        "Deno",
        "esbuild",
        "spectrum_two_dependency_specs",
    ];
    let pairs = [
        (
            "docs/design-system.md",
            "docs/i18n/zh-TW/design-system.zh-TW.md",
            "docs/i18n/ja/design-system.ja.md",
        ),
        (
            "docs/glossary.md",
            "docs/i18n/zh-TW/glossary.zh-TW.md",
            "docs/i18n/ja/glossary.ja.md",
        ),
    ];

    for (english_rel, zh_rel, ja_rel) in pairs {
        let english = read_lf_document(&root, english_rel);
        for term in retired {
            assert!(
                !english.contains(term),
                "{english_rel} must not keep the retired term {term}"
            );
        }
        for required in ["`package.json`", "`anchor.json`", "`wda tools fetch`"] {
            assert!(
                english.contains(required),
                "{english_rel} must name {required}"
            );
        }

        let digest = lf_sha256_hex(&english);
        for translation in [zh_rel, ja_rel] {
            let content = read_lf_document(&root, translation);
            for term in retired {
                assert!(
                    !content.contains(term),
                    "{translation} must not keep the retired term {term}"
                );
            }
            let front_matter = front_matter_yaml_slice(&content)
                .unwrap_or_else(|| panic!("{translation} must have front matter"));
            assert!(
                front_matter
                    .lines()
                    .any(|line| line == format!("source: {english_rel}")),
                "{translation} must name {english_rel} as its source"
            );
            assert!(
                front_matter.lines().any(|line| line == format!("source_sha256: {digest}")),
                "{translation} source_sha256 must equal the SHA-256 of the LF-normalized {english_rel}"
            );
            for required in ["`package.json`", "`anchor.json`", "`wda tools fetch`"] {
                assert!(
                    content.contains(required),
                    "{translation} must name {required}"
                );
            }
        }
    }

    for (rel, required) in [
        ("docs/design-system.md", "`initialization.fallback`"),
        ("docs/glossary.md", "`initialization.fallback`"),
        ("docs/glossary.md", "`schemas/anchor.schema.json`"),
        ("docs/glossary.md", "`packageManager`"),
    ] {
        let english = read_lf_document(&root, rel);
        assert!(english.contains(required), "{rel} must name {required}");
    }
}

/// Manifest and dependency duties in the Design System contract.
///
/// `package.json` owns website dependencies, `anchor.json` owns WDA metadata
/// and the fallback record, the external manager owns its lockfile, no command
/// migrates old projects automatically, token layering stays voluntary, and a
/// project may use a Design System other than the two built-in systems.
#[test]
fn native_manifest_documentation() {
    let root = repo_root();
    let english = read_lf_document(&root, "docs/design-system.md");

    for required in [
        "`package.json` is the only website dependency source",
        "`anchor.json` owns the WDA settings",
        "never writes, rewrites, or migrates a manager lockfile",
        "`wda tools fetch` takes no arguments",
        "WDA does not migrate existing projects automatically",
        "Token layering is voluntary",
        "A project may use a Design System other than these two",
        "The Skill can add Spectrum and Lit to a `WDA Minimal` project later",
    ] {
        assert!(
            english.contains(required),
            "design-system.md must state: {required}"
        );
    }
    assert!(
        !english.contains("Deno") && !english.contains("wda.json"),
        "design-system.md must not present Deno or wda.json as a required entry point"
    );

    let zh = read_lf_document(&root, "docs/i18n/zh-TW/design-system.zh-TW.md");
    for required in [
        "`package.json` 是唯一的網站依賴來源",
        "`anchor.json` 負責 WDA 設定",
        "WDA 絕不寫入、改寫或遷移管理器 lockfile",
        "`wda tools fetch` 不接受引數",
        "WDA 不會自動遷移既有專案",
        "token 分層都是自願的",
        "其他 Design System",
    ] {
        assert!(
            zh.contains(required),
            "zh-TW design-system must state: {required}"
        );
    }

    let ja = read_lf_document(&root, "docs/i18n/ja/design-system.ja.md");
    for required in [
        "`package.json`が唯一の",
        "`anchor.json`は、WDAの設定",
        "WDAはマネージャーのlockfileを書き込み、書き換え、移行しない",
        "`wda tools fetch`は引数を取らず",
        "WDAは既存のプロジェクトを自動的に移行しない",
        "トークンの階層化は、どのDesign Systemを使うプロジェクトでも任意",
        "ほかのDesign System",
    ] {
        assert!(
            ja.contains(required),
            "ja design-system must state: {required}"
        );
    }
}

/// Generated-project formal entry in three languages.
///
/// The English `generated-project` document and its zh-TW and ja translations
/// must name the same two contract files, the external lockfile duty and the
/// `fallback` fields, must carry the SHA-256 of the LF-normalized English source,
/// and the retired `wda.schema.json` must no longer exist.
#[test]
fn native_strict_manifest_cutover_generated_project_contract() {
    let root = repo_root();
    assert!(
        !root.join("schemas/wda.schema.json").exists(),
        "the retired wda.schema.json must be deleted by the strict cutover"
    );
    let english_rel = "docs/generated-project.md";
    let english = read_lf_document(&root, english_rel);
    let digest = lf_sha256_hex(&english);
    for required in [
        "`anchor.json`",
        "`package.json`",
        "`schemas/anchor.schema.json`",
        "`initialization.fallback`",
        "`requestedDesignSystem`",
        "`reason`",
        "`packageManager`",
        "Manager Lockfile",
        "`wda tools fetch`",
    ] {
        assert!(
            english.contains(required),
            "{english_rel} must name {required}"
        );
    }
    assert!(
        !english.contains("`wda.json` is the") && !english.contains("wda.schema.json"),
        "{english_rel} must not name the retired schema or a wda.json entry point"
    );

    for translation in [
        "docs/i18n/zh-TW/generated-project.zh-TW.md",
        "docs/i18n/ja/generated-project.ja.md",
    ] {
        let content = read_lf_document(&root, translation);
        let front_matter = front_matter_yaml_slice(&content)
            .unwrap_or_else(|| panic!("{translation} must have front matter"));
        assert!(
            front_matter
                .lines()
                .any(|line| line == format!("source: {english_rel}")),
            "{translation} must name {english_rel} as its source"
        );
        assert!(
            front_matter
                .lines()
                .any(|line| line == format!("source_sha256: {digest}")),
            "{translation} source_sha256 must equal the SHA-256 of the LF-normalized English file"
        );
        for required in [
            "`anchor.json`",
            "`package.json`",
            "`schemas/anchor.schema.json`",
            "`initialization.fallback`",
            "`requestedDesignSystem`",
            "`reason`",
            "`packageManager`",
            "Manager Lockfile",
        ] {
            assert!(
                content.contains(required),
                "{translation} must name {required}"
            );
        }
        assert!(
            !content.contains("wda.schema.json"),
            "{translation} must not name the retired schema"
        );
    }
}

#[test]
fn author_diagnostic_contract() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (file, phrases) in [
        (
            "docs/adr/03-build-validation-and-diagnostics.md",
            &[
                "1-based UTF-8 byte offsets",
                "`wda check --json`",
                "every `img`",
                "every `id`",
                "`html[lang]`",
                "symlink-escape",
            ] as &[&str],
        ),
        (
            "docs/adr/04-design-systems-tokens-and-neutrality.md",
            &[
                "Token layering is voluntary",
                "Core does not enforce a layer matrix",
            ],
        ),
        (
            "docs/adr/06-ai-skill-boundaries-and-evaluation.md",
            &[
                "separate from Core diagnostics",
                "Review alone does not authorize edits",
            ],
        ),
        (
            "docs/architecture.md",
            &[
                "commands.md#json-output",
                "UTF-8 byte columns",
                "Review does not authorize edits",
            ],
        ),
        (
            "docs/i18n/zh-TW/architecture.zh-TW.md",
            &["commands.md#json-output", "UTF-8", "檢視本身不授權編輯"],
        ),
        (
            "docs/i18n/ja/architecture.ja.md",
            &[
                "commands.md#json-output",
                "UTF-8",
                "レビュー自体は編集を許可しない",
            ],
        ),
    ] {
        let text = fs::read_to_string(root.join(file)).unwrap();
        for phrase in phrases {
            assert!(text.contains(phrase), "{file} is missing {phrase}");
        }
    }
}
