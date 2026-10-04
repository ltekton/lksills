//! A loaded bundle manifest (`bundles/<name>.toml`).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::agent::{Agent, AgentSet, all_agents};
use crate::names::{BundleName, SkillFullName};

/// The default author when a bundle omits one.
const DEFAULT_AUTHOR: &str = "ltekton";
/// The default version when a bundle omits one.
const DEFAULT_VERSION: &str = "0.0.0";

/// On-disk shape of a bundle TOML. Optional fields reproduce the Go tool's
/// `dict.get(...)` defaults precisely.
#[derive(Debug, Deserialize)]
struct BundleToml {
    name: String,
    version: Option<String>,
    description: Option<String>,
    #[serde(default)]
    skills: Vec<String>,
    author: Option<String>,
    agents: Option<Vec<String>>,
}

/// A loaded, validated bundle.
#[derive(Debug, Clone, Serialize)]
pub struct Bundle {
    /// The bundle's validated name.
    pub name: BundleName,
    /// SemVer version string (defaults to `0.0.0`).
    pub version: String,
    /// Human description.
    pub description: String,
    /// Member skills, validated into full names.
    pub skills: Vec<SkillFullName>,
    /// Author (defaults to `ltekton`).
    pub author: String,
    /// Target agent surfaces (defaults to all three).
    pub agents: AgentSet,
    /// Path to the source TOML.
    #[serde(skip)]
    pub path: PathBuf,
}

impl Bundle {
    /// Parse a bundle TOML into a validated [`Bundle`].
    ///
    /// Member names are parsed *into* [`SkillFullName`], so a bad member name
    /// (or a traversal attempt) fails the load rather than reaching the FS.
    pub fn parse(path: PathBuf, contents: &str) -> Result<Bundle> {
        let raw: BundleToml = toml::from_str(contents).map_err(|e| Error::Bundle {
            path: path.clone(),
            reason: e.to_string(),
        })?;

        let name = BundleName::parse(raw.name).map_err(|e| Error::Bundle {
            path: path.clone(),
            reason: e.to_string(),
        })?;

        let skills = raw
            .skills
            .into_iter()
            .map(SkillFullName::parse)
            .collect::<Result<Vec<_>>>()
            .map_err(|e| Error::Bundle {
                path: path.clone(),
                reason: e.to_string(),
            })?;

        let agents = match raw.agents {
            None => all_agents(),
            Some(list) => {
                let mut set = AgentSet::new();
                for a in list {
                    let agent = Agent::parse(&a).ok_or_else(|| Error::Bundle {
                        path: path.clone(),
                        reason: format!("unknown agent {a:?}"),
                    })?;
                    set.insert(agent);
                }
                set
            }
        };

        Ok(Bundle {
            name,
            version: raw.version.unwrap_or_else(|| DEFAULT_VERSION.to_string()),
            description: raw.description.unwrap_or_default(),
            skills,
            author: raw.author.unwrap_or_else(|| DEFAULT_AUTHOR.to_string()),
            agents,
            path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_apply_when_fields_omitted() {
        let b = Bundle::parse(
            PathBuf::from("bundles/lgit.toml"),
            "name = \"lgit\"\nskills = [\"lgit-pages-html\"]\n",
        )
        .unwrap();
        assert_eq!(b.version, "0.0.0");
        assert_eq!(b.author, "ltekton");
        assert_eq!(b.agents, all_agents());
        assert_eq!(b.skills[0].as_str(), "lgit-pages-html");
    }

    #[test]
    fn bad_member_name_fails_the_load() {
        let err = Bundle::parse(
            PathBuf::from("bundles/x.toml"),
            "name = \"x\"\nskills = [\"../etc/passwd\"]\n",
        );
        assert!(err.is_err());
    }

    #[test]
    fn unknown_agent_fails() {
        let err = Bundle::parse(
            PathBuf::from("bundles/x.toml"),
            "name = \"x\"\nagents = [\"gemini\"]\n",
        );
        assert!(err.is_err());
    }
}
