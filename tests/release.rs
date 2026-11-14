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

fn scalar_sequence(node: Option<&marked_yaml::Node>) -> Vec<&str> {
    match node {
        Some(marked_yaml::Node::Sequence(items)) => items
            .iter()
            .filter_map(|item| match item {
                marked_yaml::Node::Scalar(value) => Some(value.as_str()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[test]
fn ci_workflow_keeps_required_tests_blocking_and_probes_advisory() {
    let root = repo_root();
    let workflow_path = root.join(".github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .unwrap_or_else(|error| panic!("repository must include ci.yml: {error}"));
    let fixture_root = std::env::temp_dir().join(format!(
        "wda-ci-contract-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let private_source = fixture_root.join("private-source");
    let public_export = fixture_root.join("public-export");
    fs::create_dir_all(private_source.join(".github/workflows")).unwrap();
    fs::create_dir_all(private_source.join(".dev")).unwrap();
    fs::create_dir_all(public_export.join(".github/workflows")).unwrap();
    fs::write(private_source.join(".dev/private.md"), "private source\n").unwrap();
    fs::write(private_source.join(".github/workflows/ci.yml"), &workflow).unwrap();
    fs::write(public_export.join(".github/workflows/ci.yml"), &workflow).unwrap();

    assert_ci_workflow_contract(&private_source, "private source");
    assert_ci_workflow_contract(&public_export, "public export");
    fs::remove_dir_all(fixture_root).unwrap();
}

fn assert_ci_workflow_contract(root: &Path, shape: &str) {
    let workflow_path = root.join(".github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .unwrap_or_else(|error| panic!("{shape} must include ci.yml: {error}"));
    let node = marked_yaml::parse_yaml(0, &workflow)
        .unwrap_or_else(|error| panic!("ci.yml must be valid YAML: {error:?}"));

    let triggers = get_mapping_field(&node, "on").expect("ci.yml must define triggers");
    let push = get_mapping_field(triggers, "push").expect("ci.yml must trigger on push");
    assert_eq!(
        scalar_sequence(get_mapping_field(push, "branches")),
        ["main"],
        "push triggers must be limited to main in {shape}"
    );
    assert!(
        get_mapping_field(push, "tags").is_none(),
        "ci.yml must not trigger on tag pushes in {shape}"
    );
    assert!(
        get_mapping_field(triggers, "pull_request").is_some(),
        "ci.yml must retain pull request CI in {shape}"
    );
    for trigger in ["workflow_dispatch", "workflow_call"] {
        assert!(
            get_mapping_field(triggers, trigger).is_some(),
            "ci.yml must accept {trigger} in {shape}"
        );
    }

    let jobs = get_mapping_field(&node, "jobs").expect("ci.yml must define jobs");
    let private_push_guard =
        "github.event_name != 'push' || github.repository == 'monkey1wizard/Web-Design-Anchor'";
    for job_name in ["test", "ignored-probes"] {
        let job = get_mapping_field(jobs, job_name)
            .unwrap_or_else(|| panic!("ci.yml must define {job_name} in {shape}"));
        let job_if = get_scalar_field(job, "if").unwrap_or("");
        assert!(
            job_if.contains(private_push_guard),
            "{job_name} must skip private pushes in {shape}: {job_if}"
        );
    }
    let required = get_mapping_field(jobs, "test").expect("ci.yml must define required tests");
    assert_step_runs(&get_steps(required), "cargo test --locked");
    assert_eq!(
        scalar_sequence(
            get_mapping_field(
                get_mapping_field(required, "strategy").expect("required tests need a strategy"),
                "matrix"
            )
            .and_then(|matrix| get_mapping_field(matrix, "os"))
        ),
        ["ubuntu-latest", "windows-latest"],
        "required tests must retain both OS legs in {shape}"
    );
    assert!(
        get_mapping_field(required, "continue-on-error").is_none(),
        "required tests must remain blocking in {shape}"
    );
    if let Some(required_strategy) = get_mapping_field(required, "strategy") {
        assert_ne!(
            get_scalar_field(required_strategy, "fail-fast"),
            Some("false"),
            "required tests must not disable matrix fail-fast in {shape}"
        );
    }

    let probes =
        get_mapping_field(jobs, "ignored-probes").expect("ci.yml must define ignored probes");
    assert_eq!(
        get_scalar_field(probes, "continue-on-error"),
        Some("true"),
        "ignored probes must be advisory in {shape}"
    );
    let probe_strategy =
        get_mapping_field(probes, "strategy").expect("ignored probes must define a strategy");
    assert_eq!(
        get_scalar_field(probe_strategy, "fail-fast"),
        Some("false"),
        "ignored probes must run every matrix leg in {shape}"
    );
    assert_eq!(
        scalar_sequence(get_mapping_field(
            get_mapping_field(probe_strategy, "matrix").expect("probes need a matrix"),
            "os"
        )),
        ["ubuntu-latest", "windows-latest"],
        "ignored probes must retain both OS legs in {shape}"
    );
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

    // Trigger model: manual dispatch only, never a tag push.
    let triggers = get_mapping_field(&workflow_node, "on").expect("workflow must define triggers");
    assert!(
        get_mapping_field(triggers, "push").is_none(),
        "publish-public.yml must not trigger on push"
    );
    let dispatch = get_mapping_field(triggers, "workflow_dispatch")
        .expect("publish-public.yml must accept workflow_dispatch");
    let inputs = get_mapping_field(dispatch, "inputs").expect("dispatch must define inputs");
    assert!(
        get_mapping_field(inputs, "tag").is_some(),
        "dispatch must accept a tag input"
    );

    // The backup controller runs no hosted test jobs; checks run locally first.
    for removed in ["test-linux", "test-windows"] {
        assert!(
            get_mapping_field(jobs, removed).is_none(),
            "publish-public.yml must not define {removed}"
        );
    }
    let mut discrepancies = Vec::new();

    // publish job
    let publish_job = get_mapping_field(jobs, "publish").expect("workflow must have publish job");
    let publish_needs = get_needs(publish_job);
    assert_eq!(
        publish_needs,
        BTreeSet::from(["validate".to_string()]),
        "publish must need only validate"
    );
    assert_step_uses_checkout_without_credentials(&get_steps(publish_job));

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
                assert!(
                    name == "build" || name == "test",
                    "only build and test jobs may lack needs in release.yml, found {name}"
                );
                let job_if = get_scalar_field(job_val, "if")
                    .unwrap_or_else(|| panic!("{name} job must have if condition"));
                assert!(
                    job_if.contains("github.repository == 'monkey1wizard/Web-Design-Anchor'"),
                    "{name} job in release.yml must guard public repository: {job_if}"
                );
            }
        }
    }

    let release_test = get_mapping_field(rel_jobs, "test").expect("release.yml must have test job");
    assert_eq!(
        get_scalar_field(release_test, "uses"),
        Some("./.github/workflows/ci.yml"),
        "release test job must call ci.yml"
    );
    let sign_job = get_mapping_field(rel_jobs, "sign").expect("release.yml must have sign job");
    assert_eq!(
        get_needs(sign_job),
        BTreeSet::from(["build".to_string(), "test".to_string()]),
        "sign must wait for build and test"
    );

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

    let run_val = |repo: &str, event: &str, tag_input: &str| -> (bool, String) {
        let fixture = Fixture::new();
        let out_file = fixture.0.join("github_output.txt");
        let output = bash_command()
            .arg("-c")
            .arg(&script)
            .current_dir(&fixture.0)
            .env("REPOSITORY", repo)
            .env("EVENT_NAME", event)
            .env("TAG_INPUT", tag_input)
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
    let private = "monkey1wizard/web-design-anchor-private";

    // Valid dispatch tags
    let (ok, out) = run_val(private, "workflow_dispatch", "v1.2.3");
    assert!(ok, "v1.2.3 must be accepted");
    assert!(out.contains(
        "is_tag=true
tag=v1.2.3
"
    ));

    let (ok, out) = run_val(private, "workflow_dispatch", "v0.0.1-alpha.1");
    assert!(ok, "v0.0.1-alpha.1 must be accepted");
    assert!(out.contains(
        "is_tag=true
tag=v0.0.1-alpha.1
"
    ));

    // Empty tag input is a dry run
    let (ok, out) = run_val(private, "workflow_dispatch", "");
    assert!(ok, "dispatch without a tag must be accepted as a dry run");
    assert!(out.contains(
        "is_tag=false
tag=
"
    ));

    // Invalid: wrong repository
    let (ok, _) = run_val("other/repo", "workflow_dispatch", "v1.2.3");
    assert!(!ok, "wrong repository must be rejected");

    // Invalid: any event other than manual dispatch
    for event in ["push", "pull_request"] {
        let (ok, _) = run_val(private, event, "v1.2.3");
        assert!(!ok, "{event} must be rejected");
    }

    // Invalid tags
    for invalid in [
        "v1.2",
        "1.2.3",
        "v1.2.3+build",
        "v1.2.3/tag",
        "v1.2.3-",
        "refs/tags/v1.0.0",
    ] {
        let (ok, _) = run_val(private, "workflow_dispatch", invalid);
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
#[test]
fn native_toolchain_descriptor_rejects_tampering() {
    let original = include_str!("../packaging/toolchain.json");
    assert_eq!(wda_core::tools::validate_descriptor(original).unwrap().len(), 6);
    for changed in [
        original.replace("\"schema_version\": 1", "\"schema_version\": 2"),
        original.replace("\"npm\"", "\"unknown\""),
        original.replace("sha512-0BQ", "sha512-1BQ"),
        original.replace("\"platform\":\"linux-x64\"", "\"platform\":\"unknown\""),
    ] {
        assert!(wda_core::tools::validate_descriptor(&changed).is_err());
    }
}

#[test]
fn native_toolchain_descriptor_matches_official_metadata() {
    let expected = [
        ("win32-x64", "sha512-0BQ3HkAHHlKLSp1qRvf3SUhGpGsDuhB/jgFw75guyqbxJqEaS0Cw/VFO8i2nHglJUzQCRtMMR/IBAKE3ETMC4g=="),
        ("win32-arm64", "sha512-Gyl1Vy6OsWesLzmq+EP0Fb7b4Nid5232AvcA2SFcdYreldpNtYFFofPjnt62y9hQy7VTaZp65ICJjuAQRaVcIQ=="),
        ("darwin-x64", "sha512-SZ9xZInqApNlNGc9s0W1VSsktYSOe9cFqNOIqmN1Gs8SmkjKZYFt017G4VwPxASInODuAdbTW7sXiFUf893RgA=="),
        ("darwin-arm64", "sha512-gowzar9MwS/aRWp6f3a4KUqzRjAZjOsmGNCM6LcTgXum+dBfgsBVMN+AgvOCCbguXyick6LJhpBszxMebJ8syA=="),
        ("linux-x64", "sha512-EYdf2cNg7rgCWJnxCdJ+F3V39O8ihb37eHAu1LK8oAFizgTQbPOK7zHHXbPt8rX24COqODXeI3sIf0fCXG7H/A=="),
        ("linux-arm64", "sha512-Qh4eU4/y3yDjnfjjyPYihMj5/ODIlmt+Bzu17OI+fiSRDW57QmU5SiN63exPRNJPKUzcc1INa1NXdrJ+MqHjUQ=="),
    ];
    let descriptor: serde_json::Value =
        serde_json::from_str(include_str!("../packaging/toolchain.json")).unwrap();
    assert_eq!(descriptor["version"], "7.0.2");
    let platforms =
        wda_core::tools::validate_descriptor(include_str!("../packaging/toolchain.json")).unwrap();
    assert_eq!(platforms.len(), expected.len());
    for (platform, integrity) in expected {
        let row = platforms
            .iter()
            .find(|row| row.platform == platform)
            .unwrap();
        assert_eq!(row.package, format!("@typescript/typescript-{platform}"));
        assert_eq!(row.integrity, integrity);
        assert_eq!(
            row.executable,
            if platform.starts_with("win32-") {
                "package/lib/tsc.exe"
            } else {
                "package/lib/tsc"
            }
        );
    }
}

const NATIVE_SMOKE_PLATFORMS: [&str; 6] = [
    "win32-x64",
    "win32-arm64",
    "darwin-x64",
    "darwin-arm64",
    "linux-x64",
    "linux-arm64",
];

fn normalized_workflow(root: &Path, name: &str) -> String {
    fs::read_to_string(root.join(".github/workflows").join(name))
        .unwrap_or_else(|error| panic!("{name} must be readable: {error}"))
        .replace("\r\n", "\n")
}

fn parse_workflow(text: &str, name: &str) -> marked_yaml::Node {
    marked_yaml::parse_yaml(0, text)
        .unwrap_or_else(|error| panic!("{name} must be YAML: {error:?}"))
}

/// Returns the text of one top-level job, from its key line to the next job key.
fn job_section<'a>(workflow: &'a str, job: &str) -> &'a str {
    let marker = format!("\n  {job}:\n");
    let start = workflow
        .find(&marker)
        .unwrap_or_else(|| panic!("workflow must define job {job}"))
        + 1;
    let body = &workflow[start..];
    let end = body
        .match_indices("\n  ")
        .find(|(index, _)| {
            body.as_bytes()
                .get(index + 3)
                .is_some_and(|byte| *byte != b' ' && *byte != b'\n' && *byte != b'#')
        })
        .map(|(index, _)| index + 1)
        .unwrap_or(body.len());
    &body[..end]
}

fn matrix_include_rows(job: &marked_yaml::Node) -> Vec<&marked_yaml::Node> {
    let strategy = get_mapping_field(job, "strategy").expect("job must define a strategy");
    let matrix = get_mapping_field(strategy, "matrix").expect("strategy must define a matrix");
    match get_mapping_field(matrix, "include") {
        Some(marked_yaml::Node::Sequence(rows)) => rows.iter().collect(),
        _ => Vec::new(),
    }
}

fn mapping_len(node: &marked_yaml::Node) -> usize {
    match node {
        marked_yaml::Node::Mapping(map) => map.iter().count(),
        _ => 0,
    }
}

#[test]
fn compat_watch_workflow_is_isolated_and_in_the_public_snapshot() {
    let root = repo_root();
    let workflow = normalized_workflow(&root, "compat-watch.yml");
    let node = parse_workflow(&workflow, "compat-watch.yml");
    let triggers = get_mapping_field(&node, "on").expect("workflow triggers");
    assert_eq!(mapping_len(triggers), 2);
    assert!(get_mapping_field(triggers, "schedule").is_some());
    assert!(get_mapping_field(triggers, "workflow_dispatch").is_some());
    let permissions = get_mapping_field(&node, "permissions").unwrap();
    assert_eq!(mapping_len(permissions), 1);
    assert_eq!(get_scalar_field(permissions, "contents"), Some("read"));

    let jobs = get_mapping_field(&node, "jobs").unwrap();
    assert_eq!(mapping_len(jobs), 2);
    let probe = get_mapping_field(jobs, "probe").unwrap();
    let report = get_mapping_field(jobs, "report").unwrap();
    for job in [probe, report] {
        assert!(get_scalar_field(job, "if").unwrap().contains("github.repository == 'monkey1wizard/Web-Design-Anchor'"));
    }
    let probe_permissions = get_mapping_field(probe, "permissions").unwrap();
    assert_eq!(mapping_len(probe_permissions), 1);
    assert_eq!(get_scalar_field(probe_permissions, "contents"), Some("read"));
    let strategy = get_mapping_field(probe, "strategy").unwrap();
    let matrix = get_mapping_field(strategy, "matrix").unwrap();
    assert_eq!(scalar_sequence(get_mapping_field(matrix, "os")), ["ubuntu-latest", "windows-latest"]);
    assert_eq!(scalar_sequence(get_mapping_field(matrix, "manager")), ["npm", "bun"]);
    let probe_text = job_section(&workflow, "probe");
    for required in ["persist-credentials: false", "packaging/toolchain.json", "compat_external_probes_real_", "compat_external_init_real_", "--ignored --exact", "1 passed; 0 failed;", "GITHUB_STEP_SUMMARY"] {
        assert!(probe_text.contains(required), "probe must contain {required}");
    }
    assert!(!probe_text.contains("continue-on-error"));
    assert!(!probe_text.contains("write"));

    assert_eq!(get_needs(report), BTreeSet::from(["probe".to_string()]));
    let report_permissions = get_mapping_field(report, "permissions").unwrap();
    assert_eq!(mapping_len(report_permissions), 2);
    assert_eq!(get_scalar_field(report_permissions, "actions"), Some("read"));
    assert_eq!(get_scalar_field(report_permissions, "issues"), Some("write"));
    let concurrency = get_mapping_field(report, "concurrency").unwrap();
    assert_eq!(mapping_len(concurrency), 2);
    assert_eq!(get_scalar_field(concurrency, "group"), Some("compat-watch-issue"));
    assert_eq!(get_scalar_field(concurrency, "cancel-in-progress"), Some("false"));
    let report_text = job_section(&workflow, "report");
    for required in ["[ \"$PROBE_RESULT\" = failure ]", "gh issue list --repo monkey1wizard/Web-Design-Anchor --state open --search '<!-- wda-compat-watch --> in:body'", "gh issue create", "gh issue comment", "RUN_URL", "COMMIT", "missing or incomplete"] {
        assert!(report_text.contains(required), "report must contain {required}");
    }
    for forbidden in ["checkout", "download-artifact", "gh issue close", "gh issue delete", "cargo "] {
        assert!(!report_text.contains(forbidden), "report must not contain {forbidden}");
    }
    let script = fs::read_to_string(root.join("packaging/publish-public.sh")).unwrap();
    let start = script.find("EXCLUDE_TRACKED=(").unwrap();
    let exclusions = &script[start..start + script[start..].find(')').unwrap()];
    assert!(exclusions.contains("\".dev\""));
    assert!(!exclusions.contains(".github/workflows/compat-watch.yml"));
    assert!(!exclusions.contains("\".github\""));
    assert!(!exclusions.contains("\".github/workflows\""));
    assert!(script.contains("git archive --format=tar HEAD"));
    assert!(script.contains(".github/workflows/compat-watch.yml"));
}

#[test]
fn compat_watch_docs_record_public_operations() {
    let root = repo_root();
    // The public snapshot strips `.dev/`, so these private maintenance docs
    // are checked only where they exist. A private checkout that lacks one of
    // them still fails, because is_public_export needs every private path to
    // be absent.
    let public_export = is_public_export(&root);
    if public_export {
        eprintln!("skipped: .dev maintenance docs are absent from the public snapshot");
    }
    for path in [".dev/release.md", ".dev/project.md", ".dev/state.md"] {
        if public_export {
            continue;
        }
        let doc = fs::read_to_string(root.join(path)).expect("maintenance doc must exist");
        for phrase in [
            "Before", "compat-watch.yml", "native WDA",
            "monkey1wizard/Web-Design-Anchor", "public snapshot",
            "packaging/toolchain.json", "latest public release tag",
            "workflow_dispatch", "manual", "missing or incomplete",
            "<!-- wda-compat-watch -->", "close", "manually",
            "workflow never closes", ".dev/",
        ] {
            assert!(doc.contains(phrase), "{path} must state {phrase}");
        }
    }
    let workflow = normalized_workflow(&root, "compat-watch.yml");
    assert!(!workflow.contains(".dev/"), "public workflow must not read private docs");
}

#[test]
fn native_release_embedding_source_build_declares_no_embedding() {
    let root = repo_root();
    let build_script = fs::read_to_string(root.join("build.rs"))
        .expect("build.rs must exist at the package root")
        .replace("\r\n", "\n");
    for required in [
        "WDA_EMBEDDED_TS_ARCHIVE",
        "WDA_REQUIRE_EMBEDDED_TS",
        "packaging/toolchain.json",
    ] {
        assert!(
            build_script.contains(required),
            "build.rs must reference {required}"
        );
    }
    for forbidden in ["reqwest", "std::process::Command", "https://", "curl"] {
        assert!(
            !build_script.contains(forbidden),
            "build.rs must stay offline and must not contain {forbidden}"
        );
    }
    let explicit = option_env!("WDA_EMBEDDED_TS_ARCHIVE").is_some_and(|value| !value.is_empty());
    assert_eq!(
        wda_core::tools::embedded_package().is_some(),
        explicit,
        "a package is embedded only when the release input names an archive"
    );
    assert_eq!(wda_core::tools::EMBEDDED_ARCHIVE.is_some(), explicit);
}

#[test]
fn native_release_embedding_smoke_job_is_six_rows_without_release_side_effects() {
    let root = repo_root();
    let workflow = normalized_workflow(&root, "ci.yml");
    let node = parse_workflow(&workflow, "ci.yml");
    let jobs = get_mapping_field(&node, "jobs").expect("ci.yml must define jobs");
    let job = get_mapping_field(jobs, "native-smoke").expect("ci.yml must define native-smoke");

    let guard = get_scalar_field(job, "if").unwrap_or("");
    assert!(
        guard.contains(
            "github.event_name != 'push' || github.repository == 'monkey1wizard/Web-Design-Anchor'"
        ),
        "native-smoke must skip private pushes: {guard}"
    );
    assert!(
        get_needs(job).is_empty(),
        "native-smoke must not depend on release jobs"
    );
    let permissions = get_mapping_field(job, "permissions").expect("native-smoke permissions");
    assert_eq!(get_scalar_field(permissions, "contents"), Some("read"));
    assert_eq!(
        mapping_len(permissions),
        1,
        "native-smoke may hold only contents: read"
    );

    let rows = matrix_include_rows(job);
    assert_eq!(rows.len(), 6, "native-smoke must have exactly six rows");
    let descriptor =
        wda_core::tools::validate_descriptor(include_str!("../packaging/toolchain.json"))
            .expect("descriptor must validate");
    let mut seen: Vec<&str> = rows
        .iter()
        .map(|row| get_scalar_field(row, "ts_platform").expect("row needs ts_platform"))
        .collect();
    seen.sort_unstable();
    let mut expected = NATIVE_SMOKE_PLATFORMS.to_vec();
    expected.sort_unstable();
    assert_eq!(seen, expected);
    for platform in NATIVE_SMOKE_PLATFORMS {
        assert!(descriptor.iter().any(|row| row.platform == platform));
    }
    for row in &rows {
        let platform = get_scalar_field(row, "ts_platform").unwrap();
        assert_eq!(
            get_scalar_field(row, "alpine") == Some("true"),
            platform.starts_with("linux-"),
            "linux rows must run in Alpine and other rows must not: {platform}"
        );
    }

    let section = job_section(&workflow, "native-smoke");
    for forbidden in [
        "gh release",
        "gh run",
        "gh api",
        "gh workflow",
        "softprops",
        "cosign",
        "sigstore",
        "secrets.",
        "git push",
        "git tag",
        "npm publish",
        "id-token",
        "upload-release",
        "action-gh-release",
        "environment:",
        "--execute",
    ] {
        assert!(
            !section.contains(forbidden),
            "native-smoke must not contain {forbidden}"
        );
    }
    assert!(section.contains("cargo build --release --locked -p wda"));
    assert!(section.contains("WDA_REQUIRE_EMBEDDED_TS=1"));
    assert!(section.contains("alpine:3.21"));
    assert!(section.contains("native-smoke-evidence"));
}

#[test]
fn native_release_embedding_gate_requires_all_six_rows() {
    let root = repo_root();
    let workflow = normalized_workflow(&root, "ci.yml");
    let node = parse_workflow(&workflow, "ci.yml");
    let jobs = get_mapping_field(&node, "jobs").expect("ci.yml must define jobs");
    let gate = get_mapping_field(jobs, "native-smoke-gate").expect("ci.yml must define the gate");
    assert_eq!(
        get_needs(gate),
        BTreeSet::from(["native-smoke".to_string()])
    );
    let section = job_section(&workflow, "native-smoke-gate");
    for platform in NATIVE_SMOKE_PLATFORMS {
        assert!(section.contains(platform), "gate must require {platform}");
    }
    assert!(section.contains("result=pass"));
    assert!(!section.contains("upload-artifact"));
}

#[test]
fn native_release_embedding_release_matrix_embeds_each_platform() {
    let root = repo_root();
    let workflow = normalized_workflow(&root, "release.yml");
    let node = parse_workflow(&workflow, "release.yml");
    let jobs = get_mapping_field(&node, "jobs").expect("release.yml must define jobs");
    let build = get_mapping_field(jobs, "build").expect("release.yml must define build");
    let expected = [
        ("x86_64-pc-windows-msvc", "win32-x64"),
        ("aarch64-pc-windows-msvc", "win32-arm64"),
        ("x86_64-apple-darwin", "darwin-x64"),
        ("aarch64-apple-darwin", "darwin-arm64"),
        ("x86_64-unknown-linux-musl", "linux-x64"),
        ("aarch64-unknown-linux-musl", "linux-arm64"),
    ];
    let rows = matrix_include_rows(build);
    assert_eq!(rows.len(), expected.len());
    for (target, platform) in expected {
        let row = rows
            .iter()
            .find(|row| get_scalar_field(row, "target") == Some(target))
            .unwrap_or_else(|| panic!("release matrix must include {target}"));
        assert_eq!(get_scalar_field(row, "ts_platform"), Some(platform));
    }

    let section = job_section(&workflow, "build");
    assert!(section.contains("WDA_REQUIRE_EMBEDDED_TS"));
    assert!(section.contains("WDA_EMBEDDED_TS_ARCHIVE"));
    assert!(section.contains("CROSS_CONTAINER_OPTS"));
    assert!(section.contains("packaging/toolchain.json"));
    assert!(section.contains("THIRD-PARTY-LICENSES/typescript-LICENSE.txt"));
    assert!(section.contains("package/NOTICE.txt"));
    assert!(section.contains("THIRD-PARTY-LICENSES/typescript-NOTICE.txt"));

    let hosted = get_mapping_field(jobs, "test").expect("release.yml must keep the hosted gate");
    assert_eq!(
        get_scalar_field(hosted, "uses"),
        Some("./.github/workflows/ci.yml"),
        "release must reuse ci.yml, which carries native-smoke"
    );
}

#[test]
fn native_release_embedding_export_keeps_embedding_inputs() {
    let root = repo_root();
    let script = fs::read_to_string(root.join("packaging/publish-public.sh"))
        .expect("publish script must be readable");
    let start = script
        .find("EXCLUDE_TRACKED=(")
        .expect("publish script must define EXCLUDE_TRACKED");
    let block = &script[start..start + script[start..].find(')').unwrap()];
    for kept in ["build.rs", "toolchain.json", "packaging", "src/tools.rs"] {
        assert!(!block.contains(kept), "export must keep {kept}");
    }
}

#[test]
fn native_website_consumer_reads_anchor_and_package_manifests() {
    let root = repo_root();
    let website = root.join("website");
    for retired in ["wda.json", "deno.jsonc", "deno.lock"] {
        assert!(
            !website.join(retired).exists(),
            "website must not keep retired file {retired}"
        );
    }

    let anchor: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(website.join("anchor.json")).expect("website anchor.json must exist"),
    )
    .expect("website anchor.json must be JSON");
    let schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("schemas/anchor.schema.json"))
            .expect("anchor schema must be readable"),
    )
    .expect("anchor schema must be JSON");
    let validator = jsonschema::validator_for(&schema).expect("anchor schema must compile");
    assert!(
        validator.is_valid(&anchor),
        "website anchor.json must conform to the anchor schema"
    );
    assert_eq!(anchor["designSystem"]["name"], "Spectrum 2");
    assert_eq!(anchor["designSystem"]["version"], "1.12.4");
    assert!(anchor.get("initialization").is_none());
    assert!(anchor.get("tools").is_none());

    let package: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(website.join("package.json")).expect("website package.json must exist"),
    )
    .expect("website package.json must be JSON");
    let expected = [
        ("@spectrum-web-components/button", "1.12.4"),
        ("@spectrum-web-components/card", "1.12.4"),
        ("@spectrum-web-components/dialog", "1.12.4"),
        ("@spectrum-web-components/picker", "1.12.4"),
        ("@spectrum-web-components/textfield", "1.12.4"),
        ("@spectrum-web-components/theme", "1.12.4"),
        ("lit", "3.3.3"),
    ];
    let dependencies = package["dependencies"]
        .as_object()
        .expect("website package.json must declare dependencies");
    assert_eq!(dependencies.len(), expected.len());
    for (name, version) in expected {
        assert_eq!(dependencies[name], version, "dependency {name}");
    }

    let lock_path = website.join("package-lock.json");
    assert!(
        lock_path.exists(),
        "website package-lock.json must exist as the npm lockfile"
    );
    let lock: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&lock_path).expect("website package-lock.json must be readable"),
    )
    .expect("website package-lock.json must be JSON");
    assert_eq!(lock["lockfileVersion"], 3);
    assert_eq!(lock["packages"][""]["dependencies"], package["dependencies"]);

    let readme = fs::read_to_string(website.join("README.md")).expect("website README must exist");
    assert!(!readme.contains("wda.json"), "README must not name wda.json");
    assert!(!readme.to_ascii_lowercase().contains("deno"), "README must not name Deno");
    for required in ["anchor.json", "package.json", "npm ci --ignore-scripts"] {
        assert!(readme.contains(required), "README must mention {required}");
    }

    let workflow = normalized_workflow(&root, "website-release.yml");
    assert!(
        !workflow.to_ascii_lowercase().contains("deno"),
        "website workflow must not use Deno"
    );
    let install = workflow
        .find("npm ci --ignore-scripts")
        .expect("workflow must install from the lockfile with npm ci");
    assert!(!workflow.contains("npm install"), "workflow must not run npm install");
    let fetch = workflow
        .find("wda tools fetch")
        .expect("workflow must restore the tool cache");
    let build = workflow.find("wda build").expect("workflow must run wda build");
    let verify = workflow
        .find("sha256sum --check")
        .expect("workflow must verify the manifests after build");
    assert!(install < fetch && fetch < build && build < verify);
    assert!(workflow.contains("sha256sum package.json package-lock.json"));
    assert!(workflow.contains("test -f dist/index.html"));
}

/// Public install guidance in the README, SECURITY, their translations and the
/// website install answers names the same prerequisites in every language,
/// never forces Deno, Node.js or Bun, and keeps the retired manifests and
/// `wda deps` out of the instructions. Each translation carries the SHA-256 of
/// the LF-normalized English source it follows.
#[test]
fn native_public_guidance_agrees_across_languages_and_site() {
    use sha2::Digest;

    let root = repo_root();
    let read = |rel: &str| {
        fs::read_to_string(root.join(rel))
            .unwrap_or_else(|e| panic!("{rel} must be readable: {e}"))
            .replace("\r\n", "\n")
    };

    let readme_sets: [(&str, &[&str]); 3] = [
        (
            "README.md",
            &["WDA Minimal needs no Deno, Node.js, or Bun", "npm or Bun", "wda tools fetch", "anchor.json", "package.json"],
        ),
        (
            "docs/i18n/zh-TW/README.zh-TW.md",
            &["WDA Minimal 不需要 Deno、Node.js 或 Bun", "npm 或 Bun", "wda tools fetch", "anchor.json", "package.json"],
        ),
        (
            "docs/i18n/ja/README.ja.md",
            &["WDA MinimalにDeno、Node.js、Bunは不要である", "npmまたはBun", "wda tools fetch", "anchor.json", "package.json"],
        ),
    ];
    let website_sets: [(&str, &[&str]); 3] = [
        ("website/pages/index.html", &["WDA Minimal needs no Deno, Node.js, or Bun", "npm or Bun"]),
        ("website/pages/ja.html", &["WDA MinimalにはDeno、Node.js、Bunは不要です", "npmまたはBun"]),
        ("website/pages/zh-TW.html", &["WDA Minimal 不需要 Deno、Node.js 或 Bun", "npm 或 Bun"]),
    ];
    for (rel, required) in readme_sets.iter().chain(website_sets.iter()) {
        let text = read(rel);
        for phrase in required.iter() {
            assert!(text.contains(phrase), "{rel} must state: {phrase}");
        }
    }

    let public_files = [
        "README.md",
        "SECURITY.md",
        "docs/i18n/zh-TW/README.zh-TW.md",
        "docs/i18n/ja/README.ja.md",
        "docs/i18n/zh-TW/SECURITY.zh-TW.md",
        "docs/i18n/ja/SECURITY.ja.md",
        "website/pages/index.html",
        "website/pages/ja.html",
        "website/pages/zh-TW.html",
    ];
    let forbidden = [
        "wda.json",
        "deno.jsonc",
        "deno.lock",
        "npm:esbuild",
        "wda deps add",
        "Git and Deno",
        "Git 與 Deno",
        "GitとDeno",
        "Install Git and Deno",
        "Node.js is required",
        "requires Node.js",
        "需要 Node.js",
        "Node.js が必要",
    ];
    for rel in public_files {
        let text = read(rel);
        for phrase in forbidden {
            assert!(!text.contains(phrase), "{rel} must not contain: {phrase}");
        }
    }
    for rel in ["SECURITY.md", "docs/i18n/zh-TW/SECURITY.zh-TW.md", "docs/i18n/ja/SECURITY.ja.md"] {
        let text = read(rel);
        assert!(!text.contains("Deno"), "{rel} must not name Deno");
        assert!(text.contains("`sha512`"), "{rel} must state the sha512 tool SRI");
    }

    for (translation, source) in [
        ("docs/i18n/zh-TW/README.zh-TW.md", "README.md"),
        ("docs/i18n/ja/README.ja.md", "README.md"),
        ("docs/i18n/zh-TW/SECURITY.zh-TW.md", "SECURITY.md"),
        ("docs/i18n/ja/SECURITY.ja.md", "SECURITY.md"),
    ] {
        let digest: String = sha2::Sha256::digest(read(source).as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let front_matter = read(translation);
        assert!(
            front_matter.lines().any(|line| line == format!("source_sha256: {digest}")),
            "{translation} source_sha256 must equal the SHA-256 of the LF-normalized {source}"
        );
    }

    for rel in public_files {
        let text = read(rel);
        for (index, _) in text.match_indices("T-") {
            let tail = &text[index + 2..];
            let digits = tail.chars().take(2).all(|c| c.is_ascii_digit());
            let preceded = text[..index].chars().last().is_some_and(|c| c.is_ascii_alphanumeric());
            assert!(!(digits && !preceded), "{rel} must not carry a plan-task ID");
        }
    }
}
