//! The agent surfaces a bundle can target.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// A publish target surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Agent {
    /// Claude Code (plugin trees + marketplace).
    Claude,
    /// Codex.
    Codex,
    /// Pi (npm-shaped manifest).
    Pi,
}

impl Agent {
    /// Every agent surface, in canonical order.
    pub const ALL: [Agent; 3] = [Agent::Claude, Agent::Codex, Agent::Pi];

    /// The lowercase wire name (`claude`/`codex`/`pi`).
    pub fn as_str(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
            Agent::Pi => "pi",
        }
    }

    /// Parse a lowercase agent name.
    pub fn parse(name: &str) -> Option<Agent> {
        Agent::ALL.into_iter().find(|a| a.as_str() == name)
    }
}

impl std::fmt::Display for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A sorted, de-duplicated set of agents. Defaults to all three.
pub type AgentSet = BTreeSet<Agent>;

/// The default target set: all agents.
pub fn all_agents() -> AgentSet {
    Agent::ALL.into_iter().collect()
}
