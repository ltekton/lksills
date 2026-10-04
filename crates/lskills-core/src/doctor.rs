//! Aggregate health check: drift + validate + hygiene, folded into one gate.
//!
//! `doctor` is the single "is this repo publishable and clean?" verb. It runs
//! the three gating checks (generated-tree drift, structural validation, and
//! the hygiene scan) plus a token summary, and reports an overall `ok` that is
//! the AND of every gate. Each sub-report is embedded verbatim so a consumer
//! can drill in without re-running the individual verbs.

use serde::Serialize;

use crate::drift::{self, DriftReport};
use crate::error::Result;
use crate::generate;
use crate::hygiene::{self, HygieneResult};
use crate::repo::Repo;
use crate::tokens::{self, Totals};
use crate::validate::{self, ValidateResult};

/// Repo-shape counts, for a quick at-a-glance summary.
#[derive(Debug, Clone, Serialize)]
pub struct Counts {
    /// Number of loaded skills.
    pub skills: usize,
    /// Number of loaded bundles.
    pub bundles: usize,
}

/// The aggregate doctor report.
#[derive(Debug, Clone, Serialize)]
pub struct DoctorResult {
    /// Schema version.
    pub v: u32,
    /// Overall gate: true only if drift is clean and validate + hygiene pass.
    pub ok: bool,
    /// Repo-shape counts.
    pub counts: Counts,
    /// Generated-tree drift against disk.
    pub drift: DriftReport,
    /// Structural validation issues.
    pub validate: ValidateResult,
    /// Style/secret/coreutils scan.
    pub hygiene: HygieneResult,
    /// Repo-wide token totals (informational).
    pub tokens: Totals,
}

impl DoctorResult {
    /// Whether every gate passes.
    pub fn is_ok(&self) -> bool {
        self.ok
    }
}

/// Run the full health check for a loaded repo.
///
/// `tool_version` is injected (not self-read) so the pi-manifest render is
/// deterministic; it flows through to drift detection.
pub fn doctor(repo: &Repo, tool_version: &str) -> Result<DoctorResult> {
    let map = generate::render(repo, tool_version)?;
    let drift = drift::compute(&repo.root, &map)?;
    let validate = validate::validate(repo);
    let hygiene = hygiene::scan(&repo.root)?;
    let tokens = tokens::report(repo).totals;

    let ok = drift.is_clean() && validate.is_ok() && hygiene.is_ok();

    Ok(DoctorResult {
        v: 1,
        ok,
        counts: Counts {
            skills: repo.skills.len(),
            bundles: repo.bundles.len(),
        },
        drift,
        validate,
        hygiene,
        tokens,
    })
}
