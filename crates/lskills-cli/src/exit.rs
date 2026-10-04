//! Exit-code contract: 0 ok, 1 gate failure, 2 usage.

use lskills_core::Error;

/// Process exit codes.
pub const OK: i32 = 0;
/// A gate failed (drift, validation findings) or a runtime error occurred.
pub const GATE: i32 = 1;
/// The invocation was misused.
pub const USAGE: i32 = 2;

/// Map an engine error to its exit code.
///
/// Only [`Error::Usage`] maps to `2` (misuse: bad flags, unknown bundle, a
/// read-only `--repo` on an authoring verb). *Every other* error - a gate
/// failure (drift, findings), an I/O error, a failed git command, a
/// not-a-skills-root - maps to `1`. This deliberately conflates "the gate failed"
/// with "something went wrong": a scripting consumer that needs to tell them
/// apart reads the `--json` `error.code` (`io`, `command`, `not_a_root`, ...),
/// which stays distinct even though the process exit code does not.
pub fn code_for(err: &Error) -> i32 {
    match err {
        Error::Usage(_) => USAGE,
        _ => GATE,
    }
}
