use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

mod common;

#[test]
fn imports_one_bundle_and_records_provenance() {
    let (_source_guard, source) = common::copy_fixture("valid");
    let workarea = TempDir::new().unwrap();

    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .success();

    assert!(workarea.path().join("bundles/demo.toml").is_file());
    assert!(workarea.path().join("skills/demo-hello/SKILL.md").is_file());
    assert!(workarea.path().join("skills/demo-world/SKILL.md").is_file());

    let provenance = fs::read_to_string(workarea.path().join(".lskills/provenance.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&provenance).unwrap();
    assert_eq!(parsed["schema"].as_integer(), Some(1));
    assert_eq!(parsed["imports"][0]["bundle"].as_str(), Some("demo"));
    assert_eq!(
        parsed["imports"][0]["selector"].as_str(),
        Some("bundles/demo.toml")
    );
}

#[test]
fn import_check_does_not_write_the_workarea() {
    let (_source_guard, source) = common::copy_fixture("valid");
    let before = snapshot_tree(&source);
    let workarea = TempDir::new().unwrap();

    common::lskills(workarea.path())
        .args([
            "import",
            source.to_str().unwrap(),
            "--bundle",
            "demo",
            "--check",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("preview"));

    assert_eq!(snapshot_tree(&source), before);
    assert!(workarea.path().read_dir().unwrap().next().is_none());
}

#[test]
fn import_rejects_a_destination_inside_the_origin() {
    let (_source_guard, source) = common::copy_fixture("valid");
    let workarea = source.join("nested-workarea");

    common::lskills(&workarea)
        .args(["import", source.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("overlaps destination"));
    assert!(!workarea.exists());
}

#[test]
fn import_rejects_an_unknown_source_bundle() {
    let (_source_guard, source) = common::copy_fixture("valid");
    let workarea = TempDir::new().unwrap();

    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "missing"])
        .assert()
        .failure()
        .code(2);
    assert!(workarea.path().read_dir().unwrap().next().is_none());
}

#[test]
fn import_rejects_an_origin_without_both_root_directories() {
    let (_source_guard, source) = common::copy_fixture("valid");
    fs::remove_dir_all(source.join("bundles")).unwrap();
    let workarea = TempDir::new().unwrap();

    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("missing required bundles/"));
    assert!(workarea.path().read_dir().unwrap().next().is_none());
}

#[test]
fn import_rejects_a_malformed_source_before_writing() {
    let (_source_guard, source) = common::copy_fixture("valid");
    fs::write(source.join("bundles/demo.toml"), "name = [\n").unwrap();
    let workarea = TempDir::new().unwrap();

    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .failure()
        .code(1);
    assert!(workarea.path().read_dir().unwrap().next().is_none());
}

#[test]
fn import_rejects_a_missing_member_before_writing() {
    let (_source_guard, source) = common::copy_fixture("valid");
    let bundle = source.join("bundles/demo.toml");
    let contents = fs::read_to_string(&bundle)
        .unwrap()
        .replace("demo-world", "demo-missing");
    fs::write(bundle, contents).unwrap();
    let workarea = TempDir::new().unwrap();

    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .failure()
        .code(1);
    assert!(workarea.path().read_dir().unwrap().next().is_none());
}

#[test]
fn list_show_validate_and_publish_recover_import_provenance() {
    let (_source_a_guard, source_a) = common::copy_fixture("valid");
    let (_source_b_guard, source_b) = common::copy_fixture("valid");
    let workarea = TempDir::new().unwrap();

    common::lskills(workarea.path())
        .args(["import", source_a.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .success();
    common::lskills(workarea.path())
        .args(["import", source_b.to_str().unwrap(), "--bundle", "only"])
        .assert()
        .success();

    let list_output = common::lskills(workarea.path())
        .args(["--json", "list"])
        .output()
        .unwrap();
    assert!(list_output.status.success());
    let list: serde_json::Value = serde_json::from_slice(&list_output.stdout).unwrap();
    let demo = list["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|bundle| bundle["name"] == "demo")
        .unwrap();
    assert!(!demo["provenance"].is_null());
    assert_eq!(
        demo["provenance"]["source"],
        source_a.to_string_lossy().to_string()
    );

    let show_output = common::lskills(workarea.path())
        .args(["--json", "show", "demo"])
        .output()
        .unwrap();
    assert!(show_output.status.success());
    let show: serde_json::Value = serde_json::from_slice(&show_output.stdout).unwrap();
    assert_eq!(show["provenance"]["selector"], "bundles/demo.toml");

    common::lskills(workarea.path())
        .arg("validate")
        .assert()
        .success();
    common::lskills(workarea.path())
        .arg("publish")
        .assert()
        .success();
    assert!(
        workarea
            .path()
            .join("plugins/demo/skills/hello/SKILL.md")
            .is_file()
    );
    common::lskills(workarea.path())
        .args(["publish", "--check"])
        .assert()
        .success();
}

#[test]
fn inspection_rejects_dangling_provenance_entries() {
    let (_source_guard, source) = common::copy_fixture("valid");
    let workarea = TempDir::new().unwrap();

    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .success();

    let path = workarea.path().join(".lskills/provenance.toml");
    let contents = fs::read_to_string(&path)
        .unwrap()
        .replace("bundle = \"demo\"", "bundle = \"missing\"");
    fs::write(path, contents).unwrap();

    common::lskills(workarea.path())
        .arg("list")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("missing bundle"));
    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "only"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("missing bundle"));
    common::lskills(workarea.path())
        .args(["publish", "--check"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("missing bundle"));
}

#[test]
fn importing_two_origins_into_one_workarea_keeps_both_provenance_entries() {
    let (_source_a_guard, source_a) = common::copy_fixture("valid");
    let (_source_b_guard, source_b) = common::copy_fixture("valid");
    let source_b_skill = source_b.join("skills/only-codex-tool/SKILL.md");
    let contents = fs::read_to_string(&source_b_skill).unwrap().replace(
        "A codex-only skill.",
        "A distinct codex-only skill from origin B.",
    );
    fs::write(source_b_skill, contents).unwrap();
    let workarea = TempDir::new().unwrap();

    common::lskills(workarea.path())
        .args(["import", source_a.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .success();
    common::lskills(workarea.path())
        .args(["import", source_b.to_str().unwrap(), "--bundle", "only"])
        .assert()
        .success();

    let provenance = fs::read_to_string(workarea.path().join(".lskills/provenance.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&provenance).unwrap();
    let imports = parsed["imports"].as_array().unwrap();
    assert_eq!(imports.len(), 2);
    assert_eq!(imports[0]["bundle"].as_str(), Some("demo"));
    assert_eq!(imports[1]["bundle"].as_str(), Some("only"));
    assert!(
        fs::read_to_string(workarea.path().join("skills/only-codex-tool/SKILL.md"))
            .unwrap()
            .contains("distinct codex-only skill")
    );
}

#[test]
fn collision_fails_without_a_partial_import() {
    let (_source_guard, source) = common::copy_fixture("valid");
    let workarea = TempDir::new().unwrap();

    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .success();

    let before = snapshot_tree(workarea.path());
    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .failure()
        .code(1);
    assert_eq!(snapshot_tree(workarea.path()), before);
}

#[test]
fn member_skill_collision_fails_before_writing_the_bundle() {
    let (_source_guard, source) = common::copy_fixture("valid");
    let workarea = TempDir::new().unwrap();
    let existing = workarea.path().join("skills/only-codex-tool");
    fs::create_dir_all(&existing).unwrap();
    fs::copy(
        source.join("skills/only-codex-tool/SKILL.md"),
        existing.join("SKILL.md"),
    )
    .unwrap();

    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "only"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("import collision"));

    assert!(!workarea.path().join("bundles/only.toml").exists());
    assert!(!workarea.path().join(".lskills/provenance.toml").exists());
}

#[test]
fn failed_provenance_write_rolls_back_the_staged_import() {
    let (_source_guard, source) = common::copy_fixture("valid");
    let workarea = TempDir::new().unwrap();
    fs::create_dir_all(workarea.path().join(".lskills/provenance.toml.tmp")).unwrap();

    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .failure()
        .code(1);

    assert!(!workarea.path().join("bundles/demo.toml").exists());
    assert!(!workarea.path().join("skills/demo-hello").exists());
    assert!(!workarea.path().join(".lskills/provenance.toml").exists());
}

#[test]
fn import_copies_executable_looking_content_without_running_it() {
    let (_source_guard, source) = common::copy_fixture("valid");
    let workarea = TempDir::new().unwrap();
    let sentinel = workarea.path().join("executed");
    let script = source.join("skills/demo-hello/scripts/greet.sh");
    let hidden = source.join("skills/demo-hello/.hidden.bin");
    fs::write(
        &script,
        format!("#!/bin/sh\ntouch {}\n", sentinel.display()),
    )
    .unwrap();
    fs::write(&hidden, [0_u8, 1, 255, 0]).unwrap();

    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .success();

    assert!(!sentinel.exists());
    assert_eq!(
        fs::read(workarea.path().join("skills/demo-hello/.hidden.bin")).unwrap(),
        fs::read(&hidden).unwrap()
    );
    assert_eq!(
        fs::read(workarea.path().join("skills/demo-hello/scripts/greet.sh")).unwrap(),
        fs::read(&script).unwrap()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&script).unwrap().permissions().mode(),
            fs::metadata(workarea.path().join("skills/demo-hello/scripts/greet.sh"))
                .unwrap()
                .permissions()
                .mode()
        );
    }
}

#[cfg(unix)]
#[test]
fn import_rejects_non_utf8_filenames_instead_of_rewriting_them() {
    use std::os::unix::ffi::OsStringExt;

    let (_source_guard, source) = common::copy_fixture("valid");
    let name = std::ffi::OsString::from_vec(vec![0x80]);
    let non_utf8 = source.join("skills/demo-hello").join(name);
    if let Err(error) = fs::write(&non_utf8, [1_u8, 2, 3]) {
        eprintln!("skipping: filesystem rejected non-UTF-8 filename: {error}");
        return;
    }
    let workarea = TempDir::new().unwrap();

    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("non-UTF-8 filename"));
    assert!(workarea.path().read_dir().unwrap().next().is_none());
}

#[cfg(unix)]
#[test]
fn import_rejects_special_file_content_before_loading_the_source() {
    let (_source_guard, source) = common::copy_fixture("valid");
    let fifo = source.join("skills/demo-hello/fifo");
    let status = Command::new("mkfifo").arg(&fifo).status().unwrap();
    if !status.success() {
        eprintln!("skipping: mkfifo unavailable");
        return;
    }
    let workarea = TempDir::new().unwrap();

    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .failure()
        .code(1);
    assert!(!workarea.path().join("bundles/demo.toml").exists());
}

#[cfg(unix)]
#[test]
fn import_rejects_symlink_content_without_partial_writes() {
    let (_source_guard, source) = common::copy_fixture("valid");
    let workarea = TempDir::new().unwrap();
    std::os::unix::fs::symlink("/tmp", source.join("skills/demo-hello/escape")).unwrap();

    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .failure()
        .code(1);

    assert!(!workarea.path().join("bundles/demo.toml").exists());
    assert!(!workarea.path().join(".lskills/provenance.toml").exists());
}

#[test]
fn imports_a_git_origin_and_records_the_resolved_revision() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }
    let (_source_guard, source) = common::copy_fixture("valid");
    git(&source, &["init", "-q", "-b", "main"]);
    git(&source, &["add", "-A"]);
    git(&source, &["commit", "-q", "-m", "seed"]);
    let revision = git(&source, &["rev-parse", "HEAD"]);
    let workarea = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    let origin = format!("file://{}@{revision}", source.display());

    common::lskills(workarea.path())
        .args([
            "--cache-dir",
            cache.path().to_str().unwrap(),
            "import",
            &origin,
            "--bundle",
            "demo",
        ])
        .assert()
        .success();

    let provenance = fs::read_to_string(workarea.path().join(".lskills/provenance.toml")).unwrap();
    assert!(provenance.contains(&format!("git:{revision}")));
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "lskills-test")
        .env("GIT_AUTHOR_EMAIL", "lskills-test@example.invalid")
        .env("GIT_COMMITTER_NAME", "lskills-test")
        .env("GIT_COMMITTER_EMAIL", "lskills-test@example.invalid")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn git_available() -> bool {
    let available = Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    if !available && std::env::var_os("LSKILLS_REQUIRE_GIT").is_some() {
        panic!("git is required but unavailable");
    }
    available
}

#[test]
fn explicit_root_keeps_the_machinery_repository_unchanged() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }
    let (_source_guard, source) = common::copy_fixture("valid");
    let workarea = TempDir::new().unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let before = git(
        repository,
        &["status", "--porcelain", "--untracked-files=all"],
    );

    common::lskills(workarea.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .success();

    let after = git(
        repository,
        &["status", "--porcelain", "--untracked-files=all"],
    );
    assert_eq!(after, before);
}

#[test]
fn import_requires_an_explicit_root() {
    let (_source_guard, source) = common::copy_fixture("valid");
    let accidental_root = TempDir::new().unwrap();
    let mut command = Command::cargo_bin("lskills").unwrap();
    command
        .env("LSKILLS_ROOT", accidental_root.path())
        .args(["import", source.to_str().unwrap(), "--bundle", "demo"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("requires --root"));
    assert!(accidental_root.path().read_dir().unwrap().next().is_none());
}

fn snapshot_tree(root: &Path) -> Vec<String> {
    let mut paths = Vec::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.unwrap();
        let rel = entry.path().strip_prefix(root).unwrap();
        if !rel.as_os_str().is_empty() {
            paths.push(rel.to_string_lossy().into_owned());
        }
    }
    paths.sort();
    paths
}
