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
            Artifact::CopyVerbatim { from, .. } => read_regular_file(from),
        }
    }

    /// Compare this artifact to what is on disk at `dest`.
    ///
    /// Returns `Ok(true)` when the disk copy already matches (bytes, and for a
    /// verbatim copy also the executable bit on unix), `Ok(false)` when it
    /// differs or is missing.
    pub fn matches_disk(&self, dest: &Path) -> Result<bool> {
        validate_write_parents(dest)?;
        // A symlink at a generated path is drift no matter what it points at: the
        // renderer only ever writes regular files, so a link is a hand-edit to
        // report (and never something we read *through*).
        match std::fs::symlink_metadata(dest) {
            Ok(meta) if meta.file_type().is_symlink() => return Ok(false),
            Ok(meta) if !meta.is_file() => return Ok(false),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(Error::io(dest, error)),
            Ok(_) => {}
        }
        // A device/socket/fifo at the path is likewise not our file.
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt;
            match std::fs::symlink_metadata(dest) {
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
        validate_write_destination(dest)?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        validate_write_destination(dest)?;
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

/// Reject symlinked, special, or non-UTF-8 content under renderer-owned roots.
pub fn validate_generated_roots(root: &Path) -> Result<()> {
    for generated in RenderMap::GENERATED_ROOTS {
        let directory = root.join(generated);
        let metadata = match std::fs::symlink_metadata(&directory) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(Error::io(&directory, error)),
        };
        if metadata.file_type().is_symlink() {
            return Err(Error::PathEscape {
                path: directory,
                root: root.to_path_buf(),
            });
        }
        if !metadata.is_dir() {
            return Err(Error::io(
                &directory,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "generated root is not a directory",
                ),
            ));
        }
        for entry in walkdir::WalkDir::new(&directory).follow_links(false) {
            let entry = entry.map_err(|error| {
                let path = error.path().unwrap_or(&directory).to_path_buf();
                Error::io(
                    path,
                    error
                        .into_io_error()
                        .unwrap_or_else(|| std::io::Error::other("walk error")),
                )
            })?;
            let path = entry.path();
            if entry.file_type().is_symlink() {
                return Err(Error::PathEscape {
                    path: path.to_path_buf(),
                    root: root.to_path_buf(),
                });
            }
            if !entry.file_type().is_file() && !entry.file_type().is_dir() {
                return Err(Error::io(
                    path,
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "special file"),
                ));
            }
            if path
                .strip_prefix(root)
                .expect("generated path is under root")
                .components()
                .any(|component| component.as_os_str().to_str().is_none())
            {
                return Err(Error::io(
                    path,
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "non-UTF-8 filename"),
                ));
            }
        }
    }
    Ok(())
}

fn read_regular_file(path: &Path) -> Result<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path).map_err(|e| Error::io(path, e))?;
    if metadata.file_type().is_symlink() {
        return Err(Error::PathEscape {
            path: path.to_path_buf(),
            root: path.to_path_buf(),
        });
    }
    if !metadata.is_file() {
        return Err(Error::io(
            path,
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "artifact source is not a regular file",
            ),
        ));
    }
    std::fs::read(path).map_err(|e| Error::io(path, e))
}

fn validate_write_destination(dest: &Path) -> Result<()> {
    validate_write_parents(dest)?;
    match std::fs::symlink_metadata(dest) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(Error::PathEscape {
            path: dest.to_path_buf(),
            root: dest.parent().unwrap_or(dest).to_path_buf(),
        }),
        Ok(metadata) if !metadata.is_file() => Err(Error::io(
            dest,
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "artifact destination is not a regular file",
            ),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::io(dest, error)),
    }
}

fn validate_write_parents(dest: &Path) -> Result<()> {
    let root = dest.parent().unwrap_or(dest).to_path_buf();
    let mut ancestor = dest.parent().map(Path::to_path_buf);
    while let Some(path) = ancestor {
        if path.as_os_str().is_empty() {
            break;
        }
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(Error::PathEscape {
                        path,
                        root: root.clone(),
                    });
                }
                if !metadata.is_dir() {
                    return Err(Error::io(
                        &path,
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "artifact parent is not a directory",
                        ),
                    ));
                }
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                ancestor = path.parent().map(Path::to_path_buf);
            }
            Err(error) => return Err(Error::io(&path, error)),
        }
    }
    Ok(())
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
    fn write_rejects_a_symlinked_parent() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let link = tmp.path().join("generated");
        std::os::unix::fs::symlink(outside.path(), &link).unwrap();
        let artifact = Artifact::Text("must not escape\n".to_string());

        assert!(matches!(
            artifact.write_to(&link.join("file.txt")),
            Err(Error::PathEscape { .. })
        ));
        assert!(!outside.path().join("file.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn generated_root_validation_rejects_a_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), tmp.path().join("plugins")).unwrap();

        assert!(matches!(
            validate_generated_roots(tmp.path()),
            Err(Error::PathEscape { .. })
        ));
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
