//! Offline install: copy a bundle's member skills into each target agent's
//! skills directory so the agent can discover and run them.
//!
//! Each skill is written under its **short** name (bundle prefix stripped) with
//! the frontmatter `name:` rewritten to match, so the directory name is the
//! skill's identity - the layout every agent surface discovers. The per-skill
//! file set is assembled by [`crate::generate::plugin::skill_artifacts`], the
//! same path the Claude plugin tree uses, so an installed skill is byte-for-byte
//! identical to its published form.
//!
//! Install is **idempotent and atomic per skill**: each skill is staged into a
//! sibling temp directory on the same filesystem, then renamed over its final
//! location. A failure part-way leaves already-installed skills intact, and the
//! result reports exactly which landed.

pub mod target;

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{Error, Result};
use crate::generate::plugin::skill_artifacts;
use crate::model::{Agent, AgentSet};
use crate::names::BundleName;
use crate::repo::Repo;

pub use target::{InstallEnv, Scope};

/// One installed skill tree.
#[derive(Debug, Clone, Serialize)]
pub struct InstalledSkill {
    /// The owning bundle.
    pub bundle: String,
    /// The installed (short) skill name - also its directory name.
    pub skill: String,
    /// The agent surface it was installed for.
    pub agent: Agent,
    /// Install path relative to the agent's skills dir (`<short>`), for stable
    /// output; the absolute location depends on scope + env.
    pub dest: String,
}

/// The outcome of an install run.
#[derive(Debug, Clone, Serialize)]
pub struct InstallResult {
    /// Schema version.
    pub v: u32,
    /// Every skill tree written, sorted by (agent, bundle, skill).
    pub installed: Vec<InstalledSkill>,
}

impl InstallResult {
    /// Installing is informational; the gate always passes.
    pub fn is_ok(&self) -> bool {
        true
    }
}

/// Install the named bundles' skills into each target agent's skills dir.
///
/// `agents_filter`, when `Some`, restricts which of a bundle's target agents are
/// written (intersection); `None` installs to all of the bundle's agents. An
/// unknown bundle name is a usage error. A member skill missing from the repo is
/// skipped (matching the plugin generator).
pub fn install(
    repo: &Repo,
    bundles: &[BundleName],
    scope: Scope,
    agents_filter: Option<&AgentSet>,
    env: &InstallEnv,
    project_dir: &Path,
) -> Result<InstallResult> {
    let mut installed = Vec::new();

    for name in bundles {
        let bundle = repo
            .bundles
            .iter()
            .find(|b| b.name == *name)
            .ok_or_else(|| Error::Usage(format!("unknown bundle {:?}", name.as_str())))?;

        for &agent in &bundle.agents {
            if agents_filter.is_some_and(|f| !f.contains(&agent)) {
                continue;
            }
            let skills_dir = target::resolve(agent, scope, env, project_dir).ok_or_else(|| {
                Error::Usage(format!(
                    "cannot resolve {agent} skills dir: set its config-home env var or $HOME"
                ))
            })?;

            for full_name in &bundle.skills {
                let Some(skill) = repo.skills.get(full_name) else {
                    continue;
                };
                let short = full_name.require_prefix(&bundle.name)?;
                install_one(&skills_dir, short.as_str(), &skill.path)?;
                installed.push(InstalledSkill {
                    bundle: bundle.name.to_string(),
                    skill: short.to_string(),
                    agent,
                    dest: short.to_string(),
                });
            }
        }
    }

    installed.sort_by(|a, b| (a.agent, &a.bundle, &a.skill).cmp(&(b.agent, &b.bundle, &b.skill)));

    Ok(InstallResult { v: 1, installed })
}

/// Stage one skill into a sibling temp dir, then atomically swap it into
/// `<skills_dir>/<short>`.
///
/// Containment is enforced two ways. Lexically: `short` is a validated name and
/// every artifact key from [`skill_artifacts`] is routed through
/// [`crate::path::safe_join`], which rejects any `..`/absolute component before a
/// byte is written. Physically: the resolved `skills_dir` and the final skill dir
/// are rejected if either is a **symlink** (`symlink_metadata`), so an install
/// cannot write - or `remove_dir_all` - *through* a symlinked `.claude/skills` (or
/// a symlinked `<short>`) into a directory outside the intended tree.
fn install_one(skills_dir: &Path, short: &str, skill_src: &Path) -> Result<()> {
    std::fs::create_dir_all(skills_dir).map_err(|e| Error::io(skills_dir, e))?;
    reject_symlink_dir(skills_dir)?;

    let final_dir = crate::path::safe_join(skills_dir, Path::new(short))?;
    reject_symlink_dir(&final_dir)?;
    let staging = staging_dir(skills_dir, short);

    // A leftover staging dir from a crashed prior run would poison the swap.
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|e| Error::io(&staging, e))?;
    }

    let artifacts = skill_artifacts(skill_src, short)?;
    for (rel, artifact) in &artifacts {
        let dest = crate::path::safe_join(&staging, Path::new(rel))?;
        artifact.write_to(&dest)?;
    }

    // Atomic swap: remove any prior install, then rename staging into place.
    if final_dir.exists() {
        std::fs::remove_dir_all(&final_dir).map_err(|e| Error::io(&final_dir, e))?;
    }
    std::fs::rename(&staging, &final_dir).map_err(|e| Error::io(&final_dir, e))?;
    Ok(())
}

/// Reject `path` when it exists and is a symlink, so a caller never writes or
/// removes *through* it. A missing path is fine (we are about to create it); a
/// real directory is fine. Uses `symlink_metadata` so the link itself is
/// inspected rather than its target.
fn reject_symlink_dir(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(Error::PathEscape {
            path: path.to_path_buf(),
            root: path.parent().unwrap_or(path).to_path_buf(),
        }),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::io(path, e)),
    }
}

/// A hidden sibling staging path for `short` under `skills_dir` (same
/// filesystem, so the rename into place is atomic).
fn staging_dir(skills_dir: &Path, short: &str) -> PathBuf {
    skills_dir.join(format!(".{short}.lskills-staging"))
}
