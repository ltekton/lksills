//! Hand-rolled frontmatter parser.
//!
//! Skills carry a leading `---`-fenced block of flat `key: value` scalars. We
//! parse it by hand rather than with a YAML crate: the contract is deliberately
//! flat (no nesting, no lists), `serde_yaml` is archived, and a real YAML parser
//! would silently accept structure the format forbids. This mirrors the Go
//! tool's `parse_frontmatter` exactly and gives precise error control.

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{Error, Result};

/// The parsed pieces of a SKILL.md: flat frontmatter fields plus the body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frontmatter {
    /// Flat `key -> value` scalars from the fenced block.
    pub fields: BTreeMap<String, String>,
    /// The markdown body after the closing `---`.
    pub body: String,
}

/// Split text into lines like Python's `str.splitlines`: treat `\n`, `\r\n`,
/// and bare `\r` as separators and drop a single trailing separator, so there
/// is no empty final element.
///
/// Returns owned strings because CRLF normalization cannot borrow the input;
/// frontmatter blocks are tiny, so the allocation is immaterial.
fn split_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = normalized.strip_suffix('\n').unwrap_or(&normalized);
    trimmed.split('\n').map(str::to_string).collect()
}

/// Parse a SKILL.md body into frontmatter fields and the remaining body.
///
/// The block must open with a `---` line and close with another `---`. Lines
/// inside are `key: value` split on the first `:`; a line whose key would be
/// indented (leading space/tab) is ignored, matching the flat contract. An
/// unterminated block is an error.
pub fn parse(path: &Path, text: &str) -> Result<Frontmatter> {
    let lines = split_lines(text);
    let mut fields = BTreeMap::new();

    let Some(first) = lines.first() else {
        return Err(Error::Frontmatter {
            path: path.to_path_buf(),
            reason: "file is empty; expected a leading `---` frontmatter block".into(),
        });
    };
    if first.trim() != "---" {
        return Err(Error::Frontmatter {
            path: path.to_path_buf(),
            reason: "missing leading `---` frontmatter fence".into(),
        });
    }

    let mut closed_at = None;
    for (idx, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            closed_at = Some(idx);
            break;
        }
        // Flat scalars only: skip indented (nested) lines, matching the Go parser.
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_string();
            // Duplicate keys are rejected rather than silently last-wins: a
            // second `name:` must not be able to override the first unnoticed.
            if fields.contains_key(&key) {
                return Err(Error::Frontmatter {
                    path: path.to_path_buf(),
                    reason: format!("duplicate frontmatter key {key:?}"),
                });
            }
            fields.insert(key, value.trim().to_string());
        }
    }

    let Some(closed_at) = closed_at else {
        return Err(Error::Frontmatter {
            path: path.to_path_buf(),
            reason: "unterminated frontmatter block (no closing `---`)".into(),
        });
    };

    let body = lines
        .get(closed_at + 1..)
        .map(|rest| rest.join("\n"))
        .unwrap_or_default();

    Ok(Frontmatter { fields, body })
}

/// Rewrite the first `name:` line *inside the leading frontmatter fence* to
/// `new_name`, returning the rewritten SKILL.md text.
///
/// Used when assembling a plugin tree: the copied skill's frontmatter name is
/// the short (prefix-stripped) name, while the body is otherwise byte-identical.
/// Only the first column-zero `name:` line *between the opening and closing
/// `---`* is rewritten - a `name:` line in the markdown body is never touched,
/// and if there is no frontmatter fence the text is returned unchanged. Line
/// endings are preserved per line (a `\r\n` file stays `\r\n`).
pub fn rewrite_name(text: &str, new_name: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut done = false;
    let mut in_fence = false;
    let mut seen_open = false;

    for (line, eol) in SplitKeepEol::new(text) {
        let content = line;
        if !seen_open {
            // Everything up to and including the opening `---` is copied as-is.
            out.push_str(content);
            out.push_str(eol);
            if content.trim() == "---" {
                seen_open = true;
                in_fence = true;
            }
            continue;
        }
        if in_fence && content.trim() == "---" {
            // Closing fence: stop looking for a `name:` to rewrite.
            in_fence = false;
            out.push_str(content);
            out.push_str(eol);
            continue;
        }
        if in_fence
            && !done
            && !content.starts_with(' ')
            && !content.starts_with('\t')
            && content
                .split_once(':')
                .is_some_and(|(k, _)| k.trim() == "name")
        {
            out.push_str(&format!("name: {new_name}"));
            out.push_str(eol);
            done = true;
        } else {
            out.push_str(content);
            out.push_str(eol);
        }
    }
    out
}

/// An iterator over a string's lines that yields each line's content separately
/// from its terminator (`""`, `"\n"`, `"\r\n"`, or `"\r"`), so a rewrite can put
/// the original terminator back and leave CRLF files byte-stable.
struct SplitKeepEol<'a> {
    rest: &'a str,
}

impl<'a> SplitKeepEol<'a> {
    fn new(text: &'a str) -> Self {
        SplitKeepEol { rest: text }
    }
}

impl<'a> Iterator for SplitKeepEol<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        match self.rest.find(['\n', '\r']) {
            None => {
                let line = self.rest;
                self.rest = "";
                Some((line, ""))
            }
            Some(i) => {
                let bytes = self.rest.as_bytes();
                let eol_len = if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                };
                let line = &self.rest[..i];
                let eol = &self.rest[i..i + eol_len];
                self.rest = &self.rest[i + eol_len..];
                Some((line, eol))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p() -> &'static Path {
        Path::new("skills/x/SKILL.md")
    }

    #[test]
    fn parses_flat_fields_and_body() {
        let text = "---\nname: lgit-pages-html\ndescription: A skill.\n---\n# Body\n\ntext\n";
        let fm = parse(p(), text).unwrap();
        assert_eq!(fm.fields.get("name").unwrap(), "lgit-pages-html");
        assert_eq!(fm.fields.get("description").unwrap(), "A skill.");
        assert_eq!(fm.body, "# Body\n\ntext");
    }

    #[test]
    fn missing_fence_is_error() {
        assert!(parse(p(), "# no frontmatter\n").is_err());
    }

    #[test]
    fn unterminated_block_is_error() {
        assert!(parse(p(), "---\nname: x\n").is_err());
    }

    #[test]
    fn indented_lines_are_ignored() {
        let text = "---\nname: x\n  nested: value\n---\n";
        let fm = parse(p(), text).unwrap();
        assert!(!fm.fields.contains_key("nested"));
    }

    #[test]
    fn rewrite_name_replaces_first_name_line() {
        let text = "---\nname: lgit-pages-html\ndescription: d\n---\nbody\n";
        let got = rewrite_name(text, "pages-html");
        assert!(got.contains("name: pages-html"));
        assert!(!got.contains("lgit-pages-html"));
        assert!(got.ends_with('\n'));
    }

    #[test]
    fn rewrite_name_ignores_body_name_line() {
        // A `name:` at column zero in the body (past the closing fence) must not
        // be rewritten - only the frontmatter `name:` is the skill's identity.
        let text = "---\nname: lgit-hello\ndescription: d\n---\nname: not-frontmatter\n";
        let got = rewrite_name(text, "hello");
        assert!(got.contains("name: hello\n"));
        assert!(got.contains("name: not-frontmatter\n"));
    }

    #[test]
    fn rewrite_name_preserves_crlf() {
        // A CRLF file stays CRLF byte-for-byte except the rewritten value.
        let text = "---\r\nname: lgit-hello\r\ndescription: d\r\n---\r\nbody\r\n";
        let got = rewrite_name(text, "hello");
        assert_eq!(
            got,
            "---\r\nname: hello\r\ndescription: d\r\n---\r\nbody\r\n"
        );
    }

    #[test]
    fn rewrite_name_without_fence_is_unchanged() {
        // No frontmatter fence: nothing to rewrite, text returned verbatim.
        let text = "name: looks-like-frontmatter\nbut no fence\n";
        assert_eq!(rewrite_name(text, "x"), text);
    }
}
