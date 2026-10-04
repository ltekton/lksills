//! Generate the pi manifest, `package.json`.

use serde_json::{Value, json};

use crate::generate::marketplace::MARKETPLACE_DESCRIPTION;
use crate::model::Agent;
use crate::render::{Artifact, RenderMap};
use crate::repo::Repo;

/// The pi manifest repo-relative path.
pub const PI_MANIFEST_PATH: &str = "package.json";

/// Build the pi `package.json`.
///
/// pi reads `skills/` directly, so the manifest excludes (`!skills/<name>`) any
/// member skill of a bundle that does not target pi. Excludes are sorted for
/// determinism. `version` is the injected tool version — the manifest never
/// self-reads, so a hand-edit drifts and the publish gate catches it.
pub fn pi_manifest(repo: &Repo, tool_version: &str) -> Value {
    let mut excluded: Vec<String> = Vec::new();
    for bundle in &repo.bundles {
        if bundle.agents.contains(&Agent::Pi) {
            continue;
        }
        for skill in &bundle.skills {
            excluded.push(format!("!skills/{skill}"));
        }
    }
    excluded.sort();
    excluded.dedup();

    let mut skills: Vec<Value> = vec![json!("skills")];
    skills.extend(excluded.into_iter().map(Value::from));

    json!({
        "name": "lskills",
        "version": tool_version,
        "description": MARKETPLACE_DESCRIPTION,
        "pi": { "skills": skills },
    })
}

/// Add the pi manifest to `map`.
pub fn add_pi_manifest(map: &mut RenderMap, repo: &Repo, tool_version: &str) {
    map.insert(
        PI_MANIFEST_PATH,
        Artifact::Json(pi_manifest(repo, tool_version)),
    );
}
