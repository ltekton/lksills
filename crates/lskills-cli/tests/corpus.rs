//! Golden corpus tests: one directory per case under `tests/corpus/`.
//!
//! Each case has:
//!   `cmd`          — verb + flags (one line; no `--root` or `--json`)
//!   `status`       — expected exit code (optional; absent = 0)
//!   `input/`       — a complete skills-root
//!   `expected/`    — expected generated file tree (for `publish`)
//!   `expected.json`— expected JSON stdout (for read verbs)
//!
//! Run with `UPDATE=1` to regenerate expected outputs instead of diffing.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use assert_cmd::cargo::CommandCargoExt;
use common::copy_dir;
use tempfile::TempDir;

/// Generated-artifact roots that `publish` writes. The corpus tree diff only
/// looks inside these roots, ignoring the source files in `input/`.
const GENERATED_ROOTS: &[&str] = &["plugins", ".claude-plugin", "package.json"];

struct CorpusCase {
    name: String,
    input: PathBuf,
    cmd_args: Vec<String>,
    expected_status: i32,
    expected_tree: Option<PathBuf>,
    expected_json: Option<PathBuf>,
}

fn discover_cases() -> Vec<CorpusCase> {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let mut cases: Vec<CorpusCase> = std::fs::read_dir(&corpus_dir)
        .expect("tests/corpus dir must exist")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| {
            let dir = e.path();
            let name = e.file_name().to_string_lossy().into_owned();

            let cmd_text = std::fs::read_to_string(dir.join("cmd"))
                .unwrap_or_else(|_| panic!("[{name}] missing `cmd` file"));
            let cmd_args: Vec<String> = cmd_text
                .split_ascii_whitespace()
                .map(str::to_string)
                .collect();

            let status_path = dir.join("status");
            let expected_status = if status_path.exists() {
                std::fs::read_to_string(&status_path)
                    .expect("read status")
                    .trim()
                    .parse::<i32>()
                    .expect("status must be an integer")
            } else {
                0
            };

            let expected_tree = dir.join("expected").is_dir().then(|| dir.join("expected"));
            let expected_json = dir
                .join("expected.json")
                .is_file()
                .then(|| dir.join("expected.json"));

            assert!(
                expected_tree.is_some() ^ expected_json.is_some(),
                "[{name}] must have exactly one of `expected/` or `expected.json`"
            );

            CorpusCase {
                name,
                input: dir.join("input"),
                cmd_args,
                expected_status,
                expected_tree,
                expected_json,
            }
        })
        .collect();

    cases.sort_by(|a, b| a.name.cmp(&b.name));
    cases
}

fn run_case(case: &CorpusCase, update: bool) -> Result<(), String> {
    let tmp = TempDir::new().map_err(|e| format!("tempdir: {e}"))?;
    let root = tmp.path().join("root");
    copy_dir(&case.input, &root);

    let json_mode = case.expected_json.is_some();

    let mut cmd =
        std::process::Command::cargo_bin("lskills").map_err(|e| format!("binary: {e}"))?;
    cmd.arg("--root").arg(&root).env_remove("LSKILLS_ROOT");
    if json_mode {
        cmd.arg("--json");
    }
    for arg in &case.cmd_args {
        cmd.arg(arg);
    }

    let output = cmd.output().map_err(|e| format!("spawn: {e}"))?;
    let actual_code = output.status.code().unwrap_or(-1);

    if actual_code != case.expected_status {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "exit {actual_code}, expected {}\nstdout: {stdout}\nstderr: {stderr}",
            case.expected_status
        ));
    }

    if update {
        if let Some(tree_path) = &case.expected_tree {
            regenerate_tree(tree_path, &root)?;
        } else if let Some(json_path) = &case.expected_json {
            regenerate_json(json_path, &output.stdout)?;
        }
        return Ok(());
    }

    if let Some(tree_path) = &case.expected_tree {
        compare_tree(tree_path, &root)
    } else if let Some(json_path) = &case.expected_json {
        compare_json(json_path, &output.stdout)
    } else {
        unreachable!()
    }
}

// --- Tree comparison -------------------------------------------------------

/// Collect all files from a directory tree into a sorted map of relpath -> content.
fn collect_tree(root: &Path) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if root.is_file() {
        let name = root.file_name().unwrap().to_string_lossy().into_owned();
        let content = std::fs::read_to_string(root).unwrap_or_else(|_| "<binary>".into());
        map.insert(name, content);
        return map;
    }
    if !root.is_dir() {
        return map;
    }
    collect_files_into(root, root, &mut map);
    map
}

fn collect_files_into(base: &Path, dir: &Path, map: &mut BTreeMap<String, String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let ft = entry.file_type().expect("file type");
        if ft.is_dir() {
            collect_files_into(base, &path, map);
        } else if ft.is_file() {
            let rel = path
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let content = std::fs::read_to_string(&path).unwrap_or_else(|_| "<binary>".into());
            map.insert(rel, content);
        }
    }
}

/// Collect only the generated artifact files from the output root.
fn collect_generated(root: &Path) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for gen_root in GENERATED_ROOTS {
        let p = root.join(gen_root);
        let sub = collect_tree(&p);
        for (rel, content) in sub {
            let key = if *gen_root == rel {
                rel
            } else {
                format!("{gen_root}/{rel}")
            };
            map.insert(key, content);
        }
    }
    map
}

fn compare_tree(expected_dir: &Path, root: &Path) -> Result<(), String> {
    let expected = collect_tree(expected_dir);
    let actual = collect_generated(root);

    let mut msgs: Vec<String> = Vec::new();

    for key in expected.keys() {
        if !actual.contains_key(key) {
            msgs.push(format!("  Missing:  {key}"));
        }
    }
    for key in actual.keys() {
        if !expected.contains_key(key) {
            msgs.push(format!("  Extra:    {key}"));
        }
    }
    for (key, exp_content) in &expected {
        if let Some(act_content) = actual.get(key) {
            if exp_content != act_content {
                msgs.push(format!("  Changed:  {key}"));
                msgs.push(inline_diff(exp_content, act_content));
            }
        }
    }

    if msgs.is_empty() {
        Ok(())
    } else {
        Err(msgs.join("\n"))
    }
}

fn inline_diff(expected: &str, actual: &str) -> String {
    let exp_lines: Vec<&str> = expected.lines().collect();
    let act_lines: Vec<&str> = actual.lines().collect();
    let max = exp_lines.len().max(act_lines.len());
    let mut out = Vec::new();
    for i in 0..max {
        let e = exp_lines.get(i).copied().unwrap_or("<missing>");
        let a = act_lines.get(i).copied().unwrap_or("<missing>");
        if e != a {
            out.push(format!("    line {}: expected {e:?}", i + 1));
            out.push(format!("    line {}:   actual {a:?}", i + 1));
        }
    }
    out.join("\n")
}

fn regenerate_tree(expected_dir: &Path, root: &Path) -> Result<(), String> {
    if expected_dir.exists() {
        std::fs::remove_dir_all(expected_dir).map_err(|e| format!("remove expected/: {e}"))?;
    }
    std::fs::create_dir_all(expected_dir).map_err(|e| format!("create expected/: {e}"))?;

    for gen_root in GENERATED_ROOTS {
        let src = root.join(gen_root);
        if src.is_file() {
            let dst = expected_dir.join(gen_root);
            std::fs::copy(&src, &dst).map_err(|e| format!("copy {gen_root}: {e}"))?;
        } else if src.is_dir() {
            let dst = expected_dir.join(gen_root);
            copy_dir(&src, &dst);
        }
    }
    Ok(())
}

// --- JSON comparison -------------------------------------------------------

fn compare_json(expected_path: &Path, actual_stdout: &[u8]) -> Result<(), String> {
    let expected_text =
        std::fs::read_to_string(expected_path).map_err(|e| format!("read expected.json: {e}"))?;
    let expected: serde_json::Value =
        serde_json::from_str(&expected_text).map_err(|e| format!("expected.json invalid: {e}"))?;
    let actual_text =
        std::str::from_utf8(actual_stdout).map_err(|e| format!("stdout utf8: {e}"))?;
    let actual: serde_json::Value = serde_json::from_str(actual_text)
        .map_err(|e| format!("stdout not JSON: {e}\n{actual_text}"))?;

    if expected != actual {
        Err(format!(
            "JSON mismatch\n--- expected ---\n{}\n--- actual ---\n{}",
            serde_json::to_string_pretty(&expected).unwrap(),
            serde_json::to_string_pretty(&actual).unwrap(),
        ))
    } else {
        Ok(())
    }
}

fn regenerate_json(expected_path: &Path, actual_stdout: &[u8]) -> Result<(), String> {
    // Pretty-print so the file is human-readable and diffable.
    let actual_text =
        std::str::from_utf8(actual_stdout).map_err(|e| format!("stdout utf8: {e}"))?;
    let value: serde_json::Value =
        serde_json::from_str(actual_text).map_err(|e| format!("stdout not JSON: {e}"))?;
    let pretty = serde_json::to_string_pretty(&value).unwrap() + "\n";
    if let Some(parent) = expected_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    std::fs::write(expected_path, pretty).map_err(|e| format!("write expected.json: {e}"))
}

// --- Test entry point -------------------------------------------------------

#[test]
fn corpus() {
    let update = std::env::var("UPDATE").is_ok();
    let cases = discover_cases();
    assert!(
        !cases.is_empty(),
        "no corpus cases found under tests/corpus/"
    );

    let mut failures: Vec<String> = Vec::new();
    for case in &cases {
        match run_case(case, update) {
            Ok(()) => {
                if update {
                    eprintln!("[{}] updated", case.name);
                }
            }
            Err(msg) => {
                failures.push(format!("[{}]\n{msg}", case.name));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} corpus case(s) failed:\n\n{}",
            failures.len(),
            failures.join("\n\n---\n\n")
        );
    }
}
