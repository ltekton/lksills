//! Process tests for `install` (env-relocation + project scope, hermetic) and
//! the scaffold verbs. Install trees are asserted structurally rather than
//! snapshotted, since absolute install paths depend on per-test tempdirs.

mod common;

use std::path::Path;

use assert_cmd::assert::OutputAssertExt;
use common::{copy_fixture, lskills};
use tempfile::TempDir;

/// A `lskills` command rooted at `root` with all three agent home vars pointed
/// at distinct dirs under `homes`, so a global install is fully relocated and
/// cannot touch the real `~/.claude` etc.
fn lskills_relocated(root: &Path, homes: &Path) -> std::process::Command {
    let mut cmd = lskills(root);
    cmd.env("CLAUDE_CONFIG_DIR", homes.join("claude"))
        .env("CODEX_HOME", homes.join("codex"))
        .env("PI_CODING_AGENT_DIR", homes.join("pi"))
        .env("HOME", homes.join("nonexistent-home"));
    cmd
}

#[test]
fn global_install_lands_under_relocated_homes() {
    let (_tmp, root) = copy_fixture("valid");
    let homes = TempDir::new().unwrap();

    lskills_relocated(&root, homes.path())
        .args(["install", "-g", "demo"])
        .assert()
        .success();

    // demo targets claude + pi (not codex): skills land under each relocated
    // home, by short name, with rewritten frontmatter.
    for agent in ["claude", "pi"] {
        for short in ["hello", "world"] {
            let md = homes
                .path()
                .join(agent)
                .join("skills")
                .join(short)
                .join("SKILL.md");
            assert!(md.exists(), "expected {md:?}");
            let text = std::fs::read_to_string(&md).unwrap();
            assert!(
                text.contains(&format!("name: {short}")),
                "frontmatter name rewritten to short name in {md:?}"
            );
        }
    }
    // codex is not a demo target: nothing installed there.
    assert!(!homes.path().join("codex/skills/hello").exists());
    // The default HOME-based tree was never touched.
    assert!(
        !homes
            .path()
            .join("nonexistent-home/.claude/skills")
            .exists()
    );
}

#[test]
fn project_install_lands_under_dotdirs() {
    let (_tmp, root) = copy_fixture("valid");
    let proj = TempDir::new().unwrap();

    lskills(&root)
        .args(["install", "--project-dir"])
        .arg(proj.path())
        .arg("demo")
        .assert()
        .success();

    assert!(proj.path().join(".claude/skills/hello/SKILL.md").exists());
    assert!(proj.path().join(".pi/skills/world/SKILL.md").exists());
    assert!(!proj.path().join(".codex/skills").exists());
}

#[test]
fn agents_filter_restricts_targets() {
    let (_tmp, root) = copy_fixture("valid");
    let proj = TempDir::new().unwrap();

    lskills(&root)
        .args(["install", "--project-dir"])
        .arg(proj.path())
        .args(["--agents", "claude", "demo"])
        .assert()
        .success();

    assert!(proj.path().join(".claude/skills/hello/SKILL.md").exists());
    assert!(!proj.path().join(".pi/skills").exists());
}

#[test]
fn install_is_idempotent() {
    let (_tmp, root) = copy_fixture("valid");
    let proj = TempDir::new().unwrap();
    let install = || {
        lskills(&root)
            .args(["install", "--project-dir"])
            .arg(proj.path())
            .arg("demo")
            .assert()
            .success();
    };
    install();
    let first = std::fs::read(proj.path().join(".claude/skills/hello/SKILL.md")).unwrap();
    install();
    let second = std::fs::read(proj.path().join(".claude/skills/hello/SKILL.md")).unwrap();
    assert_eq!(first, second, "re-install produces an identical tree");
    // No staging dir is left behind.
    let stale: Vec<_> = std::fs::read_dir(proj.path().join(".claude/skills"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("staging"))
        .collect();
    assert!(stale.is_empty(), "staging dirs cleaned up");
}

#[test]
fn install_unknown_bundle_is_usage_error() {
    let (_tmp, root) = copy_fixture("valid");
    let proj = TempDir::new().unwrap();
    lskills(&root)
        .args(["install", "--project-dir"])
        .arg(proj.path())
        .arg("nope")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn install_without_any_bundle_is_usage_error() {
    let (_tmp, root) = copy_fixture("valid");
    lskills(&root).arg("install").assert().failure().code(2);
}

#[test]
fn new_skill_then_list_shows_it_and_refuses_overwrite() {
    let (_tmp, root) = copy_fixture("valid");

    lskills(&root)
        .args(["new-skill", "demo-extra"])
        .assert()
        .success();
    assert!(root.join("skills/demo-extra/SKILL.md").exists());

    let assert = lskills(&root).arg("list").assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("demo-extra"));

    lskills(&root)
        .args(["new-skill", "demo-extra"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn new_and_rm_bundle_round_trip() {
    let (_tmp, root) = copy_fixture("valid");

    lskills(&root)
        .args(["new-bundle", "extra"])
        .assert()
        .success();
    assert!(root.join("bundles/extra.toml").exists());

    lskills(&root)
        .args(["rm-bundle", "extra"])
        .assert()
        .success();
    assert!(!root.join("bundles/extra.toml").exists());

    // Removing a now-absent bundle is a usage error.
    lskills(&root)
        .args(["rm-bundle", "extra"])
        .assert()
        .failure()
        .code(2);
}
