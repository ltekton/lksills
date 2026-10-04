//! Structural validation of a loaded repo.
//!
//! `Repo::load` already rejects malformed *skills* (bad name, missing
//! frontmatter) — that is a hard load error in this port, stricter than the Go
//! tool which folded them into the report. What remains for `validate` are the
//! cross-cutting bundle-membership guards that only make sense once the whole
//! tree is loaded: unknown/miswired members, duplicates, empties, and orphans.
//! Also per-skill content checks that require the full repo context (e.g.
//! `unresolved_link` resolves relative paths against the skill directory).

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::finding::{Finding, gate_ok};
use crate::repo::Repo;

/// A validation finding, tagged with the skill or bundle it concerns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    /// The skill or bundle name the issue concerns.
    pub subject: String,
    /// Severity.
    pub level: crate::finding::Level,
    /// Stable machine code.
    pub code: String,
    /// Human message.
    pub message: String,
}

/// The result of validation.
#[derive(Debug, Clone, Serialize)]
pub struct ValidateResult {
    /// Schema version.
    pub v: u32,
    /// Whether the gate passes (no error-level issues).
    pub ok: bool,
    /// All issues, sorted by `(subject, code)`.
    pub issues: Vec<Issue>,
}

impl ValidateResult {
    /// Whether the gate passes.
    pub fn is_ok(&self) -> bool {
        self.ok
    }
}

/// Validate a loaded repo's bundle membership graph.
pub fn validate(repo: &Repo) -> ValidateResult {
    let mut issues: Vec<Issue> = Vec::new();

    // Track which bundles claim each skill, to catch multi-bundle membership
    // and unbundled skills.
    let mut owners: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for bundle in &repo.bundles {
        let bname = bundle.name.to_string();

        if bundle.agents.is_empty() {
            issues.push(err(&bname, "bad_agents", "bundle targets no agents"));
        }
        if bundle.skills.is_empty() {
            issues.push(err(&bname, "empty_bundle", "bundle lists no skills"));
        }

        for member in &bundle.skills {
            let mname = member.to_string();
            owners.entry(mname.clone()).or_default().push(bname.clone());

            match repo.skills.get(member) {
                None => issues.push(err(
                    &mname,
                    "unknown_skill",
                    format!(
                        "skill {mname:?} is listed by bundle {bname:?} but not found in skills/"
                    ),
                )),
                Some(_) => {
                    if member.require_prefix(&bundle.name).is_err() {
                        issues.push(err(
                            &mname,
                            "bad_prefix",
                            format!("skill {mname:?} is not prefixed by its bundle {bname:?}"),
                        ));
                    }
                }
            }
        }
    }

    // multi_bundle: a skill claimed by more than one bundle.
    for (skill, bundles) in &owners {
        if bundles.len() > 1 {
            let mut names = bundles.clone();
            names.sort();
            names.dedup();
            if names.len() > 1 {
                issues.push(err(
                    skill,
                    "multi_bundle",
                    format!("skill {skill:?} is claimed by multiple bundles: {names:?}"),
                ));
            }
        }
    }

    // unbundled (warning): a loaded skill referenced by no bundle.
    for name in repo.skills.keys() {
        let n = name.to_string();
        if !owners.contains_key(&n) {
            issues.push(Issue {
                subject: n.clone(),
                level: crate::finding::Level::Warning,
                code: "unbundled".into(),
                message: format!("skill {n:?} is not a member of any bundle"),
            });
        }
    }

    // unresolved_link (warning): a relative markdown link in the body points to
    // a file that does not exist in the skill directory.
    for (name, skill) in &repo.skills {
        let n = name.to_string();
        for target in relative_links(&skill.body) {
            let resolved = skill.path.join(&target);
            if !resolved.exists() {
                issues.push(Issue {
                    subject: n.clone(),
                    level: crate::finding::Level::Warning,
                    code: "unresolved_link".into(),
                    message: format!("relative link {target:?} does not exist in skill directory"),
                });
            }
        }
    }

    issues.sort_by(|a, b| (a.subject.as_str(), a.code.as_str()).cmp(&(&b.subject, &b.code)));

    let findings: Vec<Finding> = issues
        .iter()
        .map(|i| Finding {
            level: i.level,
            code: i.code.clone(),
            message: i.message.clone(),
        })
        .collect();

    ValidateResult {
        v: 1,
        ok: gate_ok(&findings),
        issues,
    }
}

fn err(subject: &str, code: &str, message: impl Into<String>) -> Issue {
    Issue {
        subject: subject.to_string(),
        level: crate::finding::Level::Error,
        code: code.to_string(),
        message: message.into(),
    }
}

/// Match markdown link targets: `](target)`. Skips URLs (contain `://`),
/// anchors (start with `#`), and absolute paths.
static LINK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\]\(([^)]+)\)").unwrap());

/// Extract relative link targets from markdown body text.
fn relative_links(body: &str) -> Vec<String> {
    LINK_RE
        .captures_iter(body)
        .filter_map(|cap| {
            let target = cap[1].trim().to_string();
            if target.contains("://") || target.starts_with('#') || target.starts_with('/') {
                None
            } else {
                // Strip any fragment suffix (`file.md#section` -> `file.md`)
                let path = target.split('#').next().unwrap_or(&target).to_string();
                if path.is_empty() { None } else { Some(path) }
            }
        })
        .collect()
}
