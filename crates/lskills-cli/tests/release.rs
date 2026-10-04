//! Process tests for `version` and `release plan|apply`. These exercise the
//! real binary against a fresh copy of `fixtures/valid` (SemVer scheme, the
//! default). CalVer's clock-driven golden lives as a library test in core,
//! since the binary uses the real clock.

mod common;

use std::path::Path;
use std::process::Command;

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use common::{copy_fixture, lskills};

/// Parse the JSON stdout of a successful `lskills --json <args>` run.
fn json(root: &Path, args: &[&str]) -> serde_json::Value {
    let assert = lskills(root).arg("--json").args(args).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    serde_json::from_str(&stdout).expect("valid json")
}

#[test]
fn version_json_reports_tool_version() {
    // `version` needs no skills-root; run it bare.
    let mut cmd = Command::cargo_bin("lskills").expect("binary built");
    cmd.env_remove("LSKILLS_ROOT");
    let assert = cmd.args(["--json", "version"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(value["v"], 1);
    assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn plan_semver_minor_is_deterministic() {
    let (_tmp, root) = copy_fixture("valid");
    insta::assert_json_snapshot!(
        "release_plan_demo_minor",
        json(&root, &["release", "plan", "-b", "demo=minor"])
    );
}

#[test]
fn plan_only_touches_requested_bundles() {
    let (_tmp, root) = copy_fixture("valid");
    let value = json(&root, &["release", "plan", "-b", "demo=major"]);
    let bundles = value["bundles"].as_array().unwrap();
    assert_eq!(bundles.len(), 1, "only the requested bundle appears");
    assert_eq!(bundles[0]["name"], "demo");
    assert_eq!(bundles[0]["to"], "2.0.0");
}

#[test]
fn plan_unknown_bundle_is_usage_error() {
    let (_tmp, root) = copy_fixture("valid");
    lskills(&root)
        .args(["release", "plan", "-b", "nope=minor"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn apply_rewrites_only_the_version_line() {
    let (_tmp, root) = copy_fixture("valid");
    let toml = root.join("bundles/demo.toml");
    let before = std::fs::read_to_string(&toml).unwrap();

    lskills(&root)
        .args(["release", "apply", "-b", "demo=patch"])
        .assert()
        .success();

    let after = std::fs::read_to_string(&toml).unwrap();
    assert_eq!(
        after,
        before.replace("version = \"1.2.0\"", "version = \"1.2.1\""),
        "only the version line changes; all other formatting is preserved"
    );

    // `show` reflects the new version through a fresh load.
    let shown = json(&root, &["show", "demo"]);
    assert_eq!(shown["version"], "1.2.1");
}

#[test]
fn apply_commit_without_git_repo_is_usage_error() {
    // A fresh fixture copy is not a git work tree.
    let (_tmp, root) = copy_fixture("valid");
    lskills(&root)
        .args(["release", "apply", "-b", "demo=patch", "--commit"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn apply_commit_lands_a_commit() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }
    let (_tmp, root) = copy_fixture("valid");
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "test@example.com"]);
    git(&root, &["config", "user.name", "test"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "seed"]);

    lskills(&root)
        .args(["release", "apply", "-b", "demo=patch", "--commit"])
        .assert()
        .success();

    // The working tree is clean: the bump was committed, not just written.
    let out = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--porcelain"])
        .output()
        .expect("git status");
    assert!(
        out.stdout.is_empty(),
        "working tree should be clean after --commit"
    );

    let log = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["log", "-1", "--format=%s"])
        .output()
        .expect("git log");
    let subject = String::from_utf8(log.stdout).unwrap();
    assert!(subject.contains("release"), "commit subject: {subject:?}");
}

/// Whether a `git` binary is on PATH; a missing `git` is a hard failure when
/// `LSKILLS_REQUIRE_GIT` is set (CI) so the test can never silently no-op.
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

/// Run a git command in `dir`, asserting success.
fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}
