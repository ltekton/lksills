//! Style, secret, and coreutils-portability scanning.
//!
//! Every hygiene finding is gate-failing — there is no warning tier. The scan
//! walks the repo for text files (by suffix, pruning noise dirs), and applies:
//! em-dash detection on markdown prose (frontmatter + code stripped), a set of
//! secret-shape patterns, and BSD-vs-GNU coreutils traps in shell scripts.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;
use walkdir::WalkDir;

use crate::error::{Error, Result};

/// One hygiene finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HygieneFinding {
    /// Stable machine code.
    pub code: String,
    /// Repo-relative POSIX path.
    pub file: String,
    /// 1-based line number.
    pub line: usize,
    /// Human message.
    pub message: String,
}

/// The result of a hygiene scan.
#[derive(Debug, Clone, Serialize)]
pub struct HygieneResult {
    /// Schema version.
    pub v: u32,
    /// Whether the gate passes (no findings).
    pub ok: bool,
    /// All findings, sorted by `(file, line, code)`.
    pub findings: Vec<HygieneFinding>,
}

impl HygieneResult {
    /// Whether the gate passes.
    pub fn is_ok(&self) -> bool {
        self.ok
    }
}

/// Directory segments pruned wherever they appear in the scanned tree.
///
/// These are conventional build/scratch/tooling directories whose contents are
/// not authored skill prose, so scanning them would only produce noise. The list
/// is fixed, not configurable: it is applied to *any* `--root`, so an external
/// skills-root that deliberately keeps skill sources under a directory named e.g.
/// `tests/` would have those skipped. That is an accepted trade-off - skills live
/// under `skills/`, and these names are near-universally tooling directories - but
/// it is the reason the set is intentionally small and well-known rather than
/// broad.
const SKIP_DIRS: &[&str] = &[
    ".git",
    ".venv",
    "__pycache__",
    ".pytest_cache",
    "node_modules",
    "tests",
    "evals",
    "reviews",
    "dist",
    ".astro",
];

/// Suffixes considered text and scanned.
const TEXT_SUFFIXES: &[&str] = &[
    "md", "toml", "json", "sh", "py", "txt", "adapter", "yml", "yaml", "go", "mjs", "js", "ts",
    "tsx", "css", "html",
];

/// En dash, em dash, horizontal bar (U+2212 minus deliberately excluded).
static DASH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\u{2013}\u{2014}\u{2015}]").unwrap());

/// BSD `sed -i ''` / `sed -i ""` empty-suffix form.
static SED_I_EMPTY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"sed\b[^#\n]*?-i[ \t]+(''|"")"#).unwrap());

/// Secret-shape patterns; the first match on a line wins.
static SECRET_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"gh[posru]_[A-Za-z0-9]{20,}",
        r"github_pat_[A-Za-z0-9_]{22,}",
        r"(AKIA|ASIA)[0-9A-Z]{16}",
        r"-----BEGIN [A-Z ]*PRIVATE KEY-----",
        r#"(?i)aws_secret_access_key["']?\s*[:=]\s*["']?[A-Za-z0-9/+]{40}"#,
        r"xox[baprs]-[A-Za-z0-9-]{10,}",
        r"AIza[0-9A-Za-z_\-]{35}",
        r"sk-(proj-)?[A-Za-z0-9_\-]{20,}",
        r"eyJ[A-Za-z0-9_\-]+\.eyJ[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+",
    ]
    .iter()
    .map(|p| Regex::new(p).unwrap())
    .collect()
});

/// Scan the repo rooted at `root` for hygiene findings.
pub fn scan(root: &Path) -> Result<HygieneResult> {
    let mut findings = Vec::new();

    for path in text_files(root)? {
        let rel = rel_posix(root, &path);
        let text = std::fs::read_to_string(&path).map_err(|e| Error::io(&path, e))?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        if ext == "md" {
            scan_em_dash(&rel, &text, &mut findings);
        }
        if ext == "sh" {
            scan_coreutils(&rel, &text, &mut findings);
        }
        scan_secrets(&rel, &text, &mut findings);
    }

    findings.sort_by(|a, b| {
        (a.file.as_str(), a.line, a.code.as_str()).cmp(&(&b.file, b.line, &b.code))
    });

    Ok(HygieneResult {
        v: 1,
        ok: findings.is_empty(),
        findings,
    })
}

/// Recursively collect text files (by suffix), pruning noise dirs, sorted by
/// POSIX relative path.
fn text_files(root: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !(e.file_type().is_dir()
                && e.file_name()
                    .to_str()
                    .is_some_and(|n| SKIP_DIRS.contains(&n)))
        })
    {
        let entry = entry.map_err(|e| {
            Error::io(
                e.path().unwrap_or(root).to_path_buf(),
                e.into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("walk error")),
            )
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let has_text_suffix = entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| TEXT_SUFFIXES.contains(&ext));
        if has_text_suffix {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

/// The repo-relative forward-slash path of `path` under `root`.
fn rel_posix(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Split like Python `str.splitlines`.
fn split_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = normalized.strip_suffix('\n').unwrap_or(&normalized);
    trimmed.split('\n').map(str::to_string).collect()
}

/// Em-dash scan over markdown prose: skip a leading frontmatter block, skip
/// fenced/indented code, strip inline-code spans and link/autolink targets.
fn scan_em_dash(file: &str, text: &str, out: &mut Vec<HygieneFinding>) {
    let lines = split_lines(text);
    let mut in_fence = false;
    let mut fence: Option<char> = None;

    // Skip a leading `---` frontmatter block (only if it opens on line 0).
    let mut start = 0;
    if lines.first().map(|l| l.trim()) == Some("---") {
        if let Some(end) = lines.iter().skip(1).position(|l| l.trim() == "---") {
            start = end + 2; // consume through the closing fence
        }
    }

    for (idx, line) in lines.iter().enumerate().skip(start) {
        let trimmed = line.trim_start_matches([' ', '\t']);
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let delim = trimmed.chars().next().unwrap();
            match (in_fence, fence) {
                (false, _) => {
                    in_fence = true;
                    fence = Some(delim);
                }
                (true, Some(f)) if f == delim => {
                    in_fence = false;
                    fence = None;
                }
                _ => {}
            }
            continue;
        }
        if in_fence {
            continue;
        }
        // Indented code (tab or >= 4 spaces) carries no prose.
        if line.starts_with('\t') || line.starts_with("    ") {
            continue;
        }
        let prose = strip_urls(&strip_inline_code(line));
        if DASH_RE.is_match(&prose) {
            out.push(HygieneFinding {
                code: "em_dash".into(),
                file: file.to_string(),
                line: idx + 1,
                message: "em/en dash; use a hyphen".into(),
            });
        }
    }
}

/// Remove backtick-delimited inline code spans. An opener of N backticks closes
/// on the next run of exactly N; an unclosed opener eats the rest of the line.
fn strip_inline_code(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '`' {
            let mut n = 0;
            while i + n < chars.len() && chars[i + n] == '`' {
                n += 1;
            }
            // Find a closing run of exactly n backticks.
            let mut j = i + n;
            let mut closed = None;
            while j < chars.len() {
                if chars[j] == '`' {
                    let mut m = 0;
                    while j + m < chars.len() && chars[j + m] == '`' {
                        m += 1;
                    }
                    if m == n {
                        closed = Some(j + m);
                        break;
                    }
                    j += m;
                } else {
                    j += 1;
                }
            }
            match closed {
                Some(end) => i = end,
                None => break, // unclosed: drop the rest
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Remove markdown link targets `](...)` and `<scheme://...>` autolinks.
static URL_TARGET_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\]\([^)]*\)|<[a-zA-Z][a-zA-Z0-9+.\-]*://[^>]*>").unwrap());

fn strip_urls(line: &str) -> String {
    URL_TARGET_RE.replace_all(line, "").into_owned()
}

/// Drop quoted content (keeping the delimiters) and truncate at the first
/// unquoted `#` comment, per line.
fn strip_shell(line: &str) -> String {
    let mut out = String::new();
    let mut quote: Option<char> = None;
    for ch in line.chars() {
        match quote {
            Some(q) => {
                if ch == q {
                    out.push(ch); // keep closing delimiter
                    quote = None;
                }
                // else: drop quoted interior
            }
            None => {
                if ch == '\'' || ch == '"' {
                    out.push(ch); // keep opening delimiter
                    quote = Some(ch);
                } else if ch == '#' {
                    break; // unquoted comment: truncate
                } else {
                    out.push(ch);
                }
            }
        }
    }
    out
}

/// Coreutils portability scan over shell scripts.
fn scan_coreutils(file: &str, text: &str, out: &mut Vec<HygieneFinding>) {
    let lines: Vec<String> = split_lines(text).iter().map(|l| strip_shell(l)).collect();

    let mut per_file: Vec<HygieneFinding> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let ln = idx + 1;
        if SED_I_EMPTY_RE.is_match(line) {
            per_file.push(finding("coreutils_sed", file, ln,
                "sed -i '' is the BSD in-place form; GNU sed needs `sed -i` with no arg (write portably)"));
        }
        if line.contains("date -j") {
            per_file.push(finding(
                "coreutils_date",
                file,
                ln,
                "date -j is BSD-only; use a GNU-compatible date invocation",
            ));
        }
        if line.contains("readlink -f") {
            per_file.push(finding("coreutils_readlink", file, ln,
                "readlink -f is not portable to BSD/macOS; prefer a GNU-first form or a pwd -P shim"));
        }
    }

    // stat -f / stat -c ordering: at most one finding per file, keyed on the
    // first `stat -f`'s absolute offset relative to the first `stat -c`.
    if let Some(stat) = coreutils_stat(file, &lines) {
        per_file.push(stat);
    }

    per_file.sort_by_key(|f| f.line);
    out.extend(per_file);
}

/// Detect a BSD-first `stat -f` without a GNU-first `stat -c`.
fn coreutils_stat(file: &str, lines: &[String]) -> Option<HygieneFinding> {
    let mut offset = 0usize;
    let mut pos_f: Option<(usize, usize)> = None; // (abs offset, 1-based line)
    let mut pos_c: Option<usize> = None;
    for (idx, line) in lines.iter().enumerate() {
        if pos_f.is_none() {
            if let Some(col) = line.find("stat -f") {
                pos_f = Some((offset + col, idx + 1));
            }
        }
        if pos_c.is_none() {
            if let Some(col) = line.find("stat -c") {
                pos_c = Some(offset + col);
            }
        }
        offset += line.chars().count() + 1; // +1 for the joining newline
    }

    match (pos_f, pos_c) {
        (None, _) => None,
        (Some((_, line_f)), None) => Some(finding(
            "coreutils_stat",
            file,
            line_f,
            "stat -f without GNU-first stat -c fallback",
        )),
        (Some((off_f, line_f)), Some(off_c)) if off_f < off_c => Some(finding(
            "coreutils_stat",
            file,
            line_f,
            "stat -f precedes stat -c; put the GNU stat -c form first",
        )),
        _ => None,
    }
}

/// Secret-shape scan: first matching pattern per line yields one finding.
fn scan_secrets(file: &str, text: &str, out: &mut Vec<HygieneFinding>) {
    for (idx, line) in split_lines(text).iter().enumerate() {
        if SECRET_PATTERNS.iter().any(|re| re.is_match(line)) {
            out.push(finding(
                "secret_shape",
                file,
                idx + 1,
                "possible credential",
            ));
        }
    }
}

fn finding(code: &str, file: &str, line: usize, message: &str) -> HygieneFinding {
    HygieneFinding {
        code: code.to_string(),
        file: file.to_string(),
        line,
        message: message.to_string(),
    }
}
