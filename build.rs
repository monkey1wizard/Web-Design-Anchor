//! Embeds a platform TypeScript package into release builds.
//!
//! Nothing is embedded unless `WDA_EMBEDDED_TS_ARCHIVE` names a local archive.
//! The archive is copied into `OUT_DIR` and read by `src/tools.rs`, which checks
//! its `sha512` integrity against `packaging/toolchain.json` before unpacking it.
//! This script never reaches the network. Set `WDA_REQUIRE_EMBEDDED_TS` to make
//! a missing archive a build error.

use std::env;
use std::fs;
use std::path::PathBuf;

const ARCHIVE_INPUT: &str = "WDA_EMBEDDED_TS_ARCHIVE";
const REQUIRE_INPUT: &str = "WDA_REQUIRE_EMBEDDED_TS";
const TOOL_VERSION: &str = "7.0.2";
const EMBEDDED_NAME: &str = "typescript-package.tgz";

fn target_platform() -> Result<String, String> {
    let os = match env::var("CARGO_CFG_TARGET_OS").unwrap_or_default().as_str() {
        "windows" => "win32",
        "macos" => "darwin",
        "linux" => "linux",
        other => return Err(format!("no TypeScript package for target OS `{other}`")),
    };
    let arch = match env::var("CARGO_CFG_TARGET_ARCH")
        .unwrap_or_default()
        .as_str()
    {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => return Err(format!("no TypeScript package for target arch `{other}`")),
    };
    Ok(format!("{os}-{arch}"))
}

fn main() {
    println!("cargo::rustc-check-cfg=cfg(wda_embedded_ts)");
    println!("cargo:rerun-if-env-changed={ARCHIVE_INPUT}");
    println!("cargo:rerun-if-env-changed={REQUIRE_INPUT}");
    println!("cargo:rerun-if-changed=packaging/toolchain.json");

    let input = env::var_os(ARCHIVE_INPUT).filter(|value| !value.is_empty());
    let Some(input) = input else {
        if env::var_os(REQUIRE_INPUT).is_some_and(|value| !value.is_empty()) {
            panic!("{REQUIRE_INPUT} is set but {ARCHIVE_INPUT} names no archive");
        }
        return;
    };

    let archive = PathBuf::from(&input);
    println!("cargo:rerun-if-changed={}", archive.display());
    let platform = target_platform().unwrap_or_else(|error| panic!("{error}"));
    let expected_name = format!("typescript-{platform}-{TOOL_VERSION}.tgz");
    let file_name = archive.file_name().and_then(|name| name.to_str());
    if file_name != Some(expected_name.as_str()) {
        panic!(
            "{ARCHIVE_INPUT} must name `{expected_name}` for this target, found {}",
            archive.display()
        );
    }
    let bytes = fs::read(&archive)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", archive.display()));
    if bytes.len() < 2 || bytes[..2] != [0x1f, 0x8b] {
        panic!("{} is not a gzip archive", archive.display());
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    fs::write(out_dir.join(EMBEDDED_NAME), &bytes)
        .unwrap_or_else(|error| panic!("cannot stage embedded archive: {error}"));
    println!("cargo:rustc-cfg=wda_embedded_ts");
}
