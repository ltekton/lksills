//! Walk a skill directory into sorted, copyable files, rejecting symlinks and
//! source-only paths.

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::path::is_source_only;

/// A regular file under a skill, with its skill-relative POSIX path.
pub struct SkillFile {
    /// Absolute path on disk.
    pub abs: PathBuf,
    /// Skill-relative, forward-slash path (`SKILL.md`, `scripts/x.sh`).
    pub rel_posix: String,
}

/// Return every regular file under `skill_dir`, sorted by POSIX relative path,
/// excluding source-only paths. A symlink (file or directory) is rejected: a
/// committed symlink would otherwise copy outside-the-repo bytes into a
/// published artifact.
pub fn walk_skill(skill_dir: &Path) -> Result<Vec<SkillFile>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(skill_dir)
        .follow_links(false)
        .sort_by_file_name()
    {
        let entry = entry.map_err(|e| {
            let path = e.path().unwrap_or(skill_dir).to_path_buf();
            Error::io(
                path,
                e.into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("walk error")),
            )
        })?;
        let path = entry.path();
        if path == skill_dir {
            continue;
        }
        if entry.file_type().is_symlink() {
            return Err(Error::PathEscape {
                path: path.to_path_buf(),
                root: skill_dir.to_path_buf(),
            });
        }
        if entry.file_type().is_dir() {
            continue;
        }
        if !entry.file_type().is_file() {
            return Err(Error::io(
                path,
                std::io::Error::new(std::io::ErrorKind::InvalidData, "special file"),
            ));
        }
        let rel = path
            .strip_prefix(skill_dir)
            .expect("walked path is under skill_dir");
        let rel_posix = rel
            .components()
            .map(|component| {
                component.as_os_str().to_str().ok_or_else(|| {
                    Error::io(
                        path,
                        std::io::Error::new(std::io::ErrorKind::InvalidData, "non-UTF-8 filename"),
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?
            .join("/");
        if is_source_only(&rel_posix) {
            continue;
        }
        files.push(SkillFile {
            abs: path.to_path_buf(),
            rel_posix,
        });
    }
    files.sort_by(|a, b| a.rel_posix.cmp(&b.rel_posix));
    Ok(files)
}

/// On unix, the file's mode bits; `None` off-unix.
#[cfg(unix)]
pub fn file_mode(path: &Path) -> Result<Option<u32>> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(path).map_err(|e| Error::io(path, e))?;
    Ok(Some(meta.permissions().mode()))
}

/// Off-unix there is no executable bit to track.
#[cfg(not(unix))]
pub fn file_mode(_path: &Path) -> Result<Option<u32>> {
    Ok(None)
}
