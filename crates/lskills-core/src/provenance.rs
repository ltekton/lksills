//! Import provenance stored in the workarea.

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::names::BundleName;
use crate::repo::Repo;

/// The current provenance schema.
pub const SCHEMA: u32 = 1;

/// Versioned workarea provenance sidecar.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceFile {
    /// Schema version.
    pub schema: u32,
    /// One record per successful bundle import.
    #[serde(default)]
    pub imports: Vec<ProvenanceEntry>,
}

impl Default for ProvenanceFile {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            imports: Vec::new(),
        }
    }
}

/// Provenance captured when a bundle enters the workarea.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceEntry {
    /// Destination bundle name.
    pub bundle: BundleName,
    /// Original source locator as supplied by the operator.
    pub source: String,
    /// Selected source bundle path.
    pub selector: String,
    /// Git revision, or the local content digest when no Git revision exists.
    pub revision: String,
    /// Digest of the selected bundle and referenced skill files.
    pub digest: String,
}

/// Return the sidecar path for `root`.
pub fn path(root: &Path) -> PathBuf {
    root.join(".lskills").join("provenance.toml")
}

/// Load provenance, treating an absent sidecar as an empty file.
pub fn load(root: &Path) -> Result<ProvenanceFile> {
    let path = path(root);
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ProvenanceFile::default());
        }
        Err(error) => return Err(Error::io(&path, error)),
    };
    if !metadata.file_type().is_file() {
        return Err(Error::Provenance {
            path,
            reason: "provenance path is not a regular file".into(),
        });
    }
    let text = std::fs::read_to_string(&path).map_err(|e| Error::io(&path, e))?;
    let value: ProvenanceFile = toml::from_str(&text).map_err(|e| Error::Provenance {
        path: path.clone(),
        reason: e.to_string(),
    })?;
    if value.schema != SCHEMA {
        return Err(Error::Provenance {
            path,
            reason: format!("unsupported schema {}; expected {SCHEMA}", value.schema),
        });
    }
    Ok(value)
}

/// Validate provenance structure and its references to a loaded workarea.
pub fn validate(value: &ProvenanceFile, repo: &Repo) -> Result<()> {
    let path = path(&repo.root);
    let mut bundles = BTreeSet::new();
    for entry in &value.imports {
        let name = entry.bundle.as_str();
        if !bundles.insert(name.to_string()) {
            return Err(Error::Provenance {
                path: path.clone(),
                reason: format!("duplicate import entry for bundle {name:?}"),
            });
        }
        if !repo
            .bundles
            .iter()
            .any(|bundle| bundle.name == entry.bundle)
        {
            return Err(Error::Provenance {
                path: path.clone(),
                reason: format!("import entry references missing bundle {name:?}"),
            });
        }
        let selector = format!("bundles/{name}.toml");
        if entry.selector != selector {
            return Err(Error::Provenance {
                path: path.clone(),
                reason: format!("bundle {name:?} has invalid selector {:?}", entry.selector),
            });
        }
        if entry.source.trim().is_empty() {
            return Err(Error::Provenance {
                path: path.clone(),
                reason: format!("bundle {name:?} has an empty source locator"),
            });
        }
        if entry.revision.trim().is_empty() {
            return Err(Error::Provenance {
                path: path.clone(),
                reason: format!("bundle {name:?} has an empty revision"),
            });
        }
        if !valid_revision(&entry.revision) {
            return Err(Error::Provenance {
                path: path.clone(),
                reason: format!("bundle {name:?} has an invalid revision"),
            });
        }
        if !valid_digest(&entry.digest) {
            return Err(Error::Provenance {
                path: path.clone(),
                reason: format!("bundle {name:?} has an invalid digest"),
            });
        }
    }
    Ok(())
}

fn valid_revision(value: &str) -> bool {
    if let Some(sha) = value.strip_prefix("git:") {
        return matches!(sha.len(), 40 | 64) && sha.bytes().all(|b| b.is_ascii_hexdigit());
    }
    if let Some(digest) = value.strip_prefix("local:") {
        return valid_digest(digest);
    }
    false
}

fn valid_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256-") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Atomically write the provenance sidecar.
pub fn write_atomic(root: &Path, value: &ProvenanceFile) -> Result<()> {
    let dir = root.join(".lskills");
    std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    let path = dir.join("provenance.toml");
    let temp = dir.join("provenance.toml.tmp");
    let text = toml::to_string_pretty(value).map_err(|e| Error::Provenance {
        path: path.clone(),
        reason: e.to_string(),
    })?;
    if let Ok(metadata) = std::fs::symlink_metadata(&temp) {
        if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
            return Err(Error::Provenance {
                path: temp,
                reason: "temporary provenance path is not a regular file".into(),
            });
        }
    }
    let mut file = std::fs::File::create(&temp).map_err(|e| Error::io(&temp, e))?;
    file.write_all(text.as_bytes())
        .map_err(|e| Error::io(&temp, e))?;
    file.sync_all().map_err(|e| Error::io(&temp, e))?;
    if let Err(error) = std::fs::rename(&temp, &path) {
        let _ = std::fs::remove_file(&temp);
        return Err(Error::io(&path, error));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_sidecar_is_empty() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(load(root.path()).unwrap(), ProvenanceFile::default());
    }

    #[test]
    fn round_trips_strict_schema() {
        let root = tempfile::tempdir().unwrap();
        let mut value = ProvenanceFile::default();
        value.imports.push(ProvenanceEntry {
            bundle: BundleName::parse("demo").unwrap(),
            source: "origin".into(),
            selector: "bundles/demo.toml".into(),
            revision: "local:sha256-".to_string() + &"a".repeat(64),
            digest: "sha256-".to_string() + &"b".repeat(64),
        });
        write_atomic(root.path(), &value).unwrap();
        assert_eq!(load(root.path()).unwrap(), value);
    }

    #[test]
    fn rejects_unknown_schema() {
        let root = tempfile::tempdir().unwrap();
        let path = path(root.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "schema = 2\n").unwrap();
        assert!(matches!(load(root.path()), Err(Error::Provenance { .. })));
    }
}
