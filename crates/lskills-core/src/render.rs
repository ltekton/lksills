//! The `Artifact` model and the render map.
//!
//! Every generated, drift-gated file is produced as an [`Artifact`] so that
//! *publish* (write) and *check* (compare) share one render path and can never
//! disagree. JSON artifacts carry a [`serde_json::Value`]; text artifacts carry
//! a rendered string; verbatim copies carry a source path read lazily so the
//! render map itself stays filesystem-free.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::{Error, Result};

/// A single generated file, keyed in a [`RenderMap`] by its repo-relative path.
#[derive(Debug, Clone)]
pub enum Artifact {
    /// A JSON document (plugin.json, marketplace.json, package.json).
    Json(Value),
    /// A rendered text file (a SKILL.md whose name line was rewritten).
    Text(String),
    /// A byte-for-byte copy of a source file (scripts, assets).
    CopyVerbatim {
        /// The source path to read at write/compare time.
        from: PathBuf,
        /// Unix mode bits to preserve (executable scripts). `None` off-unix.
        mode: Option<u32>,
    },
}

/// Serialize a JSON value as 2-space-pretty with a trailing newline.
pub fn json_bytes(value: &Value) -> Vec<u8> {
    let mut s = serde_json::to_string_pretty(value).expect("Value serializes");
    s.push('\n');
    s.into_bytes()
}

impl Artifact {
    /// The current rendered bytes of this artifact.
    ///
    /// For [`Artifact::CopyVerbatim`] this reads the source file; JSON and text
    /// variants are already in memory.
    pub fn current_bytes(&self) -> Result<Vec<u8>> {
        match self {
            Artifact::Json(v) => Ok(json_bytes(v)),
            Artifact::Text(s) => Ok(s.clone().into_bytes()),
            Artifact::CopyVerbatim { from, .. } => {
                std::fs::read(from).map_err(|e| Error::io(from, e))
            }
        }
    }

    /// Compare this artifact to what is on disk at `dest`.
    ///
    /// Returns `Ok(true)` when the disk copy already matches (bytes, and for a
    /// verbatim copy also the executable bit on unix), `Ok(false)` when it
    /// differs or is missing.
    pub fn matches_disk(&self, dest: &Path) -> Result<bool> {
        // A symlink at a generated path is drift no matter what it points at: the
        // renderer only ever writes regular files, so a link is a hand-edit to
        // report (and never something we read *through*).
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt;
            match std::fs::symlink_metadata(dest) {
                Ok(meta) if meta.file_type().is_symlink() => return Ok(false),
                // A device/socket/fifo at the path is likewise not our file.
                Ok(meta)
                    if meta.file_type().is_block_device()
                        || meta.file_type().is_char_device()
                        || meta.file_type().is_fifo()
                        || meta.file_type().is_socket() =>
                {
                    return Ok(false);
                }
                _ => {}
            }
        }
        let want = self.current_bytes()?;
        let have = match std::fs::read(dest) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(Error::io(dest, e)),
        };
        if want != have {
            return Ok(false);
        }
        Ok(self.mode_matches(dest))
    }

    /// On unix, whether the destination's mode matches the mode
    /// [`Artifact::apply_mode`] would write. Compares the *same* bits that are
    /// written (the low permission bits), so a clean `publish` never reports the
    /// file it just wrote as drifted. Always true when no mode is tracked or
    /// off-unix.
    #[cfg(unix)]
    fn mode_matches(&self, dest: &Path) -> bool {
        use std::os::unix::fs::PermissionsExt;
        let Artifact::CopyVerbatim { mode: Some(m), .. } = self else {
            return true;
        };
        match std::fs::symlink_metadata(dest) {
            // A symlink at a generated path is never a faithful copy: report drift.
            Ok(meta) if meta.file_type().is_symlink() => false,
            Ok(meta) => (meta.permissions().mode() & 0o7777) == (m & 0o7777),
            Err(_) => false,
        }
    }

    #[cfg(not(unix))]
    fn mode_matches(&self, _dest: &Path) -> bool {
        true
    }

    /// Write this artifact to `dest`, creating parent directories and applying
    /// the executable bit for a verbatim copy on unix.
    ///
    /// Shared by `publish` (into the generated tree) and `install` (into an
    /// agent's skills dir) so the two write paths cannot diverge.
    pub fn write_to(&self, dest: &Path) -> Result<()> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let bytes = self.current_bytes()?;
        std::fs::write(dest, &bytes).map_err(|e| Error::io(dest, e))?;
        self.apply_mode(dest)
    }

    /// Preserve the executable bit for a verbatim copy on unix.
    #[cfg(unix)]
    fn apply_mode(&self, dest: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        if let Artifact::CopyVerbatim { mode: Some(m), .. } = self {
            let perms = std::fs::Permissions::from_mode(*m);
            std::fs::set_permissions(dest, perms).map_err(|e| Error::io(dest, e))?;
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn apply_mode(&self, _dest: &Path) -> Result<()> {
        Ok(())
    }
}

/// Every generated file, keyed by repo-relative (forward-slash) path.
#[derive(Debug, Clone, Default)]
pub struct RenderMap(pub BTreeMap<String, Artifact>);

impl RenderMap {
    /// Directory prefixes owned entirely by the renderer. Prune deletes
    /// anything under these not present in the map.
    pub const GENERATED_ROOTS: &'static [&'static str] = &["plugins", ".claude-plugin"];

    /// Insert an artifact at a repo-relative path.
    pub fn insert(&mut self, rel: impl Into<String>, artifact: Artifact) {
        self.0.insert(rel.into(), artifact);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_then_check_roundtrips_bytes() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("plugin.json");
        let art = Artifact::Text("hello\n".to_string());
        art.write_to(&dest).unwrap();
        assert!(art.matches_disk(&dest).unwrap(), "clean write must match");

        // Different content is drift.
        std::fs::write(&dest, b"changed\n").unwrap();
        assert!(!art.matches_disk(&dest).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn verbatim_write_then_check_roundtrips_mode() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("script.sh");
        std::fs::write(&src, b"#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o750)).unwrap();

        let art = Artifact::CopyVerbatim {
            from: src.clone(),
            mode: Some(0o750),
        };
        let dest = tmp.path().join("out.sh");
        art.write_to(&dest).unwrap();
        // The mode written is the mode checked: a clean publish never self-drifts.
        assert!(art.matches_disk(&dest).unwrap());

        // Flip the exec bits: now it drifts.
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(!art.matches_disk(&dest).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_at_dest_is_drift() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("real.txt");
        std::fs::write(&real, b"hello\n").unwrap();
        let link = tmp.path().join("link.txt");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        // Even though the link resolves to identical bytes, a symlink standing in
        // for a generated file is drift - the renderer only writes regular files.
        let art = Artifact::Text("hello\n".to_string());
        assert!(!art.matches_disk(&link).unwrap());
    }
}
