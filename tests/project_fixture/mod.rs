use std::path::Path;

/// Writes the strict two-file project contract into `root`.
///
/// `anchor.json` carries the canonical clean project metadata and `package.json`
/// carries an empty dependency map. The legacy `wda.json` input is no longer
/// produced by this helper; rejection tests write it explicitly.
pub fn write_manifests(root: &Path) {
    let anchor = include_str!("../fixtures/clean_project/anchor.json");
    std::fs::write(root.join("anchor.json"), anchor).unwrap();
    std::fs::write(root.join("package.json"), "{\"dependencies\":{}}").unwrap();
}
