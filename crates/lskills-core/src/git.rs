//! Thin wrapper over the `git` CLI via `std::process::Command`.
//!
//! Chosen over `git2`/`gitoxide` to avoid libgit2/OpenSSL linkage and to reuse
//! the user's existing git config/auth. Every invocation uses `Command::arg`
//! (never a shell) and a `--` separator before any user-controlled path, so a
//! crafted argument cannot be mistaken for a git option.

use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};

/// Whether `dir` is inside a git work tree.
pub fn is_repo(dir: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Return the current commit for a Git checkout, or `None` for a plain local
/// directory or an unavailable Git executable.
pub fn head(dir: &Path) -> Result<Option<String>> {
    let output = match Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
    {
        Ok(output) => output,
        Err(_) => return Ok(None),
    };
    if !output.status.success() {
        return Ok(None);
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

/// Stage `paths` (relative to `dir`) and commit them with `message`.
///
/// Fails with [`Error::Command`] if `git add` or `git commit` returns nonzero
/// (e.g. nothing staged, or `git` is missing).
pub fn commit(dir: &Path, message: &str, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }

    let mut add = Command::new("git");
    add.arg("-C").arg(dir).arg("add").arg("--");
    for p in paths {
        add.arg(p);
    }
    run(add, "git add")?;

    let mut ci = Command::new("git");
    ci.arg("-C").arg(dir).args(["commit", "-m", message, "--"]);
    for p in paths {
        ci.arg(p);
    }
    run(ci, "git commit")?;

    Ok(())
}

/// `git clone [--depth 1] [--branch <reference>] -- <spec> <dir>`.
///
/// `reference` names a branch or tag (git resolves either); `None` clones the
/// remote's default branch. `depth1` requests a shallow clone.
pub fn clone(spec: &str, reference: Option<&str>, depth1: bool, dir: &Path) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("clone");
    if depth1 {
        cmd.args(["--depth", "1"]);
    }
    if let Some(r) = reference {
        cmd.arg("--branch").arg(r);
    }
    cmd.arg("--").arg(spec).arg(dir);
    run(cmd, "git clone")
}

/// `git init <dir>` (creates the directory if needed).
pub fn init(dir: &Path) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("init").arg("--").arg(dir);
    run(cmd, "git init")
}

/// `git -C <dir> remote add <name> -- <spec>`.
pub fn remote_add(dir: &Path, name: &str, spec: &str) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(dir)
        .args(["remote", "add", name])
        .arg("--")
        .arg(spec);
    run(cmd, "git remote add")
}

/// `git -C <dir> fetch [--depth 1] <remote> [<refspec>]`.
pub fn fetch(dir: &Path, remote: &str, refspec: Option<&str>, depth1: bool) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).arg("fetch");
    if depth1 {
        cmd.args(["--depth", "1"]);
    }
    cmd.arg("--").arg(remote);
    if let Some(r) = refspec {
        cmd.arg(r);
    }
    run(cmd, "git fetch")
}

/// `git -C <dir> checkout <rev>`.
pub fn checkout(dir: &Path, rev: &str) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).arg("checkout").arg(rev);
    run(cmd, "git checkout")
}

/// `git -C <dir> reset --hard <rev>`.
pub fn reset_hard(dir: &Path, rev: &str) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).args(["reset", "--hard"]).arg(rev);
    run(cmd, "git reset --hard")
}

/// Run a prepared command, mapping failure to [`Error::Command`].
fn run(mut cmd: Command, label: &str) -> Result<()> {
    let output = cmd.output().map_err(|e| Error::Command {
        command: label.to_string(),
        reason: e.to_string(),
    })?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(Error::Command {
        command: label.to_string(),
        reason: if stderr.is_empty() {
            format!("exited with {}", output.status)
        } else {
            stderr
        },
    })
}
