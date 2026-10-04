//! Parse a Brewfile-style `Skillfile`: one bundle name per line.
//!
//! Blank lines and `#` comments are ignored; every other line is a bundle name
//! validated into a [`BundleName`]. A malformed name is a usage error so a typo
//! in the file surfaces clearly rather than silently installing nothing.

use std::path::Path;

use crate::error::{Error, Result};
use crate::names::BundleName;

/// Parse a `Skillfile` at `path` into its listed bundle names, in file order.
pub fn load(path: &Path) -> Result<Vec<BundleName>> {
    let text = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
    parse(&text)
}

/// Parse `Skillfile` text into bundle names.
pub fn parse(text: &str) -> Result<Vec<BundleName>> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = match raw.split_once('#') {
            Some((before, _)) => before.trim(),
            None => raw.trim(),
        };
        if line.is_empty() {
            continue;
        }
        let name = BundleName::parse(line).map_err(|e| Error::Usage(format!("Skillfile: {e}")))?;
        out.push(name);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_comments_and_blanks() {
        let names = parse("# header\n\ndemo\n  only  # trailing comment\n").unwrap();
        assert_eq!(
            names.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
            vec!["demo", "only"]
        );
    }

    #[test]
    fn bad_name_is_usage_error() {
        let err = parse("Demo Bundle\n").unwrap_err();
        assert!(matches!(err, Error::Usage(_)));
    }

    #[test]
    fn empty_file_is_empty_list() {
        assert!(parse("\n#only comments\n").unwrap().is_empty());
    }
}
