use base64::{engine::general_purpose::STANDARD, Engine};
use flate2::read::GzDecoder;
use sha2::{Digest, Sha512};
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

const DESCRIPTOR: &str = include_str!("../packaging/toolchain.json");
const MAX_RESPONSE: u64 = 32 * 1024 * 1024;
const MAX_UNPACKED: u64 = 512 * 1024 * 1024;
#[cfg(wda_embedded_ts)]
const EMBEDDED_PACKAGE: Option<&[u8]> = Some(include_bytes!(concat!(
    env!("OUT_DIR"),
    "/typescript-package.tgz"
)));
#[cfg(not(wda_embedded_ts))]
const EMBEDDED_PACKAGE: Option<&[u8]> = None;
/// Set only when a release build embedded this platform's TypeScript package.
pub const EMBEDDED_ARCHIVE: Option<&str> = if EMBEDDED_PACKAGE.is_some() {
    Some("typescript-package")
} else {
    None
};

/// The platform TypeScript package embedded at compile time, if any.
pub fn embedded_package() -> Option<&'static [u8]> {
    EMBEDDED_PACKAGE
}

#[derive(Debug, Clone)]
pub struct Platform {
    pub platform: String,
    pub package: String,
    pub executable: String,
    pub integrity: String,
}

pub fn validate_descriptor(input: &str) -> Result<Vec<Platform>, String> {
    let value: serde_json::Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let canonical: serde_json::Value =
        serde_json::from_str(DESCRIPTOR).map_err(|e| e.to_string())?;
    if value != canonical {
        return Err("tool descriptor does not match the embedded trust root".into());
    }
    let root = value.as_object().ok_or("invalid descriptor")?;
    if root.len() != 5
        || root.get("schema_version").and_then(|v| v.as_u64()) != Some(1)
        || root.get("name").and_then(|v| v.as_str()) != Some("typescript")
        || root.get("version").and_then(|v| v.as_str()) != Some("7.0.2")
        || root.get("package_managers") != Some(&serde_json::json!(["npm", "bun"]))
    {
        return Err("invalid tool descriptor".into());
    }
    let rows = root
        .get("platforms")
        .and_then(|v| v.as_array())
        .ok_or("missing platforms")?;
    let expected = [
        "win32-x64",
        "win32-arm64",
        "darwin-x64",
        "darwin-arm64",
        "linux-x64",
        "linux-arm64",
    ];
    if rows.len() != expected.len() {
        return Err("invalid platform count".into());
    }
    let mut platforms = Vec::new();
    for name in expected {
        let matching: Vec<_> = rows
            .iter()
            .filter(|row| row.get("platform").and_then(|v| v.as_str()) == Some(name))
            .collect();
        if matching.len() != 1 {
            return Err("missing or duplicate platform".into());
        }
        let row = matching[0].as_object().ok_or("invalid platform")?;
        if row.len() != 4 {
            return Err("unknown platform field".into());
        }
        let string = |key: &str| {
            row.get(key)
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("missing {key}"))
        };
        let package = string("package")?;
        let executable = string("executable")?;
        let integrity = string("integrity")?;
        let expected_exe = if name.starts_with("win32-") {
            "package/lib/tsc.exe"
        } else {
            "package/lib/tsc"
        };
        let digest = integrity
            .strip_prefix("sha512-")
            .ok_or("invalid integrity algorithm")?;
        if package != format!("@typescript/typescript-{name}")
            || executable != expected_exe
            || STANDARD
                .decode(digest)
                .map_err(|_| "invalid integrity")?
                .len()
                != 64
        {
            return Err("invalid platform identity".into());
        }
        platforms.push(Platform {
            platform: name.into(),
            package: package.into(),
            executable: executable.into(),
            integrity: integrity.into(),
        });
    }
    Ok(platforms)
}

fn current_platform() -> Result<Platform, String> {
    let os = match std::env::consts::OS {
        "windows" => "win32",
        "macos" => "darwin",
        "linux" => "linux",
        _ => return Err("unsupported operating system".into()),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        _ => return Err("unsupported architecture".into()),
    };
    validate_descriptor(DESCRIPTOR)?
        .into_iter()
        .find(|p| p.platform == format!("{os}-{arch}"))
        .ok_or_else(|| "unsupported platform".into())
}

fn cache_root() -> Result<PathBuf, String> {
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA");
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CACHE_HOME").or_else(|| {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache").into_os_string())
    });
    base.map(|p| PathBuf::from(p).join("wda").join("tools"))
        .ok_or_else(|| "cache directory unavailable".into())
}

fn cache_path(root: &Path, platform: &Platform) -> Result<PathBuf, String> {
    let digest = STANDARD
        .decode(
            platform
                .integrity
                .strip_prefix("sha512-")
                .ok_or("invalid integrity")?,
        )
        .map_err(|e| e.to_string())?;
    // Windows caps paths near 260 characters, so the directory name uses only the first 128 bits
    // of the digest. The full SHA-512 integrity is still verified against the archive on unpack.
    let key: String = digest.iter().take(16).map(|byte| format!("{byte:02x}")).collect();
    Ok(root.join(key))
}

fn complete(path: &Path, platform: &Platform) -> bool {
    path.join(&platform.executable).is_file()
        && path.join("package/LICENSE").is_file()
        && path.join("package/lib/lib.d.ts").is_file()
}

pub fn cache_status() -> Result<Option<PathBuf>, String> {
    let platform = current_platform()?;
    let path = cache_path(&cache_root()?, &platform)?;
    if complete(&path, &platform) {
        return Ok(Some(path.join(platform.executable)));
    }
    if EMBEDDED_PACKAGE.is_some() {
        // An embedded release build restores its own package offline.
        return fetch().map(Some);
    }
    Ok(None)
}

fn trusted_url(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("registry.npmjs.org")
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
}

fn http_client(
    connect_timeout: Duration,
    request_timeout: Duration,
) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .connect_timeout(connect_timeout)
        .timeout(request_timeout)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 || !trusted_url(attempt.url()) {
                attempt.error("untrusted redirect")
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|e| e.to_string())
}

fn download(platform: &Platform) -> Result<Vec<u8>, String> {
    let url = format!(
        "https://registry.npmjs.org/{}/-/typescript-{}-7.0.2.tgz",
        platform.package, platform.platform
    );
    let client = http_client(Duration::from_secs(10), Duration::from_secs(120))?;
    let mut response = client
        .get(url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    if !trusted_url(response.url()) {
        return Err("untrusted response".into());
    }
    read_response(&mut response)
}

fn read_response(response: &mut reqwest::blocking::Response) -> Result<Vec<u8>, String> {
    if response.content_length().is_some_and(|n| n > MAX_RESPONSE) {
        return Err("oversized response".into());
    }
    let mut data = Vec::new();
    response
        .by_ref()
        .take(MAX_RESPONSE + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() as u64 > MAX_RESPONSE {
        return Err("oversized response".into());
    }
    Ok(data)
}

fn unpack(data: &[u8], platform: &Platform, destination: &Path) -> Result<(), String> {
    let actual = format!("sha512-{}", STANDARD.encode(Sha512::digest(data)));
    if actual != platform.integrity {
        return Err("archive integrity mismatch".into());
    }
    let mut archive = tar::Archive::new(GzDecoder::new(Cursor::new(data)));
    let mut total = 0u64;
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.header().entry_type();
        if !kind.is_file() && !kind.is_dir() {
            return Err("unsupported archive entry".into());
        }
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        if !path.starts_with("package")
            || path
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err("unsafe archive path".into());
        }
        total = total.checked_add(entry.size()).ok_or("archive too large")?;
        if total > MAX_UNPACKED {
            return Err("archive too large".into());
        }
        if !entry.unpack_in(destination).map_err(|e| e.to_string())? {
            return Err("unsafe archive path".into());
        }
    }
    if !complete(destination, platform) {
        return Err("incomplete tool package".into());
    }
    Ok(())
}

pub fn fetch() -> Result<PathBuf, String> {
    let platform = current_platform()?;
    let root = cache_root()?;
    match EMBEDDED_PACKAGE {
        Some(bytes) => fetch_to(&root, &platform, || Ok(bytes.to_vec())),
        None => fetch_to(&root, &platform, || download(&platform)),
    }
}

fn fetch_to(
    root: &Path,
    platform: &Platform,
    get_archive: impl FnOnce() -> Result<Vec<u8>, String>,
) -> Result<PathBuf, String> {
    let target = cache_path(root, platform)?;
    if complete(&target, platform) {
        return Ok(target.join(&platform.executable));
    }
    let data = get_archive()?;
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let stage = tempfile::Builder::new()
        .prefix(".stage-")
        .tempdir_in(&root)
        .map_err(|e| e.to_string())?;
    unpack(&data, platform, stage.path())?;
    match std::fs::rename(stage.path(), &target) {
        Ok(()) => Ok(target.join(&platform.executable)),
        Err(_) if complete(&target, platform) => Ok(target.join(&platform.executable)),
        Err(e) => Err(format!("cannot publish tool cache: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;

    #[test]
    fn cache_path_stays_under_windows_max_path_and_keys_differ() {
        let root = PathBuf::from(format!(r"C:\Users\{}\AppData\Local\wda\tools", "x".repeat(70)));
        assert!(root.as_os_str().len() >= 100);
        let mut first = current_platform().unwrap();
        first.integrity = format!("sha512-{}", STANDARD.encode(Sha512::digest(b"first")));
        let mut second = first.clone();
        second.integrity = format!("sha512-{}", STANDARD.encode(Sha512::digest(b"second")));
        let first_path = cache_path(&root, &first).unwrap();
        let second_path = cache_path(&root, &second).unwrap();
        assert_ne!(first_path, second_path);
        let tsc = first_path.join("package").join("lib").join("tsc.exe");
        assert!(tsc.as_os_str().len() < 260, "{} characters", tsc.as_os_str().len());
    }

    #[test]
    fn rejects_bad_integrity_and_archive_links() {
        let mut tar = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_link(&mut header, "package/link", "../outside")
            .unwrap();
        let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
        gzip.write_all(&tar.into_inner().unwrap()).unwrap();
        let bytes = gzip.finish().unwrap();
        let mut platform = current_platform().unwrap();
        let stage = tempfile::tempdir().unwrap();
        assert!(unpack(&bytes, &platform, stage.path())
            .unwrap_err()
            .contains("integrity"));
        platform.integrity = format!("sha512-{}", STANDARD.encode(Sha512::digest(&bytes)));
        assert!(unpack(&bytes, &platform, stage.path())
            .unwrap_err()
            .contains("entry"));
        assert!(!stage.path().join("outside").exists());
    }

    #[test]
    fn native_release_embedding_unit_matches_explicit_input() {
        let explicit = option_env!("WDA_EMBEDDED_TS_ARCHIVE").is_some_and(|v| !v.is_empty());
        assert_eq!(EMBEDDED_PACKAGE.is_some(), explicit);
        assert_eq!(EMBEDDED_ARCHIVE.is_some(), explicit);
        let Some(bytes) = EMBEDDED_PACKAGE else {
            return;
        };
        let platform = current_platform().unwrap();
        let digest = format!("sha512-{}", STANDARD.encode(Sha512::digest(bytes)));
        assert_eq!(digest, platform.integrity);
        let root = tempfile::tempdir().unwrap();
        let path = fetch_to(root.path(), &platform, || Ok(bytes.to_vec())).unwrap();
        assert!(path.is_file());
        assert!(complete(
            &cache_path(root.path(), &platform).unwrap(),
            &platform
        ));
    }

    #[test]
    fn rejects_untrusted_redirect_destinations() {
        for url in [
            "http://registry.npmjs.org/a",
            "https://example.com/a",
            "https://registry.npmjs.org:444/a",
            "https://user@registry.npmjs.org/a",
        ] {
            assert!(!trusted_url(&reqwest::Url::parse(url).unwrap()));
        }
        assert!(trusted_url(
            &reqwest::Url::parse("https://registry.npmjs.org/a").unwrap()
        ));
    }
    #[test]
    fn native_tools_fetch_unit_rejects_oversize_and_timeout() {
        use std::net::TcpListener;
        use std::thread;

        for streaming in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 1024];
                let _ = socket.read(&mut request);
                if streaming {
                    socket.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").unwrap();
                    let chunk = vec![b'x'; 65536];
                    for _ in 0..513 {
                        if socket
                            .write_all(b"10000\r\n")
                            .and_then(|_| socket.write_all(&chunk))
                            .and_then(|_| socket.write_all(b"\r\n"))
                            .is_err()
                        {
                            break;
                        }
                    }
                } else {
                    write!(
                        socket,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                        MAX_RESPONSE + 1
                    )
                    .unwrap();
                }
            });
            let client = http_client(Duration::from_secs(1), Duration::from_secs(10)).unwrap();
            let mut response = client.get(format!("http://{address}/")).send().unwrap();
            assert!(read_response(&mut response)
                .unwrap_err()
                .contains("oversized"));
            server.join().unwrap();
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (_socket, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_millis(500));
        });
        let client = http_client(Duration::from_secs(1), Duration::from_millis(100)).unwrap();
        assert!(client
            .get(format!("http://{address}/"))
            .send()
            .unwrap_err()
            .is_timeout());
        server.join().unwrap();
    }

    #[test]
    fn native_tools_fetch_unit_rejects_traversal_and_preserves_cache() {
        let mut header = tar::Header::new_gnu();
        header.set_path("package/safe").unwrap();
        header.set_size(0);
        header.set_mode(0o644);
        let name = b"package/../../escape";
        header.as_mut_bytes()[..100].fill(0);
        header.as_mut_bytes()[..name.len()].copy_from_slice(name);
        header.set_cksum();
        let mut raw = header.as_bytes().to_vec();
        raw.extend_from_slice(&[0; 1024]);
        let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
        gzip.write_all(&raw).unwrap();
        let bytes = gzip.finish().unwrap();
        let root = tempfile::tempdir().unwrap();
        let old = current_platform().unwrap();
        let old_path = cache_path(root.path(), &old).unwrap();
        for file in [
            &old.executable[..],
            "package/LICENSE",
            "package/lib/lib.d.ts",
        ] {
            let path = old_path.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"old cache").unwrap();
        }
        let mut incoming = old.clone();
        incoming.integrity = format!("sha512-{}", STANDARD.encode(Sha512::digest(&bytes)));
        let result = fetch_to(root.path(), &incoming, || Ok(bytes));
        assert!(result.is_err());
        assert!(!root.path().join("escape").exists());
        assert!(!cache_path(root.path(), &incoming).unwrap().exists());
        assert!(complete(&old_path, &old));
        assert_eq!(
            std::fs::read(old_path.join(&old.executable)).unwrap(),
            b"old cache"
        );
        assert!(fetch_to(root.path(), &old, || Err("network failure".into())).is_ok());
        assert!(fetch_to(root.path(), &incoming, || Err("network failure".into())).is_err());
        assert_eq!(
            std::fs::read(old_path.join(&old.executable)).unwrap(),
            b"old cache"
        );
    }

    #[test]
    fn native_tools_fetch_unit_concurrent_publish_is_atomic() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let mut platform = current_platform().unwrap();
        let mut tar = tar::Builder::new(Vec::new());
        let files: [(&str, &[u8]); 3] = [
            (platform.executable.as_str(), b"#!/bin/sh\n"),
            ("package/LICENSE", b"license"),
            ("package/lib/lib.d.ts", b"export {};"),
        ];
        for (path, body) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o755);
            header.set_entry_type(tar::EntryType::Regular);
            tar.append_data(&mut header, path, body).unwrap();
        }
        let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
        gzip.write_all(&tar.into_inner().unwrap()).unwrap();
        let bytes = gzip.finish().unwrap();
        platform.integrity = format!("sha512-{}", STANDARD.encode(Sha512::digest(&bytes)));

        for _ in 0..20 {
            let root = tempfile::tempdir().unwrap();
            let barrier = Arc::new(Barrier::new(2));
            let handles: Vec<_> = (0..2)
                .map(|_| {
                    let root = root.path().to_path_buf();
                    let platform = platform.clone();
                    let bytes = bytes.clone();
                    let barrier = Arc::clone(&barrier);
                    thread::spawn(move || {
                        barrier.wait();
                        fetch_to(&root, &platform, || Ok(bytes))
                    })
                })
                .collect();
            for handle in handles {
                let path = handle.join().unwrap().unwrap();
                assert!(path.is_file());
            }
            let names: Vec<String> = std::fs::read_dir(root.path())
                .unwrap()
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            assert!(!names.iter().any(|n| n.starts_with(".stage-")));
            let published: Vec<String> = names
                .iter()
                .filter(|n| !n.starts_with(".stage-"))
                .cloned()
                .collect();
            let target = cache_path(root.path(), &platform).unwrap();
            let target_name = target.file_name().unwrap().to_string_lossy().into_owned();
            assert_eq!(published, vec![target_name]);
            assert!(target.join(&platform.executable).is_file());
            assert!(target.join("package/LICENSE").is_file());
            assert!(target.join("package/lib/lib.d.ts").is_file());
        }
    }
}
