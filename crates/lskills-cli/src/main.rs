mod cli;
mod dispatch;
mod exit;
mod output;

use clap::Parser;
use clap::error::ErrorKind;

fn main() {
    match cli::Cli::try_parse() {
        Ok(cli) => std::process::exit(dispatch::run(cli)),
        Err(err) => std::process::exit(handle_parse_error(err)),
    }
}

/// Handle a clap parse failure.
///
/// `--help`/`--version` are reported by clap as "errors" that should print to
/// stdout and exit 0; we defer to clap's own rendering for those. A genuine
/// usage error exits 2 (matching [`exit::USAGE`]); under `--json` it is emitted
/// as the same `{"error":{code,message}}` envelope every other error uses, so a
/// scripting consumer sees one stable shape even for a malformed invocation.
fn handle_parse_error(err: clap::Error) -> i32 {
    if matches!(
        err.kind(),
        ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    ) {
        let _ = err.print();
        return exit::OK;
    }
    if wants_json() {
        // clap renders a multi-line, ANSI-decorated message; flatten it to a
        // single clean line for the JSON envelope.
        let message = strip_ansi(&err.to_string())
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("; ");
        output::emit_error(true, "usage", &message);
    } else {
        let _ = err.print();
    }
    exit::USAGE
}

/// Whether `--json` appears anywhere in the raw arguments. Used only when clap
/// failed before producing a parsed `Cli`, so the flag cannot be read normally.
fn wants_json() -> bool {
    std::env::args().skip(1).any(|a| a == "--json")
}

/// Drop ANSI escape sequences clap may embed in its error text.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Skip until the terminating letter of the escape sequence.
            for e in chars.by_ref() {
                if e.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}
