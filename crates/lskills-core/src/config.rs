//! Repo-level tool config: `<root>/.lskills.toml`.
//!
//! The only repo-level configuration the tool reads. Currently it carries the
//! release versioning scheme. An absent file or unset key yields defaults
//! (scheme = SemVer), so a skills-root needs no config at all to work; the file
//! exists only to opt into non-default behavior like CalVer.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// The versioning scheme `release` applies to bundle versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    /// Explicit SemVer bumps via `--bump <bundle>=major|minor|patch`.
    #[default]
    Semver,
    /// Date-derived `YYYY.M.PATCH` versions from an injected clock.
    Calver,
}

/// The `[release]` config table.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ReleaseConfig {
    /// The active versioning scheme (default: SemVer).
    pub scheme: Scheme,
}

/// The whole `.lskills.toml`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RepoConfig {
    /// Release/versioning settings.
    pub release: ReleaseConfig,
}

impl RepoConfig {
    /// The config file name at the skills-root.
    pub const FILE: &'static str = ".lskills.toml";

    /// Load `<root>/.lskills.toml`, or the defaults if it is absent.
    ///
    /// A present-but-malformed config is a hard error; the tool never guesses
    /// past a config it cannot parse.
    pub fn load(root: &Path) -> Result<RepoConfig> {
        let path = root.join(Self::FILE);
        match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).map_err(|e| Error::Config {
                path,
                reason: e.to_string(),
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(RepoConfig::default()),
            Err(e) => Err(Error::io(&path, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_file_yields_semver_default() {
        let dir = std::env::temp_dir().join("lskills-config-absent-test");
        // A directory with no `.lskills.toml`.
        let _ = std::fs::create_dir_all(&dir);
        let cfg = RepoConfig::load(&dir).unwrap();
        assert_eq!(cfg.release.scheme, Scheme::Semver);
    }

    #[test]
    fn parses_calver_scheme() {
        let cfg: RepoConfig = toml::from_str("[release]\nscheme = \"calver\"\n").unwrap();
        assert_eq!(cfg.release.scheme, Scheme::Calver);
    }

    #[test]
    fn empty_config_is_all_defaults() {
        let cfg: RepoConfig = toml::from_str("").unwrap();
        assert_eq!(cfg.release.scheme, Scheme::Semver);
    }

    #[test]
    fn malformed_scheme_is_error() {
        let err: std::result::Result<RepoConfig, _> =
            toml::from_str("[release]\nscheme = \"bogus\"\n");
        assert!(err.is_err());
    }
}
