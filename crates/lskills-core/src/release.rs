//! Plan and apply per-bundle version bumps.
//!
//! A bundle's version lives in its `bundles/<name>.toml` `version` field. The
//! scheme is chosen by [`crate::config::Scheme`]: **SemVer** applies an explicit
//! `--bump` level per bundle; **CalVer** derives a dated `YYYY.M.PATCH` from an
//! injected clock. `plan` computes the changes as pure data; `apply` writes them
//! back, rewriting only the `version` line so the rest of each TOML is untouched.
//! Neither tags nor pushes — the human cuts the tag.

use std::collections::BTreeMap;
use std::path::Path;

use semver::Version;
use serde::Serialize;
use time::OffsetDateTime;

use crate::config::Scheme;
use crate::error::{Error, Result};
use crate::git;
use crate::names::BundleName;
use crate::repo::Repo;

/// A SemVer bump level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BumpLevel {
    /// `x.y.z` -> `x+1.0.0`.
    Major,
    /// `x.y.z` -> `x.y+1.0`.
    Minor,
    /// `x.y.z` -> `x.y.z+1`.
    Patch,
}

impl BumpLevel {
    /// Parse `major`/`minor`/`patch`.
    pub fn parse(s: &str) -> Option<BumpLevel> {
        match s {
            "major" => Some(BumpLevel::Major),
            "minor" => Some(BumpLevel::Minor),
            "patch" => Some(BumpLevel::Patch),
            _ => None,
        }
    }
}

/// One bundle's proposed version change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BundleBump {
    /// Bundle name.
    pub name: String,
    /// Current version string.
    pub from: String,
    /// Proposed new version string.
    pub to: String,
}

/// The proposed set of version changes.
#[derive(Debug, Clone, Serialize)]
pub struct ReleasePlan {
    /// Schema version.
    pub v: u32,
    /// The scheme used.
    pub scheme: Scheme,
    /// Per-bundle changes, sorted by name.
    pub bundles: Vec<BundleBump>,
}

impl ReleasePlan {
    /// Planning is informational; the gate always passes.
    pub fn is_ok(&self) -> bool {
        true
    }
}

/// The outcome of applying a plan.
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseResult {
    /// Schema version.
    pub v: u32,
    /// Repo-relative TOML paths whose `version` line was rewritten, sorted.
    pub written: Vec<String>,
    /// Whether a git commit was made.
    pub committed: bool,
    /// Annotated tag names to create, one per written bundle: `<name>--v<version>`.
    /// Use `git tag -a <tag> -m <tag>` then `git push --follow-tags origin main`.
    pub tags: Vec<String>,
}

impl ReleaseResult {
    /// Applying is informational; the gate always passes.
    pub fn is_ok(&self) -> bool {
        true
    }
}

/// Compute a version plan without touching disk.
///
/// `now` is injected (not read from the system clock) so CalVer output is
/// deterministic and testable. For SemVer, `bumps` selects which bundles change
/// and at what level; a bump naming an unknown bundle is a usage error. For
/// CalVer, every bundle advances to the dated version and passing `--bump` is a
/// usage error (it would otherwise silently no-op).
pub fn plan(
    repo: &Repo,
    scheme: Scheme,
    bumps: &BTreeMap<BundleName, BumpLevel>,
    now: OffsetDateTime,
) -> Result<ReleasePlan> {
    // A bump naming a bundle that does not exist is a usage error.
    for name in bumps.keys() {
        if !repo.bundles.iter().any(|b| b.name == *name) {
            return Err(Error::Usage(format!(
                "--bump names unknown bundle {:?}",
                name.as_str()
            )));
        }
    }

    // CalVer derives every bundle's version from the clock, so `--bump` has no
    // meaning under it. Reject it rather than silently ignoring it, so a caller
    // never believes a level they passed took effect.
    if scheme == Scheme::Calver && !bumps.is_empty() {
        return Err(Error::Usage(
            "--bump is not valid under the CalVer scheme; CalVer bumps every bundle from the clock"
                .to_string(),
        ));
    }

    let mut bundles = Vec::new();
    for bundle in &repo.bundles {
        let from = bundle.version.clone();
        let to = match scheme {
            Scheme::Semver => match bumps.get(&bundle.name) {
                None => continue, // only requested bundles change under SemVer
                Some(level) => semver_bump(&from, *level).map_err(|reason| Error::Bundle {
                    path: bundle.path.clone(),
                    reason,
                })?,
            },
            Scheme::Calver => match calver_next(&from, now) {
                Some(next) => next,
                None => continue, // already at or ahead of the dated version
            },
        };
        bundles.push(BundleBump {
            name: bundle.name.to_string(),
            from,
            to,
        });
    }
    bundles.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(ReleasePlan {
        v: 1,
        scheme,
        bundles,
    })
}

/// Apply a plan: rewrite each bundle's `version` line, optionally commit.
///
/// `--commit` requires a git work tree; otherwise it is a usage error (so a
/// caller does not silently skip the commit they asked for).
pub fn apply(repo: &Repo, plan: &ReleasePlan, commit: bool) -> Result<ReleaseResult> {
    if commit && !git::is_repo(&repo.root) {
        return Err(Error::Usage(format!(
            "--commit requires a git repository at {:?}",
            repo.root
        )));
    }

    let mut written = Vec::new();
    let mut tags = Vec::new();
    for bump in &plan.bundles {
        let bundle = repo
            .bundles
            .iter()
            .find(|b| b.name.as_str() == bump.name)
            .ok_or_else(|| Error::Usage(format!("plan names unknown bundle {:?}", bump.name)))?;

        let text = std::fs::read_to_string(&bundle.path).map_err(|e| Error::io(&bundle.path, e))?;
        let rewritten = rewrite_version(&text, &bump.to).ok_or_else(|| Error::Bundle {
            path: bundle.path.clone(),
            reason: "no `version` key to rewrite".to_string(),
        })?;
        std::fs::write(&bundle.path, rewritten).map_err(|e| Error::io(&bundle.path, e))?;

        written.push(rel_posix(&repo.root, &bundle.path));
        tags.push(format!("{}--v{}", bump.name, bump.to));
    }
    written.sort();
    tags.sort();

    let committed = if commit && !written.is_empty() {
        git::commit(
            &repo.root,
            &format!("release: bump {} bundle(s)", written.len()),
            &written,
        )?;
        true
    } else {
        false
    };

    Ok(ReleaseResult {
        v: 1,
        written,
        committed,
        tags,
    })
}

/// Apply a SemVer bump level to a version string.
fn semver_bump(current: &str, level: BumpLevel) -> std::result::Result<String, String> {
    let v =
        Version::parse(current).map_err(|e| format!("version {current:?} is not semver: {e}"))?;
    let next = match level {
        BumpLevel::Major => Version::new(v.major + 1, 0, 0),
        BumpLevel::Minor => Version::new(v.major, v.minor + 1, 0),
        BumpLevel::Patch => Version::new(v.major, v.minor, v.patch + 1),
    };
    Ok(next.to_string())
}

/// Compute the next CalVer `YYYY.M.PATCH` for `now`, or `None` if `current` is
/// already at or ahead of it.
///
/// Within the same `YYYY.M` as `current`, PATCH increments; a new month resets
/// to `.0`. Monotonic: a `current` that already sorts >= the candidate (e.g. a
/// future-dated version) yields `None` rather than going backward.
fn calver_next(current: &str, now: OffsetDateTime) -> Option<String> {
    let year: u64 = now.year() as u64;
    let month: u64 = u8::from(now.month()) as u64;

    // Compare structurally, not by string prefix: a zero-padded or otherwise
    // off-form current version (`2026.09.3`) must still be recognized as the same
    // year.month, or the bump silently no-ops. Within the same YYYY.M, PATCH
    // increments; a new (later) month resets to `.0`.
    let candidate = match parse_calver(current) {
        Ok((y, m, p)) if (y, m) == (year, month) => (year, month, p + 1),
        _ => (year, month, 0),
    };

    // Monotonic guard: never go backward (e.g. a future-dated current version).
    if let Ok(cur) = parse_calver(current) {
        if candidate <= cur {
            return None;
        }
    }
    Some(format!("{}.{}.{}", candidate.0, candidate.1, candidate.2))
}

/// Parse a `YYYY.M.PATCH` triple for ordering; `None` if not that shape.
fn parse_calver(s: &str) -> std::result::Result<(u64, u64, u64), ()> {
    let mut parts = s.split('.');
    let y = parts.next().and_then(|p| p.parse().ok()).ok_or(())?;
    let m = parts.next().and_then(|p| p.parse().ok()).ok_or(())?;
    let p = parts.next().and_then(|p| p.parse().ok()).ok_or(())?;
    if parts.next().is_some() {
        return Err(());
    }
    Ok((y, m, p))
}

/// Rewrite the first `version = "..."` line in a bundle TOML to `new`, leaving
/// everything else byte-for-byte. Returns `None` if there is no `version` key.
fn rewrite_version(text: &str, new: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut done = false;
    let ends_with_newline = text.ends_with('\n');
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        if !done && is_version_key(line) {
            let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
            out.push_str(&format!("{indent}version = \"{new}\""));
            done = true;
        } else {
            out.push_str(line);
        }
        if lines.peek().is_some() || ends_with_newline {
            out.push('\n');
        }
    }
    done.then_some(out)
}

/// Whether a TOML line assigns the top-level `version` key.
fn is_version_key(line: &str) -> bool {
    let trimmed = line.trim_start();
    matches!(trimmed.strip_prefix("version"), Some(rest) if rest.trim_start().starts_with('='))
}

/// A path's repo-relative forward-slash form under `root`.
fn rel_posix(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Bundle, all_agents};
    use std::path::PathBuf;

    /// A one-bundle repo at `version`, enough to drive `plan`.
    fn repo_with(bundle: &str, version: &str) -> Repo {
        let name = BundleName::parse(bundle.to_string()).unwrap();
        Repo {
            root: PathBuf::from("/tmp/x"),
            skills: BTreeMap::new(),
            bundles: vec![Bundle {
                name: name.clone(),
                version: version.to_string(),
                description: "d".to_string(),
                skills: Vec::new(),
                author: "ltekton".to_string(),
                agents: all_agents(),
                path: PathBuf::from(format!("/tmp/x/bundles/{bundle}.toml")),
            }],
        }
    }

    #[test]
    fn calver_rejects_bump_flag() {
        let repo = repo_with("demo", "2026.9.0");
        let mut bumps = BTreeMap::new();
        bumps.insert(
            BundleName::parse("demo".to_string()).unwrap(),
            BumpLevel::Patch,
        );
        let err = plan(&repo, Scheme::Calver, &bumps, at(2026, 9, 22)).unwrap_err();
        assert!(
            matches!(err, Error::Usage(_)),
            "expected usage error: {err:?}"
        );
    }

    #[test]
    fn calver_no_bump_advances_all_bundles() {
        let repo = repo_with("demo", "2026.8.0");
        let plan = plan(&repo, Scheme::Calver, &BTreeMap::new(), at(2026, 9, 22)).unwrap();
        assert_eq!(plan.bundles.len(), 1);
        assert_eq!(plan.bundles[0].to, "2026.9.0");
    }

    #[test]
    fn semver_bump_levels() {
        assert_eq!(semver_bump("1.2.3", BumpLevel::Major).unwrap(), "2.0.0");
        assert_eq!(semver_bump("1.2.3", BumpLevel::Minor).unwrap(), "1.3.0");
        assert_eq!(semver_bump("1.2.3", BumpLevel::Patch).unwrap(), "1.2.4");
        assert!(semver_bump("not-semver", BumpLevel::Patch).is_err());
    }

    fn at(year: i32, month: u8, day: u8) -> OffsetDateTime {
        use time::{Date, Month, Time};
        let month = Month::try_from(month).unwrap();
        OffsetDateTime::new_utc(
            Date::from_calendar_date(year, month, day).unwrap(),
            Time::MIDNIGHT,
        )
    }

    #[test]
    fn calver_new_month_resets_patch() {
        // A version from a prior month advances to .0 for the current month.
        assert_eq!(
            calver_next("2025.8.4", at(2026, 9, 22)).unwrap(),
            "2026.9.0"
        );
    }

    #[test]
    fn calver_same_month_increments_patch() {
        assert_eq!(
            calver_next("2026.9.0", at(2026, 9, 22)).unwrap(),
            "2026.9.1"
        );
        assert_eq!(
            calver_next("2026.9.7", at(2026, 9, 22)).unwrap(),
            "2026.9.8"
        );
    }

    #[test]
    fn calver_zero_padded_month_still_increments() {
        // A zero-padded current (`2026.09.3`) is the same YYYY.M as an unpadded
        // `2026.9`, so it must bump PATCH, not silently no-op back to `.0`.
        assert_eq!(
            calver_next("2026.09.3", at(2026, 9, 22)).unwrap(),
            "2026.9.4"
        );
    }

    #[test]
    fn calver_future_version_does_not_go_backward() {
        // current is dated ahead of `now`: no bump.
        assert!(calver_next("2027.1.0", at(2026, 9, 22)).is_none());
    }

    #[test]
    fn calver_non_calver_current_starts_fresh() {
        // A SemVer-shaped current under CalVer just starts this month at .0.
        assert_eq!(calver_next("1.2.3", at(2026, 9, 22)).unwrap(), "2026.9.0");
    }

    #[test]
    fn rewrite_version_preserves_other_lines() {
        let toml = "name = \"demo\"\nversion = \"1.2.0\"\nauthor = \"ltekton\"\n";
        let out = rewrite_version(toml, "1.3.0").unwrap();
        assert_eq!(
            out,
            "name = \"demo\"\nversion = \"1.3.0\"\nauthor = \"ltekton\"\n"
        );
    }

    #[test]
    fn rewrite_version_without_key_is_none() {
        assert!(rewrite_version("name = \"demo\"\n", "1.0.0").is_none());
    }

    #[test]
    fn rewrite_version_only_first_key() {
        // A stray later `version =` inside another table is left alone.
        let toml = "version = \"1.0.0\"\n[meta]\nversion = \"x\"\n";
        let out = rewrite_version(toml, "2.0.0").unwrap();
        assert_eq!(out, "version = \"2.0.0\"\n[meta]\nversion = \"x\"\n");
    }
}
