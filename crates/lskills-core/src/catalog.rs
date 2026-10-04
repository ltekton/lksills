//! Consumer-facing read-model of a repo.
//!
//! The catalog is what a *consumer* sees: bundles, their member skills, target
//! agents, and an inferred `depends_on` edge — a skill `a` depends on skill `o`
//! when `a`'s description backticks `o`'s exact full name (and `o != a`). It is
//! a heuristic surfacing of implied prerequisites, not a declared graph.

use serde::Serialize;

use crate::repo::Repo;

/// One skill in the catalog.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogSkill {
    /// Full name.
    pub name: String,
    /// Frontmatter description.
    pub description: String,
    /// Other skills whose exact name this skill's description backticks, sorted.
    pub depends_on: Vec<String>,
}

/// One bundle in the catalog.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogBundle {
    /// Bundle name.
    pub name: String,
    /// Bundle version.
    pub version: String,
    /// Bundle description.
    pub description: String,
    /// Target agent surfaces, sorted.
    pub agents: Vec<String>,
    /// Member skill full names, sorted.
    pub skills: Vec<String>,
}

/// The full catalog.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogResult {
    /// Schema version.
    pub v: u32,
    /// All bundles, sorted by name.
    pub bundles: Vec<CatalogBundle>,
    /// All skills, sorted by name.
    pub skills: Vec<CatalogSkill>,
}

impl CatalogResult {
    /// The catalog is informational; the gate always passes.
    pub fn is_ok(&self) -> bool {
        true
    }
}

/// Build the catalog for a loaded repo.
pub fn catalog(repo: &Repo) -> CatalogResult {
    // All known full names, for the depends_on scan.
    let all_names: Vec<String> = repo.skills.keys().map(ToString::to_string).collect();

    let mut skills: Vec<CatalogSkill> = repo
        .skills
        .values()
        .map(|skill| {
            let name = skill.name.to_string();
            let mut depends_on: Vec<String> = all_names
                .iter()
                .filter(|other| **other != name && backticks(&skill.description, other))
                .cloned()
                .collect();
            depends_on.sort();
            depends_on.dedup();
            CatalogSkill {
                name,
                description: skill.description.clone(),
                depends_on,
            }
        })
        .collect();
    skills.sort_by(|a, b| a.name.cmp(&b.name));

    let mut bundles: Vec<CatalogBundle> = repo
        .bundles
        .iter()
        .map(|bundle| {
            let mut agents: Vec<String> = bundle
                .agents
                .iter()
                .map(|a| a.as_str().to_string())
                .collect();
            agents.sort();
            let mut members: Vec<String> = bundle.skills.iter().map(ToString::to_string).collect();
            members.sort();
            CatalogBundle {
                name: bundle.name.to_string(),
                version: bundle.version.clone(),
                description: bundle.description.clone(),
                agents,
                skills: members,
            }
        })
        .collect();
    bundles.sort_by(|a, b| a.name.cmp(&b.name));

    CatalogResult {
        v: 1,
        bundles,
        skills,
    }
}

/// Whether `text` contains `name` wrapped in backticks (`` `name` ``).
fn backticks(text: &str, name: &str) -> bool {
    let needle = format!("`{name}`");
    text.contains(&needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backticks_requires_exact_wrapped_name() {
        assert!(backticks("run `lgit-pages-html` first", "lgit-pages-html"));
        assert!(!backticks("run lgit-pages-html first", "lgit-pages-html"));
        // Substring of a backticked longer name does not match.
        assert!(!backticks("`lgit-pages-html-extra`", "lgit-pages-html"));
    }
}
