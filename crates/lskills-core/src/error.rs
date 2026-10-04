//! Error and result types for the lskills engine.
//!
//! The library uses `thiserror` so callers (the CLI) can match on variants to
//! choose exit codes. `anyhow` lives only at the CLI boundary.

use std::path::PathBuf;

/// The engine's result alias.
pub type Result<T> = std::result::Result<T, Error>;

/// Everything the engine can fail with.
///
/// Variants map to CLI exit codes: [`Error::Usage`] is exit 2, most others are
/// exit 1. Gate *failures* (drift, validation findings) are not errors — they
/// are represented by typed results whose `gate()` reports failure.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A name did not satisfy the `^[a-z0-9]+(-[a-z0-9]+)*$` (≤64) contract.
    #[error("invalid {kind} name {value:?}: {reason}")]
    InvalidName {
        /// What kind of name was being parsed (bundle, skill, ...).
        kind: &'static str,
        /// The offending input.
        value: String,
        /// Why it was rejected.
        reason: &'static str,
    },

    /// A skill's full name is not prefixed by its bundle.
    #[error("skill {skill:?} is not prefixed by its bundle {bundle:?}")]
    BadPrefix {
        /// The bundle that should prefix the skill.
        bundle: String,
        /// The skill name that lacked the prefix.
        skill: String,
    },

    /// A path escaped its containing root (traversal or symlink).
    #[error("path {path:?} escapes its root {root:?}")]
    PathEscape {
        /// The offending path.
        path: PathBuf,
        /// The root it was expected to stay within.
        root: PathBuf,
    },

    /// Frontmatter was missing, unterminated, or otherwise malformed.
    #[error("frontmatter error in {path:?}: {reason}")]
    Frontmatter {
        /// The file whose frontmatter failed.
        path: PathBuf,
        /// Why it failed.
        reason: String,
    },

    /// A bundle TOML failed to parse or was structurally invalid.
    #[error("bundle {path:?}: {reason}")]
    Bundle {
        /// The bundle file.
        path: PathBuf,
        /// Why it failed.
        reason: String,
    },

    /// The repo-level `.lskills.toml` config failed to parse.
    #[error("config {path:?}: {reason}")]
    Config {
        /// The config file.
        path: PathBuf,
        /// Why it failed.
        reason: String,
    },

    /// A provenance sidecar failed validation or serialization.
    #[error("provenance {path:?}: {reason}")]
    Provenance {
        /// The provenance sidecar.
        path: PathBuf,
        /// Why the record is invalid.
        reason: String,
    },

    /// An imported bundle or skill already exists in the workarea.
    #[error("import collision at {path:?}")]
    ImportCollision {
        /// The colliding destination.
        path: PathBuf,
    },

    /// The skills-root was missing or not a directory.
    #[error("skills root {path:?} is not a directory")]
    NotARoot {
        /// The candidate root.
        path: PathBuf,
    },

    /// A misused command-line invocation (maps to exit 2).
    #[error("{0}")]
    Usage(String),

    /// An external command (git/gh) failed.
    #[error("command {command:?} failed: {reason}")]
    Command {
        /// The command that failed.
        command: String,
        /// Captured reason (stderr or spawn error).
        reason: String,
    },

    /// Underlying I/O failure, annotated with the path in play.
    #[error("io error at {path:?}: {source}")]
    Io {
        /// The path being operated on when the failure occurred.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
}

impl Error {
    /// Construct an [`Error::Io`] tagged with the path being touched.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.into(),
            source,
        }
    }

    /// A short, stable machine code for the `--json` error envelope.
    pub fn code(&self) -> &'static str {
        match self {
            Error::InvalidName { .. } => "invalid_name",
            Error::BadPrefix { .. } => "bad_prefix",
            Error::PathEscape { .. } => "path_escape",
            Error::Frontmatter { .. } => "frontmatter",
            Error::Bundle { .. } => "bundle",
            Error::Config { .. } => "config",
            Error::Provenance { .. } => "provenance",
            Error::ImportCollision { .. } => "import_collision",
            Error::NotARoot { .. } => "not_a_root",
            Error::Usage(_) => "usage",
            Error::Command { .. } => "command",
            Error::Io { .. } => "io",
        }
    }
}
