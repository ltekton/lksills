//! Generate the top-level `.claude-plugin/marketplace.json`.

use serde_json::{Value, json};

use crate::generate::plugin::{marketplace_entry, targets_claude};
use crate::render::{Artifact, RenderMap};
use crate::repo::Repo;

/// The marketplace repo-relative path.
pub const MARKETPLACE_PATH: &str = ".claude-plugin/marketplace.json";
/// The marketplace schema URL.
const MARKETPLACE_SCHEMA: &str = "https://anthropic.com/claude-code/marketplace.schema.json";
/// The marketplace `name`.
const MARKETPLACE_NAME: &str = "lskills";
/// The marketplace description, shared with the pi manifest.
pub const MARKETPLACE_DESCRIPTION: &str = "lskills: agent skills for pi, Codex, and Claude Code";
/// The marketplace owner.
const OWNER: &str = "ltekton";

/// Build the marketplace document from every Claude-targeting bundle (sorted).
pub fn marketplace_json(repo: &Repo) -> Value {
    let plugins: Vec<Value> = repo
        .bundles
        .iter()
        .filter(|b| targets_claude(b))
        .map(marketplace_entry)
        .collect();

    json!({
        "$schema": MARKETPLACE_SCHEMA,
        "name": MARKETPLACE_NAME,
        "description": MARKETPLACE_DESCRIPTION,
        "owner": { "name": OWNER, "url": format!("https://github.com/{OWNER}") },
        "plugins": plugins,
    })
}

/// Add the marketplace document to `map`.
pub fn add_marketplace(map: &mut RenderMap, repo: &Repo) {
    map.insert(MARKETPLACE_PATH, Artifact::Json(marketplace_json(repo)));
}
