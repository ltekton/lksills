//! The tool's own cache/config/state directories.
//!
//! Distinct from *agent install targets* (which follow each agent's own
//! `CLAUDE_CONFIG_DIR`/`CODEX_HOME`/`PI_CODING_AGENT_DIR`). These dirs hold the
//! remote-clone cache and any future tool state, via `directories::ProjectDirs`.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// The environment variable that overrides the tool's cache directory.
pub const CACHE_ENV: &str = "LSKILLS_CACHE_DIR";

/// Resolve the tool's cache directory.
///
/// Precedence: an explicit override (the `--cache-dir` flag) → `$LSKILLS_CACHE_DIR`
/// → the platform `ProjectDirs` cache dir. The environment lookup is injected
/// rather than read from [`std::env`] so this stays hermetic and unit-testable;
/// the CLI passes `std::env::var(..).ok()` at the boundary. This does not create
/// the directory - callers that write into it do that.
pub fn cache_dir(
    override_: Option<&Path>,
    env: impl Fn(&str) -> Option<String>,
) -> Result<PathBuf> {
    if let Some(dir) = override_ {
        return Ok(dir.to_path_buf());
    }
    if let Some(dir) = env(CACHE_ENV) {
        return Ok(PathBuf::from(dir));
    }
    directories::ProjectDirs::from("", "ltekton", "lskills")
        .map(|d| d.cache_dir().to_path_buf())
        .ok_or_else(|| {
            Error::Usage(format!(
                "cannot determine a cache directory; set --cache-dir or ${CACHE_ENV}"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_wins_over_env_and_default() {
        let got = cache_dir(Some(Path::new("/explicit")), |_| Some("/from-env".into())).unwrap();
        assert_eq!(got, PathBuf::from("/explicit"));
    }

    #[test]
    fn env_wins_over_default() {
        let got = cache_dir(None, |k| (k == CACHE_ENV).then(|| "/from-env".into())).unwrap();
        assert_eq!(got, PathBuf::from("/from-env"));
    }

    #[test]
    fn falls_back_to_project_dirs() {
        // On CI and dev machines ProjectDirs resolves; assert the app segment is
        // present rather than hard-coding a platform-specific absolute path.
        let got = cache_dir(None, |_| None).unwrap();
        let s = got.to_string_lossy();
        assert!(
            s.contains("lskills") || s.contains("ltekton"),
            "expected an app-named segment in {got:?}"
        );
    }
}
