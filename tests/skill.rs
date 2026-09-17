use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use marked_yaml::Node;

const ALLOWED_KEYS: &[&str] = &["name", "compatibility", "metadata", "description", "license"];

/// The references named in `docs/architecture.md` §2.11, each carrying
/// its own load trigger.
const REQUIRED_REFERENCES: &[&str] = &[
    "install.md",
    "intake.md",
    "assets.md",
    "reversibility.md",
    "reporting.md",
    "troubleshooting.md",
    "iteration.md",
];

/// L2 forbidden-content scan (`docs/architecture.md` §2.11): the only fenced
/// code block language tags allowed once a block exceeds 5 lines. An
/// untagged fence does not count as `text`.
const ALLOWED_LONG_FENCE_LANGS: &[&str] = &["text", "bash"];

/// A fenced block longer than this many content lines is subject to the
/// `ALLOWED_LONG_FENCE_LANGS` whitelist (`docs/architecture.md` §2.11).
const MAX_UNRESTRICTED_FENCE_LINES: usize = 5;

/// Normative marker set for `metadata.language: zh-TW` (`docs/architecture.md`
/// section 2.11, "regulatory sentences carrying 必須／不得／只能 must each have
/// an authority pointer in the same paragraph").
const NORMATIVE_MARKERS_ZH_TW: &[&str] = &["必須", "不得", "只能"];

/// Normative marker set for `metadata.language: en`. The reference section
/// names only the Chinese markers; this is the English equivalent selected
/// via `metadata.language` rather than scanning Chinese unconditionally.
const NORMATIVE_MARKERS_EN: &[&str] = &["must", "must not", "never", "only"];

/// Known external skill/tool names. §2.11 prohibits naming a specific
/// external skill in `SKILL.md` or `references/**`, because a skill name is
/// a moving target and naming one ties WDA to a single vendor's ecosystem.
/// This list is the oracle for that prohibition; it is not derived from
/// anything scanned at test time. Matching is case-insensitive substring.
const EXTERNAL_SKILL_NAMES: &[&str] = &[
    "shadcn",
    "v0.dev",
    "Cursor Composer",
    "Windsurf",
    "bolt.new",
    "Lovable",
    "GitHub Copilot",
    "Figma Make",
];

/// Reads `metadata.language` from `SKILL.md`'s front matter, the field that
/// pins the Skill body's authoring language. Selects which normative marker
/// set the authority-pointer guardrail scans for.
fn skill_metadata_language() -> String {
    let path = skill_md_path();
    let content = fs::read_to_string(&path).expect("skills/wda/SKILL.md must be readable");
    let yaml_slice = extract_front_matter(&content);
    let node =
        marked_yaml::parse_yaml(0, yaml_slice).expect("SKILL.md front matter must be valid YAML");
    let map = node
        .as_mapping()
        .expect("SKILL.md front matter must be a YAML mapping");
    let metadata = map
        .iter()
        .find(|(k, _)| k.as_str() == "metadata")
        .map(|(_, v)| v)
        .expect("'metadata' key must be present");
    let metadata_map = metadata
        .as_mapping()
        .expect("'metadata' must be a YAML mapping");
    scalar_str(metadata_map.iter(), |k| k.as_str(), "language")
        .expect("'metadata.language' must be a scalar string")
        .to_string()
}

/// Returns the normative marker set for `language`, panicking on any value
/// other than the two `metadata.language` may hold (`zh-TW` or `en`).
fn normative_markers_for(language: &str) -> &'static [&'static str] {
    match language {
        "zh-TW" => NORMATIVE_MARKERS_ZH_TW,
        "en" => NORMATIVE_MARKERS_EN,
        other => panic!(
            "'metadata.language' value '{other}' has no normative marker set; expected 'zh-TW' or 'en'"
        ),
    }
}

/// True if `text` carries one of the three authority-pointer shapes pinned
/// by `docs/architecture.md` section 2.11: a `docs/`-rooted path, a section
/// mark (`§`) followed by a digit, or `ADR-` followed by a digit.
fn has_authority_pointer(text: &str) -> bool {
    if text.contains("docs/") {
        return true;
    }
    if text.match_indices('§').any(|(idx, m)| {
        text[idx + m.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
    }) {
        return true;
    }
    if text.match_indices("ADR-").any(|(idx, m)| {
        text[idx + m.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
    }) {
        return true;
    }
    false
}

/// Splits `content` into blank-line-delimited paragraphs, pairing each with
/// its starting line number (1-indexed) for assertion messages.
fn paragraphs_with_line_numbers(content: &str) -> Vec<(usize, String)> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut start_line = 0usize;

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        if line.trim().is_empty() {
            if !current.is_empty() {
                result.push((start_line, std::mem::take(&mut current)));
            }
        } else {
            if current.is_empty() {
                start_line = line_no;
            } else {
                current.push('\n');
            }
            current.push_str(line);
        }
    }
    if !current.is_empty() {
        result.push((start_line, current));
    }

    result
}

/// Returns `SKILL.md` plus every file under `skills/wda/references/`, the
/// full set of files the L2 guardrails scan.
fn scanned_files() -> Vec<PathBuf> {
    let mut files = vec![skill_md_path()];
    let references_dir = skill_dir().join("references");
    let mut reference_files: Vec<PathBuf> = fs::read_dir(&references_dir)
        .expect("skills/wda/references must be readable")
        .map(|entry| entry.expect("directory entry must be readable").path())
        .filter(|p| p.is_file())
        .collect();
    reference_files.sort();
    files.extend(reference_files);
    files
}

fn skill_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("skills/wda")
}

fn skill_md_path() -> PathBuf {
    skill_dir().join("SKILL.md")
}

/// Extracts the YAML slice between the opening and closing `---` delimiters
/// of a `SKILL.md` front matter block.
fn extract_front_matter(content: &str) -> &str {
    // Split on '\n' alone, not `.lines()`: on a CRLF file `.lines()` strips
    // the trailing '\r' from each element, which would then under-count
    // every line's byte length by one and misplace `slice_end` below.
    let lines: Vec<&str> = content.split('\n').collect();
    assert_eq!(
        lines.first().map(|l| l.trim_end()),
        Some("---"),
        "SKILL.md must open with a '---' front matter delimiter"
    );

    let close_idx = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| line.trim_end() == "---")
        .map(|(idx, _)| idx)
        .expect("SKILL.md front matter must be terminated with a '---' delimiter");

    let slice_start = content.find('\n').map(|i| i + 1).unwrap_or(content.len());
    let slice_end = lines[..close_idx]
        .iter()
        .map(|l| l.len() + 1)
        .sum::<usize>();
    &content[slice_start..slice_end]
}

/// Finds `key` in `entries` (an iterator of `(scalar-key, value-node)` pairs,
/// where the key exposes `as_str`) and returns its value as a string,
/// panicking if the value is not a scalar.
fn scalar_str<'a, K: 'a>(
    entries: impl Iterator<Item = (&'a K, &'a Node)>,
    as_str: impl Fn(&'a K) -> &'a str,
    key: &str,
) -> Option<&'a str> {
    entries
        .filter(|(k, _)| as_str(k) == key)
        .map(|(_, v)| match v {
            Node::Scalar(s) => s.as_str(),
            _ => panic!("front matter key '{key}' must be a scalar"),
        })
        .next()
}

#[test]
fn skill_frontmatter_conforms_to_agent_skills_specification() {
    let path = skill_md_path();
    let content = fs::read_to_string(&path).expect("skills/wda/SKILL.md must be readable");
    let yaml_slice = extract_front_matter(&content);
    let node =
        marked_yaml::parse_yaml(0, yaml_slice).expect("SKILL.md front matter must be valid YAML");
    let map = node
        .as_mapping()
        .expect("SKILL.md front matter must be a YAML mapping");

    // L1: every key is one of the five allowed by the Agent Skills specification
    // (pinned via ADR-022, named as the source of truth by Agent Plugins 1.0.0 §Skills).
    for (k, _) in map.iter() {
        let key = k.as_str();
        assert!(
            ALLOWED_KEYS.contains(&key),
            "front matter key '{key}' is not one of the allowed keys {ALLOWED_KEYS:?}"
        );
    }

    // required keys present
    assert!(
        map.iter().any(|(k, _)| k.as_str() == "name"),
        "front matter must contain 'name'"
    );
    assert!(
        map.iter().any(|(k, _)| k.as_str() == "description"),
        "front matter must contain 'description'"
    );
    assert!(
        map.iter().any(|(k, _)| k.as_str() == "metadata"),
        "front matter must contain 'metadata'"
    );

    // `allowed-tools` must be absent per the §2.11 allowlist.
    assert!(
        !map.iter().any(|(k, _)| k.as_str() == "allowed-tools"),
        "front matter must not contain 'allowed-tools'"
    );

    let metadata = map
        .iter()
        .find(|(k, _)| k.as_str() == "metadata")
        .map(|(_, v)| v)
        .expect("'metadata' key must be present");
    let metadata_map = metadata
        .as_mapping()
        .expect("'metadata' must be a YAML mapping");

    assert!(
        metadata_map.iter().any(|(k, _)| k.as_str() == "language"),
        "'metadata.language' must be present"
    );

    let version = scalar_str(metadata_map.iter(), |k| k.as_str(), "version")
        .expect("'metadata.version' must be a scalar string");
    assert_eq!(
        version,
        env!("CARGO_PKG_VERSION"),
        "'metadata.version' must equal the crate version"
    );

    // every metadata value must be a string
    for (k, v) in metadata_map.iter() {
        assert!(
            matches!(v, Node::Scalar(_)),
            "metadata value for key '{}' must be a string",
            k.as_str()
        );
    }

    // `name` equals "wda" and equals the parent directory name of SKILL.md.
    let name = scalar_str(map.iter(), |k| k.as_str(), "name")
        .expect("'name' must be a scalar string");
    assert_eq!(name, "wda", "'name' must equal 'wda'");
    let parent_dir_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .expect("SKILL.md must have a parent directory name");
    assert_eq!(
        name, parent_dir_name,
        "'name' must equal the parent directory name of SKILL.md"
    );

    // `description` is non-empty and at most 1024 characters.
    let description = scalar_str(map.iter(), |k| k.as_str(), "description")
        .expect("'description' must be a scalar string");
    assert!(!description.is_empty(), "'description' must be non-empty");
    assert!(
        description.chars().count() <= 1024,
        "'description' must be at most 1024 characters"
    );

    // `compatibility` is required, bounded, and locked to the crate version.
    let compatibility = scalar_str(map.iter(), |k| k.as_str(), "compatibility")
        .expect("'compatibility' must be a scalar string");
    let len = compatibility.chars().count();
    assert!(
        (1..=500).contains(&len),
        "'compatibility' must be 1 to 500 characters"
    );
    assert_eq!(
        compatibility,
        format!("wda-binary=={}", env!("CARGO_PKG_VERSION")),
        "'compatibility' must equal the crate version"
    );

    let license = scalar_str(map.iter(), |k| k.as_str(), "license")
        .expect("'license' must be a scalar string");
    assert_eq!(
        license,
        env!("CARGO_PKG_LICENSE"),
        "'license' must equal the crate license"
    );
}

/// L2: `skills/wda/` carries no `scripts/` or `assets/` directory, and
/// `skills/wda/references/` holds exactly the named files (ADR-022,
/// `docs/architecture.md` §2.11).
#[test]
fn skill_directory_structure_matches_allowlist() {
    let skill_dir = skill_dir();

    assert!(
        !skill_dir.join("scripts").exists(),
        "skills/wda/scripts must not exist"
    );
    assert!(
        !skill_dir.join("assets").exists(),
        "skills/wda/assets must not exist"
    );

    let references_dir = skill_dir.join("references");
    let entries = fs::read_dir(&references_dir)
        .expect("skills/wda/references must be readable")
        .map(|entry| {
            entry
                .expect("directory entry must be readable")
                .file_name()
                .to_str()
                .expect("reference file name must be valid UTF-8")
                .to_owned()
        })
        .collect::<BTreeSet<_>>();

    let expected = REQUIRED_REFERENCES
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        entries, expected,
        "skills/wda/references/ must hold exactly the named files"
    );
}

/// L2: no fenced code block longer than `MAX_UNRESTRICTED_FENCE_LINES`
/// content lines may carry a language tag outside `ALLOWED_LONG_FENCE_LANGS`
/// (`docs/architecture.md` §2.11). An untagged fence does not count as
/// `text`. This is the intersection of §2.11's two phrasings ("zero
/// `html`/`json`/`css`/`toml` blocks over 5 lines" and "only `text` and
/// `bash` allowed"): scanning only the four named tags would let an
/// over-length `ts` or `js` block pass, and TypeScript/JavaScript are part
/// of Standard Web, so such a block is exactly the shape of a second
/// generator.
#[test]
fn skill_text_has_no_forbidden_long_fenced_blocks() {
    for path in scanned_files() {
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));

        let mut open_lang: Option<String> = None;
        let mut open_line_no = 0usize;
        let mut body_line_count = 0usize;

        for (idx, line) in content.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("```") {
                match &open_lang {
                    None => {
                        // Opening fence; the language tag is whatever
                        // follows the backticks, trimmed. Empty means
                        // untagged, which does not count as `text`.
                        open_lang = Some(rest.trim().to_string());
                        open_line_no = line_no;
                        body_line_count = 0;
                    }
                    Some(lang) => {
                        // Closing fence.
                        assert!(
                            body_line_count <= MAX_UNRESTRICTED_FENCE_LINES
                                || ALLOWED_LONG_FENCE_LANGS.contains(&lang.as_str()),
                            "{}:{} fenced block tagged '{}' has {} lines, over the \
                             {}-line limit for non-text/bash blocks (§2.11)",
                            path.display(),
                            open_line_no,
                            lang,
                            body_line_count,
                            MAX_UNRESTRICTED_FENCE_LINES
                        );
                        open_lang = None;
                    }
                }
            } else if open_lang.is_some() {
                body_line_count += 1;
            }
        }

        assert!(
            open_lang.is_none(),
            "{}:{} has an unterminated fenced code block",
            path.display(),
            open_line_no
        );
    }
}

/// L2: `SKILL.md` and `references/**` must not name a specific external
/// skill (`docs/architecture.md` §2.11 "不得依賴外部 skill"). `EXTERNAL_SKILL_NAMES`
/// is the pinned oracle; matching is case-insensitive substring.
#[test]
fn skill_text_names_no_external_skill() {
    for path in scanned_files() {
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
        let lower = content.to_lowercase();

        for name in EXTERNAL_SKILL_NAMES {
            assert!(
                !lower.contains(&name.to_lowercase()),
                "{} names the external skill '{}', forbidden by §2.11",
                path.display(),
                name
            );
        }
    }
}

/// L2 Core-string duplication guardrail (`docs/architecture.md` §2.11): no
/// diagnostic code literal from `src/codes.rs` — the only declaration site,
/// per that module's header — may appear in the scanned Skill text. A pasted
/// code literal is exactly the kind of Core-string duplication the Skill
/// layer must not carry; the Skill routes to `wda check`/`wda build` output
/// instead of restating what a diagnostic code means. This does not scan
/// `schemas/wda.schema.json` property names (`name`, `version`,
/// `wdaVersion`): the front matter is required to carry `name`, and the
/// version-comparison text is required to name `metadata.version` and
/// `wdaVersion`, so scanning those property names would fail on text this
/// plan itself demands. The schema half of §2.11 is
/// covered by the `json` fenced-block ban asserted above, since schema text
/// can only be copied in as a `json` block.
#[test]
fn skill_text_has_no_diagnostic_code_literals() {
    for path in scanned_files() {
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));

        for code in wda_core::codes::ALL_CODES {
            assert!(
                !content.contains(code),
                "{} contains diagnostic code literal '{}', which must be declared \
                 only in src/codes.rs (§2.11 Core-string duplication guardrail)",
                path.display(),
                code
            );
        }
    }
}

/// L2 size limit (`docs/architecture.md` §2.11): `SKILL.md` must not exceed
/// this many lines. The count is `lines()` over the whole file, frontmatter
/// included, with blank lines counted — §2.11 names a line ceiling but
/// defines no further counting method, and the frontmatter-inclusive count
/// is the stricter reading.
const MAX_SKILL_MD_LINES: usize = 500;

/// L2 review-flag threshold in lines (`docs/architecture.md` §2.11): at or
/// above this line count, a review flag is printed without failing the
/// test.
const REVIEW_FLAG_LINE_THRESHOLD: usize = 300;

/// L2 review-flag threshold in estimated tokens (`docs/architecture.md`
/// §2.11): at or above this proxy-estimated token count, a review flag is
/// printed without failing the test.
const REVIEW_FLAG_TOKEN_THRESHOLD: usize = 5000;

/// Over-estimating proxy for token count, used only for the `SKILL.md` size
/// review flag. §2.11 names a token limit but no tokenizer, so this test
/// cannot compute an exact token count; it uses a proxy that is guaranteed
/// to over-estimate rather than under-estimate, so the flag never misses a
/// file that a real tokenizer would flag. ASCII characters divide by 3
/// (a real BPE tokenizer typically packs more than 3 ASCII characters per
/// token, so dividing by 3 over-counts tokens for ASCII text); non-ASCII
/// characters (the CJK case this Skill's zh-TW body is written in) multiply
/// by 2 (a real tokenizer often merges multiple CJK characters per token,
/// and dividing bytes by 4 — the common English-text heuristic — would
/// under-count CJK, where one character is 3 UTF-8 bytes and frequently one
/// or more tokens on its own).
fn estimate_tokens_over(content: &str) -> usize {
    let mut ascii_count = 0usize;
    let mut non_ascii_count = 0usize;
    for c in content.chars() {
        if c.is_ascii() {
            ascii_count += 1;
        } else {
            non_ascii_count += 1;
        }
    }
    ascii_count / 3 + non_ascii_count * 2
}

/// L2 size test (`docs/architecture.md` §2.11): `SKILL.md` must be at most
/// `MAX_SKILL_MD_LINES` lines, counting the whole file including the
/// frontmatter with blank lines counted. At `REVIEW_FLAG_LINE_THRESHOLD`
/// lines or more, or when `estimate_tokens_over` exceeds
/// `REVIEW_FLAG_TOKEN_THRESHOLD`, this prints a named review flag via
/// `eprintln!` without failing. `cargo test` captures the stdout and stderr
/// of passing tests, so a bare `cargo test` run never shows this flag; it is
/// visible only under `cargo test --test skill -- --nocapture`, which makes
/// the L2 size review a command the pipeline audit step runs on purpose,
/// not a signal CI surfaces on its own.
#[test]
fn skill_md_size_is_within_limit() {
    let path = skill_md_path();
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));

    let line_count = content.lines().count();
    let estimated_tokens = estimate_tokens_over(&content);

    if line_count >= REVIEW_FLAG_LINE_THRESHOLD || estimated_tokens > REVIEW_FLAG_TOKEN_THRESHOLD {
        eprintln!(
            "SKILL_MD_SIZE_REVIEW_FLAG: {} has {} lines (review threshold {}) and an \
             estimated {} tokens (review threshold {}); review for trimming before it \
             reaches the {}-line hard limit (§2.11)",
            path.display(),
            line_count,
            REVIEW_FLAG_LINE_THRESHOLD,
            estimated_tokens,
            REVIEW_FLAG_TOKEN_THRESHOLD,
            MAX_SKILL_MD_LINES
        );
    }

    assert!(
        line_count <= MAX_SKILL_MD_LINES,
        "{} has {} lines, over the {}-line limit (§2.11)",
        path.display(),
        line_count,
        MAX_SKILL_MD_LINES
    );
}

/// Path to the Skill's `install.md` routing reference.
fn install_md_path() -> PathBuf {
    skill_dir().join("references/install.md")
}

/// Extracts the content lines of every fenced code block tagged `bash` in
/// `content`, concatenated in order. Used to scan `install.md` for
/// imperative install/download command tokens without flagging identical
/// text that appears only in prose (e.g. this file's own description of
/// what it must not contain).
fn bash_fence_bodies(content: &str) -> String {
    let mut result = String::new();
    let mut in_bash_block = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("```") {
            if in_bash_block {
                in_bash_block = false;
            } else if rest.trim() == "bash" {
                in_bash_block = true;
            }
            continue;
        }
        if in_bash_block {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

/// Install/download command tokens forbidden inside `install.md`'s `bash`
/// fenced blocks. `install.md` routes to `docs/install.md` (not yet written;
/// owned by the release-packaging plan) for the actual install procedure
/// rather than carrying an imperative install step itself; this list is the
/// oracle for "imperative install step" and is not derived from anything
/// scanned at test time. `wda --version` is deliberately absent from this
/// list: that is the detection call `install.md` demonstrates, not an
/// install/download command. Leaving the cross-file duplication scan
/// against the eventual `docs/install.md` content to whichever plan lands
/// that file, which already owns it.
const INSTALL_COMMAND_TOKENS: &[&str] = &[
    "curl",
    "wget",
    "cargo install",
    "Invoke-WebRequest",
    "winget",
    "scoop",
    "brew",
];

/// L2: `install.md` must point to `docs/install.md` for the actual install
/// procedure, and must carry no imperative install step of its own, checked
/// as the absence of `INSTALL_COMMAND_TOKENS` inside its `bash` fenced
/// blocks.
#[test]
fn install_md_points_to_docs_and_carries_no_install_step() {
    let path = install_md_path();
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));

    assert!(
        content.contains("docs/install.md"),
        "{} must point to 'docs/install.md' for the install procedure",
        path.display()
    );

    let bash_bodies = bash_fence_bodies(&content);
    for token in INSTALL_COMMAND_TOKENS {
        assert!(
            !bash_bodies.contains(token),
            "{}'s bash fenced block(s) contain install/download command \
             token '{}'; install.md must route to docs/install.md instead \
             of carrying an imperative install step",
            path.display(),
            token
        );
    }
}

/// L2: every paragraph in the scanned files that carries a normative marker
/// (selected by `metadata.language`; §2.11 names only the Chinese markers)
/// must carry an authority pointer in that same paragraph. A pointer is a
/// `docs/`-rooted path, `§<digits>`, or `ADR-<digits>` (`docs/architecture.md`
/// §2.11).
#[test]
fn skill_normative_paragraphs_carry_authority_pointer() {
    let language = skill_metadata_language();
    let markers = normative_markers_for(&language);

    for path in scanned_files() {
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));

        for (line_no, paragraph) in paragraphs_with_line_numbers(&content) {
            let matched_marker = markers.iter().find(|m| paragraph.contains(*m));
            if let Some(marker) = matched_marker {
                assert!(
                    has_authority_pointer(&paragraph),
                    "{}:{} paragraph contains normative marker '{}' but no authority \
                     pointer (`docs/...`, `§<digits>`, or `ADR-<digits>`) in the same \
                     paragraph:\n{}",
                    path.display(),
                    line_no,
                    marker,
                    paragraph
                );
            }
        }
    }
}

/// The exact count of L3 deterministic case fixtures under
/// `tests/fixtures/skill/cases/` (`docs/architecture.md` §2.11): the three
/// version-comparison branches, the exit `0` with-Warning process-contract
/// case, and the `wda`-not-on-PATH install case.
const L3_CASE_COUNT: usize = 19;

/// The three fixed section headings every L3 case fixture under
/// `tests/fixtures/skill/cases/` must carry, in the order the case template
/// establishes them.
const L3_CASE_HEADINGS: &[&str] = &[
    "## Host Input",
    "## Expected AI Behavior",
    "## Observable Pass Signal",
];

/// Minimum `- ` list item count required under `## should-trigger` in
/// `tests/fixtures/skill/prompts.md` (`docs/architecture.md` §2.11's L3
/// undertriggering requirement).
const MIN_SHOULD_TRIGGER_ITEMS: usize = 20;

/// Minimum `- ` list item count required under `## should-not-trigger` in
/// `tests/fixtures/skill/prompts.md`.
const MIN_SHOULD_NOT_TRIGGER_ITEMS: usize = 10;

fn skill_fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/skill")
}

fn l3_cases_dir() -> PathBuf {
    skill_fixtures_dir().join("cases")
}

fn l3_prompts_path() -> PathBuf {
    skill_fixtures_dir().join("prompts.md")
}

/// Returns the `- `-prefixed list items found directly under the H2 heading
/// `heading` in `content`, stopping at the next `## ` heading or end of
/// file. Panics if `heading` is not present.
fn list_items_under_heading<'a>(content: &'a str, heading: &str) -> Vec<&'a str> {
    let mut lines = content.lines();
    let found = lines.by_ref().any(|line| line.trim_end() == heading);
    assert!(
        found,
        "expected heading '{heading}' not found in prompts fixture"
    );

    lines
        .take_while(|line| !line.starts_with("## "))
        .filter_map(|line| line.strip_prefix("- "))
        .collect()
}

/// Fixture well-formedness (`docs/architecture.md` §2.11): CI has no AI host,
/// so this asserts only the L3 fixture files are structurally well-formed,
/// never their L3 outcomes. Verifies `tests/fixtures/skill/cases/` holds
/// exactly `L3_CASE_COUNT` files, each carrying `L3_CASE_HEADINGS`, and that
/// `tests/fixtures/skill/prompts.md` carries the `should-trigger` /
/// `should-not-trigger` H2 headings with at least their minimum `- ` list
/// item counts.
#[test]
fn skill_l3_fixtures_are_well_formed() {
    let cases_dir = l3_cases_dir();
    let case_files: Vec<PathBuf> = fs::read_dir(&cases_dir)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", cases_dir.display()))
        .map(|entry| entry.expect("directory entry must be readable").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();

    assert_eq!(
        case_files.len(),
        L3_CASE_COUNT,
        "{} must hold exactly {} case fixture files, found {}: {:?}",
        cases_dir.display(),
        L3_CASE_COUNT,
        case_files.len(),
        case_files
    );

    for path in &case_files {
        let content = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
        for heading in L3_CASE_HEADINGS {
            assert!(
                content.lines().any(|line| line.trim_end() == *heading),
                "{} must carry the heading '{}'",
                path.display(),
                heading
            );
        }
    }

    let prompts_path = l3_prompts_path();
    let prompts_content = fs::read_to_string(&prompts_path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", prompts_path.display()));

    let should_trigger = list_items_under_heading(&prompts_content, "## should-trigger");
    assert!(
        should_trigger.len() >= MIN_SHOULD_TRIGGER_ITEMS,
        "{} '## should-trigger' must carry at least {} '- ' items, found {}",
        prompts_path.display(),
        MIN_SHOULD_TRIGGER_ITEMS,
        should_trigger.len()
    );

    let should_not_trigger = list_items_under_heading(&prompts_content, "## should-not-trigger");
    assert!(
        should_not_trigger.len() >= MIN_SHOULD_NOT_TRIGGER_ITEMS,
        "{} '## should-not-trigger' must carry at least {} '- ' items, found {}",
        prompts_path.display(),
        MIN_SHOULD_NOT_TRIGGER_ITEMS,
        should_not_trigger.len()
    );
}
