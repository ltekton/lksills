//! Read-model for the `list` and `show` verbs.
//!
//! `list` is a whole-repo overview: every bundle with its members, and every
//! skill with the bundles that own it. `show <name>` resolves a single name to
//! either a bundle or a skill and returns a detailed view. Both are pure views
//! over a loaded [`Repo`] — no drift, no gating; they always succeed.

use serde::Serialize;

use crate::error::{Error, Result};
use crate::generate::walk;
use crate::provenance::{ProvenanceEntry, ProvenanceFile};
use crate::repo::Repo;

/// A compact source record shown alongside an imported bundle.
#[derive(Debug, Clone, Serialize)]
pub struct ProvenanceSummary {
    /// Original source locator.
    pub source: String,
    /// Source bundle selector.
    pub selector: String,
    /// Resolved source revision or local digest.
    pub revision: String,
    /// Digest captured during import.
    pub digest: String,
}

impl From<&ProvenanceEntry> for ProvenanceSummary {
    fn from(entry: &ProvenanceEntry) -> Self {
        Self {
            source: entry.source.clone(),
            selector: entry.selector.clone(),
            revision: entry.revision.clone(),
            digest: entry.digest.clone(),
        }
    }
}

/// One bundle row in a `list`.
#[derive(Debug, Clone, Serialize)]
pub struct BundleEntry {
    /// Bundle name.
    pub name: String,
    /// Bundle version.
    pub version: String,
    /// Target agent surfaces, sorted.
    pub agents: Vec<String>,
    /// Member skill full names, sorted.
    pub skills: Vec<String>,
    /// Source provenance, when this bundle was imported by lskills.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<ProvenanceSummary>,
}

/// One skill row in a `list`.
#[derive(Debug, Clone, Serialize)]
pub struct SkillEntry {
    /// Full name.
    pub name: String,
    /// Frontmatter description.
    pub description: String,
    /// Bundles that list this skill, sorted.
    pub bundles: Vec<String>,
}

/// The whole-repo overview.
#[derive(Debug, Clone, Serialize)]
pub struct ListResult {
    /// Schema version.
    pub v: u32,
    /// All bundles, sorted by name.
    pub bundles: Vec<BundleEntry>,
    /// All skills, sorted by name.
    pub skills: Vec<SkillEntry>,
}

impl ListResult {
    /// Listing is informational; the gate always passes.
    pub fn is_ok(&self) -> bool {
        true
    }
}

/// Build the whole-repo overview without provenance.
pub fn list(repo: &Repo) -> ListResult {
    list_with_provenance(repo, &ProvenanceFile::default())
}

/// Build the whole-repo overview with validated provenance data.
pub fn list_with_provenance(repo: &Repo, provenance: &ProvenanceFile) -> ListResult {
    let mut bundles: Vec<BundleEntry> = repo
        .bundles
        .iter()
        .map(|bundle| {
            let mut agents: Vec<String> = bundle
                .agents
                .iter()
                .map(|a| a.as_str().to_string())
                .collect();
            agents.sort();
            let mut skills: Vec<String> = bundle.skills.iter().map(ToString::to_string).collect();
            skills.sort();
            BundleEntry {
                name: bundle.name.to_string(),
                version: bundle.version.clone(),
                agents,
                skills,
                provenance: provenance_for(provenance, bundle.name.as_str()),
            }
        })
        .collect();
    bundles.sort_by(|a, b| a.name.cmp(&b.name));

    let mut skills: Vec<SkillEntry> = repo
        .skills
        .values()
        .map(|skill| {
            let name = skill.name.to_string();
            let mut owners: Vec<String> = repo
                .bundles
                .iter()
                .filter(|b| b.skills.iter().any(|m| m.as_str() == name))
                .map(|b| b.name.to_string())
                .collect();
            owners.sort();
            SkillEntry {
                name,
                description: skill.description.clone(),
                bundles: owners,
            }
        })
        .collect();
    skills.sort_by(|a, b| a.name.cmp(&b.name));

    ListResult {
        v: 1,
        bundles,
        skills,
    }
}

fn provenance_for(provenance: &ProvenanceFile, bundle: &str) -> Option<ProvenanceSummary> {
    provenance
        .imports
        .iter()
        .find(|entry| entry.bundle.as_str() == bundle)
        .map(ProvenanceSummary::from)
}

fn provenances_for(provenance: &ProvenanceFile, bundles: &[String]) -> Vec<ProvenanceSummary> {
    bundles
        .iter()
        .filter_map(|bundle| provenance_for(provenance, bundle))
        .collect()
}

/// The detail returned by `show`: a bundle or a skill.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ShowResult {
    /// A bundle and its resolved members.
    Bundle {
        /// Schema version.
        v: u32,
        /// Bundle name.
        name: String,
        /// Bundle version.
        version: String,
        /// Bundle description.
        description: String,
        /// Author.
        author: String,
        /// Target agent surfaces, sorted.
        agents: Vec<String>,
        /// Source provenance for the bundle, when available.
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<ProvenanceSummary>,
        /// Member skills, sorted, each with a present/missing flag.
        skills: Vec<BundleMember>,
    },
    /// A skill and its files.
    Skill {
        /// Schema version.
        v: u32,
        /// Full name.
        name: String,
        /// Owning bundle names, sorted.
        bundles: Vec<String>,
        /// Frontmatter description.
        description: String,
        /// Publishable, skill-relative file paths (source-only excluded), sorted.
        files: Vec<String>,
        /// Source provenance for owning bundles, when available.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        provenance: Vec<ProvenanceSummary>,
    },
}

/// One member of a bundle in a `show`.
#[derive(Debug, Clone, Serialize)]
pub struct BundleMember {
    /// Member full name.
    pub name: String,
    /// Whether the skill is present in the repo.
    pub present: bool,
}

impl ShowResult {
    /// Showing is informational; the gate always passes.
    pub fn is_ok(&self) -> bool {
        true
    }
}

/// Resolve `name` to a bundle or a skill without provenance.
pub fn show(repo: &Repo, name: &str) -> Result<ShowResult> {
    show_with_provenance(repo, name, &ProvenanceFile::default())
}

/// Resolve `name` to a bundle or a skill with provenance data.
pub fn show_with_provenance(
    repo: &Repo,
    name: &str,
    provenance: &ProvenanceFile,
) -> Result<ShowResult> {
    if let Some(bundle) = repo.bundles.iter().find(|b| b.name.as_str() == name) {
        let mut agents: Vec<String> = bundle
            .agents
            .iter()
            .map(|a| a.as_str().to_string())
            .collect();
        agents.sort();
        let mut skills: Vec<BundleMember> = bundle
            .skills
            .iter()
            .map(|m| BundleMember {
                name: m.to_string(),
                present: repo.skills.contains_key(m),
            })
            .collect();
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        return Ok(ShowResult::Bundle {
            v: 1,
            name: bundle.name.to_string(),
            version: bundle.version.clone(),
            description: bundle.description.clone(),
            author: bundle.author.clone(),
            agents,
            provenance: provenance_for(provenance, bundle.name.as_str()),
            skills,
        });
    }

    if let Some(skill) = repo.skills.values().find(|s| s.name.as_str() == name) {
        let mut bundles: Vec<String> = repo
            .bundles
            .iter()
            .filter(|b| b.skills.iter().any(|m| m.as_str() == name))
            .map(|b| b.name.to_string())
            .collect();
        bundles.sort();
        let files = walk::walk_skill(&skill.path)?
            .into_iter()
            .map(|f| f.rel_posix)
            .collect();
        let source_provenance = provenances_for(provenance, &bundles);
        return Ok(ShowResult::Skill {
            v: 1,
            name: skill.name.to_string(),
            bundles,
            description: skill.description.clone(),
            files,
            provenance: source_provenance,
        });
    }

    Err(Error::Usage(format!("no bundle or skill named {name:?}")))
}
