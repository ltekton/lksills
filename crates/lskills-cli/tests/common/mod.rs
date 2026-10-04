//! Shared helpers for integration tests: copy a fixture into a tempdir and run
//! the real `lskills` binary against it.

use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::cargo::CommandCargoExt;
use tempfile::TempDir;

/// Absolute path to a fixture tree under `tests/fixtures/`.
pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Recursively copy `src` into a fresh tempdir, preserving unix mode bits, and
/// return the tempdir (kept alive by the caller) plus the copied root.
///
/// `#[allow(dead_code)]`: not every integration-test binary copies a fixture
/// (the `remote` binary builds its own git work tree), but this module compiles
/// once per binary.
#[allow(dead_code)]
pub fn copy_fixture(name: &str) -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("create tempdir");
    let dst = tmp.path().join(name);
    copy_dir(&fixture(name), &dst);
    (tmp, dst)
}

/// Copy any directory (not necessarily under `tests/fixtures/`) into a fresh tempdir.
/// Returns `(guard, dst_path)`.
#[allow(dead_code)]
pub fn copy_dir_into_tmp(src: &Path) -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("create tempdir");
    let name = src.file_name().unwrap_or_default();
    let dst = tmp.path().join(name);
    copy_dir(src, &dst);
    (tmp, dst)
}

/// Recursively copy a directory tree, preserving symlinks and (on unix) modes.
pub fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("create dst dir");
    for entry in std::fs::read_dir(src).expect("read src dir") {
        let entry = entry.expect("dir entry");
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ft = entry.file_type().expect("file type");
        if ft.is_dir() {
            copy_dir(&from, &to);
        } else if ft.is_symlink() {
            copy_symlink(&from, &to);
        } else {
            std::fs::copy(&from, &to).expect("copy file");
            preserve_mode(&from, &to);
        }
    }
}

#[cfg(unix)]
#[allow(dead_code)]
fn copy_symlink(from: &Path, to: &Path) {
    let target = std::fs::read_link(from).expect("read link");
    std::os::unix::fs::symlink(target, to).expect("recreate symlink");
}

#[cfg(not(unix))]
fn copy_symlink(_from: &Path, _to: &Path) {
    panic!("symlink fixtures are unix-only");
}

#[cfg(unix)]
#[allow(dead_code)]
fn preserve_mode(from: &Path, to: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(from)
        .expect("src meta")
        .permissions()
        .mode();
    std::fs::set_permissions(to, std::fs::Permissions::from_mode(mode)).expect("set mode");
}

#[cfg(not(unix))]
fn preserve_mode(_from: &Path, _to: &Path) {}

/// A `lskills` command rooted at `root`, with a clean environment (no ambient
/// `$LSKILLS_ROOT` leaking in).
#[allow(dead_code)]
pub fn lskills(root: &Path) -> Command {
    let mut cmd = Command::cargo_bin("lskills").expect("binary built");
    cmd.arg("--root").arg(root).env_remove("LSKILLS_ROOT");
    cmd
}

/// An insta settings block that redacts the tempdir path prefix from snapshots.
///
/// The pi manifest `version` is the tool's own `CARGO_PKG_VERSION`, which is
/// stable between deliberate version bumps, so it is snapshotted as-is (a bump
/// is a reviewed change that legitimately updates the snapshot).
///
/// `#[allow(dead_code)]`: this module is compiled once per integration-test
/// binary, and not every binary needs the path-redacting settings (the view
/// verbs emit repo-relative paths only).
#[allow(dead_code)]
pub fn insta_settings(root: &Path) -> insta::Settings {
    let mut settings = insta::Settings::clone_current();
    let root_str = root.to_string_lossy().to_string();
    settings.add_filter(&regex::escape(&root_str), "[ROOT]");
    settings
}
