//! Integration tests for the build boundary on website packages.
//!
//! `wda deps` was removed. Packages are installed by external managers, and
//! `wda build` reads only what is already present in `node_modules`.

use std::fs;
use std::path::Path;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_wda")
}

fn require_tool_cache() {
    match wda_core::tools::cache_status() {
        Ok(Some(_)) => {}
        Ok(None) => panic!(
            "the TypeScript tool cache is missing; run `wda tools fetch` once while online, then rerun this test"
        ),
        Err(error) => panic!(
            "the TypeScript tool cache cannot be checked ({error}); run `wda tools fetch` once while online, then rerun this test"
        ),
    }
}

fn temporary_project(name: &str) -> std::path::PathBuf {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must be available")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "wda_deps_test_{name}_{}_{}",
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

fn mapping_project(name: &str) -> std::path::PathBuf {
    let fixture =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/build/mapping");
    let project = temporary_project(name);
    copy_tree(&fixture, &project);
    project
}

fn run(command: &str, project: &Path) -> std::process::Output {
    std::process::Command::new(binary())
        .arg(command)
        .current_dir(project)
        .output()
        .expect("wda must start")
}

fn write_installed_is_number(project: &Path) {
    let package = project.join("node_modules/is-number");
    fs::create_dir_all(&package).expect("installed package directory must be created");
    fs::write(
        package.join("package.json"),
        r#"{"name":"is-number","version":"7.0.0","type":"module","main":"index.js","types":"index.d.ts"}"#,
    )
    .expect("installed package.json must be written");
    fs::write(
        package.join("index.js"),
        "export default function isNumber(value) { return typeof value === 'number'; }\n",
    )
    .expect("installed index.js must be written");
    fs::write(
        package.join("index.d.ts"),
        "export default function isNumber(value: unknown): boolean;\n",
    )
    .expect("installed index.d.ts must be written");
}

fn import_is_number(project: &Path) {
    fs::write(
        project.join("scripts/main.ts"),
        "import isNumber from 'is-number';\nconsole.log(isNumber(42));\n",
    )
    .expect("script importing is-number must be written");
}

fn snapshot(project: &Path, names: &[&str]) -> Vec<Option<Vec<u8>>> {
    names
        .iter()
        .map(|name| fs::read(project.join(name)).ok())
        .collect()
}

#[test]
fn test_build_boundary_enforces_locked_dependencies_and_preserves_manifest_and_lock() {
    require_tool_cache();
    // 1. A script importing a package that is absent from node_modules fails the build
    // with exit 1 and wda.build.dependency-unlocked, and writes no dist/, package.json,
    // or lock file.
    let unlocked_project = mapping_project("unlocked_boundary");
    import_is_number(&unlocked_project);
    let watched = ["package.json", "deno.lock", "deno.jsonc"];
    let unlocked_before = snapshot(&unlocked_project, &watched);

    let unlocked_output = run("build", &unlocked_project);
    assert_eq!(
        unlocked_output.status.code(),
        Some(1),
        "build with an uninstalled import must fail with exit 1; stderr: {}",
        String::from_utf8_lossy(&unlocked_output.stderr)
    );
    let unlocked_stderr = String::from_utf8_lossy(&unlocked_output.stderr);
    assert!(
        unlocked_stderr.contains("wda.build.dependency-unlocked"),
        "expected wda.build.dependency-unlocked in stderr, got: {unlocked_stderr}"
    );
    assert_eq!(
        unlocked_before,
        snapshot(&unlocked_project, &watched),
        "failed build must leave manifest and lock files byte-identical"
    );
    assert!(
        !unlocked_project.join("dist").exists(),
        "failed build must not create dist/"
    );
    let _ = fs::remove_dir_all(&unlocked_project);

    // 2. The same import, once installed into node_modules by an external manager,
    // builds with exit 0 and leaves node_modules and the manifest untouched.
    let installed_project = mapping_project("installed_boundary");
    import_is_number(&installed_project);
    write_installed_is_number(&installed_project);
    let installed_before = snapshot(&installed_project, &["node_modules/is-number/index.js"]);
    let installed_package = fs::read(installed_project.join("package.json")).unwrap();

    let installed_output = run("build", &installed_project);
    assert_eq!(
        installed_output.status.code(),
        Some(0),
        "build with an installed import must succeed with exit 0; stderr: {}",
        String::from_utf8_lossy(&installed_output.stderr)
    );
    assert!(
        installed_project.join("dist").is_dir(),
        "successful build must create dist/"
    );
    assert_eq!(
        installed_before,
        snapshot(&installed_project, &["node_modules/is-number/index.js"]),
        "build must not modify installed packages"
    );
    assert_eq!(
        installed_package,
        fs::read(installed_project.join("package.json")).unwrap(),
        "build must not modify package.json"
    );
    assert!(
        !installed_project.join("deno.lock").exists(),
        "build must not create deno.lock"
    );
    let _ = fs::remove_dir_all(&installed_project);
}

#[test]
fn test_build_boundary_empty_state_unlocked_dependency_and_mapping_regression() {
    require_tool_cache();
    // 1. Zero-dependency project (empty dependency map in package.json, no node_modules) with a
    // bare package import in a script entry fails build with exit 1 and emits wda.build.dependency-unlocked.
    let empty_unlocked_project = mapping_project("empty_unlocked_boundary");
    import_is_number(&empty_unlocked_project);
    let empty_package = fs::read(empty_unlocked_project.join("package.json")).unwrap();

    let output = run("build", &empty_unlocked_project);
    assert_eq!(
        output.status.code(),
        Some(1),
        "build on zero-dependency unlocked project must fail with exit 1; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("wda.build.dependency-unlocked"),
        "expected wda.build.dependency-unlocked in stderr, got: {stderr}"
    );
    assert!(
        !empty_unlocked_project.join("dist").exists(),
        "failed build must not create dist/"
    );
    assert_eq!(
        empty_package,
        fs::read(empty_unlocked_project.join("package.json")).unwrap(),
        "build must not modify package.json"
    );
    assert!(
        !empty_unlocked_project.join("deno.lock").exists(),
        "build must not create deno.lock"
    );
    let _ = fs::remove_dir_all(&empty_unlocked_project);

    // 2. Regression half: tests/fixtures/build/mapping (an anchor.json and package.json with an empty
    // dependency map, and carrying no external specifier) must still pass wda build with zero diagnostics (exit 0).
    let mapping_project = mapping_project("mapping_regression");
    let mapping_output = run("build", &mapping_project);
    assert_eq!(
        mapping_output.status.code(),
        Some(0),
        "build on mapping fixture must succeed with exit 0; stderr: {}",
        String::from_utf8_lossy(&mapping_output.stderr)
    );
    assert!(
        mapping_project.join("dist").is_dir(),
        "successful build must create dist/"
    );
    let _ = fs::remove_dir_all(&mapping_project);
}
