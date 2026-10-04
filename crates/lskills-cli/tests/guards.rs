//! Guard tests: each `malformed/*` fixture and the `drifted/` fixture asserts a
//! specific error code (or finding) plus the exit-code contract (2 usage / 1
//! gate). Complements `golden.rs`/`views.rs`, which cover the happy path.
//!
//! All fixtures are copied into a tempdir first (via `copy_fixture`, which
//! recreates symlinks and preserves modes) so a run never mutates the checked-in
//! tree and the symlink case exercises a real link.

mod common;

use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use common::{copy_fixture, lskills};
use serde_json::Value;
use std::process::Command;

/// Run `lskills --json <args...>` against a fresh copy of `fixtures/<name>`,
/// expect the given exit code, and return the parsed JSON stdout.
fn json_fail(name: &str, args: &[&str], code: i32) -> Value {
    let (_tmp, root) = copy_fixture(name);
    let assert = lskills(&root)
        .arg("--json")
        .args(args)
        .assert()
        .failure()
        .code(code);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    serde_json::from_str(&stdout).expect("json stdout")
}

/// Run `lskills --json <args...>` against a fresh copy of `fixtures/<name>`,
/// expect exit 0 (success), and return the parsed JSON stdout.
fn json_ok(name: &str, args: &[&str]) -> Value {
    let (_tmp, root) = copy_fixture(name);
    let assert = lskills(&root).arg("--json").args(args).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    serde_json::from_str(&stdout).expect("json stdout")
}

/// The `{"error":{"code":...}}` envelope's code, or a panic if the shape is off.
fn error_code(value: &Value) -> &str {
    value["error"]["code"].as_str().expect("error.code string")
}

// --- Per-skill malformity: hard `Repo::load` errors, surfaced on a load-only
// --- verb (`list`). All are exit 1 (only `Error::Usage` is exit 2).

#[test]
fn bad_frontmatter_is_load_error() {
    let v = json_fail("malformed/bad-frontmatter", &["list"], 1);
    assert_eq!(error_code(&v), "frontmatter");
}

#[test]
fn bad_skill_name_is_load_error() {
    let v = json_fail("malformed/bad-skill-name", &["list"], 1);
    assert_eq!(error_code(&v), "invalid_name");
}

#[test]
fn bad_bundle_toml_is_load_error() {
    let v = json_fail("malformed/bad-bundle-toml", &["list"], 1);
    assert_eq!(error_code(&v), "bundle");
}

#[test]
fn bad_config_is_load_error_on_release() {
    // `.lskills.toml` is only read by `release`, which loads `RepoConfig`.
    let v = json_fail(
        "malformed/bad-config",
        &["release", "plan", "-b", "demo=patch"],
        1,
    );
    assert_eq!(error_code(&v), "config");
}

// --- Bundle-membership problems: the repo loads; `validate` reports a failing
// --- gate (exit 1) with the expected finding code.

#[test]
fn multi_bundle_fails_validate() {
    let v = json_fail("malformed/multi-bundle", &["validate"], 1);
    let codes = finding_codes(&v);
    assert!(
        codes.contains(&"multi_bundle".to_string()),
        "codes: {codes:?}"
    );
}

#[test]
fn bad_prefix_fails_validate() {
    let v = json_fail("malformed/bad-prefix", &["validate"], 1);
    let codes = finding_codes(&v);
    assert!(
        codes.contains(&"bad_prefix".to_string()),
        "codes: {codes:?}"
    );
}

/// The `code` of every issue in a `validate` result.
fn finding_codes(value: &Value) -> Vec<String> {
    value["issues"]
        .as_array()
        .expect("issues array")
        .iter()
        .map(|i| i["code"].as_str().expect("code string").to_string())
        .collect()
}

// --- Symlink under a skill dir: `walk_skill` rejects *any* symlink (file or
// --- directory) on `publish` - a verb that walks skill files - not on load-only
// --- verbs. The fixture carries both an in-tree link (`link.sh -> ../SKILL.md`)
// --- and one whose target escapes the repo (`escape.sh -> ../../../etc/passwd`);
// --- either alone is rejected, proving the guard is symlink-kind based, not
// --- target-resolution based (so it also stops a traversal escape). Unix-only:
// --- `copy_fixture` recreates the links via `copy_symlink`.

#[cfg(unix)]
#[test]
fn symlink_under_skill_is_rejected_on_publish() {
    let v = json_fail("malformed/symlink-escape", &["publish"], 1);
    assert_eq!(error_code(&v), "path_escape");
}

// --- Drift: a committed-but-stale generated tree. `publish --check` reports the
// --- Changed + Extra entries and exits 1.

#[test]
fn drifted_tree_fails_publish_check() {
    let v = json_fail("drifted", &["publish", "--check"], 1);
    let entries = v["drift"]["entries"].as_array().expect("drift entries");
    let mut kinds: Vec<&str> = entries
        .iter()
        .map(|e| e["kind"].as_str().expect("kind string"))
        .collect();
    kinds.sort_unstable();
    assert!(kinds.contains(&"changed"), "kinds: {kinds:?}");
    assert!(kinds.contains(&"extra"), "kinds: {kinds:?}");
}

// --- Clap parse failure: an invalid invocation is caught before any repo load.
// --- Under `--json` it emits the same `{"error":{code,message}}` envelope to
// --- stdout (code `usage`) and exits 2, so scripting consumers see one shape.

#[test]
fn clap_error_emits_json_envelope_and_exits_2() {
    let assert = Command::cargo_bin("lskills")
        .expect("binary built")
        .env_remove("LSKILLS_ROOT")
        .args(["--json", "no-such-verb"])
        .assert()
        .failure()
        .code(2);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: Value = serde_json::from_str(&stdout).expect("json stdout");
    assert_eq!(error_code(&v), "usage");
}

#[test]
fn clap_error_without_json_writes_stderr_not_stdout() {
    let assert = Command::cargo_bin("lskills")
        .expect("binary built")
        .env_remove("LSKILLS_ROOT")
        .arg("no-such-verb")
        .assert()
        .failure()
        .code(2);
    let out = assert.get_output();
    assert!(out.stdout.is_empty(), "stdout should be empty");
    assert!(!out.stderr.is_empty(), "stderr should carry the message");
}

// --- Description validation: empty description is a hard frontmatter error.

#[test]
fn empty_description_is_load_error() {
    let v = json_fail("malformed/empty-description", &["list"], 1);
    assert_eq!(error_code(&v), "frontmatter");
}

// --- Unresolved link: a relative link in the body pointing to a missing file
// --- is a Warning-level validate finding (gate ok, exit 0).

#[test]
fn unresolved_link_is_validate_warning() {
    let v = json_ok("malformed/unresolved-link", &["validate"]);
    assert!(v["ok"].as_bool().unwrap_or(false), "gate should pass");
    let codes = finding_codes(&v);
    assert!(
        codes.contains(&"unresolved_link".to_string()),
        "codes: {codes:?}"
    );
}

// --- Release apply tags: the JSON result includes a `tags` array with
// --- annotated tag names for every written bundle.

#[test]
fn release_apply_json_includes_tags() {
    let (_tmp, root) = copy_fixture("valid");
    let assert = lskills(&root)
        .arg("--json")
        .args(["release", "apply", "-b", "demo=patch"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: Value = serde_json::from_str(&stdout).expect("json stdout");
    let tags = v["tags"].as_array().expect("tags array");
    assert!(!tags.is_empty(), "tags should not be empty");
    for tag in tags {
        let s = tag.as_str().expect("tag string");
        assert!(s.contains("--v"), "tag {s:?} should contain '--v'");
    }
}

// --- show unknown name: exits 2 (usage error) with no JSON flag.

#[test]
fn show_unknown_name_is_usage_error() {
    let (_tmp, root) = copy_fixture("valid");
    lskills(&root)
        .args(["show", "nope"])
        .assert()
        .failure()
        .code(2);
}
