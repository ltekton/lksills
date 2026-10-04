//! Import one existing bundle from a local or Git skills-root.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ImportTransaction {
    schema: u32,
    bundle: BundleName,
    digest: String,
    phase: TransactionPhase,
    items: Vec<TransactionItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum TransactionPhase {
    Staging,
    Ready,
    Moved,
    Committed,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct TransactionItem {
    staged: String,
    destination: String,
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

/// Normalize a path lexically and make it absolute without following links.
fn absolute_lexical(path: &Path) -> Result<PathBuf> {
    let base = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| Error::io("current directory", e))?
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in base.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(std::path::MAIN_SEPARATOR.to_string()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(value) => normalized.push(value),
        }
    }
    Ok(normalized)
}

/// Reject a symlink at the declared root itself. Ancestor links are resolved
/// for identity checks, but standard system paths such as macOS `/var` may be
/// links and are not treated as unsafe by themselves.
fn reject_symlink_root(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(Error::PathEscape {
            path: path.to_path_buf(),
            root: path.to_path_buf(),
        }),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::io(path, error)),
    }
}

/// Resolve an existing ancestor while preserving the not-yet-created suffix.
fn canonicalize_with_missing(path: &Path) -> Result<PathBuf> {
    let absolute = absolute_lexical(path)?;
    let mut current = absolute.clone();
    let mut suffix = Vec::new();
    loop {
        match std::fs::symlink_metadata(&current) {
            Ok(_) => {
                let mut resolved =
                    std::fs::canonicalize(&current).map_err(|e| Error::io(&current, e))?;
                for component in suffix.iter().rev() {
                    resolved.push(component);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let component = current.file_name().ok_or_else(|| Error::PathEscape {
                    path: path.to_path_buf(),
                    root: path.to_path_buf(),
                })?;
                suffix.push(component.to_os_string());
                current.pop();
            }
            Err(error) => return Err(Error::io(&current, error)),
        }
    }
}

/// Ensure an import source and destination are disjoint, resolving existing
/// ancestors while preserving not-yet-created suffixes.
pub fn ensure_roots_disjoint(source: &Path, destination: &Path) -> Result<()> {
    let source = canonicalize_with_missing(source)?;
    let destination = canonicalize_with_missing(destination)?;
    if source == destination || source.starts_with(&destination) || destination.starts_with(&source)
    {
        return Err(Error::ImportRootsOverlap {
            origin: source,
            destination,
        });
    }
    Ok(())
}

/// Validate an origin against the existing skills-root source contract.
pub fn validate_source_tree(root: &Path) -> Result<()> {
    reject_symlink_root(root)?;
    crate::repo::validate_tree(root)?;
    for name in ["skills", "bundles"] {
        let directory = root.join(name);
        if !directory.is_dir() {
            return Err(Error::Source {
                path: root.to_path_buf(),
                reason: format!("missing required {name}/ directory"),
            });
        }
    }
    Ok(())
}

const TRANSACTION_SCHEMA: u32 = 1;

fn staging_path(root: &Path, bundle: &BundleName) -> PathBuf {
    root.join(".lskills")
        .join("staging")
        .join(format!("import-{}", bundle.as_str()))
}

fn transaction_path(staging: &Path) -> PathBuf {
    staging.join("transaction.toml")
}

fn transaction_error(path: &Path, reason: impl Into<String>) -> Error {
    Error::Provenance {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

fn write_transaction(staging: &Path, transaction: &ImportTransaction) -> Result<()> {
    let path = transaction_path(staging);
    let temp = path.with_extension("toml.tmp");
    let text = toml::to_string_pretty(transaction)
        .map_err(|error| transaction_error(&path, error.to_string()))?;
    if let Ok(metadata) = std::fs::symlink_metadata(&temp) {
        if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
            return Err(transaction_error(
                &temp,
                "temporary transaction path is not a regular file",
            ));
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

fn read_transaction(staging: &Path) -> Result<ImportTransaction> {
    let path = transaction_path(staging);
    let metadata = std::fs::symlink_metadata(&path).map_err(|e| Error::io(&path, e))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(transaction_error(
            &path,
            "transaction path is not a regular file",
        ));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| Error::io(&path, e))?;
    let transaction: ImportTransaction =
        toml::from_str(&text).map_err(|error| transaction_error(&path, error.to_string()))?;
    if transaction.schema != TRANSACTION_SCHEMA {
        return Err(transaction_error(
            &path,
            format!(
                "unsupported transaction schema {}; expected {TRANSACTION_SCHEMA}",
                transaction.schema
            ),
        ));
    }
    Ok(transaction)
}

fn transaction_path_under_root(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative.is_empty() {
        return Err(transaction_error(root, "transaction path is empty"));
    }
    safe_join(root, Path::new(relative))
}

fn present(path: &Path) -> Result<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(Error::io(path, error)),
    }
}

fn rollback_transaction(
    root: &Path,
    staging: &Path,
    transaction: &ImportTransaction,
) -> Result<()> {
    for item in &transaction.items {
        let staged = transaction_path_under_root(root, &item.staged)?;
        let destination = transaction_path_under_root(root, &item.destination)?;
        let staged_present = present(&staged)?;
        let destination_present = present(&destination)?;
        if staged_present {
            let metadata = std::fs::symlink_metadata(&staged).map_err(|e| Error::io(&staged, e))?;
            if metadata.file_type().is_symlink() {
                return Err(Error::PathEscape {
                    path: staged,
                    root: staging.to_path_buf(),
                });
            }
        }
        if destination_present {
            let metadata =
                std::fs::symlink_metadata(&destination).map_err(|e| Error::io(&destination, e))?;
            if metadata.file_type().is_symlink() {
                return Err(Error::PathEscape {
                    path: destination,
                    root: root.to_path_buf(),
                });
            }
        }
        match (staged_present, destination_present) {
            (true, true) => {
                return Err(transaction_error(
                    &transaction_path(staging),
                    format!(
                        "transaction item exists at both {:?} and {:?}",
                        staged, destination
                    ),
                ));
            }
            (false, true) => {
                if let Some(parent) = staged.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
                }
                std::fs::rename(&destination, &staged).map_err(|e| Error::io(&staged, e))?;
            }
            (true, false) | (false, false) => {}
        }
    }
    std::fs::remove_dir_all(staging).map_err(|e| Error::io(staging, e))
}

/// Recover interrupted imports before starting another mutating import.
///
/// The transaction marker is intentionally internal state. Recovery never
/// runs for `--check`, so preview remains read-only.
pub fn recover(root: &Path) -> Result<()> {
    let metadata = match std::fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(Error::io(root, error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::PathEscape {
            path: root.to_path_buf(),
            root: root.to_path_buf(),
        });
    }
    validate_destination_layout(root)?;
    let staging_root = root.join(".lskills/staging");
    let metadata = match std::fs::symlink_metadata(&staging_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(Error::io(&staging_root, error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::PathEscape {
            path: staging_root,
            root: root.to_path_buf(),
        });
    }

    let mut entries = std::fs::read_dir(&staging_root)
        .map_err(|e| Error::io(&staging_root, e))?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|e| Error::io(&staging_root, e))
        })
        .collect::<Result<Vec<_>>>()?;
    entries.sort();
    for staging in entries {
        let metadata = std::fs::symlink_metadata(&staging).map_err(|e| Error::io(&staging, e))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(Error::PathEscape {
                path: staging,
                root: staging_root.clone(),
            });
        }
        let transaction_path = transaction_path(&staging);
        if !present(&transaction_path)? {
            continue;
        }
        let transaction = read_transaction(&staging)?;
        let provenance = provenance::load(root)?;
        let destination = Repo::load(root)?;
        provenance::validate(&provenance, &destination)?;
        let has_entry = provenance
            .imports
            .iter()
            .any(|entry| entry.bundle == transaction.bundle && entry.digest == transaction.digest);
        let destination_states = transaction
            .items
            .iter()
            .map(|item| {
                transaction_path_under_root(root, &item.destination).and_then(|path| present(&path))
            })
            .collect::<Result<Vec<_>>>()?;
        let all_destinations_present = destination_states.into_iter().all(|state| state);
        if has_entry && all_destinations_present {
            std::fs::remove_dir_all(&staging).map_err(|e| Error::io(&staging, e))?;
            continue;
        }
        if transaction.phase == TransactionPhase::Committed {
            return Err(transaction_error(
                &transaction_path,
                "committed transaction has no matching complete provenance entry",
            ));
        }
        if has_entry {
            return Err(transaction_error(
                &transaction_path,
                "provenance was committed before all destination paths existed",
            ));
        }
        rollback_transaction(root, &staging, &transaction)?;
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
    ensure_roots_disjoint(&source.root, root)?;

    let bundle = source
        .bundles
        .iter()
        .find(|bundle| &bundle.name == bundle_name)
        .ok_or_else(|| Error::Usage(format!("unknown source bundle {:?}", bundle_name.as_str())))?;
    validate_bundle(source, bundle)?;

    let provenance = provenance::load(root)?;
    if root.is_dir() {
        let destination = Repo::load(root)?;
        provenance::validate(&provenance, &destination)?;
    }
    if provenance
        .imports
        .iter()
        .any(|entry| entry.bundle == *bundle_name)
    {
        return Err(Error::ImportCollision {
            path: provenance::path(root),
        });
    }

    let staging = staging_path(root, bundle_name);
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
            let relative = file.relative.clone();
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
    recover(root)?;
    prepare_destination(root)?;
    let mut provenance = provenance::load(root)?;
    let destination = Repo::load(root)?;
    provenance::validate(&provenance, &destination)?;
    provenance.imports.push(ProvenanceEntry {
        bundle: plan.bundle.clone(),
        source: plan.source.clone(),
        selector: format!("bundles/{}.toml", plan.bundle.as_str()),
        revision: plan.revision.clone(),
        digest: plan.digest.clone(),
    });

    let bundle_destination = root
        .join("bundles")
        .join(format!("{}.toml", plan.bundle.as_str()));
    reject_existing(&bundle_destination)?;
    for skill_name in &plan.skills {
        reject_existing(&root.join("skills").join(skill_name.as_str()))?;
    }

    let staging = staging_path(root, &plan.bundle);
    reject_existing(&staging)?;
    std::fs::create_dir_all(staging.join("bundles")).map_err(|e| Error::io(&staging, e))?;
    std::fs::create_dir_all(staging.join("skills")).map_err(|e| Error::io(&staging, e))?;

    let mut items = vec![TransactionItem {
        staged: format!(
            ".lskills/staging/import-{}/bundles/{}.toml",
            plan.bundle.as_str(),
            plan.bundle.as_str()
        ),
        destination: format!("bundles/{}.toml", plan.bundle.as_str()),
    }];
    for skill_name in &plan.skills {
        items.push(TransactionItem {
            staged: format!(
                ".lskills/staging/import-{}/skills/{}",
                plan.bundle.as_str(),
                skill_name.as_str()
            ),
            destination: format!("skills/{}", skill_name.as_str()),
        });
    }
    let mut transaction = ImportTransaction {
        schema: TRANSACTION_SCHEMA,
        bundle: plan.bundle.clone(),
        digest: plan.digest.clone(),
        phase: TransactionPhase::Staging,
        items,
    };
    if let Err(error) = write_transaction(&staging, &transaction) {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(error);
    }

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

    transaction.phase = TransactionPhase::Ready;
    if let Err(error) = write_transaction(&staging, &transaction) {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(error);
    }

    let mut moved = Vec::new();
    for item in &transaction.items {
        let staged = transaction_path_under_root(root, &item.staged)?;
        let destination = transaction_path_under_root(root, &item.destination)?;
        if let Err(error) = move_staged(&staged, &destination, &mut moved) {
            return finish_failed_import(&staging, &mut moved, error);
        }
    }

    transaction.phase = TransactionPhase::Moved;
    if let Err(error) = write_transaction(&staging, &transaction) {
        return finish_failed_import(&staging, &mut moved, error);
    }

    if let Err(error) = provenance::write_atomic(root, &provenance) {
        return finish_failed_import(&staging, &mut moved, error);
    }

    transaction.phase = TransactionPhase::Committed;
    // Materialization and provenance are complete. Leave the marker for the
    // next mutating invocation to clean up safely if this write fails.
    write_transaction(&staging, &transaction)?;

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

fn finish_failed_import(
    staging: &Path,
    moved: &mut Vec<(PathBuf, PathBuf)>,
    error: Error,
) -> Result<ImportResult> {
    rollback(moved)?;
    let _ = std::fs::remove_dir_all(staging);
    Err(error)
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
    reject_symlink_root(root)?;
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
            .to_str()
            .ok_or_else(|| {
                Error::io(
                    path,
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "non-UTF-8 filename"),
                )
            })?
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

fn rollback(moved: &mut Vec<(PathBuf, PathBuf)>) -> Result<()> {
    while let Some((destination, source)) = moved.pop() {
        if let Some(parent) = source.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        std::fs::rename(&destination, &source).map_err(|e| Error::io(&source, e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn recovery_rolls_back_a_partially_moved_transaction() {
        let root = TempDir::new().unwrap();
        for directory in ["skills", "bundles", ".lskills/staging"] {
            std::fs::create_dir_all(root.path().join(directory)).unwrap();
        }
        let staging = staging_path(root.path(), &BundleName::parse("demo").unwrap());
        std::fs::create_dir_all(&staging).unwrap();
        let transaction = ImportTransaction {
            schema: TRANSACTION_SCHEMA,
            bundle: BundleName::parse("demo").unwrap(),
            digest: "sha256-".to_string() + &"a".repeat(64),
            phase: TransactionPhase::Ready,
            items: vec![TransactionItem {
                staged: ".lskills/staging/import-demo/bundles/demo.toml".into(),
                destination: "bundles/demo.toml".into(),
            }],
        };
        write_transaction(&staging, &transaction).unwrap();
        std::fs::write(
            root.path().join("bundles/demo.toml"),
            "name = \"demo\"\nskills = []\n",
        )
        .unwrap();

        recover(root.path()).unwrap();

        assert!(!root.path().join("bundles/demo.toml").exists());
        assert!(!staging.exists());
    }

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
