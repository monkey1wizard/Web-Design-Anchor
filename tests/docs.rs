use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The six front matter keys every Markdown file under `docs/` must carry,
/// independent of their `type`. `docs/architecture.md`「本倉庫的文件規範」owns
/// the key set; this scan is the enforcing side.
const REQUIRED_FRONT_MATTER_KEYS: &[&str] =
    &["title", "status", "updated", "type", "description", "tags"];

/// The exact five h2 sections every ADR must carry, in this order
/// (`docs/architecture.md`「本倉庫的文件規範」ADR 格式).
const EXPECTED_ADR_SECTIONS: &[&str] = &["## 背景", "## 選項", "## 決策", "## 後果", "## 狀態沿革"];

/// Exact ADR count under `docs/adr/`. The 26 atomic ADRs have been
/// consolidated into 6 thematic ADRs (ADR-01 through ADR-06).
const EXPECTED_ADR_FILES: usize = 6;

/// The exact Markdown file set under `docs/` (R1:「docs/ 文件集等於 Approach
/// 的目標樹」). Freezing the set here means a missing or renamed directory
/// cannot pass silently (R18), and an unexpected file is caught the same
/// way the forbidden-strings scan catches unexpected text. Counts: 6
/// contract files + 6 thematic ADR files + 2 design-system files + 3 generated-
/// project templates = 17.
const EXPECTED_DOCS_MD_FILES: &[&str] = &[
    "docs/architecture.md",
    "docs/commands.md",
    "docs/design-system.md",
    "docs/generated-project.md",
    "docs/glossary.md",
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
    "docs/templates/generated-project/naming.md",
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
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()))
}

/// README distinguishes computer-wide PATH tools from project-local Lit.
#[test]
fn readme_distinguishes_computer_and_project_tools() {
    let content = readme();
    assert!(
        content.contains("Git 與 Deno 是電腦層級工具"),
        "README must identify Git and Deno as computer-level tools"
    );
    assert!(
        content.contains("Lit 是每個 WDA 專案自己的相依項目，不是全域工具"),
        "README must identify Lit as a project dependency, not a global tool"
    );
}

/// README's no-Skill path requires a readable source before giving guidance.
#[test]
fn readme_no_skill_first_contact_requires_source() {
    let content = readme();
    assert!(
        content.contains("如果 AI 沒有 WDA Skill，請先提供本 README 或另一份可讀取的官方 WDA 來源"),
        "README must direct first contact without the Skill to an official readable source"
    );
    assert!(
        content.contains("必須請你提供可存取的來源，然後停止"),
        "README must require the AI to stop when it cannot read the source"
    );
    assert!(
        content.contains("AI 不應自行猜測安裝指令"),
        "README must forbid guessing installation commands"
    );
}

/// README requires read-only probes and distinguishes absent from unstartable.
#[test]
fn readme_documents_tool_probes_and_launch_failures() {
    let content = readme();
    for probe in ["`git --version`", "`deno --version`", "`wda --version`"] {
        assert!(content.contains(probe), "README must document the {probe} probe");
    }
    assert!(
        content.contains("以唯讀方式執行"),
        "README probes must be read-only"
    );
    assert!(
        content.contains("PATH 中找不到"),
        "README must distinguish a tool absent from PATH"
    );
    assert!(
        content.contains("找到但無法啟動"),
        "README must distinguish a tool that cannot start"
    );
    assert!(
        content.contains("啟動錯誤") && content.contains("不得虛構結束狀態"),
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
        check_section.contains("Check 與 Build 都執行四個專案驗證規則模組"),
        "Check and Build must be documented as running the same four project rule modules"
    );
    assert!(
        check_section.contains("這些 Warning 不會改變成功狀態"),
        "missing or unstartable optional tools must not change Check status"
    );
    assert!(
        check_section.contains("Build 不執行 PATH 工具檢查"),
        "Build must be documented as skipping the optional PATH probe"
    );
}

/// The command reference keeps the init fallback and diagnostic severity contract accurate.
#[test]
fn commands_document_git_abort_and_limited_deno_init() {
    let path = docs_dir().join("commands.md");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
    assert!(
        content.contains("`wda.init.tool-unavailable` Error，並在寫入前中止")
            && content.contains("git` 不在 PATH，或雖已找到但無法啟動"),
        "missing Git must stop init before writing"
    );
    assert!(
        content.contains("Deno 不在 PATH 或無法啟動時，`wda init` 會以 WDA Minimal 有限初始化。此路徑不進行線上 Design System 解析或相依套件安裝")
            && content.contains("不進行線上 Design System 解析或相依套件安裝"),
        "missing Deno must allow only limited WDA Minimal init"
    );
    assert!(
        content.contains("`wda.tool.fault` | Error 或 Warning"),
        "command reference must document wda.tool.fault severity by impact"
    );
    assert!(
        content.contains("`wda.toolchain.git-missing` | Warning")
            && content.contains("`wda.toolchain.deno-missing` | Warning"),
        "command reference must list both toolchain warning codes"
    );
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
    ("02-authoring-dependencies-and-toolchain.md", &["006", "011", "013"]),
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
        let node = marked_yaml::parse_yaml(0, yaml_slice)
            .expect("front matter must be valid YAML");
        let map = node.as_mapping().expect("front matter must be a mapping");
        let absorbs_node = map
            .get("absorbs")
            .unwrap_or_else(|| panic!("{} must have 'absorbs' node", filename));
        let seq = absorbs_node
            .as_sequence()
            .unwrap_or_else(|| panic!("{} 'absorbs' must be a YAML sequence", filename));

        let actual_absorbs: Vec<String> = seq
            .iter()
            .map(|item| item.as_scalar().map(|s| s.as_str().to_string()).unwrap_or_default())
            .collect();

        assert_eq!(
            actual_absorbs,
            *expected_absorbs,
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
    let adr04_content = fs::read_to_string(adr_dir.join("04-design-systems-tokens-and-neutrality.md"))
        .expect("ADR-04 must be readable");
    assert!(
        adr04_content.contains("\"008\""),
        "ADR-04 front matter must retain quoted leading-zero \"008\""
    );
}

/// Every replacement ADR preserves every absorbed status-history entry
/// verbatim in source-ADR order, followed by the consolidation entry.
#[test]
fn adr_history_preserved_in_mapped_order() {
    let adr_dir = docs_dir().join("adr");

    for (filename, _) in THEMATIC_ADR_MAPPING {
        let path = adr_dir.join(filename);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));

        let history_start = content.find("## 狀態沿革").expect("must contain ## 狀態沿革");
        let history_section = &content[history_start..];
        let history_lines: Vec<&str> = history_section
            .lines()
            .map(|l| l.trim_end_matches('\r'))
            .filter(|l| l.starts_with("- "))
            .collect();

        // The final line must be the consolidation entry
        let last_entry = history_lines
            .last()
            .expect("must have at least one history entry");
        assert_eq!(
            *last_entry,
            "- 2026-09-21：本記錄由既有原子決策收斂而成，來源見 front matter absorbs。",
            "{} final history entry must be the consolidation notice",
            filename
        );
        // It must NOT contain literal ADR-<digits>
        assert!(
            !has_adr_number(last_entry),
            "{} consolidation entry must not contain literal ADR-<digits>",
            filename
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
        "docs/templates/generated-project/naming.md",
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

    // Also check docs/architecture.md up to "## 已廢止的 ADR 編號"
    let arch_content = fs::read_to_string(root.join("docs/architecture.md"))
        .expect("architecture.md must be readable");
    let active_arch = arch_content
        .split("## 已廢止的 ADR 編號")
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

