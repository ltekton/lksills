//! Path safety helpers shared by the render and install walkers.

use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Result};

/// Names that are authoring-only: never copied into an assembled or installed
/// tree. Matched case-folded against *any* path component.
const SOURCE_ONLY: &[&str] = &["master.md", "mirrors"];

/// Reports whether a skill-relative (forward-slash) path is authoring-only.
///
/// Matches if any component equals a [`SOURCE_ONLY`] name, case-folded — so a
/// nested `sub/mirrors/pi.md` or a `MASTER.MD` is excluded, not only a
/// top-level exact-case match. Both render and install route through this one
/// classifier so their exclusion behavior cannot silently diverge.
pub fn is_source_only(rel_posix: &str) -> bool {
    rel_posix.split('/').any(|part| {
        let lower = part.to_ascii_lowercase();
        SOURCE_ONLY.contains(&lower.as_str())
    })
}

/// Join `rel` onto `root` and prove the result stays inside `root`.
///
/// Purely lexical: rejects any `..` or absolute component before touching the
/// filesystem, so a traversal is caught even if the target does not yet exist.
/// Newtype-validated names never contain `..`, but install targets and
/// caller-supplied relative paths route through here as defense in depth.
pub fn safe_join(root: &Path, rel: &Path) -> Result<PathBuf> {
    for comp in rel.components() {
        match comp {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(Error::PathEscape {
                    path: rel.to_path_buf(),
                    root: root.to_path_buf(),
                });
            }
        }
    }
    Ok(root.join(rel))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_only_matches_any_component_case_folded() {
        assert!(is_source_only("master.md"));
        assert!(is_source_only("sub/MASTER.MD"));
        assert!(is_source_only("mirrors/pi.md"));
        assert!(!is_source_only("scripts/deploy.sh"));
        assert!(!is_source_only("SKILL.md"));
    }

    #[test]
    fn safe_join_allows_normal_and_rejects_traversal() {
        let root = Path::new("/repo/skills");
        assert_eq!(
            safe_join(root, Path::new("a/b.md")).unwrap(),
            PathBuf::from("/repo/skills/a/b.md")
        );
        assert!(safe_join(root, Path::new("../escape")).is_err());
        assert!(safe_join(root, Path::new("/abs")).is_err());
    }
}
