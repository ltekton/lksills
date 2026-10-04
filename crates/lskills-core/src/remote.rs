//! Resolve a `--repo <spec>[@ref]` to a local skills-root by fetching into a
//! ref-keyed cache under the tool's cache dir.
//!
//! Read/delivery verbs accept a git repo in place of a local `--root`; the tool
//! clones/fetches it (plain `git`, reusing the consumer's credential helper) and
//! hands the checkout to [`crate::repo::Repo::load`], which is oblivious to its
//! origin. A pinned SHA/tag is reused as-is; a moving branch fast-forwards;
//! `--refresh` forces a clean re-fetch.
//!
//! Every git invocation goes through [`crate::git`] (`Command::arg` + a `--`
//! separator, never a shell), so a crafted spec/ref cannot inject git options.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::git;

/// The git remote name used for the pinned-SHA fetch path.
const ORIGIN: &str = "origin";
/// Cache-key ref segment when no `@ref` was given.
const DEFAULT_REF: &str = "default";

/// A parsed `--repo` argument: the git spec plus an optional ref.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSpec {
    /// The clone spec handed to git (e.g. `https://github.com/owner/repo`,
    /// `https://…/repo.git`, `git@github.com:owner/repo`). GitHub `owner/repo`
    /// shorthands are expanded to `https://github.com/owner/repo` at parse time
    /// so git credential helpers can authenticate private repos without the
    /// caller needing to spell out the full URL.
    pub spec: String,
    /// An optional branch, tag, or SHA to check out; `None` = default branch.
    pub reference: Option<String>,
}

/// Whether a ref names an object (SHA) or a symbolic branch/tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefKind {
    /// A branch or tag - git resolves either via `clone --branch`.
    Branch,
    /// A full commit hash (SHA-1 40-hex or SHA-256 64-hex) - fetched by object.
    Sha,
}

impl RepoSpec {
    /// Parse `<spec>[@ref]`.
    ///
    /// Splits on the **last** `@`, taking the trailing segment as the ref only
    /// when the leading part still looks like a full spec (contains a `/` after
    /// that `@`). This preserves the `@` in scp-style specs like
    /// `git@github.com:owner/repo` (no `/` follows the `@`) while splitting
    /// `owner/repo@v1.2` and `https://h/o/r.git@main`.
    ///
    /// A bare `owner/repo` shorthand (no scheme, no `:`, exactly one `/`) is
    /// expanded to `https://github.com/owner/repo` so git credential helpers
    /// authenticate private repos without the caller spelling out the full URL.
    pub fn parse(input: &str) -> Result<RepoSpec> {
        let input = input.trim();
        if input.is_empty() {
            return Err(Error::Usage("--repo spec is empty".into()));
        }

        let (raw_spec, reference) = if let Some((spec, reference)) = input.rsplit_once('@') {
            // Only a split whose left side still contains a path separator is a
            // real spec@ref; otherwise the `@` belongs to the spec (scp-style).
            if spec.contains('/') && !reference.is_empty() {
                (spec, Some(reference.to_string()))
            } else {
                (input, None)
            }
        } else {
            (input, None)
        };

        Ok(RepoSpec {
            spec: expand_shorthand(raw_spec),
            reference,
        })
    }

    /// Classify [`Self::reference`] as a SHA or a branch/tag.
    fn ref_kind(&self) -> RefKind {
        match self.reference.as_deref() {
            Some(r) if is_sha(r) => RefKind::Sha,
            _ => RefKind::Branch,
        }
    }
}

/// Expand a bare GitHub shorthand (`owner/repo`) to a full HTTPS URL.
///
/// A shorthand is `owner/repo` with no scheme (`://`), no scp colon (`:` before
/// any `/`), and exactly one `/`. Everything else (full URLs, scp-style,
/// `github.com/owner/repo`) is returned unchanged - git handles those directly.
fn expand_shorthand(spec: &str) -> String {
    let has_scheme = spec.contains("://");
    let has_scp_colon = spec
        .find(':')
        .is_some_and(|i| spec[..i].find('/').is_none());
    let slash_count = spec.chars().filter(|&c| c == '/').count();
    if !has_scheme && !has_scp_colon && slash_count == 1 {
        format!("https://github.com/{spec}")
    } else {
        spec.to_string()
    }
}

/// A ref is treated as a full commit hash when it is 40 or 64 *lowercase*-hex
/// characters (SHA-1 / SHA-256). Anything else is a branch or tag.
///
/// This is a heuristic: a branch or tag that happens to be exactly 40/64
/// lowercase-hex characters would be misclassified as a SHA and fetched by
/// object rather than by `--branch`. Such a ref name is pathological and git can
/// still resolve it either way in most cases; the lowercase requirement keeps a
/// conventional `V1.0`-style or mixed-case tag out of the SHA path.
fn is_sha(r: &str) -> bool {
    matches!(r.len(), 40 | 64)
        && r.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// Fetch `spec` into the ref-keyed cache under `cache_root` and return the local
/// checkout path (a skills-root handed to `Repo::load`).
///
/// `refresh` removes any existing cache entry first, forcing a clean re-fetch.
/// Without it, a pinned SHA/tag entry is reused as-is and a branch/default entry
/// is fast-forwarded to the remote head.
pub fn resolve(spec: &RepoSpec, cache_root: &Path, refresh: bool) -> Result<PathBuf> {
    let dir = cache_root.join("repos").join(cache_key(spec));
    let kind = spec.ref_kind();

    if refresh && dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    }

    if dir.exists() {
        // A pinned SHA is immutable - reuse the checkout untouched. A branch,
        // tag, or the default head may have moved, so fast-forward it (a tag
        // fetch is a harmless no-op).
        if kind == RefKind::Branch {
            update_branch(spec, &dir)?;
        }
        return Ok(dir);
    }

    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }

    match kind {
        RefKind::Branch => {
            git::clone(&spec.spec, spec.reference.as_deref(), true, &dir)?;
        }
        RefKind::Sha => fetch_sha(spec, &dir)?,
    }
    Ok(dir)
}

/// Fast-forward an existing branch/default checkout to its remote head.
fn update_branch(spec: &RepoSpec, dir: &Path) -> Result<()> {
    git::fetch(dir, ORIGIN, spec.reference.as_deref(), true)?;
    // `FETCH_HEAD` is the just-fetched tip for the requested (or default) ref.
    git::reset_hard(dir, "FETCH_HEAD")
}

/// Fetch a pinned commit by object: `init` + `remote add` + `fetch <sha>` +
/// `checkout FETCH_HEAD`, falling back to a full (non-shallow) fetch when the
/// server rejects the shallow SHA-want.
fn fetch_sha(spec: &RepoSpec, dir: &Path) -> Result<()> {
    let sha = spec
        .reference
        .as_deref()
        .expect("ref_kind == Sha implies a reference");

    git::init(dir)?;
    git::remote_add(dir, ORIGIN, &spec.spec)?;

    if git::fetch(dir, ORIGIN, Some(sha), true).is_err() {
        // Some hosts reject a shallow want for an arbitrary SHA; retry full.
        git::fetch(dir, ORIGIN, None, false)?;
    }
    git::checkout(dir, "FETCH_HEAD")
}

/// A filesystem-safe cache-dir name for a spec: `<sanitized-spec>@<ref>`.
///
/// The sanitized form is only a directory name; the raw [`RepoSpec::spec`] is
/// what git actually clones, so lossy sanitization here is fine.
fn cache_key(spec: &RepoSpec) -> String {
    let sanitized: String = spec
        .spec
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let reference = spec.reference.as_deref().unwrap_or(DEFAULT_REF);
    let ref_sanitized: String = reference
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("{sanitized}@{ref_sanitized}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> RepoSpec {
        RepoSpec::parse(s).unwrap()
    }

    #[test]
    fn parses_shorthand_without_ref() {
        // owner/repo shorthand expands to a full GitHub HTTPS URL.
        let got = parse("owner/repo");
        assert_eq!(got.spec, "https://github.com/owner/repo");
        assert_eq!(got.reference, None);
    }

    #[test]
    fn splits_ref_on_shorthand() {
        let got = parse("owner/repo@v1.2");
        assert_eq!(got.spec, "https://github.com/owner/repo");
        assert_eq!(got.reference.as_deref(), Some("v1.2"));
    }

    #[test]
    fn github_com_prefix_not_double_expanded() {
        // github.com/owner/repo has two slashes so it is NOT a bare shorthand
        // and must not become https://github.com/github.com/owner/repo.
        let got = parse("github.com/owner/repo");
        assert_eq!(got.spec, "github.com/owner/repo");
    }

    #[test]
    fn splits_ref_on_https_url() {
        let got = parse("https://host/o/r.git@main");
        assert_eq!(got.spec, "https://host/o/r.git");
        assert_eq!(got.reference.as_deref(), Some("main"));
    }

    #[test]
    fn preserves_scp_style_at() {
        // No `/` follows the `@`, so this is a spec, not spec@ref.
        let got = parse("git@github.com:owner/repo");
        assert_eq!(got.spec, "git@github.com:owner/repo");
        assert_eq!(got.reference, None);
    }

    #[test]
    fn splits_ref_on_scp_style_with_ref() {
        let got = parse("git@github.com:owner/repo@abc123");
        assert_eq!(got.spec, "git@github.com:owner/repo");
        assert_eq!(got.reference.as_deref(), Some("abc123"));
    }

    #[test]
    fn empty_spec_is_usage_error() {
        assert!(matches!(RepoSpec::parse("  "), Err(Error::Usage(_))));
    }

    #[test]
    fn sha_classification() {
        let sha1 = "a".repeat(40);
        let sha256 = "b".repeat(64);
        assert_eq!(parse(&format!("o/r@{sha1}")).ref_kind(), RefKind::Sha);
        assert_eq!(parse(&format!("o/r@{sha256}")).ref_kind(), RefKind::Sha);
        assert_eq!(parse("o/r@main").ref_kind(), RefKind::Branch);
        assert_eq!(parse("o/r@v1.2").ref_kind(), RefKind::Branch);
        // 40 chars but non-hex is a branch name.
        assert_eq!(parse("o/r@zzzz").ref_kind(), RefKind::Branch);
        // Uppercase-hex is not a SHA (matches the lowercase-only heuristic).
        let upper = "A".repeat(40);
        assert_eq!(parse(&format!("o/r@{upper}")).ref_kind(), RefKind::Branch);
    }

    #[test]
    fn cache_key_is_filesystem_safe() {
        let key = cache_key(&parse("https://host/o/r.git@main"));
        assert!(!key.contains('/'));
        assert!(key.ends_with("@main"));
        let default = cache_key(&parse("owner/repo"));
        assert!(default.ends_with("@default"));
    }
}
