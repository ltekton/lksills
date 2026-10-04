//! Output helpers: `--json` structured envelopes vs human text.

use serde::Serialize;

/// Print a successful result: JSON to stdout, or a human summary.
pub fn emit<T: Serialize>(json: bool, value: &T, human: impl FnOnce()) {
    if json {
        let mut s = serde_json::to_string_pretty(value).expect("result serializes");
        s.push('\n');
        print!("{s}");
    } else {
        human();
    }
}

/// Print an error. Under `--json`, a `{"error":{code,message}}` envelope goes to
/// stdout for scripting consumers; otherwise the message goes to stderr.
pub fn emit_error(json: bool, code: &str, message: &str) {
    if json {
        let env = serde_json::json!({ "error": { "code": code, "message": message } });
        let mut s = serde_json::to_string_pretty(&env).expect("envelope serializes");
        s.push('\n');
        print!("{s}");
    } else {
        eprintln!("error: {message}");
    }
}
