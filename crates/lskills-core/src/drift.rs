//! Compare a [`RenderMap`] against what is committed on disk.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Serialize;
use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::render::RenderMap;

/// One drift entry: a generated file that is missing, extra, or changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum DriftEntry {
    /// The render map has this path but disk does not.
    Missing {
        /// Repo-relative path.
        path: String,
    },
    /// Disk has this path under a generated root but the render map does not.
    Extra {
        /// Repo-relative path.
        path: String,
    },
    /// The path exists on both but the bytes (or exec bit) differ.
    Changed {
        /// Repo-relative path.
        path: String,
    },
}

/// The result of a drift comparison.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DriftReport {
    /// All drift entries, sorted by path within each kind.
    pub entries: Vec<DriftEntry>,
}

impl DriftReport {
    /// Whether the tree is in sync (no drift).
    pub fn is_clean(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Compute drift between the rendered artifacts and disk under `root`.
///
/// Missing/Changed come from the render map; Extra comes from walking the
/// generated roots for files the map does not claim.
pub fn compute(root: &Path, map: &RenderMap) -> Result<DriftReport> {
    let mut entries = Vec::new();
    let expected: BTreeSet<&String> = map.0.keys().collect();

    for (rel, artifact) in &map.0 {
        let dest = root.join(rel);
        // `symlink_metadata` so a symlink standing in for a generated file is not
        // followed - `matches_disk` treats such a link as drift rather than
        // reading through it. A truly absent path is Missing.
        match std::fs::symlink_metadata(&dest) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                entries.push(DriftEntry::Missing { path: rel.clone() });
            }
            Err(e) => return Err(Error::io(&dest, e)),
            Ok(_) => {
                if !artifact.matches_disk(&dest)? {
                    entries.push(DriftEntry::Changed { path: rel.clone() });
                }
            }
        }
    }

    for gen_root in RenderMap::GENERATED_ROOTS {
        let dir = root.join(gen_root);
        if !dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&dir).follow_links(false) {
            let entry = entry.map_err(|e| {
                Error::io(
                    e.path().unwrap_or(&dir).to_path_buf(),
                    e.into_io_error()
                        .unwrap_or_else(|| std::io::Error::other("walk error")),
                )
            })?;
            // Directories are structure, not content; every non-directory under a
            // generated root (regular file, symlink, or special file) that the map
            // does not claim is Extra. Using the entry's own file type (WalkDir
            // does not follow links) means a stray symlink is reported, not
            // silently skipped by an `is_file()` that a link fails.
            if entry.file_type().is_dir() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(root)
                .expect("walked path under root")
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            if !expected.contains(&rel) {
                entries.push(DriftEntry::Extra { path: rel });
            }
        }
    }

    entries.sort_by(|a, b| entry_key(a).cmp(entry_key(b)));
    Ok(DriftReport { entries })
}

/// Sort key: path, so entries are deterministic regardless of discovery order.
fn entry_key(e: &DriftEntry) -> &str {
    match e {
        DriftEntry::Missing { path }
        | DriftEntry::Extra { path }
        | DriftEntry::Changed { path } => path,
    }
}
