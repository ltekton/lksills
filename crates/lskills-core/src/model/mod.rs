//! The source-of-truth domain model: agents, skills, and bundles.

pub mod agent;
pub mod bundle;
pub mod skill;

pub use agent::{Agent, AgentSet, all_agents};
pub use bundle::Bundle;
pub use skill::Skill;
