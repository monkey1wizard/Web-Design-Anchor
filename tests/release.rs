use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

fn plugin_manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugin.json")
}

#[test]
fn plugin_manifest_has_exact_shape() {
    let path = plugin_manifest_path();
    let content = fs::read_to_string(&path).expect("plugin.json must be readable");
    let value: serde_json::Value =
        serde_json::from_str(&content).expect("plugin.json must be valid JSON");
    let object = value
        .as_object()
        .expect("plugin.json must contain a JSON object");

    let expected_keys: BTreeSet<&str> = ["$schema", "name"].into_iter().collect();
    let actual_keys: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected_keys.difference(&actual_keys).copied().collect();
    let extra: Vec<_> = actual_keys.difference(&expected_keys).copied().collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "plugin.json keys differ; missing: {missing:?}; extra: {extra:?}"
    );

    assert_eq!(
        object.get("$schema").and_then(serde_json::Value::as_str),
        Some("https://agent-plugins.org/schemas/1.0.0/plugin.schema.json")
    );
    assert_eq!(
        object.get("name").and_then(serde_json::Value::as_str),
        Some("wda")
    );
}

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("wda-release-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        let fixture = Self(path);
        fixture.git(&["init", "-q", "-b", "main"]);
        fs::write(
            fixture.gitconfig(),
            "[protocol]\n\tallow = never\n[protocol \"file\"]\n\tallow = always\n",
        )
        .unwrap();
        fixture.git(&["config", "user.name", "Release Test"]);
        fixture.git(&["config", "user.email", "release-test@example.invalid"]);
        fixture.write(
            "packaging/publish-public.sh",
            include_str!("../packaging/publish-public.sh"),
        );
        fixture.write(".dev/private.md", "private planning data\n");
        fixture.write(".github/workflows/publish-public.yml", "private workflow\n");
        fixture.write("public.txt", "public content\n");
        fixture.git(&["add", "-A"]);
        fixture.git(&["commit", "-qm", "fixture source"]);
        fixture
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn git(&self, args: &[&str]) -> Output {
        let output = self
            .git_command()
            .arg("-C")
            .arg(&self.0)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn git_command(&self) -> Command {
        isolated_git(&self.0)
    }

    fn gitconfig(&self) -> PathBuf {
        self.0.join(".git").join("test-global-config")
    }

    fn remote(&self) -> PathBuf {
        let remote = self.0.join("public.git");
        let output = self
            .git_command()
            .args(["init", "--bare", "-q"])
            .arg(&remote)
            .output()
            .unwrap();
        assert!(output.status.success());
        remote
    }

    fn script(&self, url: &str, args: &[&str]) -> Output {
        let _ = self
            .git_command()
            .arg("-C")
            .arg(&self.0)
            .args(["remote", "remove", "github"])
            .output();
        self.git(&["remote", "add", "github", url]);
        let mut command = bash_command();
        command
            .arg(self.0.join("packaging/publish-public.sh"))
            .args(args)
            .current_dir(&self.0)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.gitconfig())
            .env("HOME", &self.0)
            .env("XDG_CONFIG_HOME", self.0.join("xdg"))
            .env("GIT_TERMINAL_PROMPT", "0");
        command
            .output()
            .expect("bash must be available for release integration tests")
    }

    fn mapped_remote(&self, remote: &Path) {
        // The rewritten URL is supplied only to this fixture's Git configuration.
        let config = self.gitconfig();
        let target = format!("file://{}", remote.display().to_string().replace('\\', "/"));
        let urls = [
            "https://github.com/monkey1wizard/web-design-anchor",
            "https://github.com/monkey1wizard/web-design-anchor.git",
            "https://user:dummy-secret@github.com/monkey1wizard/web-design-anchor",
            "https://github.com.evil/monkey1wizard/web-design-anchor",
            "https://github.com/monkey1wizard/web-design-anchor-extra",
            "https://github.com/monkey1wizard/web-design-anchor?query=dummy-secret",
            "https://github.com/monkey1wizard/web-design-anchor#dummy-secret",
            "https://github.com/monkey1wizard/web-design-anchor-private",
            "https://gitlab.com/monkey1wizard/web-design-anchor",
        ];
        let mappings = urls
            .iter()
            .map(|url| format!("\tinsteadOf = {url}\n\tpushInsteadOf = {url}\n"))
            .collect::<String>();
        fs::write(&config, format!("[protocol]\n\tallow = never\n[protocol \"file\"]\n\tallow = always\n[url \"{target}\"]\n{mappings}")).unwrap();
    }
}

fn isolated_git(root: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            root.join(".git").join("test-global-config"),
        )
        .env("HOME", root)
        .env("XDG_CONFIG_HOME", root.join("xdg"))
        .env("GIT_TERMINAL_PROMPT", "0");
    command
}

fn bash_command() -> Command {
    #[cfg(windows)]
    {
        let candidates = [
            PathBuf::from(r"C:\Program Files\Git\bin\bash.exe"),
            PathBuf::from(r"C:\Program Files\Git\usr\bin\bash.exe"),
        ];
        let path = candidates
            .into_iter()
            .find(|path| path.is_file())
            .expect("Git Bash must be installed for release integration tests");
        Command::new(path)
    }
    #[cfg(not(windows))]
    {
        Command::new("bash")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const PUBLIC_HTTPS: &str = "https://github.com/monkey1wizard/web-design-anchor";

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_publish_workflow(root: &Path) -> String {
    fs::read_to_string(root.join(".github/workflows/publish-public.yml"))
        .expect("publish workflow must be readable")
        .replace("\r\n", "\n")
}

fn is_public_export(root: &Path) -> bool {
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

fn get_mapping_field<'a>(node: &'a marked_yaml::Node, key: &str) -> Option<&'a marked_yaml::Node> {
    if let marked_yaml::Node::Mapping(map) = node {
        map.iter().find(|(k, _)| k.as_str() == key).map(|(_, v)| v)
    } else {
        None
    }
}

fn get_scalar_field<'a>(node: &'a marked_yaml::Node, key: &str) -> Option<&'a str> {
    get_mapping_field(node, key).and_then(|n| {
        if let marked_yaml::Node::Scalar(s) = n {
            Some(s.as_str())
        } else {
            None
        }
    })
}

fn get_needs(job: &marked_yaml::Node) -> BTreeSet<String> {
    let mut needs = BTreeSet::new();
    if let Some(needs_node) = get_mapping_field(job, "needs") {
        match needs_node {
            marked_yaml::Node::Scalar(s) => {
                needs.insert(s.as_str().to_string());
            }
            marked_yaml::Node::Sequence(seq) => {
                for item in seq.iter() {
                    if let marked_yaml::Node::Scalar(s) = item {
                        needs.insert(s.as_str().to_string());
                    }
                }
            }
            _ => {}
        }
    }
    needs
}

fn get_steps<'a>(job: &'a marked_yaml::Node) -> Vec<&'a marked_yaml::Node> {
    if let Some(marked_yaml::Node::Sequence(steps)) = get_mapping_field(job, "steps") {
        steps.iter().collect()
    } else {
        Vec::new()
    }
}

fn assert_step_uses_checkout_without_credentials(steps: &[&marked_yaml::Node]) {
    let found = steps.iter().any(|step| {
        let uses = get_scalar_field(step, "uses").unwrap_or("");
        if uses.starts_with("actions/checkout@") {
            if let Some(with_node) = get_mapping_field(step, "with") {
                get_scalar_field(with_node, "persist-credentials") == Some("false")
            } else {
                false
            }
        } else {
            false
        }
    });
    assert!(
        found,
        "job steps must include actions/checkout with persist-credentials: false"
    );
}

fn assert_step_has_deno_version(steps: &[&marked_yaml::Node], version: &str) {
    let found = steps.iter().any(|step| {
        let uses = get_scalar_field(step, "uses").unwrap_or("");
        if uses.starts_with("denoland/setup-deno@") {
            if let Some(with_node) = get_mapping_field(step, "with") {
                get_scalar_field(with_node, "deno-version") == Some(version)
            } else {
                false
            }
        } else {
            false
        }
    });
    assert!(
        found,
        "job steps must include denoland/setup-deno with deno-version: {version}"
    );
}

fn assert_step_runs(steps: &[&marked_yaml::Node], command_substring: &str) {
    let found = steps.iter().any(|step| {
        let run = get_scalar_field(step, "run").unwrap_or("");
        run.contains(command_substring)
    });
    assert!(
        found,
        "job steps must run a step containing: {command_substring}"
    );
}

#[test]
fn controller_workflow_encodes_private_gates_and_required_checks() {
    let root = repo_root();
    if is_public_export(&root) {
        assert!(
            !root.join(".github/workflows/publish-public.yml").exists(),
            "public export must exclude publish-public.yml"
        );
        let release_content = fs::read_to_string(root.join(".github/workflows/release.yml"))
            .expect("public export must retain release.yml");
        let release_node = marked_yaml::parse_yaml(0, &release_content).unwrap();
        let jobs = get_mapping_field(&release_node, "jobs").expect("release.yml must have jobs");
        let build_job = get_mapping_field(jobs, "build").expect("release.yml must have build job");
        let build_if = get_scalar_field(build_job, "if").expect("build job must have if condition");
        assert!(
            build_if.contains("github.repository == 'monkey1wizard/Web-Design-Anchor'"),
            "build job in release.yml must guard repository: {build_if}"
        );
        return;
    }

    let workflow_path = root.join(".github/workflows/publish-public.yml");
    assert!(
        workflow_path.exists(),
        "private source must contain publish-public.yml"
    );
    let workflow_str = fs::read_to_string(&workflow_path).unwrap();
    let workflow_node =
        marked_yaml::parse_yaml(0, &workflow_str).expect("publish-public.yml must be valid YAML");

    // Concurrency model
    let concurrency = get_mapping_field(&workflow_node, "concurrency")
        .expect("publish-public.yml must specify concurrency");
    assert_eq!(
        get_scalar_field(concurrency, "group"),
        Some("publish-public-main")
    );
    assert_eq!(get_scalar_field(concurrency, "queue"), Some("max"));
    assert_eq!(
        get_scalar_field(concurrency, "cancel-in-progress"),
        Some("false")
    );

    // Jobs model
    let jobs = get_mapping_field(&workflow_node, "jobs").expect("workflow must have jobs");

    // validate job
    let validate_job =
        get_mapping_field(jobs, "validate").expect("workflow must have validate job");
    let validate_if =
        get_scalar_field(validate_job, "if").expect("validate job must have if condition");
    assert!(
        validate_if.contains("github.repository == 'monkey1wizard/web-design-anchor-private'"),
        "validate job must guard private repository: {validate_if}"
    );

    // test-linux job
    let test_linux =
        get_mapping_field(jobs, "test-linux").expect("workflow must have test-linux job");
    let linux_needs = get_needs(test_linux);
    assert!(
        linux_needs.contains("validate"),
        "test-linux must need validate"
    );
    let linux_steps = get_steps(test_linux);
    assert_step_uses_checkout_without_credentials(&linux_steps);
    assert_step_has_deno_version(&linux_steps, "v2.x");
    assert_step_runs(
        &linux_steps,
        "deno install --config tools/writing/deno.json",
    );
    assert_step_runs(&linux_steps, "git config --global user.name 'WDA CI'");
    assert_step_runs(
        &linux_steps,
        "git config --global user.email 'ci@web-design-anchor.invalid'",
    );
    let mut discrepancies = Vec::new();

    // Authoritative frozen writing check command
    let authoritative_check = "deno run --cached-only --frozen --allow-read --allow-env --allow-sys --config tools/writing/deno.json tools/writing/check.ts --required";
    let has_auth_check = linux_steps.iter().any(|step| {
        let run = get_scalar_field(step, "run").unwrap_or("");
        run.contains(authoritative_check)
    });
    if !has_auth_check {
        let actual_check = linux_steps
            .iter()
            .find_map(|step| {
                let run = get_scalar_field(step, "run").unwrap_or("");
                if run.contains("check.ts") {
                    Some(run)
                } else {
                    None
                }
            })
            .unwrap_or("<none>");
        discrepancies.push(format!(
            "test-linux writing check command differs from exact authoritative command:\n  expected: {}\n  observed: {}",
            authoritative_check, actual_check
        ));
    }
    assert_step_runs(&linux_steps, "cargo test --locked");

    // test-windows job
    let test_windows =
        get_mapping_field(jobs, "test-windows").expect("workflow must have test-windows job");
    let windows_needs = get_needs(test_windows);
    assert!(
        windows_needs.contains("validate"),
        "test-windows must need validate"
    );
    let windows_steps = get_steps(test_windows);
    assert_step_uses_checkout_without_credentials(&windows_steps);
    assert_step_has_deno_version(&windows_steps, "v2.x");
    assert_step_runs(&windows_steps, "git config --global user.name 'WDA CI'");
    assert_step_runs(
        &windows_steps,
        "git config --global user.email 'ci@web-design-anchor.invalid'",
    );
    assert_step_runs(&windows_steps, "cargo test --locked");

    // publish job
    let publish_job = get_mapping_field(jobs, "publish").expect("workflow must have publish job");
    let publish_needs = get_needs(publish_job);
    assert!(
        publish_needs.contains("validate"),
        "publish must need validate"
    );
    assert!(
        publish_needs.contains("test-linux"),
        "publish must need test-linux"
    );
    assert!(
        publish_needs.contains("test-windows"),
        "publish must need test-windows"
    );

    // Environment check: on GitHub Free with sole-maintainer, no private environments are supported.
    if let Some(env_val) = get_scalar_field(publish_job, "environment") {
        discrepancies.push(format!(
            "publish job declares 'environment: {}'; GitHub Free lacks private Environments",
            env_val
        ));
    } else if get_mapping_field(publish_job, "environment").is_some() {
        discrepancies.push(
            "publish job declares an 'environment' block; GitHub Free lacks private Environments"
                .to_string(),
        );
    }

    // release.yml check
    let release_path = root.join(".github/workflows/release.yml");
    let release_str = fs::read_to_string(&release_path).unwrap();
    let release_node =
        marked_yaml::parse_yaml(0, &release_str).expect("release.yml must be valid YAML");
    let rel_jobs = get_mapping_field(&release_node, "jobs").expect("release.yml must have jobs");
    if let marked_yaml::Node::Mapping(rel_jobs_map) = rel_jobs {
        for (job_name, job_val) in rel_jobs_map.iter() {
            let name = job_name.as_str();
            let needs = get_needs(job_val);
            if needs.is_empty() {
                assert_eq!(
                    name, "build",
                    "only build job may lack needs in release.yml"
                );
                let build_if =
                    get_scalar_field(job_val, "if").expect("build job must have if condition");
                assert!(
                    build_if.contains("github.repository == 'monkey1wizard/Web-Design-Anchor'"),
                    "build job in release.yml must guard public repository: {build_if}"
                );
            }
        }
    }

    assert!(
        !root.join(".gitlab-ci.yml").exists(),
        ".gitlab-ci.yml must be deleted"
    );

    assert!(
        discrepancies.is_empty(),
        "Controller workflow discrepancies found ({}):\n{}",
        discrepancies.len(),
        discrepancies.join("\n")
    );
}

#[test]
fn workflow_dispatch_dry_run_executes_without_tag_or_push_token() {
    let root = repo_root();
    if is_public_export(&root) {
        assert!(!root.join(".github/workflows/publish-public.yml").exists());
        return;
    }
    let workflow = read_publish_workflow(&root);
    let section = workflow
        .split("      - name: Verify public remote and residual scan without credentials\n")
        .nth(1)
        .unwrap();
    let run = section.split("        run: |\n").nth(1).unwrap();
    let mut script = String::new();
    for line in run.lines() {
        if !line.starts_with("          ") && !line.is_empty() {
            break;
        }
        script.push_str(line.strip_prefix("          ").unwrap_or(line));
        script.push('\n');
    }
    let fixture = Fixture::new();
    let remote = fixture.remote();
    fixture.mapped_remote(&remote);
    fixture.git(&["remote", "add", "github", PUBLIC_HTTPS]);
    let output = bash_command()
        .arg("-c")
        .arg(script)
        .current_dir(&fixture.0)
        .env("RELEASE_TAG", "")
        .env_remove("PUBLIC_REPO_PUSH_TOKEN")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", fixture.gitconfig())
        .env("HOME", &fixture.0)
        .env("XDG_CONFIG_HOME", fixture.0.join("xdg"))
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", output_text(&output));
    assert!(output_text(&output).contains("DRY RUN - nothing pushed"));
    assert!(
        !remote_has_refs(&remote),
        "dispatch dry run changed public refs"
    );
}

#[test]
fn workflow_validation_step_accepts_valid_tags_and_rejects_invalid() {
    let root = repo_root();
    if is_public_export(&root) {
        assert!(!root.join(".github/workflows/publish-public.yml").exists());
        return;
    }
    let workflow = read_publish_workflow(&root);
    let section = workflow
        .split("      - name: Validate repository, event, and complete tag\n")
        .nth(1)
        .expect("validation step must exist");
    let run = section
        .split("        run: |\n")
        .nth(1)
        .expect("validation run block must exist");
    let mut script = String::new();
    for line in run.lines() {
        if !line.starts_with("          ") && !line.is_empty() {
            break;
        }
        let line = line.strip_prefix("          ").unwrap_or(line);
        script.push_str(line);
        script.push('\n');
    }

    let run_val = |repo: &str, event: &str, ref_type: &str, ref_name: &str| -> (bool, String) {
        let fixture = Fixture::new();
        let out_file = fixture.0.join("github_output.txt");
        let output = bash_command()
            .arg("-c")
            .arg(&script)
            .current_dir(&fixture.0)
            .env("REPOSITORY", repo)
            .env("EVENT_NAME", event)
            .env("REF_TYPE", ref_type)
            .env("REF_NAME", ref_name)
            .env("GITHUB_OUTPUT", &out_file)
            .output()
            .expect("bash must execute validation step");
        let content = if out_file.exists() {
            fs::read_to_string(&out_file).unwrap_or_default()
        } else {
            String::new()
        };
        (output.status.success(), content)
    };

    // Valid push tags
    let (ok, out) = run_val(
        "monkey1wizard/web-design-anchor-private",
        "push",
        "tag",
        "v1.2.3",
    );
    assert!(ok, "v1.2.3 must be accepted");
    assert!(out.contains("is_tag=true\ntag=v1.2.3\n"));

    let (ok, out) = run_val(
        "monkey1wizard/web-design-anchor-private",
        "push",
        "tag",
        "v0.0.1-alpha.1",
    );
    assert!(ok, "v0.0.1-alpha.1 must be accepted");
    assert!(out.contains("is_tag=true\ntag=v0.0.1-alpha.1\n"));

    // Valid workflow dispatch
    let (ok, out) = run_val(
        "monkey1wizard/web-design-anchor-private",
        "workflow_dispatch",
        "",
        "",
    );
    assert!(ok, "workflow_dispatch must be accepted");
    assert!(out.contains("is_tag=false\ntag=\n"));

    // Invalid: wrong repository
    let (ok, _) = run_val("other/repo", "push", "tag", "v1.2.3");
    assert!(!ok, "wrong repository must be rejected");

    // Invalid: unsupported event
    let (ok, _) = run_val(
        "monkey1wizard/web-design-anchor-private",
        "pull_request",
        "tag",
        "v1.2.3",
    );
    assert!(!ok, "pull_request must be rejected");

    // Invalid: push branch instead of tag
    let (ok, _) = run_val(
        "monkey1wizard/web-design-anchor-private",
        "push",
        "branch",
        "main",
    );
    assert!(!ok, "push branch must be rejected");

    // Invalid tags
    for invalid in [
        "v1.2",
        "1.2.3",
        "v1.2.3+build",
        "v1.2.3/tag",
        "",
        "v1.2.3-",
        "refs/tags/v1.0.0",
    ] {
        let (ok, _) = run_val(
            "monkey1wizard/web-design-anchor-private",
            "push",
            "tag",
            invalid,
        );
        assert!(!ok, "tag '{invalid}' must be rejected");
    }
}

#[test]
fn exporter_snapshot_has_public_handoff_and_private_source_requires_controller() {
    let root = repo_root();
    if is_public_export(&root) {
        assert!(
            !root.join(".github/workflows/publish-public.yml").exists(),
            "public export must not have publish-public.yml"
        );
        return;
    }
    let workflow_rel = ".github/workflows/publish-public.yml";
    let fixture = Fixture::new();
    fixture.write("AGENTS.md", "private instructions\n");
    fixture.write("CLAUDE.md", "private instructions\n");
    fixture.write(".claude/rules/private.md", "private instructions\n");
    fixture.write(".github/instructions/rule.md", "private instructions\n");
    fixture.git(&["add", "-A"]);
    fixture.git(&["commit", "-qm", "private markers"]);
    let output = fixture.script(PUBLIC_HTTPS, &["--keep-snapshot", "public-export"]);
    assert!(output.status.success(), "{}", output_text(&output));
    let export = fixture.0.join("public-export");
    for marker in [
        "AGENTS.md",
        "CLAUDE.md",
        ".dev",
        ".claude",
        ".github/instructions",
        workflow_rel,
    ] {
        assert!(
            !export.join(marker).exists(),
            "private marker leaked: {marker}"
        );
    }
    assert!(
        is_public_export(&export),
        "clean export must be recognized as public export"
    );

    // Negative case: partial stripping (e.g. publish-public.yml missing, but AGENTS.md present)
    let partial = fixture.0.join("partial-export");
    fs::create_dir_all(&partial).unwrap();
    fs::write(partial.join("AGENTS.md"), "private").unwrap();
    assert!(
        !is_public_export(&partial),
        "partially stripped tree must not be accepted as public export"
    );

    // Private source check
    assert!(
        root.join(workflow_rel).exists(),
        "private source lacks controller workflow"
    );
    let markers = [
        "AGENTS.md",
        "CLAUDE.md",
        ".dev",
        ".claude",
        ".github/instructions",
    ];
    let source_has_private_marker = markers.iter().any(|path| root.join(path).exists());
    assert!(
        source_has_private_marker,
        "source tree was misclassified as public export"
    );
}

#[test]
fn workflow_askpass_helper_reads_runtime_token_and_cleans_up() {
    let root = repo_root();
    if is_public_export(&root) {
        assert!(!root.join(".github/workflows/publish-public.yml").exists());
        return;
    }
    let workflow = read_publish_workflow(&root);
    let section = workflow
        .split("      - name: Publish using ephemeral askpass\n")
        .nth(1)
        .expect("publish helper step must exist");
    let run = section
        .split("        run: |\n")
        .nth(1)
        .expect("publish helper run block must exist");
    let mut script_success = String::new();
    let mut script_failure = String::new();
    for line in run.lines() {
        if !line.starts_with("          ") && !line.is_empty() {
            break;
        }
        let line = line.strip_prefix("          ").unwrap_or(line);
        if line.contains("bash packaging/publish-public.sh --tag") {
            script_success.push_str("[[ \"$(\"$GIT_ASKPASS\" 'Username for https://github.com:')\" == x-access-token ]]\n");
            script_success.push_str("[[ \"$(\"$GIT_ASKPASS\" 'Password for https://github.com:')\" == runtime-dummy-token ]]\n");
            script_success.push_str("bash -c 'exit 0'\n");

            script_failure.push_str("[[ \"$(\"$GIT_ASKPASS\" 'Username for https://github.com:')\" == x-access-token ]]\n");
            script_failure.push_str("[[ \"$(\"$GIT_ASKPASS\" 'Password for https://github.com:')\" == runtime-dummy-token ]]\n");
            script_failure.push_str("bash -c 'exit 42'\n");
            break;
        }
        script_success.push_str(line);
        script_success.push('\n');
        script_failure.push_str(line);
        script_failure.push('\n');
    }
    assert!(script_success.contains("GIT_ASKPASS=\"$askpass_dir/askpass\""));

    // Case 1: success path
    let runner_temp = std::env::temp_dir().join(format!(
        "wda-askpass-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&runner_temp).unwrap();
    let fixture = Fixture::new();
    let output = bash_command()
        .arg("-c")
        .arg(script_success)
        .current_dir(&fixture.0)
        .env("RUNNER_TEMP", &runner_temp)
        .env("PUBLIC_REPO_PUSH_TOKEN", "runtime-dummy-token")
        .env("RELEASE_TAG", "v1.2.3")
        .output()
        .expect("bash must execute the workflow helper");
    assert!(output.status.success(), "{}", output_text(&output));
    assert_eq!(
        fs::read_dir(&runner_temp).unwrap().count(),
        0,
        "askpass temporary directory remained after success"
    );
    assert!(!output_text(&output).contains("runtime-dummy-token"));
    let _ = fs::remove_dir_all(&runner_temp);

    // Case 2: failure path (trap cleanup must remove directory on non-zero exit)
    let runner_temp_fail = std::env::temp_dir().join(format!(
        "wda-askpass-fail-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&runner_temp_fail).unwrap();
    let output_fail = bash_command()
        .arg("-c")
        .arg(script_failure)
        .current_dir(&fixture.0)
        .env("RUNNER_TEMP", &runner_temp_fail)
        .env("PUBLIC_REPO_PUSH_TOKEN", "runtime-dummy-token")
        .env("RELEASE_TAG", "v1.2.3")
        .output()
        .expect("bash must execute the workflow helper");
    assert_eq!(
        output_fail.status.code(),
        Some(42),
        "script failure code was not propagated"
    );
    assert_eq!(
        fs::read_dir(&runner_temp_fail).unwrap().count(),
        0,
        "askpass temporary directory remained after failure"
    );
    assert!(!output_text(&output_fail).contains("runtime-dummy-token"));
    let _ = fs::remove_dir_all(&runner_temp_fail);
}

#[test]
fn exporter_admits_only_exact_credential_free_public_urls() {
    let accepted = [
        PUBLIC_HTTPS,
        "https://GITHUB.com/Monkey1Wizard/Web-Design-Anchor.git",
        "ssh://git@github.com/monkey1wizard/web-design-anchor.git",
        "git@github.com:monkey1wizard/web-design-anchor",
    ];
    for url in accepted {
        let fixture = Fixture::new();
        let output = fixture.script(url, &[]);
        assert!(output.status.success(), "{url}: {}", output_text(&output));
        assert!(output_text(&output).contains("residual scan   : PASS"));
    }

    let rejected = [
        "https://user:dummy-secret@github.com/monkey1wizard/web-design-anchor",
        "https://github.com.evil/monkey1wizard/web-design-anchor",
        "https://github.com/monkey1wizard/web-design-anchor-extra",
        "https://github.com/monkey1wizard/web-design-anchor?query=dummy-secret",
        "https://github.com/monkey1wizard/web-design-anchor#dummy-secret",
        "https://github.com/monkey1wizard/web-design-anchor-private",
        "https://gitlab.com/monkey1wizard/web-design-anchor",
    ];
    for url in rejected {
        let fixture = Fixture::new();
        let remote = fixture.remote();
        fixture.mapped_remote(&remote);
        let output = fixture.script(url, &["--execute"]);
        let text = output_text(&output);
        assert!(!output.status.success(), "unexpectedly accepted {url}");
        assert!(
            !text.contains("dummy-secret"),
            "rejected URL leaked: {text}"
        );
        assert!(!remote_has_refs(&remote), "rejected URL reached transport");
    }
}

#[test]
fn exporter_fixture_rejects_unmapped_network_transport() {
    let fixture = Fixture::new();
    let output = fixture.script(PUBLIC_HTTPS, &["--execute"]);
    let text = output_text(&output);
    assert!(
        !output.status.success(),
        "unmapped HTTPS transport unexpectedly succeeded"
    );
    assert!(
        text.contains("transport 'https' not allowed"),
        "unexpected failure: {text}"
    );
}

#[test]
fn exporter_dry_run_builds_parentless_curated_snapshot_without_remote_changes() {
    for args in [&[][..], &["--tag", "v1.2.3"][..]] {
        let fixture = Fixture::new();
        let remote = fixture.remote();
        fixture.mapped_remote(&remote);
        let keep = fixture.0.join("retained");
        let mut actual = args.to_vec();
        actual.extend(["--keep-snapshot", keep.to_str().unwrap()]);
        let output = fixture.script(PUBLIC_HTTPS, &actual);
        assert!(output.status.success(), "{}", output_text(&output));
        assert!(output_text(&output).contains("residual scan   : PASS"));
        let snap = isolated_git(&keep)
            .arg("-C")
            .arg(&keep)
            .args(["rev-list", "--parents", "-n", "1", "HEAD"])
            .output()
            .unwrap();
        assert!(snap.status.success());
        assert_eq!(
            String::from_utf8_lossy(&snap.stdout)
                .trim()
                .split_whitespace()
                .count(),
            1
        );
        assert!(!keep.join(".dev").exists());
        assert!(!keep.join(".github/workflows/publish-public.yml").exists());
        assert!(keep.join("public.txt").exists());
        assert!(!remote_has_refs(&remote));
    }
}

fn remote_has_refs(remote: &Path) -> bool {
    let output = isolated_git(remote.parent().unwrap())
        .args(["--git-dir"])
        .arg(remote)
        .args(["for-each-ref", "--format=%(refname)"])
        .output()
        .unwrap();
    assert!(output.status.success());
    !output.stdout.is_empty()
}

#[test]
fn exporter_atomically_publishes_branch_and_tag_on_same_orphan_commit() {
    let fixture = Fixture::new();
    let remote = fixture.remote();
    fixture.mapped_remote(&remote);
    let output = fixture.script(PUBLIC_HTTPS, &["--tag", "v2.3.4", "--execute"]);
    assert!(output.status.success(), "{}", output_text(&output));
    let main = remote_rev(&remote, "refs/heads/main");
    let tag = remote_rev(&remote, "refs/tags/v2.3.4");
    assert_eq!(main, tag);
    let parents = isolated_git(&fixture.0)
        .arg("--git-dir")
        .arg(&remote)
        .args(["rev-list", "--parents", "-n", "1"])
        .arg(&main)
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&parents.stdout)
            .trim()
            .split_whitespace()
            .count(),
        1
    );
}

fn remote_rev(remote: &Path, reference: &str) -> String {
    let output = isolated_git(remote.parent().unwrap())
        .arg("--git-dir")
        .arg(remote)
        .args(["rev-parse", reference])
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

#[test]
fn exporter_refuses_existing_tags_and_preserves_branch_only_behavior() {
    let fixture = Fixture::new();
    let remote = fixture.remote();
    fixture.mapped_remote(&remote);
    let first = fixture.script(PUBLIC_HTTPS, &["--tag", "v1.0.0", "--execute"]);
    assert!(first.status.success(), "{}", output_text(&first));
    let before_main = remote_rev(&remote, "refs/heads/main");
    let before_tag = remote_rev(&remote, "refs/tags/v1.0.0");
    let retry = fixture.script(PUBLIC_HTTPS, &["--tag", "v1.0.0", "--execute"]);
    assert!(!retry.status.success());
    assert!(output_text(&retry).contains("tag already exists on the public remote; refusing."));
    assert_eq!(remote_rev(&remote, "refs/heads/main"), before_main);
    assert_eq!(remote_rev(&remote, "refs/tags/v1.0.0"), before_tag);

    let branch_fixture = Fixture::new();
    let branch_remote = branch_fixture.remote();
    branch_fixture.mapped_remote(&branch_remote);
    let branch = branch_fixture.script(PUBLIC_HTTPS, &["--execute"]);
    assert!(branch.status.success(), "{}", output_text(&branch));
    assert!(branch_fixture
        .git_command()
        .arg("--git-dir")
        .arg(&branch_remote)
        .args(["show-ref", "--verify", "--quiet", "refs/heads/main"])
        .status()
        .unwrap()
        .success());
    assert!(remote_rev(&branch_remote, "refs/heads/main").len() == 40);
    assert!(!branch_fixture
        .git_command()
        .arg("--git-dir")
        .arg(&branch_remote)
        .args(["show-ref", "--verify", "--quiet", "refs/tags/v1.0.0"])
        .status()
        .unwrap()
        .success());
}

#[test]
fn exporter_tagged_failures_leave_public_refs_unchanged() {
    let missing_fixture = Fixture::new();
    let missing_remote = missing_fixture.0.join("missing.git");
    missing_fixture.mapped_remote(&missing_remote);
    let lookup = missing_fixture.script(PUBLIC_HTTPS, &["--tag", "v3.0.0", "--execute"]);
    assert!(!lookup.status.success(), "lookup unexpectedly succeeded");
    assert!(!missing_remote.exists());

    let unsupported_fixture = Fixture::new();
    let unsupported_remote = unsupported_fixture.remote();
    unsupported_fixture.mapped_remote(&unsupported_remote);
    let configured = unsupported_fixture
        .git_command()
        .arg("--git-dir")
        .arg(&unsupported_remote)
        .args(["config", "receive.advertiseAtomic", "false"])
        .status()
        .unwrap();
    assert!(configured.success());
    let rejected = unsupported_fixture.script(PUBLIC_HTTPS, &["--tag", "v3.0.0", "--execute"]);
    assert!(
        !rejected.status.success(),
        "server without atomic support unexpectedly accepted publication"
    );
    assert!(
        !remote_has_refs(&unsupported_remote),
        "failed atomic push changed remote refs"
    );

    let receiver_fixture = Fixture::new();
    let receiver_remote = receiver_fixture.remote();
    receiver_fixture.mapped_remote(&receiver_remote);
    let configured = receiver_fixture
        .git_command()
        .arg("--git-dir")
        .arg(&receiver_remote)
        .args(["config", "receive.maxInputSize", "1"])
        .status()
        .unwrap();
    assert!(configured.success());
    let denied = receiver_fixture.script(PUBLIC_HTTPS, &["--tag", "v3.0.0", "--execute"]);
    assert!(
        !denied.status.success(),
        "receiver rejection unexpectedly succeeded"
    );
    assert!(
        !remote_has_refs(&receiver_remote),
        "receiver rejection changed remote refs"
    );
}

#[test]
fn exporter_execute_leaves_no_dummy_credential_in_snapshot_or_remote_config() {
    let fixture = Fixture::new();
    let remote = fixture.remote();
    fixture.mapped_remote(&remote);
    fixture.git(&["remote", "add", "github", PUBLIC_HTTPS]);
    let mut command = bash_command();
    command
        .arg(fixture.0.join("packaging/publish-public.sh"))
        .args(["--tag", "v4.0.0", "--execute"])
        .current_dir(&fixture.0)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", fixture.gitconfig())
        .env("HOME", &fixture.0)
        .env("XDG_CONFIG_HOME", fixture.0.join("xdg"))
        .env("GIT_TERMINAL_PROMPT", "0")
        .env(
            "PUBLIC_REPO_PUSH_TOKEN",
            "dummy-credential-must-not-persist",
        );
    let output = command.output().unwrap();
    assert!(output.status.success(), "{}", output_text(&output));
    assert!(!output_text(&output).contains("dummy-credential-must-not-persist"));
    let contents = fixture
        .git_command()
        .arg("--git-dir")
        .arg(&remote)
        .args([
            "grep",
            "-n",
            "dummy-credential-must-not-persist",
            "refs/heads/main",
        ])
        .output()
        .unwrap();
    assert!(!contents.status.success());
    let config = fixture
        .git_command()
        .arg("--git-dir")
        .arg(&remote)
        .args(["config", "--list"])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&config.stdout).contains("dummy-credential-must-not-persist"));
}

#[test]
fn exporter_validates_release_tag_arguments_with_status_two() {
    let help_fixture = Fixture::new();
    let help = help_fixture.script(PUBLIC_HTTPS, &["--help"]);
    assert!(help.status.success());
    assert!(output_text(&help).contains("--tag <tag> [--execute]"));
    assert!(output_text(&help).contains("default is a dry run"));
    assert!(!output_text(&help).contains("set -euo pipefail"));

    let valid = ["v1.2.3", "v1.2.3-rc.1", "v0.0.0-alpha.build"];
    for tag in valid {
        let fixture = Fixture::new();
        let output = fixture.script(PUBLIC_HTTPS, &["--tag", tag]);
        assert!(output.status.success(), "{tag}: {}", output_text(&output));
    }
    for tag in [
        "",
        "v1",
        "1.2.3",
        "v1.2.3+build",
        "v1.2.3-rc_",
        "v1.2.3-..",
        "v1.2.3-.lock",
    ] {
        let fixture = Fixture::new();
        let output = fixture.script(PUBLIC_HTTPS, &["--tag", tag]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{tag}: {}",
            output_text(&output)
        );
    }
    for args in [&["--tag"][..], &["--tag", "--execute"][..]] {
        let fixture = Fixture::new();
        let output = fixture.script(PUBLIC_HTTPS, args);
        assert_eq!(output.status.code(), Some(2), "{}", output_text(&output));
    }
}
