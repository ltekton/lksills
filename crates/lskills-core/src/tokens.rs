//! Estimate the context cost of skills and bundles.
//!
//! A skill spends context two ways: an *always-on* cost (`name: description`,
//! injected into every session's skill index) and an *on-invoke* cost (the full
//! SKILL.md body, loaded only when it fires). The estimate is a deterministic
//! `ceil(chars / CHARS_PER_TOKEN)` heuristic — rough, not a real tokenizer, but
//! stable enough to golden-test and watch for drift.

use serde::Serialize;

use crate::repo::Repo;

/// The heuristic divisor.
pub const CHARS_PER_TOKEN: usize = 4;

/// A rough token count for a string; empty is zero. Counts Unicode scalar
/// values (chars), matching `ceil(len / CHARS_PER_TOKEN)`.
pub fn estimate(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    let n = text.chars().count();
    n.div_ceil(CHARS_PER_TOKEN)
}

/// One skill's token cost.
#[derive(Debug, Clone, Serialize)]
pub struct SkillRow {
    /// Skill full name.
    pub name: String,
    /// Always-on cost (`name: description`).
    pub always_on: usize,
    /// On-invoke cost (SKILL.md body).
    pub body: usize,
}

/// One bundle's aggregated token cost.
#[derive(Debug, Clone, Serialize)]
pub struct BundleRow {
    /// Bundle name.
    pub name: String,
    /// Sum of member always-on costs.
    pub always_on: usize,
    /// Sum of member body costs.
    pub body: usize,
    /// Member skill names present in the repo, sorted.
    pub skills: Vec<String>,
}

/// Repo-wide totals.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Totals {
    /// Total always-on across all skills.
    pub always_on: usize,
    /// Total body across all skills.
    pub body: usize,
}

/// The full token report for a repo.
#[derive(Debug, Clone, Serialize)]
pub struct TokensResult {
    /// Schema version.
    pub v: u32,
    /// The heuristic divisor used.
    pub chars_per_token: usize,
    /// Per-skill rows, sorted by name.
    pub skills: Vec<SkillRow>,
    /// Per-bundle rows, sorted by name.
    pub bundles: Vec<BundleRow>,
    /// Repo-wide totals over all skills.
    pub totals: Totals,
}

impl TokensResult {
    /// Token estimates are always informational; the gate always passes.
    pub fn is_ok(&self) -> bool {
        true
    }
}

/// Build the token report. Skills sum into their bundles by full name; totals
/// are over all skills (not bundle sums, which would double-count shared skills).
pub fn report(repo: &Repo) -> TokensResult {
    let mut skills = Vec::new();
    let mut totals = Totals::default();

    for (name, skill) in &repo.skills {
        let always_on = estimate(&format!("{}: {}", skill.name, skill.description));
        // On-invoke cost is the whole SKILL.md as loaded when the skill fires
        // (frontmatter included), read from disk; fall back to the parsed body.
        let md = skill.path.join("SKILL.md");
        let body_text = std::fs::read_to_string(&md).unwrap_or_else(|_| skill.body.clone());
        let body = estimate(&body_text);
        totals.always_on += always_on;
        totals.body += body;
        skills.push(SkillRow {
            name: name.to_string(),
            always_on,
            body,
        });
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));

    let mut bundles = Vec::new();
    for bundle in &repo.bundles {
        let mut row = BundleRow {
            name: bundle.name.to_string(),
            always_on: 0,
            body: 0,
            skills: Vec::new(),
        };
        for member in &bundle.skills {
            if let Some(skill) = skills.iter().find(|s| s.name == member.as_str()) {
                row.always_on += skill.always_on;
                row.body += skill.body;
                row.skills.push(member.to_string());
            }
        }
        row.skills.sort();
        bundles.push(row);
    }
    bundles.sort_by(|a, b| a.name.cmp(&b.name));

    TokensResult {
        v: 1,
        chars_per_token: CHARS_PER_TOKEN,
        skills,
        bundles,
        totals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_is_ceil_div_by_chars_per_token() {
        assert_eq!(estimate(""), 0);
        assert_eq!(estimate("abcd"), 1);
        assert_eq!(estimate("abcde"), 2); // ceil(5/4)
        // Counts scalar values, not bytes: a 2-char multibyte string is 1 token.
        assert_eq!(estimate("é!"), 1);
    }
}
