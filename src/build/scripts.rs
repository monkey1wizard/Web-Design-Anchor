//! Native script type-check and module bundling for the build pipeline.
//!
//! Type checking runs the embedded, SRI-verified TypeScript native tool from the local tool
//! cache (`crate::tools`). Bundling uses the Rolldown Rust API in browser/ESM mode and hands
//! back in-memory files; nothing is written until the caller's staging writer runs. Neither
//! step uses the network, a package manager, Node.js, Bun, or Deno.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use oxc::allocator::Allocator;
use oxc::ast::ast::{
    CallExpression, ExportAllDeclaration, ExportFromDeclaration, Expression, ImportDeclaration,
    ImportExpression,
};
use oxc::ast_visit::{walk, Visit};
use oxc::parser::Parser;
use oxc::span::SourceType;
use rolldown::{
    AttachDebugInfo, Bundler, BundlerOptions, ExperimentalOptions, InputItem, LegalComments,
    OutputFormat, Platform, ResolveOptions, TsConfig,
};

use crate::codes;
use crate::diagnostics::{Diagnostic, Location, Severity, ToolFault};

/// Upper bound for one type-check run; a slower tool counts as a tool fault.
const TYPE_CHECK_TIMEOUT: Duration = Duration::from_secs(30);

/// Largest captured tool output kept for diagnostics.
const MAX_TOOL_OUTPUT: u64 = 8 * 1024 * 1024;

/// Browser library surface fixed for every project; project settings cannot change it.
const CHECK_LIB: &[&str] = &["dom", "dom.iterable", "dom.asynciterable", "esnext"];

/// Conditions the bundler uses for package `exports`. The type checker receives the same
/// browser condition through `customConditions` so both agree on which file a package maps to.
const BROWSER_CONDITIONS: &[&str] = &["browser", "import", "module", "default"];

/// Node.js built-in module names that can never run in a browser.
const NODE_BUILTINS: &[&str] = &[
    "assert",
    "async_hooks",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "constants",
    "crypto",
    "dgram",
    "diagnostics_channel",
    "dns",
    "domain",
    "events",
    "fs",
    "http",
    "http2",
    "https",
    "inspector",
    "module",
    "net",
    "os",
    "path",
    "perf_hooks",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "stream",
    "string_decoder",
    "sys",
    "timers",
    "tls",
    "trace_events",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "wasi",
    "worker_threads",
    "zlib",
];

/// Failure of a script stage.
///
/// `Project` carries diagnostics about the user's project (exit 1); `Fault` is a tool or
/// environment failure (exit 2).
#[derive(Debug)]
pub(crate) enum ScriptFailure {
    Project(Vec<Diagnostic>),
    Fault(ToolFault),
}

/// One bundled output file, addressed relative to `dist/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BundledFile {
    pub(crate) path: PathBuf,
    pub(crate) contents: Vec<u8>,
}

fn fault(message: impl Into<String>) -> ScriptFailure {
    ScriptFailure::Fault(ToolFault::new(message, None::<PathBuf>))
}

fn forward(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Strips the Windows verbatim prefix so paths compare and print the same way everywhere.
fn plain_path(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => PathBuf::from(rest),
        _ => path.to_path_buf(),
    }
}

/// Spells `target` relative to `from`, for tools that print paths relative to the process
/// working directory.
fn relative_spelling(from: &Path, target: &Path) -> Option<String> {
    let from_plain = plain_path(from);
    let target_plain = plain_path(target);
    let from: Vec<_> = from_plain.components().collect();
    let target: Vec<_> = target_plain.components().collect();
    let common = from
        .iter()
        .zip(target.iter())
        .take_while(|(a, b)| a == b)
        .count();
    if common == 0 {
        return None;
    }
    let mut parts: Vec<String> = vec!["..".to_string(); from.len() - common];
    parts.extend(
        target[common..]
            .iter()
            .map(|component| component.as_os_str().to_string_lossy().into_owned()),
    );
    let joined = parts.join("/");
    (!joined.is_empty()).then_some(joined)
}

/// Removes every spelling of the given host directories from tool output so a diagnostic
/// never leaks the machine's absolute paths.
fn scrub_host_paths(text: &str, bases: &[&Path]) -> String {
    let mut spellings = BTreeSet::new();
    for base in bases {
        let mut candidates = vec![base.to_path_buf(), plain_path(base)];
        if let Ok(canonical) = fs::canonicalize(base) {
            candidates.push(plain_path(&canonical));
            candidates.push(canonical);
        }
        for candidate in candidates {
            let native = candidate.to_string_lossy().into_owned();
            if native.is_empty() {
                continue;
            }
            spellings.insert(native.replace('\\', "/"));
            spellings.insert(native);
        }
        if let Ok(cwd) = std::env::current_dir() {
            if let Some(relative) = relative_spelling(&cwd, base) {
                spellings.insert(relative);
            }
        }
    }
    let mut ordered: Vec<String> = spellings.into_iter().collect();
    ordered.sort_by_key(|spelling| std::cmp::Reverse(spelling.len()));
    let mut scrubbed = text.to_string();
    for spelling in ordered {
        for separator in ["/", "\\"] {
            scrubbed = scrubbed.replace(&format!("{spelling}{separator}"), "");
        }
        scrubbed = scrubbed.replace(&spelling, ".");
    }
    scrubbed
}

/// Finds the project-relative `scripts/**/*.ts` entries referenced by expanded pages.
pub(crate) fn discover_script_entries(
    root: &Path,
    expanded_pages: &std::collections::BTreeMap<PathBuf, String>,
) -> Vec<PathBuf> {
    let mut entries = BTreeSet::new();
    for (page, content) in expanded_pages {
        for reference in crate::build::layout::extract_references_from_html(content, page) {
            if !matches!(
                crate::build::layout::classify_reference(&reference.raw_url),
                crate::build::layout::ReferenceKind::FilePath(_)
            ) {
                continue;
            }
            let candidate =
                crate::build::layout::resolve_reference_target(&reference.raw_url, page);
            let normalized = forward(&candidate);
            if normalized.starts_with("scripts/")
                && normalized.ends_with(".ts")
                && root.join(&candidate).is_file()
            {
                entries.insert(candidate);
            }
        }
    }
    entries.into_iter().collect()
}

// ---------------------------------------------------------------------------
// Type checking
// ---------------------------------------------------------------------------

/// Type-checks the script entries with the cached native TypeScript tool.
///
/// With no entry nothing is looked up or launched. A missing cache is a project error that
/// points at `wda tools fetch`; the build itself never downloads anything.
pub(crate) fn type_check_entries(root: &Path, entries: &[PathBuf]) -> Result<(), ScriptFailure> {
    if entries.is_empty() {
        return Ok(());
    }
    let tool = match crate::tools::cache_status() {
        Ok(Some(path)) => path,
        Ok(None) => {
            return Err(ScriptFailure::Project(vec![Diagnostic::new(
                codes::BUILD_TOOL_UNAVAILABLE,
                Severity::Error,
                None,
                "The TypeScript tool cache is missing, so the scripts cannot be type-checked",
                "Run `wda tools fetch` once while online, then run the build again; the build never downloads tools",
            )]));
        }
        Err(reason) => {
            return Err(ScriptFailure::Project(vec![Diagnostic::new(
                codes::BUILD_TOOL_UNAVAILABLE,
                Severity::Error,
                None,
                format!("The TypeScript tool cache cannot be located: {reason}"),
                "Run `wda tools fetch` on a supported platform, then run the build again",
            )]));
        }
    };
    run_type_check(root, entries, &tool, TYPE_CHECK_TIMEOUT)
}

fn tsconfig_content(root: &Path, entries: &[PathBuf]) -> String {
    let files: Vec<String> = entries
        .iter()
        .map(|entry| forward(&root.join(entry)))
        .collect();
    let config = serde_json::json!({
        "compilerOptions": {
            "strict": true,
            "noEmit": true,
            "isolatedModules": true,
            "verbatimModuleSyntax": true,
            "skipLibCheck": true,
            "target": "esnext",
            "module": "esnext",
            "moduleResolution": "bundler",
            "customConditions": ["browser"],
            "lib": CHECK_LIB,
            "types": [],
            "allowImportingTsExtensions": true,
            "resolveJsonModule": true,
            "forceConsistentCasingInFileNames": true,
        },
        "files": files,
        "include": [],
    });
    serde_json::to_string_pretty(&config).unwrap_or_default()
}

fn run_type_check(
    root: &Path,
    entries: &[PathBuf],
    tool: &Path,
    timeout: Duration,
) -> Result<(), ScriptFailure> {
    let root = std::path::absolute(root).unwrap_or_else(|_| root.to_path_buf());
    let config_dir = tempfile::Builder::new()
        .prefix("wda-tsc-")
        .tempdir()
        .map_err(|error| {
            fault(format!(
                "Failed to create the type-check configuration: {error}"
            ))
        })?;
    let config_path = config_dir.path().join("tsconfig.json");
    fs::write(&config_path, tsconfig_content(&root, entries)).map_err(|error| {
        fault(format!(
            "Failed to write the type-check configuration: {error}"
        ))
    })?;

    let args = [
        OsString::from("--project"),
        config_path.clone().into_os_string(),
        OsString::from("--pretty"),
        OsString::from("false"),
    ];
    let run = run_with_timeout(tool, &args, &root, timeout)
        .map_err(|reason| fault(format!("The TypeScript tool {reason}")))?;
    if run.success {
        return Ok(());
    }

    let diagnostics = parse_tsc_output(&run.output, &root, config_dir.path());
    if diagnostics.is_empty() {
        let shown = scrub_host_paths(run.output.trim(), &[&root, config_dir.path()]);
        return Err(fault(format!(
            "The TypeScript tool failed ({}) without a readable diagnostic: {shown}",
            run.status_text
        )));
    }
    Err(ScriptFailure::Project(diagnostics))
}

/// Converts `path(line,col): error TSxxxx: message` lines into diagnostics. Lines that do not
/// name a source location are not source errors and yield nothing.
fn parse_tsc_output(output: &str, root: &Path, config_dir: &Path) -> Vec<Diagnostic> {
    struct Pending {
        path: String,
        line: usize,
        column: usize,
        message: String,
    }
    let mut parsed: Vec<Pending> = Vec::new();
    for raw in output.lines() {
        let line = raw.trim_end();
        if let Some(index) = line.find("): error TS") {
            let head = &line[..index];
            let Some(open) = head.rfind('(') else {
                continue;
            };
            let mut numbers = head[open + 1..].split(',');
            let (Some(Ok(row)), Some(Ok(column))) = (
                numbers.next().map(|n| n.trim().parse::<usize>()),
                numbers.next().map(|n| n.trim().parse::<usize>()),
            ) else {
                continue;
            };
            parsed.push(Pending {
                path: head[..open].to_string(),
                line: row,
                column,
                message: line[index + 3..].to_string(),
            });
        } else if line.starts_with(char::is_whitespace) && !line.trim().is_empty() {
            if let Some(last) = parsed.last_mut() {
                last.message.push(' ');
                last.message.push_str(line.trim());
            }
        }
    }

    parsed
        .into_iter()
        .map(|item| {
            let path = normalize_diagnostic_path(&item.path, root);
            let message = scrub_host_paths(&item.message, &[root, config_dir]);
            let location = Some(Location::new(
                PathBuf::from(path),
                Some(item.line),
                Some(item.column),
            ));
            if let Some(specifier) = unresolved_module_specifier(&message) {
                let empty = BTreeSet::new();
                match classify_specifier(&specifier, "", &empty) {
                    SpecifierClass::NodeOnly => {
                        return Diagnostic::new(
                            codes::BUILD_REFERENCE_UNRESOLVABLE,
                            Severity::Error,
                            location,
                            format!("The Node.js-only module '{specifier}' cannot run in a browser"),
                            "Remove the import or use a browser-compatible package",
                        );
                    }
                    SpecifierClass::Remote => {
                        return Diagnostic::new(
                            codes::BUILD_REFERENCE_UNRESOLVABLE,
                            Severity::Error,
                            location,
                            format!("The remote module '{specifier}' is not bundled"),
                            "Install the package with your package manager and import it by package name",
                        );
                    }
                    SpecifierClass::Unresolved => {
                        return Diagnostic::new(
                            codes::BUILD_DEPENDENCY_UNLOCKED,
                            Severity::Error,
                            location,
                            format!("The package '{specifier}' is not installed in node_modules"),
                            "Install the project's packages with your package manager, then run the build again; the build never installs packages",
                        );
                    }
                    SpecifierClass::Local => {}
                }
            }
            Diagnostic::new(
                codes::BUILD_TYPE_ERROR,
                Severity::Error,
                location,
                format!("Type check failed: {message}"),
                "Fix the TypeScript diagnostics and run the build again",
            )
        })
        .collect()
}

/// The module name from a "cannot find module" message (TS2307, or TS2882 for a side-effect
/// import, or TS2591 for a `node:` name), when the module is not a relative path.
fn unresolved_module_specifier(message: &str) -> Option<String> {
    let rest = message
        .split("error TS2307:")
        .nth(1)
        .or_else(|| message.split("error TS2882:").nth(1))
        .or_else(|| {
            message
                .split("error TS2591: Cannot find name")
                .nth(1)
                .filter(|rest| rest.trim_start().starts_with("'node:"))
        })?;
    let start = rest.find('\'')? + 1;
    let end = start + rest[start..].find('\'')?;
    let specifier = &rest[start..end];
    (!specifier.is_empty() && !specifier.starts_with('.') && !specifier.starts_with('/'))
        .then(|| specifier.to_string())
}

fn normalize_diagnostic_path(raw: &str, root: &Path) -> String {
    let unified = raw.replace('\\', "/");
    let root_forward = forward(root);
    let trimmed = unified
        .strip_prefix(&format!("{root_forward}/"))
        .unwrap_or(&unified);
    trimmed.strip_prefix("./").unwrap_or(trimmed).to_string()
}

struct ToolRun {
    success: bool,
    status_text: String,
    output: String,
}

/// Runs a tool with captured output and a hard time limit.
fn run_with_timeout(
    program: &Path,
    args: &[OsString],
    cwd: &Path,
    timeout: Duration,
) -> Result<ToolRun, String> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not be started: {error}"))?;

    let reader = |stream: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(stream) = stream {
                let _ = stream.take(MAX_TOOL_OUTPUT).read_to_end(&mut bytes);
            }
            String::from_utf8_lossy(&bytes).into_owned()
        })
    };
    let stdout = reader(
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
    );
    let stderr = reader(
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
    );

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    // Reader threads are detached: a grandchild may still hold the pipes open.
                    return Err(format!("timed out after {} s", timeout.as_secs().max(1)));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(format!("could not be awaited: {error}")),
        }
    };
    let mut output = stdout.join().unwrap_or_default();
    let errors = stderr.join().unwrap_or_default();
    if !errors.is_empty() {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(&errors);
    }
    Ok(ToolRun {
        success: status.success(),
        status_text: status.to_string(),
        output,
    })
}

// ---------------------------------------------------------------------------
// Bundling
// ---------------------------------------------------------------------------

/// Bundles the script entries with Rolldown into in-memory files addressed relative to
/// `dist/`. The engine's exit status is never trusted alone: every resolved module and every
/// import left in the emitted code is checked before anything is returned.
pub(crate) fn bundle_entries(
    root: &Path,
    entries: &[PathBuf],
) -> Result<Vec<BundledFile>, ScriptFailure> {
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    let root_canonical = plain_path(
        &fs::canonicalize(root)
            .map_err(|error| fault(format!("Failed to resolve the project root: {error}")))?,
    );

    let mut expected = Vec::new();
    let mut inputs = Vec::new();
    for entry in entries {
        let Some(mapped) = crate::build::layout::map_source_to_dist(entry) else {
            return Err(ScriptFailure::Project(vec![Diagnostic::new(
                codes::BUILD_REFERENCE_UNRESOLVABLE,
                Severity::Error,
                Some(Location::path_only(entry)),
                format!(
                    "Script entry '{}' is outside the supported scripts mapping",
                    forward(entry)
                ),
                "Reference a TypeScript entry below scripts/",
            )]));
        };
        let output_rel = forward(mapped.strip_prefix("dist").unwrap_or(&mapped));
        let name = output_rel
            .strip_suffix(".js")
            .unwrap_or(&output_rel)
            .to_string();
        inputs.push(InputItem {
            name: Some(name),
            import: format!("./{}", forward(entry)),
        });
        expected.push(output_rel);
    }

    let options = bundler_options(&root_canonical, inputs);
    let (output, scanned) = run_bundler(options).map_err(|message| {
        ScriptFailure::Project(vec![Diagnostic::new(
            codes::BUILD_REFERENCE_UNRESOLVABLE,
            Severity::Error,
            None,
            format!(
                "Module bundling failed: {}",
                scrub_host_paths(&message, &[root, &root_canonical])
            ),
            "Install the missing packages with your package manager, remove unsupported imports, and run the build again",
        )])
    })?;
    check_bundle(&root_canonical, &expected, output, scanned)
}

fn bundler_options(root: &Path, inputs: Vec<InputItem>) -> BundlerOptions {
    BundlerOptions {
        input: Some(inputs),
        cwd: Some(root.to_path_buf()),
        dir: Some("out".into()),
        platform: Some(Platform::Browser),
        format: Some(OutputFormat::Esm),
        entry_filenames: Some("[name].js".to_string().into()),
        chunk_filenames: Some("scripts/chunks/[name]-[hash].js".to_string().into()),
        legal_comments: Some(LegalComments::Inline),
        tsconfig: Some(TsConfig::Auto(false)),
        resolve: Some(ResolveOptions {
            condition_names: Some(BROWSER_CONDITIONS.iter().map(|c| c.to_string()).collect()),
            ..Default::default()
        }),
        experimental: Some(ExperimentalOptions {
            attach_debug_info: Some(AttachDebugInfo::None),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn run_bundler(
    options: BundlerOptions,
) -> Result<(rolldown::BundleOutput, Vec<rolldown_common::ModuleId>), String> {
    let attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|error| format!("the bundler runtime could not start: {error}"))?;
        runtime.block_on(async {
            let mut bundler = Bundler::new(options).map_err(|error| error.to_string())?;
            let result = bundler.generate().await.map_err(|error| error.to_string());
            let scanned: Vec<rolldown_common::ModuleId> = bundler
                .module_infos()
                .iter()
                .map(|entry| entry.value().id.clone())
                .collect();
            let _ = bundler.close().await;
            result.map(|output| (output, scanned))
        })
    }));
    match attempt {
        Ok(result) => result,
        Err(_) => Err("the bundler stopped unexpectedly".to_string()),
    }
}

/// Everything a module or an emitted import can be rejected for.
fn check_bundle(
    root: &Path,
    expected_entries: &[String],
    output: rolldown::BundleOutput,
    scanned: Vec<rolldown_common::ModuleId>,
) -> Result<Vec<BundledFile>, ScriptFailure> {
    let mut chunks = Vec::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    for item in &output.assets {
        match item {
            rolldown_common::Output::Chunk(chunk) => chunks.push(chunk.clone()),
            rolldown_common::Output::Asset(asset) => diagnostics.push(Diagnostic::new(
                codes::BUILD_REFERENCE_UNRESOLVABLE,
                Severity::Error,
                None,
                format!(
                    "The bundler emitted the unsupported asset '{}'; imports must resolve to script modules",
                    asset.filename
                ),
                "Reference static files from the page instead of importing them from a script",
            )),
        }
    }

    let mut chunk_names = BTreeSet::new();
    for chunk in &chunks {
        let name = chunk.filename.to_string();
        if !safe_output_name(&name) {
            return Err(fault(format!(
                "The bundler produced the unsafe output name '{name}'"
            )));
        }
        if !chunk_names.insert(name.clone()) {
            return Err(fault(format!("The bundler produced '{name}' twice")));
        }
    }

    for expected in expected_entries {
        let found = chunks
            .iter()
            .any(|chunk| chunk.is_entry && chunk.filename.as_str() == expected.as_str());
        if !found {
            return Err(fault(format!(
                "The bundler did not emit the entry '{expected}'"
            )));
        }
    }

    let mut seen = BTreeSet::new();
    let mut problems: Vec<(String, Diagnostic)> = Vec::new();
    // Every module the engine read is checked, including ones whose code was folded away
    // and therefore appears in no chunk.
    let mut module_ids: Vec<&rolldown_common::ModuleId> = scanned.iter().collect();
    for chunk in &chunks {
        module_ids.extend(chunk.modules.keys.iter());
        module_ids.extend(chunk.module_ids.iter());
    }
    let mut checked = BTreeSet::new();
    for id in module_ids {
        if !checked.insert(id.as_str().to_string()) {
            continue;
        }
        if let Some(problem) = check_module_id(root, id) {
            let key = problem.message.clone();
            if seen.insert(key.clone()) {
                problems.push((key, problem));
            }
        }
    }
    for chunk in &chunks {
        scan_chunk(
            chunk.filename.as_str(),
            &chunk.code,
            &chunk_names,
            &mut problems,
            &mut seen,
        )?;
    }
    problems.sort_by(|a, b| a.0.cmp(&b.0));
    diagnostics.extend(problems.into_iter().map(|(_, diagnostic)| diagnostic));
    if !diagnostics.is_empty() {
        return Err(ScriptFailure::Project(diagnostics));
    }

    let mut files: Vec<BundledFile> = chunks
        .iter()
        .map(|chunk| BundledFile {
            path: PathBuf::from(chunk.filename.as_str()),
            contents: chunk.code.clone().into_bytes(),
        })
        .collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn safe_output_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('\\')
        && Path::new(name)
            .components()
            .all(|component| match component {
                Component::Normal(part) => !part.to_string_lossy().starts_with('.'),
                _ => false,
            })
}

/// Validates one module the engine resolved: it must live inside the project and must not be a
/// secret path. Real paths are compared, so a link inside `node_modules` that stays within the
/// project (for example Bun's `.bun` store) is legal while any link that leaves it is rejected.
fn check_module_id(root: &Path, id: &rolldown_common::ModuleId) -> Option<Diagnostic> {
    let path = id.as_path()?;
    let canonical = match fs::canonicalize(path) {
        Ok(canonical) => plain_path(&canonical),
        Err(_) => {
            return Some(Diagnostic::new(
                codes::BUILD_REFERENCE_UNRESOLVABLE,
                Severity::Error,
                None,
                "A resolved module could not be read from disk",
                "Run the build from an unchanged project directory",
            ))
        }
    };
    let Ok(relative) = canonical.strip_prefix(root) else {
        return Some(Diagnostic::new(
            codes::BUILD_REFERENCE_UNRESOLVABLE,
            Severity::Error,
            None,
            "A script imports a module whose real path is outside the project root, for example through a link",
            "Keep imported modules and package links inside the project",
        ));
    };
    let relative_text = forward(relative);
    let in_packages = relative
        .components()
        .any(|component| component.as_os_str() == "node_modules");
    let secret = if in_packages {
        relative
            .components()
            .any(|component| component.as_os_str().to_string_lossy().starts_with(".env"))
            || relative
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with('.'))
    } else {
        crate::build::layout::is_secret_path(relative)
    };
    if secret {
        return Some(Diagnostic::new(
            codes::BUILD_SECRET_PATH_REFERENCED,
            Severity::Error,
            Some(Location::path_only(relative_text.clone())),
            format!(
                "A script imports the secret path '{relative_text}', which is forbidden in dist/"
            ),
            "Remove the import of the dotfile or .env file",
        ));
    }
    None
}

#[derive(Default)]
struct ModuleReferences {
    statics: Vec<String>,
    dynamics: Vec<String>,
    unanalyzable: usize,
    requires: usize,
}

impl<'a> Visit<'a> for ModuleReferences {
    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        self.statics.push(it.source.value.display().to_string());
        walk::walk_import_declaration(self, it);
    }

    fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
        self.statics.push(it.source.value.display().to_string());
        walk::walk_export_from_declaration(self, it);
    }

    fn visit_export_all_declaration(&mut self, it: &ExportAllDeclaration<'a>) {
        self.statics.push(it.source.value.display().to_string());
        walk::walk_export_all_declaration(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        match &it.source {
            Expression::StringLiteral(literal) => {
                self.dynamics.push(literal.value.display().to_string())
            }
            Expression::TemplateLiteral(template)
                if template.expressions.is_empty() && template.quasis.len() == 1 =>
            {
                match &template.quasis[0].value.cooked {
                    Some(cooked) => self.dynamics.push(cooked.display().to_string()),
                    None => self.unanalyzable += 1,
                }
            }
            _ => self.unanalyzable += 1,
        }
        walk::walk_import_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::Identifier(callee) = &it.callee {
            if callee.name == "require" || callee.name == "__require" {
                self.requires += 1;
            }
        }
        walk::walk_call_expression(self, it);
    }
}

/// Parses an emitted chunk and checks that each import is a relative path to another chunk of
/// the same output, that every dynamic import is a literal, and that no `require` survives.
fn scan_chunk(
    filename: &str,
    code: &str,
    chunk_names: &BTreeSet<String>,
    problems: &mut Vec<(String, Diagnostic)>,
    seen: &mut BTreeSet<String>,
) -> Result<(), ScriptFailure> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::mjs()).parse();
    if !parsed.diagnostics.is_empty() {
        return Err(fault(format!(
            "The bundler produced '{filename}' with invalid JavaScript"
        )));
    }
    let mut references = ModuleReferences::default();
    references.visit_program(&parsed.program);

    let mut push = |message: String, next: &str| {
        if seen.insert(message.clone()) {
            problems.push((
                message.clone(),
                Diagnostic::new(
                    codes::BUILD_REFERENCE_UNRESOLVABLE,
                    Severity::Error,
                    None,
                    message,
                    next,
                ),
            ));
        }
    };

    if references.unanalyzable > 0 {
        push(
            format!(
                "Output '{filename}' keeps a dynamic import whose target is not a string literal, so its module cannot be emitted"
            ),
            "Use import('./literal-path.ts') so every lazily loaded module can be bundled",
        );
    }
    if references.requires > 0 {
        push(
            format!("Output '{filename}' keeps a CommonJS require that cannot run in a browser"),
            "Import a browser-ready ES module build of the package instead",
        );
    }
    for specifier in references.statics.iter().chain(references.dynamics.iter()) {
        match classify_specifier(specifier, filename, chunk_names) {
            SpecifierClass::Local => {}
            SpecifierClass::NodeOnly => push(
                format!("A script imports the Node.js-only module '{specifier}', which cannot run in a browser"),
                "Remove the import or use a browser-compatible package",
            ),
            SpecifierClass::Remote => push(
                format!("A script imports the remote module '{specifier}'; remote modules are not bundled"),
                "Install the package with your package manager and import it by package name",
            ),
            SpecifierClass::Unresolved => push(
                format!("A script imports '{specifier}', which is not installed or not bundleable"),
                "Install the package with your package manager, then run the build again",
            ),
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum SpecifierClass {
    Local,
    NodeOnly,
    Remote,
    Unresolved,
}

fn classify_specifier(
    specifier: &str,
    chunk_filename: &str,
    chunk_names: &BTreeSet<String>,
) -> SpecifierClass {
    if specifier.starts_with("./") || specifier.starts_with("../") {
        let base = Path::new(chunk_filename).parent().unwrap_or(Path::new(""));
        let mut parts: Vec<String> = base
            .components()
            .filter_map(|c| match c {
                Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();
        for piece in specifier.split('/') {
            match piece {
                "" | "." => {}
                ".." => {
                    if parts.pop().is_none() {
                        return SpecifierClass::Unresolved;
                    }
                }
                other => parts.push(other.to_string()),
            }
        }
        return if chunk_names.contains(&parts.join("/")) {
            SpecifierClass::Local
        } else {
            SpecifierClass::Unresolved
        };
    }
    if specifier.contains("://") || specifier.starts_with("//") || specifier.starts_with("data:") {
        return SpecifierClass::Remote;
    }
    let bare = specifier.strip_prefix("node:");
    let name = bare.unwrap_or(specifier);
    let first = name.split('/').next().unwrap_or(name);
    if bare.is_some() || NODE_BUILTINS.contains(&first) {
        return SpecifierClass::NodeOnly;
    }
    SpecifierClass::Unresolved
}

/// Writes bundled files below `stage`, refusing to overwrite anything already staged.
pub(crate) fn write_bundle(stage: &Path, files: &[BundledFile]) -> Result<(), ToolFault> {
    for file in files {
        let destination = stage.join(&file.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                ToolFault::new(
                    format!(
                        "Failed to prepare output directory for '{}': {error}",
                        forward(&file.path)
                    ),
                    Some(parent.to_path_buf()),
                )
            })?;
        }
        let mut handle = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .map_err(|error| {
                ToolFault::new(
                    format!(
                        "Failed to create bundled script '{}': {error}",
                        forward(&file.path)
                    ),
                    Some(destination.clone()),
                )
            })?;
        handle.write_all(&file.contents).map_err(|error| {
            ToolFault::new(
                format!(
                    "Failed to write bundled script '{}': {error}",
                    forward(&file.path)
                ),
                Some(destination),
            )
        })?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Generic tool lookup
// ---------------------------------------------------------------------------

/// Resolves a tool by name from the system PATH.
///
/// Searches PATH for an executable matching `tool_name`.
/// Returns `Ok(path)` if found and runnable, or an Error [`Diagnostic`] carrying
/// `wda.build.tool-unavailable` if not found or if the executable cannot start.
pub fn resolve_tool(tool_name: &str) -> Result<PathBuf, Diagnostic> {
    resolve_tool_with_path(tool_name, std::env::var_os("PATH").as_deref())
}

/// Resolves a tool with an explicit PATH environment variable value.
pub(crate) fn resolve_tool_with_path(
    tool_name: &str,
    path_env: Option<&OsStr>,
) -> Result<PathBuf, Diagnostic> {
    let trimmed = tool_name.trim();
    if trimmed.is_empty() {
        return Err(Diagnostic::new(
            codes::BUILD_TOOL_UNAVAILABLE,
            Severity::Error,
            None,
            "Required tool name cannot be empty",
            "Specify a valid tool name",
        ));
    }

    let has_separator = trimmed.contains('/') || trimmed.contains('\\');
    if has_separator {
        let p = Path::new(trimmed);
        for name in candidate_names(p) {
            let candidate = PathBuf::from(name);
            if is_executable(&candidate) {
                return verify_can_start(trimmed, &candidate);
            }
        }
        return Err(Diagnostic::new(
            codes::BUILD_TOOL_UNAVAILABLE,
            Severity::Error,
            None,
            format!("Required tool '{trimmed}' was not found at specified path"),
            format!("Ensure '{trimmed}' exists and is executable"),
        ));
    }

    let search_dirs: Vec<PathBuf> = match path_env {
        Some(val) => std::env::split_paths(val).collect(),
        None => Vec::new(),
    };

    let candidates = candidate_names(Path::new(trimmed));

    for dir in search_dirs {
        for cand in &candidates {
            let candidate_path = dir.join(cand);
            if is_executable(&candidate_path) {
                return verify_can_start(trimmed, &candidate_path);
            }
        }
    }

    Err(Diagnostic::new(
        codes::BUILD_TOOL_UNAVAILABLE,
        Severity::Error,
        None,
        format!("Required tool '{trimmed}' was not found on PATH"),
        format!("Ensure '{trimmed}' is installed and available on PATH"),
    ))
}

#[cfg(windows)]
fn candidate_names(p: &Path) -> Vec<String> {
    let base = p.to_string_lossy().into_owned();
    if p.extension().is_some() {
        return vec![base];
    }

    let mut names = vec![base.clone()];
    let default_exts = [".exe", ".cmd", ".bat", ".com"];
    if let Some(pathext) = std::env::var_os("PATHEXT") {
        if let Some(s) = pathext.to_str() {
            for ext in s.split(';') {
                let trimmed = ext.trim();
                if !trimmed.is_empty() {
                    let with_dot = if trimmed.starts_with('.') {
                        trimmed.to_string()
                    } else {
                        format!(".{trimmed}")
                    };
                    let upper = format!("{base}{with_dot}");
                    let lower = format!("{base}{}", with_dot.to_lowercase());
                    if !names.contains(&upper) {
                        names.push(upper);
                    }
                    if !names.contains(&lower) {
                        names.push(lower);
                    }
                }
            }
        }
    } else {
        for ext in &default_exts {
            let cand = format!("{base}{ext}");
            if !names.contains(&cand) {
                names.push(cand);
            }
        }
    }
    names
}

#[cfg(not(windows))]
fn candidate_names(p: &Path) -> Vec<String> {
    vec![p.to_string_lossy().into_owned()]
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = path.metadata() {
            return meta.permissions().mode() & 0o111 != 0;
        }
        false
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn verify_can_start(tool_name: &str, path: &Path) -> Result<PathBuf, Diagnostic> {
    let mut cmd = Command::new(path);
    cmd.arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    match cmd.output() {
        Ok(output) => {
            let is_deno = tool_name == "deno" || tool_name == "deno.exe";
            if is_deno && !output.status.success() {
                Err(Diagnostic::new(
                    codes::BUILD_TOOL_UNAVAILABLE,
                    Severity::Error,
                    None,
                    format!(
                        "Required tool '{tool_name}' at '{}' failed to start (exit status: {})",
                        path.display(),
                        output.status
                    ),
                    format!("Ensure '{tool_name}' is properly installed and functional"),
                ))
            } else {
                Ok(path.to_path_buf())
            }
        }
        Err(err) => Err(Diagnostic::new(
            codes::BUILD_TOOL_UNAVAILABLE,
            Severity::Error,
            None,
            format!(
                "Required tool '{tool_name}' at '{}' could not be executed: {err}",
                path.display()
            ),
            format!("Ensure '{tool_name}' has execute permissions and can run"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_tool_impossible_name_yields_tool_unavailable_error() {
        let impossible_name = "impossible_tool_name_wda_test_nonexistent";
        let res = resolve_tool(impossible_name);
        assert!(res.is_err(), "impossible tool name must fail resolution");
        let diag = res.unwrap_err();
        assert_eq!(diag.code, codes::BUILD_TOOL_UNAVAILABLE);
        assert_eq!(diag.severity, Severity::Error);
        assert!(
            diag.message.contains(impossible_name),
            "diagnostic message must name the tool '{}', got: {}",
            impossible_name,
            diag.message
        );
        assert!(diag.location.is_none());
    }

    #[test]
    fn test_resolve_tool_empty_name_fails() {
        let res = resolve_tool("");
        assert!(res.is_err());
        let diag = res.unwrap_err();
        assert_eq!(diag.code, codes::BUILD_TOOL_UNAVAILABLE);
        assert_eq!(diag.severity, Severity::Error);
    }

    #[test]
    fn test_resolve_tool_with_custom_empty_path() {
        let res = resolve_tool_with_path("deno", None);
        assert!(res.is_err());
        let diag = res.unwrap_err();
        assert_eq!(diag.code, codes::BUILD_TOOL_UNAVAILABLE);
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("deno"));
    }

    #[test]
    fn test_resolve_tool_cannot_start_yields_tool_unavailable_error() {
        let temp_dir =
            std::env::temp_dir().join(format!("wda_test_cannot_start_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create an invalid/corrupted binary file that cannot be executed by the OS
        #[cfg(windows)]
        let tool_file = temp_dir.join("corrupted_tool.exe");
        #[cfg(not(windows))]
        let tool_file = temp_dir.join("corrupted_tool");

        std::fs::write(&tool_file, "not an executable binary").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&tool_file).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&tool_file, perms).unwrap();
        }

        let res = resolve_tool_with_path("corrupted_tool", Some(temp_dir.as_os_str()));
        assert!(res.is_err(), "non-startable tool must yield Error");
        let diag = res.unwrap_err();
        assert_eq!(diag.code, codes::BUILD_TOOL_UNAVAILABLE);
        assert_eq!(diag.severity, Severity::Error);
        assert!(
            diag.message.contains("corrupted_tool"),
            "error message must name the tool"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    fn scratch(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "wda_scripts_{label}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("scripts")).unwrap();
        root
    }

    /// Writes an executable stand-in for the type-check tool.
    fn fake_tool(dir: &Path, body: &str) -> PathBuf {
        #[cfg(windows)]
        {
            let path = dir.join("fake-tool.cmd");
            fs::write(&path, format!("@echo off\r\n{body}\r\n")).unwrap();
            path
        }
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = dir.join("fake-tool.sh");
            fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
            let mut permissions = fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).unwrap();
            path
        }
    }

    #[cfg(windows)]
    const SLEEP_BODY: &str = "ping -n 8 127.0.0.1 >nul";
    #[cfg(not(windows))]
    const SLEEP_BODY: &str = "sleep 8";
    #[cfg(windows)]
    const UNREADABLE_BODY: &str = "echo something broke\r\nexit /b 3";
    #[cfg(not(windows))]
    const UNREADABLE_BODY: &str = "echo something broke\nexit 3";
    #[cfg(windows)]
    const TYPE_ERROR_BODY: &str = "echo scripts\\main.ts(2,7): error TS2322: Type 'string' is not assignable to type 'number'.\r\necho   because of %CD%\r\nexit /b 2";
    #[cfg(not(windows))]
    const TYPE_ERROR_BODY: &str = "echo \"scripts/main.ts(2,7): error TS2322: Type 'string' is not assignable to type 'number'.\"\necho \"  because of $PWD\"\nexit 2";

    #[test]
    fn native_build_pipeline_type_check_without_entries_needs_no_tool() {
        let root = scratch("no_entries");
        assert!(type_check_entries(&root, &[]).is_ok());
        assert!(bundle_entries(&root, &[]).unwrap().is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_build_pipeline_type_errors_are_project_errors_with_clean_paths() {
        let root = scratch("type_error");
        let tool = fake_tool(&root, TYPE_ERROR_BODY);
        let result = run_type_check(
            &root,
            &[PathBuf::from("scripts/main.ts")],
            &tool,
            Duration::from_secs(20),
        );
        let Err(ScriptFailure::Project(diagnostics)) = result else {
            panic!("a located type error must be a project error: {result:?}");
        };
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, codes::BUILD_TYPE_ERROR);
        let location = diagnostics[0].location.as_ref().unwrap();
        assert_eq!(location.path, PathBuf::from("scripts/main.ts"));
        assert_eq!(location.line, Some(2));
        assert_eq!(location.column, Some(7));
        let root_text = root.to_string_lossy().replace('\\', "/");
        assert!(
            !diagnostics[0]
                .message
                .replace('\\', "/")
                .contains(&root_text),
            "host path leaked: {}",
            diagnostics[0].message
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_build_pipeline_unreadable_failure_and_timeout_are_tool_faults() {
        let root = scratch("faults");
        let entries = [PathBuf::from("scripts/main.ts")];

        let broken = fake_tool(&root, UNREADABLE_BODY);
        let result = run_type_check(&root, &entries, &broken, Duration::from_secs(20));
        assert!(
            matches!(&result, Err(ScriptFailure::Fault(fault)) if fault.message.contains("without a readable diagnostic")),
            "unparseable failure must be a tool fault: {result:?}"
        );

        let missing = root.join("no-such-tool");
        let result = run_type_check(&root, &entries, &missing, Duration::from_secs(20));
        assert!(
            matches!(&result, Err(ScriptFailure::Fault(fault)) if fault.message.contains("could not be started")),
            "launch failure must be a tool fault: {result:?}"
        );

        let slow = fake_tool(&root, SLEEP_BODY);
        let started = Instant::now();
        let result = run_type_check(&root, &entries, &slow, Duration::from_millis(300));
        assert!(
            matches!(&result, Err(ScriptFailure::Fault(fault)) if fault.message.contains("timed out")),
            "timeout must be a tool fault: {result:?}"
        );
        assert!(started.elapsed() < Duration::from_secs(6));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_build_pipeline_unresolved_modules_get_specific_diagnostics() {
        let root = scratch("ts2307");
        let line = |specifier: &str| {
            format!("scripts/main.ts(1,8): error TS2307: Cannot find module '{specifier}' or its corresponding type declarations.\n")
        };
        let code_for = |specifier: &str| parse_tsc_output(&line(specifier), &root, &root)[0].code;
        assert_eq!(code_for("lit"), codes::BUILD_DEPENDENCY_UNLOCKED);
        assert_eq!(code_for("@scope/pkg/sub"), codes::BUILD_DEPENDENCY_UNLOCKED);
        assert_eq!(code_for("node:fs"), codes::BUILD_REFERENCE_UNRESOLVABLE);
        assert_eq!(code_for("path"), codes::BUILD_REFERENCE_UNRESOLVABLE);
        assert_eq!(
            code_for("https://x.test/a.js"),
            codes::BUILD_REFERENCE_UNRESOLVABLE
        );
        assert_eq!(code_for("./missing.ts"), codes::BUILD_TYPE_ERROR);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_build_pipeline_tsc_output_without_location_yields_no_diagnostics() {
        let root = scratch("global_error");
        let parsed = parse_tsc_output("error TS5023: Unknown compiler option.\n", &root, &root);
        assert!(parsed.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_build_pipeline_tsconfig_fixes_the_strict_settings() {
        let root = scratch("tsconfig");
        let text = tsconfig_content(&root, &[PathBuf::from("scripts/main.ts")]);
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let options = &value["compilerOptions"];
        for key in [
            "strict",
            "noEmit",
            "isolatedModules",
            "verbatimModuleSyntax",
            "skipLibCheck",
        ] {
            assert_eq!(options[key], serde_json::Value::Bool(true), "{key}");
        }
        assert_eq!(options["moduleResolution"], "bundler");
        assert_eq!(options["customConditions"][0], "browser");
        assert!(options["lib"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l == "dom"));
        assert!(value.get("extends").is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_build_pipeline_specifier_classes() {
        let names: BTreeSet<String> = ["scripts/main.js", "scripts/chunks/lazy-abc.js"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        let class = |spec: &str| classify_specifier(spec, "scripts/main.js", &names);
        assert_eq!(class("./chunks/lazy-abc.js"), SpecifierClass::Local);
        assert_eq!(class("./chunks/missing.js"), SpecifierClass::Unresolved);
        assert_eq!(class("../../escape.js"), SpecifierClass::Unresolved);
        assert_eq!(class("node:fs"), SpecifierClass::NodeOnly);
        assert_eq!(class("fs/promises"), SpecifierClass::NodeOnly);
        assert_eq!(class("https://cdn.example/x.js"), SpecifierClass::Remote);
        assert_eq!(class("//cdn.example/x.js"), SpecifierClass::Remote);
        assert_eq!(class("preact"), SpecifierClass::Unresolved);
    }

    fn scan(code: &str) -> Vec<String> {
        let names: BTreeSet<String> = ["scripts/main.js"].iter().map(|n| n.to_string()).collect();
        let mut problems = Vec::new();
        let mut seen = BTreeSet::new();
        scan_chunk("scripts/main.js", code, &names, &mut problems, &mut seen).unwrap();
        problems.into_iter().map(|(message, _)| message).collect()
    }

    #[test]
    fn native_build_pipeline_output_scan_rejects_non_local_imports() {
        assert!(scan("import './main.js';\nexport const a = 1;\n").is_empty());
        let node = scan("import fs from 'node:fs';\nexport { fs };\n");
        assert!(node.iter().any(|m| m.contains("Node.js-only")), "{node:?}");
        let remote = scan("export * from 'https://cdn.example/x.js';\n");
        assert!(
            remote.iter().any(|m| m.contains("remote module")),
            "{remote:?}"
        );
        let bare = scan("export { a } from 'left-pad';\n");
        assert!(bare.iter().any(|m| m.contains("not installed")), "{bare:?}");
        let dynamic = scan("const n = './x';\nimport(n + '.js');\n");
        assert!(
            dynamic.iter().any(|m| m.contains("not a string literal")),
            "{dynamic:?}"
        );
        let literal = scan("import('./main.js');\nimport(`./main.js`);\n");
        assert!(literal.is_empty(), "{literal:?}");
        let required = scan("const x = __require('fs');\n");
        assert!(
            required.iter().any(|m| m.contains("require")),
            "{required:?}"
        );
    }

    #[test]
    fn native_build_pipeline_module_ids_reject_escapes_and_secret_paths() {
        let parent = scratch("ids");
        let root = parent.join("project");
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::create_dir_all(root.join("node_modules/.bun/pkg@1.0.0/node_modules/pkg")).unwrap();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::write(root.join("scripts/ok.ts"), "export {};").unwrap();
        fs::write(root.join("scripts/.hidden.ts"), "export {};").unwrap();
        fs::write(root.join("node_modules/pkg/index.js"), "export {};").unwrap();
        fs::write(root.join("node_modules/pkg/.env"), "SECRET=1").unwrap();
        fs::write(
            root.join("node_modules/.bun/pkg@1.0.0/node_modules/pkg/index.js"),
            "export {};",
        )
        .unwrap();
        fs::write(parent.join("outside.ts"), "export {};").unwrap();
        let canonical = plain_path(&fs::canonicalize(&root).unwrap());
        let check = |path: PathBuf| {
            let id = rolldown_common::ModuleId::new(path.to_string_lossy().into_owned());
            check_module_id(&canonical, &id)
        };
        assert!(check(root.join("scripts/ok.ts")).is_none());
        assert!(check(root.join("node_modules/pkg/index.js")).is_none());
        assert!(
            check(root.join("node_modules/.bun/pkg@1.0.0/node_modules/pkg/index.js")).is_none()
        );
        let hidden = check(root.join("scripts/.hidden.ts")).unwrap();
        assert_eq!(hidden.code, codes::BUILD_SECRET_PATH_REFERENCED);
        let env = check(root.join("node_modules/pkg/.env")).unwrap();
        assert_eq!(env.code, codes::BUILD_SECRET_PATH_REFERENCED);
        let outside = check(parent.join("outside.ts")).unwrap();
        assert_eq!(outside.code, codes::BUILD_REFERENCE_UNRESOLVABLE);
        assert!(outside.message.contains("outside the project root"));
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn native_build_pipeline_bundles_shared_circular_and_lazy_modules_locally() {
        let root = scratch("bundle");
        fs::create_dir_all(root.join("node_modules/lib/dist")).unwrap();
        fs::write(
            root.join("node_modules/lib/package.json"),
            r#"{"name":"lib","version":"1.0.0","type":"module","exports":{".":{"browser":"./dist/browser.js","default":"./dist/node.js"},"./extra":"./dist/extra.js"}}"#,
        )
        .unwrap();
        fs::write(
            root.join("node_modules/lib/dist/browser.js"),
            "export const flavor = 'browser-flavor';\n",
        )
        .unwrap();
        fs::write(
            root.join("node_modules/lib/dist/node.js"),
            "export const flavor = 'node-flavor';\n",
        )
        .unwrap();
        fs::write(
            root.join("node_modules/lib/dist/extra.js"),
            "export const extra = 'extra';\n",
        )
        .unwrap();
        fs::write(
            root.join("scripts/shared.ts"),
            "import { flavor } from 'lib';\nimport { extra } from 'lib/extra';\nimport { ping } from './ping.ts';\nexport const shared: string = flavor + extra + ping();\n",
        )
        .unwrap();
        fs::write(
            root.join("scripts/ping.ts"),
            "import { shared } from './shared.ts';\nexport function ping(): string { return typeof shared; }\n",
        )
        .unwrap();
        fs::write(
            root.join("scripts/lazy.ts"),
            "export const lazy = 'lazy';\n",
        )
        .unwrap();
        fs::write(
            root.join("scripts/a.ts"),
            "import { shared } from './shared.ts';\nconsole.log(shared);\nimport('./lazy.ts').then((m) => console.log(m.lazy));\n",
        )
        .unwrap();
        fs::write(
            root.join("scripts/b.ts"),
            "import { shared } from './shared.ts';\nconsole.log(shared + '!');\n",
        )
        .unwrap();
        let entries = [PathBuf::from("scripts/a.ts"), PathBuf::from("scripts/b.ts")];
        let first = bundle_entries(&root, &entries).expect("bundle must succeed");
        let second = bundle_entries(&root, &entries).expect("bundle must succeed again");
        assert_eq!(first, second, "output must be deterministic");
        let names: Vec<String> = first.iter().map(|f| forward(&f.path)).collect();
        assert!(names.contains(&"scripts/a.js".to_string()), "{names:?}");
        assert!(names.contains(&"scripts/b.js".to_string()), "{names:?}");
        assert!(
            names.iter().any(|n| n.starts_with("scripts/chunks/")),
            "shared and lazy chunks expected: {names:?}"
        );
        let all: String = first
            .iter()
            .map(|f| String::from_utf8(f.contents.clone()).unwrap())
            .collect();
        assert!(all.contains("browser-flavor"));
        assert!(!all.contains("node-flavor"));
        let root_text = root.to_string_lossy().replace('\\', "/");
        assert!(
            !all.replace('\\', "/").contains(&root_text),
            "host path leaked"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_build_pipeline_bundle_rejects_unanalyzable_dynamic_and_missing_package() {
        let root = scratch("reject");
        fs::write(
            root.join("scripts/dynamic.ts"),
            "export function load(name: string) {\n  return import(name);\n}\n",
        )
        .unwrap();
        let result = bundle_entries(&root, &[PathBuf::from("scripts/dynamic.ts")]);
        let Err(ScriptFailure::Project(diagnostics)) = result else {
            panic!("dynamic import must be rejected: {result:?}");
        };
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("not a string literal")),
            "{diagnostics:?}"
        );

        fs::write(
            root.join("scripts/missing.ts"),
            "import { x } from 'not-installed-anywhere';\nconsole.log(x);\n",
        )
        .unwrap();
        let result = bundle_entries(&root, &[PathBuf::from("scripts/missing.ts")]);
        assert!(
            matches!(result, Err(ScriptFailure::Project(_))),
            "a missing package must be a project error: {result:?}"
        );

        fs::write(
            root.join("scripts/folded.ts"),
            "const name = './gone';\nimport(name + '.js');\nexport {};\n",
        )
        .unwrap();
        let result = bundle_entries(&root, &[PathBuf::from("scripts/folded.ts")]);
        let Err(ScriptFailure::Project(diagnostics)) = result else {
            panic!("an unresolvable dynamic import must be rejected: {result:?}");
        };
        let root_name = root.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            diagnostics.iter().all(|d| !d.message.contains(&root_name)),
            "engine message leaked the host path: {diagnostics:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_build_pipeline_write_bundle_refuses_to_overwrite() {
        let stage = scratch("write");
        let files = vec![BundledFile {
            path: PathBuf::from("scripts/a.js"),
            contents: b"1".to_vec(),
        }];
        write_bundle(&stage, &files).unwrap();
        assert_eq!(fs::read(stage.join("scripts/a.js")).unwrap(), b"1");
        assert!(write_bundle(&stage, &files).is_err());
        let _ = fs::remove_dir_all(&stage);
    }
}
