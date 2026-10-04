//! Import one existing bundle from a local or Git skills-root.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::Repo;
use crate::error::{Error, Result};
use crate::model::Bundle;
use crate::names::{BundleName, SkillFullName};
use crate::path::safe_join;
use crate::provenance::{self, ProvenanceEntry};

/// Source identity supplied to an import plan.
#[derive(Debug, Clone)]
pub struct OriginInfo {
    /// Original source string supplied by the operator.
    pub locator: String,
    /// An immutable Git revision when the source is a Git checkout.
    pub resolved_revision: Option<String>,
}

/// A validated, read-only import plan.
#[derive(Debug, Clone, Serialize)]
pub struct ImportPlan {
    /// Result schema version.
    pub v: u32,
    /// Destination root.
    pub root: PathBuf,
    /// Origin locator.
    pub source: String,
    /// Selected bundle.
    pub bundle: BundleName,
    /// Referenced skills copied by the import.
    pub skills: Vec<SkillFullName>,
    /// Git revision or local digest.
    pub revision: String,
    /// Selected content digest.
    pub digest: String,
    /// Destination-relative files in the plan.
    pub files: Vec<String>,
    #[serde(skip)]
    source_bundle: PathBuf,
    #[serde(skip)]
    source_skills: Vec<(SkillFullName, PathBuf)>,
}

/// The result of previewing or applying an import.
#[derive(Debug, Clone, Serialize)]
pub struct ImportResult {
    /// Result schema version.
    pub v: u32,
    /// `previewed` or `imported`.
    pub status: &'static str,
    /// Destination root.
    pub root: PathBuf,
    /// Origin locator.
    pub source: String,
    /// Selected bundle.
    pub bundle: BundleName,
    /// Referenced skills.
    pub skills: Vec<SkillFullName>,
    /// Git revision or local digest.
    pub revision: String,
    /// Selected content digest.
    pub digest: String,
    /// Destination-relative files.
    pub files: Vec<String>,
}

/// Validate the source paths that `Repo::load` will inspect before loading them.
///
/// This keeps import acquisition from following a source symlink or opening a
/// special file while the existing skills-root loader reads manifests.
pub fn validate_source_tree(root: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(root).map_err(|e| Error::io(root, e))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::PathEscape {
            path: root.to_path_buf(),
            root: root.to_path_buf(),
        });
    }
    for name in ["skills", "bundles"] {
        let directory = root.join(name);
        let metadata = match std::fs::symlink_metadata(&directory) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(Error::io(&directory, error)),
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(Error::PathEscape {
                path: directory,
                root: root.to_path_buf(),
            });
        }
        for entry in WalkDir::new(&directory).follow_links(false) {
            let entry = entry.map_err(|error| {
                let path = error.path().unwrap_or(&directory).to_path_buf();
                Error::io(
                    path,
                    error
                        .into_io_error()
                        .unwrap_or_else(|| std::io::Error::other("walk error")),
                )
            })?;
            let file_type = entry.file_type();
            if file_type.is_symlink() {
                return Err(Error::PathEscape {
                    path: entry.path().to_path_buf(),
                    root: directory.clone(),
                });
            }
            if !file_type.is_file() && !file_type.is_dir() {
                return Err(Error::io(
                    entry.path(),
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "special file"),
                ));
            }
        }
    }
    Ok(())
}

/// Build a plan without writing to the destination.
pub fn plan(
    source: &Repo,
    root: &Path,
    bundle_name: &BundleName,
    origin: &OriginInfo,
) -> Result<ImportPlan> {
    if origin.locator.trim().is_empty() {
        return Err(Error::Usage("import origin must not be empty".into()));
    }
    validate_source_tree(&source.root)?;
    validate_destination_layout(root)?;

    let bundle = source
        .bundles
        .iter()
        .find(|bundle| &bundle.name == bundle_name)
        .ok_or_else(|| Error::Usage(format!("unknown source bundle {:?}", bundle_name.as_str())))?;
    validate_bundle(source, bundle)?;

    let provenance = provenance::load(root)?;
    if provenance
        .imports
        .iter()
        .any(|entry| entry.bundle == *bundle_name)
    {
        return Err(Error::ImportCollision {
            path: provenance::path(root),
        });
    }

    let staging = root
        .join(".lskills")
        .join("staging")
        .join(format!("import-{}", bundle_name.as_str()));
    reject_existing(&staging)?;

    let bundle_destination = safe_join(
        &root.join("bundles"),
        Path::new(&format!("{}.toml", bundle_name.as_str())),
    )?;
    reject_existing(&bundle_destination)?;

    let mut seen = BTreeSet::new();
    let mut source_skills = Vec::new();
    let mut files = vec![format!("bundles/{}.toml", bundle_name.as_str())];
    for skill_name in &bundle.skills {
        if !seen.insert(skill_name.clone()) {
            return Err(Error::Bundle {
                path: bundle.path.clone(),
                reason: format!("skill {:?} is listed more than once", skill_name.as_str()),
            });
        }
        let skill = source.skills.get(skill_name).ok_or_else(|| Error::Bundle {
            path: bundle.path.clone(),
            reason: format!("referenced skill {:?} is missing", skill_name.as_str()),
        })?;
        let destination = safe_join(&root.join("skills"), Path::new(skill_name.as_str()))?;
        reject_existing(&destination)?;
        let skill_files = collect_files(&skill.path)?;
        for file in skill_files {
            let relative = file
                .path
                .strip_prefix(&skill.path)
                .expect("skill file is under skill path")
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            files.push(format!("skills/{}/{relative}", skill_name.as_str()));
        }
        source_skills.push((skill_name.clone(), skill.path.clone()));
    }
    files.sort();

    let digest = digest_bundle(bundle, &source_skills)?;
    let revision = origin
        .resolved_revision
        .clone()
        .unwrap_or_else(|| format!("local:{digest}"));

    Ok(ImportPlan {
        v: 1,
        root: root.to_path_buf(),
        source: origin.locator.clone(),
        bundle: bundle.name.clone(),
        skills: bundle.skills.clone(),
        revision,
        digest,
        files,
        source_bundle: bundle.path.clone(),
        source_skills,
    })
}

/// Apply a validated import plan.
pub fn apply(plan: &ImportPlan) -> Result<ImportResult> {
    let root = &plan.root;
    prepare_destination(root)?;
    let mut provenance = provenance::load(root)?;
    provenance.imports.push(ProvenanceEntry {
        bundle: plan.bundle.clone(),
        source: plan.source.clone(),
        selector: format!("bundles/{}.toml", plan.bundle.as_str()),
        revision: plan.revision.clone(),
        digest: plan.digest.clone(),
    });

    let staging = root
        .join(".lskills")
        .join("staging")
        .join(format!("import-{}", plan.bundle.as_str()));
    if std::fs::symlink_metadata(&staging).is_ok() {
        return Err(Error::ImportCollision { path: staging });
    }
    std::fs::create_dir_all(staging.join("bundles")).map_err(|e| Error::io(&staging, e))?;
    std::fs::create_dir_all(staging.join("skills")).map_err(|e| Error::io(&staging, e))?;

    let bundle_stage = staging
        .join("bundles")
        .join(format!("{}.toml", plan.bundle.as_str()));
    if let Err(error) = copy_regular_file(&plan.source_bundle, &bundle_stage) {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(error);
    }

    for (skill_name, source_path) in &plan.source_skills {
        let destination = staging.join("skills").join(skill_name.as_str());
        if let Err(error) = copy_tree(source_path, &destination) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(error);
        }
    }

    let mut moved = Vec::new();
    let move_result = (|| -> Result<()> {
        let staged_bundle = staging
            .join("bundles")
            .join(format!("{}.toml", plan.bundle.as_str()));
        let destination_bundle = root
            .join("bundles")
            .join(format!("{}.toml", plan.bundle.as_str()));
        move_staged(&staged_bundle, &destination_bundle, &mut moved)?;

        for skill_name in &plan.skills {
            let staged_skill = staging.join("skills").join(skill_name.as_str());
            let destination_skill = root.join("skills").join(skill_name.as_str());
            move_staged(&staged_skill, &destination_skill, &mut moved)?;
        }
        Ok(())
    })();

    if let Err(error) = move_result {
        rollback(&mut moved);
        let _ = std::fs::remove_dir_all(&staging);
        return Err(error);
    }

    if let Err(error) = provenance::write_atomic(root, &provenance) {
        rollback(&mut moved);
        let _ = std::fs::remove_dir_all(&staging);
        return Err(error);
    }

    let _ = std::fs::remove_dir_all(&staging);
    Ok(ImportResult {
        v: 1,
        status: "imported",
        root: root.to_path_buf(),
        source: plan.source.clone(),
        bundle: plan.bundle.clone(),
        skills: plan.skills.clone(),
        revision: plan.revision.clone(),
        digest: plan.digest.clone(),
        files: plan.files.clone(),
    })
}

impl ImportPlan {
    /// Convert a plan into a read-only result.
    pub fn preview(&self) -> ImportResult {
        ImportResult {
            v: self.v,
            status: "previewed",
            root: self.root.clone(),
            source: self.source.clone(),
            bundle: self.bundle.clone(),
            skills: self.skills.clone(),
            revision: self.revision.clone(),
            digest: self.digest.clone(),
            files: self.files.clone(),
        }
    }
}

fn validate_bundle(source: &Repo, bundle: &Bundle) -> Result<()> {
    if bundle.skills.is_empty() {
        return Err(Error::Bundle {
            path: bundle.path.clone(),
            reason: "bundle lists no skills".into(),
        });
    }
    if bundle.agents.is_empty() {
        return Err(Error::Bundle {
            path: bundle.path.clone(),
            reason: "bundle targets no agents".into(),
        });
    }
    for member in &bundle.skills {
        member.require_prefix(&bundle.name)?;
        if !source.skills.contains_key(member) {
            return Err(Error::Bundle {
                path: bundle.path.clone(),
                reason: format!("referenced skill {:?} is missing", member.as_str()),
            });
        }
    }
    Ok(())
}

fn validate_destination_root(root: &Path) -> Result<()> {
    if let Ok(metadata) = std::fs::symlink_metadata(root) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(Error::PathEscape {
                path: root.to_path_buf(),
                root: root.to_path_buf(),
            });
        }
    }
    Ok(())
}

fn validate_destination_layout(root: &Path) -> Result<()> {
    validate_destination_root(root)?;
    for name in ["skills", "bundles", ".lskills", ".lskills/staging"] {
        let path = root.join(name);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(Error::PathEscape {
                        path,
                        root: root.to_path_buf(),
                    });
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Error::io(&path, error)),
        }
    }
    Ok(())
}

fn prepare_destination(root: &Path) -> Result<()> {
    validate_destination_layout(root)?;
    std::fs::create_dir_all(root).map_err(|e| Error::io(root, e))?;
    for name in ["skills", "bundles", ".lskills", ".lskills/staging"] {
        let path = root.join(name);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(Error::PathEscape {
                        path,
                        root: root.to_path_buf(),
                    });
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir_all(&path).map_err(|e| Error::io(&path, e))?;
            }
            Err(error) => return Err(Error::io(&path, error)),
        }
    }
    Ok(())
}

fn reject_existing(path: &Path) -> Result<()> {
    if path.exists() || std::fs::symlink_metadata(path).is_ok() {
        return Err(Error::ImportCollision {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

struct SourceFile {
    path: PathBuf,
    relative: String,
}

fn collect_files(root: &Path) -> Result<Vec<SourceFile>> {
    let root_metadata = std::fs::symlink_metadata(root).map_err(|e| Error::io(root, e))?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(Error::PathEscape {
            path: root.to_path_buf(),
            root: root.to_path_buf(),
        });
    }
    let mut files = Vec::new();
    for entry in WalkDir::new(root).follow_links(false).sort_by_file_name() {
        let entry = entry.map_err(|e| {
            let path = e.path().unwrap_or(root).to_path_buf();
            Error::io(
                path,
                e.into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("walk error")),
            )
        })?;
        let path = entry.path();
        if path == root {
            continue;
        }
        let file_type = entry.file_type();
        if file_type.is_symlink() {
            return Err(Error::PathEscape {
                path: path.to_path_buf(),
                root: root.to_path_buf(),
            });
        }
        if !file_type.is_file() {
            if !file_type.is_dir() {
                return Err(Error::io(
                    path,
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "special file"),
                ));
            }
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .expect("walked path is under root")
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        files.push(SourceFile {
            path: path.to_path_buf(),
            relative,
        });
    }
    files.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(files)
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination).map_err(|e| Error::io(destination, e))?;
    for file in collect_files(source)? {
        let target = safe_join(destination, Path::new(&file.relative))?;
        copy_regular_file(&file.path, &target)?;
    }
    Ok(())
}

fn copy_regular_file(source: &Path, destination: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(source).map_err(|e| Error::io(source, e))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::PathEscape {
            path: source.to_path_buf(),
            root: source.to_path_buf(),
        });
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    std::fs::copy(source, destination).map_err(|e| Error::io(destination, e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        std::fs::set_permissions(destination, std::fs::Permissions::from_mode(mode))
            .map_err(|e| Error::io(destination, e))?;
    }
    Ok(())
}

fn digest_bundle(bundle: &Bundle, skills: &[(SkillFullName, PathBuf)]) -> Result<String> {
    let mut hasher = Sha256::new();
    digest_file(&mut hasher, "bundles/manifest.toml", &bundle.path)?;
    let mut ordered = skills.to_vec();
    ordered.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, path) in ordered {
        for file in collect_files(&path)? {
            digest_file(
                &mut hasher,
                &format!("skills/{}/{}", name.as_str(), file.relative),
                &file.path,
            )?;
        }
    }
    Ok(hex_digest(&hasher.finalize()))
}

fn digest_file(hasher: &mut Sha256, logical: &str, path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path).map_err(|e| Error::io(path, e))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::PathEscape {
            path: path.to_path_buf(),
            root: path.to_path_buf(),
        });
    }
    let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
    let mode = file_mode(&metadata);
    hasher.update((logical.len() as u64).to_be_bytes());
    hasher.update(logical.as_bytes());
    hasher.update(mode.to_be_bytes());
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    Ok(())
}

fn file_mode(metadata: &std::fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        0
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(7 + bytes.len() * 2);
    out.push_str("sha256-");
    for byte in bytes {
        write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}

fn move_staged(
    source: &Path,
    destination: &Path,
    moved: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<()> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    std::fs::rename(source, destination).map_err(|e| Error::io(destination, e))?;
    moved.push((destination.to_path_buf(), source.to_path_buf()));
    Ok(())
}

fn rollback(moved: &mut Vec<(PathBuf, PathBuf)>) {
    while let Some((destination, source)) = moved.pop() {
        if let Some(parent) = source.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::rename(destination, source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn digest_is_stable_for_a_fixture() {
        let source = TempDir::new().unwrap();
        let bundle_dir = source.path().join("bundles");
        let skill_dir = source.path().join("skills/demo-hello");
        std::fs::create_dir_all(&bundle_dir).unwrap();
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            bundle_dir.join("demo.toml"),
            "name = \"demo\"\nskills = [\"demo-hello\"]\nagents = [\"claude\"]\n",
        )
        .unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: demo-hello\ndescription: hello\n---\nbody\n",
        )
        .unwrap();
        let repo = Repo::load(source.path()).unwrap();
        let bundle = &repo.bundles[0];
        let skills = vec![(SkillFullName::parse("demo-hello").unwrap(), skill_dir)];
        assert_eq!(
            digest_bundle(bundle, &skills).unwrap(),
            digest_bundle(bundle, &skills).unwrap()
        );
    }
}
