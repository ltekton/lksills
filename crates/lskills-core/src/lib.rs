//! lskills engine: author agent skills once, publish them natively to claude,
//! codex, and pi.
//!
//! This crate holds all domain logic and never calls `clap` or `process::exit`.
//! The CLI crate is a thin shell over it.

pub mod catalog;
pub mod config;
pub mod dirs;
pub mod doctor;
pub mod drift;
pub mod error;
pub mod finding;
pub mod frontmatter;
pub mod generate;
pub mod git;
pub mod hygiene;
pub mod import;
pub mod install;
pub mod list;
pub mod model;
pub mod names;
pub mod path;
pub mod provenance;
pub mod publish;
pub mod release;
pub mod remote;
pub mod render;
pub mod repo;
pub mod root;
pub mod scaffold;
pub mod skillfile;
pub mod tokens;
pub mod validate;

pub use error::{Error, Result};
pub use finding::{Finding, Level};
pub use install::{InstallEnv, InstallResult, Scope};
pub use model::{Agent, AgentSet, Bundle, Skill, all_agents};
pub use names::{BundleName, SkillFullName, SkillShortName};
pub use remote::RepoSpec;
pub use render::{Artifact, RenderMap};
pub use repo::Repo;
