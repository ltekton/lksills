//! Process tests for `--repo` remote fetch, hermetic (no network): a local bare
//! git repo built in a tempdir stands in for a remote, exercised over `file://`.

mod common;

use std::path::Path;
use std::process::Command;

use assert_cmd::assert::OutputAssertExt;
use common::{fixture, lskills};
use tempfile::TempDir;

/// Run a git command in `dir`, panicking with its stderr on failure. Returns
/// captured stdout (trimmed) so callers can read e.g. a rev-parse result.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .expect("run git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Whether `git` is on PATH; tests skip cleanly when it is not - unless
/// `LSKILLS_REQUIRE_GIT` is set (CI), in which case a missing `git` is a hard
/// failure so the remote suite can never silently no-op on a misconfigured runner.
fn git_available() -> bool {
    let present = Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !present && std::env::var_os("LSKILLS_REQUIRE_GIT").is_some() {
        panic!("git is required (LSKILLS_REQUIRE_GIT set) but not available");
    }
    present
}

/// Copy `tests/fixtures/valid` into a work tree, commit it, and create a bare
/// clone. Returns (tempdir guard, bare-repo path, HEAD sha, tag name).
fn fake_remote() -> (TempDir, std::path::PathBuf, String, String) {
    let tmp = TempDir::new().unwrap();
    let work = tmp.path().join("work");
    copy_tree(&fixture("valid"), &work);

    git(&work, &["init", "-q", "-b", "main"]);
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "seed"]);
    let sha = git(&work, &["rev-parse", "HEAD"]);
    git(&work, &["tag", "v1"]);

    let bare = tmp.path().join("remote.git");
    git(
        tmp.path(),
        &[
            "clone",
            "-q",
            "--bare",
            "--",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );

    (tmp, bare, sha, "v1".to_string())
}

/// A `file://` spec for a bare repo path.
fn file_spec(bare: &Path) -> String {
    format!("file://{}", bare.display())
}

fn copy_tree(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&from, &to);
        } else {
            std::fs::copy(&from, &to).unwrap();
        }
    }
}

/// A `lskills` invocation over `--repo <spec>` with a hermetic cache dir, no
/// `--root`. Mirrors `common::lskills` but for the remote path.
fn lskills_repo(spec: &str, cache: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_lskills"));
    cmd.args(["--repo", spec, "--cache-dir"])
        .arg(cache)
        .env_remove("LSKILLS_ROOT")
        .env_remove("LSKILLS_CACHE_DIR");
    cmd
}

#[test]
fn repo_default_branch_lists_bundles() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }
    let (_tmp, bare, _sha, _tag) = fake_remote();
    let cache = TempDir::new().unwrap();

    let assert = lskills_repo(&file_spec(&bare), cache.path())
        .arg("list")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("demo"), "expected demo bundle: {stdout}");
    assert!(stdout.contains("only"), "expected only bundle: {stdout}");
}

#[test]
fn repo_pinned_tag_and_sha_resolve() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }
    let (_tmp, bare, sha, tag) = fake_remote();
    let cache = TempDir::new().unwrap();
    let spec = file_spec(&bare);

    for reference in [tag.clone(), sha.clone()] {
        lskills_repo(&format!("{spec}@{reference}"), cache.path())
            .arg("list")
            .assert()
            .success();
    }
}

#[test]
fn repo_cache_is_reused_and_refresh_refetches() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }
    let (_tmp, bare, _sha, tag) = fake_remote();
    let cache = TempDir::new().unwrap();
    let spec = format!("{}@{}", file_spec(&bare), tag);

    lskills_repo(&spec, cache.path())
        .arg("list")
        .assert()
        .success();

    // The ref-keyed cache dir now exists under <cache>/repos/.
    let repos = cache.path().join("repos");
    let entry = std::fs::read_dir(&repos)
        .unwrap()
        .next()
        .expect("one cache entry")
        .unwrap()
        .path();
    assert!(entry.join("bundles").is_dir(), "cached skills-root");

    // Drop a sentinel into the cached checkout. A run that *reuses* the pinned
    // cache leaves it untouched; a re-fetch (which removes the dir first) deletes
    // it. This distinguishes reuse from refetch instead of only checking exit 0.
    let sentinel = entry.join(".reuse-sentinel");
    std::fs::write(&sentinel, b"x").unwrap();

    // A pinned-tag second run reuses the cache: the sentinel survives.
    lskills_repo(&spec, cache.path())
        .arg("list")
        .assert()
        .success();
    assert!(
        sentinel.exists(),
        "pinned cache should be reused, not refetched"
    );

    // --refresh forces a clean re-fetch: the dir is recreated, sentinel gone.
    lskills_repo(&spec, cache.path())
        .args(["--refresh", "list"])
        .assert()
        .success();
    assert!(
        !sentinel.exists(),
        "--refresh should re-fetch, dropping the sentinel"
    );
}

#[test]
fn repo_rejected_by_authoring_verb() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }
    let (_tmp, bare, _sha, _tag) = fake_remote();
    let cache = TempDir::new().unwrap();

    lskills_repo(&file_spec(&bare), cache.path())
        .arg("publish")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn repo_conflicts_with_root() {
    // Clap rejects `--repo` + `--root` together (usage error, exit 2). No git
    // needed: parsing fails before any fetch.
    let root = fixture("valid");
    let mut cmd = lskills(&root);
    cmd.args(["--repo", "owner/repo", "list"])
        .assert()
        .failure()
        .code(2);
}
