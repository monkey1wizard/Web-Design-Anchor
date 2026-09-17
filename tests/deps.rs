//! Integration tests for `wda deps`.

use std::fs;
use std::path::Path;

#[test]
fn test_crate_root_pub_seam_accessibility() {
    // 1. External crate consumer calls all three add/remove/update entry points via wda_core:: prefix.
    let non_existent_root = Path::new("non_existent_project_path_for_tp18_test");

    // add_dependency called through wda_core:: prefix
    let add_result = wda_core::add_dependency(non_existent_root, "is-number@7.0.0");
    match add_result {
        Err(wda_core::DepsError::ToolFault(fault)) => {
            assert!(fault.message.contains("not found"));
        }
        other => panic!("expected ToolFault for non-existent root, got: {other:?}"),
    }

    // remove_dependency called through wda_core:: prefix
    let remove_result = wda_core::remove_dependency(non_existent_root, "is-number");
    match remove_result {
        Err(wda_core::DepsError::ToolFault(fault)) => {
            assert!(fault.message.contains("not found"));
        }
        other => panic!("expected ToolFault for non-existent root, got: {other:?}"),
    }

    // update_dependencies called through wda_core:: prefix
    let update_result = wda_core::update_dependencies(non_existent_root, None);
    match update_result {
        Err(wda_core::DepsError::ToolFault(fault)) => {
            assert!(fault.message.contains("not found"));
        }
        other => panic!("expected ToolFault for non-existent root, got: {other:?}"),
    }

    // Direct aliases are also callable via wda_core:: prefix
    let _ = wda_core::deps_add(non_existent_root, "pkg@1.0.0");
    let _ = wda_core::deps_remove(non_existent_root, "pkg");
    let _ = wda_core::deps_update(non_existent_root, None);
    let _ = wda_core::update_dependency(non_existent_root, None);
    let _ = wda_core::add(non_existent_root, "pkg@1.0.0");
    let _ = wda_core::remove(non_existent_root, "pkg");
    let _ = wda_core::update(non_existent_root, None);

    // 2. Functional behavior on clean temporary project
    let temp_dir = std::env::temp_dir().join(format!(
        "wda_test_tp18_seam_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir must be created");

    // Empty project: update_dependencies returns Ok(None) without invoking Deno or writing files
    let update_clean = wda_core::update_dependencies(&temp_dir, None)
        .expect("update on empty project succeeds with Ok(None)");
    assert!(update_clean.is_none());
    assert!(!temp_dir.join("package.json").exists());
    assert!(!temp_dir.join("deno.lock").exists());

    // Empty project: remove_dependency returns CLI_USAGE error without invoking Deno
    let remove_clean = wda_core::remove_dependency(&temp_dir, "non-existent-dep");
    match remove_clean {
        Err(wda_core::DepsError::Diagnostic(diag)) => {
            assert_eq!(diag.code, wda_core::codes::CLI_USAGE);
        }
        other => panic!("expected CLI_USAGE Diagnostic, got: {other:?}"),
    }
    assert!(!temp_dir.join("package.json").exists());
    assert!(!temp_dir.join("deno.lock").exists());

    let _ = fs::remove_dir_all(&temp_dir);
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_wda")
}

fn deno_available() -> bool {
    std::process::Command::new("deno").arg("--version").output().is_ok()
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

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/deps")
        .join(name)
}

fn run(command: &str, project: &Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new(binary())
        .arg(command)
        .args(args)
        .current_dir(project)
        .output()
        .expect("wda must start")
}

#[test]
fn test_build_boundary_enforces_locked_dependencies_and_preserves_manifest_and_lock() {
    if !deno_available() {
        return;
    }

    // 1. Fixture requiring an unlocked dependency run through real `wda build`
    // yields Error, exit 1, and leaves both files byte-identical before and after.
    let unlocked_project = temporary_project("unlocked_boundary");
    copy_tree(&fixture("unlocked"), &unlocked_project);

    let unlocked_pkg_before = fs::read(unlocked_project.join("package.json"))
        .expect("unlocked fixture package.json must exist");
    let unlocked_lock_before = fs::read(unlocked_project.join("deno.lock"))
        .expect("unlocked fixture deno.lock must exist");

    let unlocked_output = run("build", &unlocked_project, &[]);
    assert_eq!(
        unlocked_output.status.code(),
        Some(1),
        "build on unlocked dependency fixture must fail with exit 1; stderr: {}",
        String::from_utf8_lossy(&unlocked_output.stderr)
    );
    let unlocked_stderr = String::from_utf8_lossy(&unlocked_output.stderr);
    assert!(
        unlocked_stderr.contains("wda.build.dependency-unlocked"),
        "expected wda.build.dependency-unlocked in stderr, got: {unlocked_stderr}"
    );

    let unlocked_pkg_after = fs::read(unlocked_project.join("package.json"))
        .expect("unlocked fixture package.json must exist after build");
    let unlocked_lock_after = fs::read(unlocked_project.join("deno.lock"))
        .expect("unlocked fixture deno.lock must exist after build");

    assert_eq!(
        unlocked_pkg_before, unlocked_pkg_after,
        "failed build must leave package.json byte-identical"
    );
    assert_eq!(
        unlocked_lock_before, unlocked_lock_after,
        "failed build must leave deno.lock byte-identical"
    );
    assert!(
        !unlocked_project.join("dist").exists(),
        "failed build must not create dist/"
    );

    let _ = fs::remove_dir_all(&unlocked_project);

    // 2. Fixture with dependencies already locked builds successfully with exit 0,
    // and both package.json and deno.lock remain byte-identical before and after.
    let locked_project = temporary_project("locked_boundary");
    copy_tree(&fixture("locked"), &locked_project);

    let locked_pkg_before = fs::read(locked_project.join("package.json"))
        .expect("locked fixture package.json must exist");
    let locked_lock_before = fs::read(locked_project.join("deno.lock"))
        .expect("locked fixture deno.lock must exist");

    let locked_output = run("build", &locked_project, &[]);
    assert_eq!(
        locked_output.status.code(),
        Some(0),
        "build on locked fixture must succeed with exit 0; stderr: {}",
        String::from_utf8_lossy(&locked_output.stderr)
    );
    assert!(
        locked_project.join("dist").is_dir(),
        "successful build must create dist/"
    );

    let locked_pkg_after = fs::read(locked_project.join("package.json"))
        .expect("locked fixture package.json must exist after build");
    let locked_lock_after = fs::read(locked_project.join("deno.lock"))
        .expect("locked fixture deno.lock must exist after build");

    assert_eq!(
        locked_pkg_before, locked_pkg_after,
        "successful build must leave package.json byte-identical"
    );
    assert_eq!(
        locked_lock_before, locked_lock_after,
        "successful build must leave deno.lock byte-identical"
    );

    let _ = fs::remove_dir_all(&locked_project);
}

#[test]
fn test_build_boundary_empty_state_unlocked_dependency_and_mapping_regression() {
    if !deno_available() {
        return;
    }

    // 1. Zero-dependency project (no package.json, no deno.lock) with an npm: import
    // in pages/ script entry fails build with exit 1 and emits wda.build.dependency-unlocked.
    let empty_unlocked_project = temporary_project("empty_unlocked_boundary");
    copy_tree(&fixture("empty_unlocked"), &empty_unlocked_project);

    let output = run("build", &empty_unlocked_project, &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "build on zero-dependency unlocked fixture must fail with exit 1; stderr: {}",
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
    assert!(
        !empty_unlocked_project.join("package.json").exists(),
        "build must not create package.json"
    );
    assert!(
        !empty_unlocked_project.join("deno.lock").exists(),
        "build must not create deno.lock"
    );

    let _ = fs::remove_dir_all(&empty_unlocked_project);

    // 2. Regression half: tests/fixtures/build/mapping (missing both files, and carrying
    // no external specifier) must still pass wda build with zero diagnostics (exit 0).
    let mapping_fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/build/mapping");
    let mapping_project = temporary_project("mapping_regression");
    copy_tree(&mapping_fixture, &mapping_project);

    let mapping_output = run("build", &mapping_project, &[]);
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

