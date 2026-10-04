//! Skills-root resolution: explicit `--root`, else `$LSKILLS_ROOT`, else cwd.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// The environment variable that names the skills-root when `--root` is absent.
pub const ROOT_ENV: &str = "LSKILLS_ROOT";

/// Resolve the skills-root from an explicit override, an injected environment,
/// and the current directory — always returned absolute.
///
/// The environment lookup is injected (`env`) rather than read from
/// [`std::env`] so this stays unit-testable and hermetic.
pub fn resolve(
    explicit: Option<&Path>,
    env: impl Fn(&str) -> Option<String>,
    cwd: &Path,
) -> PathBuf {
    let base = explicit
        .map(Path::to_path_buf)
        .or_else(|| env(ROOT_ENV).map(PathBuf::from))
        .unwrap_or_else(|| cwd.to_path_buf());

    if base.is_absolute() {
        base
    } else {
        cwd.join(base)
    }
}

/// Assert that `root` exists and is a directory.
pub fn ensure_dir(root: &Path) -> Result<()> {
    if root.is_dir() {
        Ok(())
    } else {
        Err(Error::NotARoot {
            path: root.to_path_buf(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_wins_over_env_and_cwd() {
        let got = resolve(
            Some(Path::new("/explicit")),
            |_| Some("/from-env".into()),
            Path::new("/cwd"),
        );
        assert_eq!(got, PathBuf::from("/explicit"));
    }

    #[test]
    fn env_wins_over_cwd() {
        let got = resolve(
            None,
            |k| (k == ROOT_ENV).then(|| "/from-env".into()),
            Path::new("/cwd"),
        );
        assert_eq!(got, PathBuf::from("/from-env"));
    }

    #[test]
    fn relative_is_joined_onto_cwd() {
        let got = resolve(Some(Path::new("sub")), |_| None, Path::new("/cwd"));
        assert_eq!(got, PathBuf::from("/cwd/sub"));
    }
}
