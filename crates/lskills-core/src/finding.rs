//! Shared finding types used by validate, hygiene, and doctor.

use serde::Serialize;

/// Severity of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// Blocks the gate.
    Error,
    /// Advisory; does not block the gate.
    Warning,
    /// Informational.
    Info,
}

/// A single validation or hygiene finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// Severity.
    pub level: Level,
    /// A short, stable machine code (e.g. `missing_description`).
    pub code: String,
    /// A human-readable message.
    pub message: String,
}

impl Finding {
    /// An error-level finding.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Finding {
            level: Level::Error,
            code: code.into(),
            message: message.into(),
        }
    }

    /// A warning-level finding.
    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Finding {
            level: Level::Warning,
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Whether a set of findings passes its gate (no error-level findings).
pub fn gate_ok(findings: &[Finding]) -> bool {
    !findings.iter().any(|f| f.level == Level::Error)
}
