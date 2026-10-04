//! Publish: write the render map to disk (pruning stale generated files), or
//! check for drift without writing.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Serialize;
use walkdir::WalkDir;

use crate::drift::{self, DriftReport};
use crate::error::{Error, Result};
use crate::generate;
use crate::render::{self, RenderMap};
use crate::repo::Repo;

/// Result of a publish run.
#[derive(Debug, Clone, Serialize)]
pub struct PublishResult {
    /// Repo-relative paths written (in `--check` mode, this is empty).
    pub written: Vec<String>,
    /// Repo-relative paths pruned (deleted as stale).
    pub pruned: Vec<String>,
    /// In `--check` mode, the drift found; `None` when writing.
    pub drift: Option<DriftReport>,
}

impl PublishResult {
    /// Whether this run passes its gate. Writing always passes; a check passes
    /// only when the tree was already clean.
    pub fn is_ok(&self) -> bool {
        self.drift.as_ref().is_none_or(DriftReport::is_clean)
    }
}

/// Check for drift without modifying the tree.
pub fn check(repo: &Repo, tool_version: &str) -> Result<PublishResult> {
    let map = generate::render(repo, tool_version)?;
    render::validate_generated_roots(&repo.root)?;
    let report = drift::compute(&repo.root, &map)?;
    Ok(PublishResult {
        written: Vec::new(),
        pruned: Vec::new(),
        drift: Some(report),
    })
}

/// Write the render map to disk, pruning stale files under the generated roots.
///
/// Every artifact is materialized; then any file under a generated root not in
/// the render map is deleted, and empty directories are removed.
///
/// **Not atomic.** Unlike `install` (which stages each skill and renames it into
/// place), `publish` writes and prunes in place: a failure part-way leaves the
/// generated tree partially updated. This is deliberate - the generated roots
/// (`plugins/`, `.claude-plugin/`) are wholly owned by the renderer and are
/// expected to be committed, so a partial write is recovered by re-running
/// `publish` (or `git checkout`), and `publish --check` will flag any drift a
/// failed run left behind. **Destructive:** every file under a generated root
/// that the map does not claim is deleted, so those roots must never hold
/// hand-authored files.
pub fn write(repo: &Repo, tool_version: &str) -> Result<PublishResult> {
    let map = generate::render(repo, tool_version)?;
    render::validate_generated_roots(&repo.root)?;
    let mut written = Vec::new();

    for (rel, artifact) in &map.0 {
        let dest = repo.root.join(rel);
        artifact.write_to(&dest)?;
        written.push(rel.clone());
    }

    let pruned = prune_stale(&repo.root, &map)?;

    Ok(PublishResult {
        written,
        pruned,
        drift: None,
    })
}

/// Delete files (and emptied dirs) under the generated roots that the render
/// map does not claim. Returns the pruned repo-relative paths, sorted.
fn prune_stale(root: &Path, map: &RenderMap) -> Result<Vec<String>> {
    let expected: BTreeSet<&String> = map.0.keys().collect();
    let mut pruned = Vec::new();

    for gen_root in RenderMap::GENERATED_ROOTS {
        let dir = root.join(gen_root);
        if !dir.is_dir() {
            continue;
        }
        // Collect files first so we don't mutate the tree mid-walk.
        let mut files = Vec::new();
        for entry in WalkDir::new(&dir).follow_links(false) {
            let entry = entry.map_err(|e| {
                Error::io(
                    e.path().unwrap_or(&dir).to_path_buf(),
                    e.into_io_error()
                        .unwrap_or_else(|| std::io::Error::other("walk error")),
                )
            })?;
            if entry.file_type().is_file() {
                files.push(entry.path().to_path_buf());
            }
        }
        for file in files {
            let rel = file
                .strip_prefix(root)
                .expect("under root")
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            if !expected.contains(&rel) {
                std::fs::remove_file(&file).map_err(|e| Error::io(&file, e))?;
                pruned.push(rel);
            }
        }
        remove_empty_dirs(&dir)?;
    }

    pruned.sort();
    Ok(pruned)
}

/// Recursively remove empty directories under `dir` (but not `dir` itself).
fn remove_empty_dirs(dir: &Path) -> Result<()> {
    let mut dirs = Vec::new();
    for entry in WalkDir::new(dir).follow_links(false) {
        let entry = entry.map_err(|e| {
            Error::io(
                e.path().unwrap_or(dir).to_path_buf(),
                e.into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("walk error")),
            )
        })?;
        if entry.file_type().is_dir() && entry.path() != dir {
            dirs.push(entry.path().to_path_buf());
        }
    }
    // Deepest first so a parent empties after its children.
    dirs.sort_by_key(|a| std::cmp::Reverse(a.components().count()));
    for d in dirs {
        if std::fs::read_dir(&d)
            .map_err(|e| Error::io(&d, e))?
            .next()
            .is_none()
        {
            std::fs::remove_dir(&d).map_err(|e| Error::io(&d, e))?;
        }
    }
    Ok(())
}
