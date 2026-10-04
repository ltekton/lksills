//! Resolve an agent's on-disk skills directory for install.
//!
//! A pure resolver: every environment value it needs is injected via
//! [`InstallEnv`], so the resolution matrix is unit-testable with no ambient
//! `std::env` access and the process tests can set real env vars. The CLI builds
//! one [`InstallEnv`] from `std::env` at the boundary and passes it down.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::model::Agent;

/// Install scope: a project-local tree (default) or the agent's global home.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// `<project>/.{claude,codex,pi}/skills` (default).
    #[default]
    Project,
    /// `<agent-home>/skills`.
    Global,
}

/// Injected environment for target resolution.
///
/// Each field mirrors an environment variable the agents own; `home` is `$HOME`
/// used to build each agent's default config home when its env var is unset.
#[derive(Debug, Clone, Default)]
pub struct InstallEnv {
    /// `$HOME`, for default config homes.
    pub home: Option<PathBuf>,
    /// `$CLAUDE_CONFIG_DIR`.
    pub claude_config_dir: Option<PathBuf>,
    /// `$CODEX_HOME`.
    pub codex_home: Option<PathBuf>,
    /// `$PI_CODING_AGENT_DIR`.
    pub pi_coding_agent_dir: Option<PathBuf>,
}

impl InstallEnv {
    /// The agent's global config home: its env var if set, else the `$HOME`-based
    /// default (`~/.claude`, `~/.codex`, `~/.pi/agent`). `None` only if the env
    /// var is unset *and* `$HOME` is unknown.
    fn config_home(&self, agent: Agent) -> Option<PathBuf> {
        let (explicit, default_rel): (&Option<PathBuf>, &[&str]) = match agent {
            Agent::Claude => (&self.claude_config_dir, &[".claude"]),
            Agent::Codex => (&self.codex_home, &[".codex"]),
            Agent::Pi => (&self.pi_coding_agent_dir, &[".pi", "agent"]),
        };
        if let Some(dir) = explicit {
            return Some(dir.clone());
        }
        let mut home = self.home.clone()?;
        for part in default_rel {
            home.push(part);
        }
        Some(home)
    }
}

/// The project-scope dotdir name for an agent (`.claude`/`.codex`/`.pi`).
fn project_dotdir(agent: Agent) -> &'static str {
    match agent {
        Agent::Claude => ".claude",
        Agent::Codex => ".codex",
        Agent::Pi => ".pi",
    }
}

/// Resolve the skills directory for `agent` at `scope`.
///
/// Global: `<config-home>/skills`. Project: `<project_dir>/.<agent>/skills`.
/// Returns `None` for global scope only when the home cannot be determined
/// (env var unset and `$HOME` unknown).
pub fn resolve(
    agent: Agent,
    scope: Scope,
    env: &InstallEnv,
    project_dir: &Path,
) -> Option<PathBuf> {
    match scope {
        Scope::Global => Some(env.config_home(agent)?.join("skills")),
        Scope::Project => Some(project_dir.join(project_dotdir(agent)).join("skills")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> InstallEnv {
        InstallEnv {
            home: Some(PathBuf::from("/home/u")),
            ..Default::default()
        }
    }

    #[test]
    fn global_uses_env_var_when_set() {
        let mut e = env();
        e.claude_config_dir = Some(PathBuf::from("/custom/claude"));
        assert_eq!(
            resolve(Agent::Claude, Scope::Global, &e, Path::new("/proj")).unwrap(),
            PathBuf::from("/custom/claude/skills")
        );
    }

    #[test]
    fn global_falls_back_to_home_defaults() {
        let e = env();
        assert_eq!(
            resolve(Agent::Claude, Scope::Global, &e, Path::new("/proj")).unwrap(),
            PathBuf::from("/home/u/.claude/skills")
        );
        assert_eq!(
            resolve(Agent::Codex, Scope::Global, &e, Path::new("/proj")).unwrap(),
            PathBuf::from("/home/u/.codex/skills")
        );
        assert_eq!(
            resolve(Agent::Pi, Scope::Global, &e, Path::new("/proj")).unwrap(),
            PathBuf::from("/home/u/.pi/agent/skills")
        );
    }

    #[test]
    fn global_without_home_or_env_is_none() {
        let e = InstallEnv::default();
        assert!(resolve(Agent::Claude, Scope::Global, &e, Path::new("/proj")).is_none());
    }

    #[test]
    fn project_uses_dotdirs_under_project() {
        let e = InstallEnv::default();
        assert_eq!(
            resolve(Agent::Claude, Scope::Project, &e, Path::new("/proj")).unwrap(),
            PathBuf::from("/proj/.claude/skills")
        );
        assert_eq!(
            resolve(Agent::Codex, Scope::Project, &e, Path::new("/proj")).unwrap(),
            PathBuf::from("/proj/.codex/skills")
        );
        assert_eq!(
            resolve(Agent::Pi, Scope::Project, &e, Path::new("/proj")).unwrap(),
            PathBuf::from("/proj/.pi/skills")
        );
    }
}
