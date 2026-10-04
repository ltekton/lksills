//! Generate a bundle's Claude plugin tree: `plugins/<bundle>/…`.

use std::path::Path;

use serde_json::json;

use crate::error::Result;
use crate::frontmatter;
use crate::generate::walk::{file_mode, walk_skill};
use crate::model::Bundle;
use crate::names::SkillFullName;
use crate::render::{Artifact, RenderMap};
use crate::repo::Repo;

/// The Claude plugin schema URL.
const PLUGIN_SCHEMA: &str = "https://anthropic.com/claude-code/plugin.schema.json";

/// The `.claude-plugin/plugin.json` document for a bundle.
pub fn plugin_json(bundle: &Bundle) -> serde_json::Value {
    json!({
        "$schema": PLUGIN_SCHEMA,
        "name": bundle.name.as_str(),
        "version": bundle.version,
        "description": bundle.description,
        "author": { "name": bundle.author },
        "skills": ["./skills"],
    })
}

/// Add a bundle's plugin tree (plugin.json + copied skills) to `map`.
///
/// Each member skill is copied under `plugins/<bundle>/skills/<short>/…` with
/// the `<bundle>-` prefix stripped from its first frontmatter `name:` line.
/// Non-member or missing skills are skipped; a skill whose name is not prefixed
/// by the bundle fails the render.
pub fn add_plugin_tree(map: &mut RenderMap, repo: &Repo, bundle: &Bundle) -> Result<()> {
    let plugin_dir = format!("plugins/{}", bundle.name);
    map.insert(
        format!("{plugin_dir}/.claude-plugin/plugin.json"),
        Artifact::Json(plugin_json(bundle)),
    );

    for full_name in &bundle.skills {
        let Some(skill) = repo.skills.get(full_name) else {
            continue;
        };
        let short = full_name.require_prefix(&bundle.name)?;
        for (rel, artifact) in skill_artifacts(&skill.path, short.as_str())? {
            map.insert(format!("{plugin_dir}/skills/{short}/{rel}"), artifact);
        }
    }
    Ok(())
}

/// Assemble one skill's installable files, keyed by skill-relative POSIX path.
///
/// The `SKILL.md` frontmatter `name:` line is rewritten to `install_name` (the
/// short name, so the directory name matches the skill's identity and it stays
/// discoverable); every other file is a verbatim copy preserving its `+x` bit.
/// Shared by the Claude plugin tree and `install` so the two produce byte-for-
/// byte identical skill trees.
pub fn skill_artifacts(skill_dir: &Path, install_name: &str) -> Result<Vec<(String, Artifact)>> {
    let mut out = Vec::new();
    for file in walk_skill(skill_dir)? {
        let artifact = if file.rel_posix == "SKILL.md" {
            let text = std::fs::read_to_string(&file.abs)
                .map_err(|e| crate::error::Error::io(&file.abs, e))?;
            Artifact::Text(frontmatter::rewrite_name(&text, install_name))
        } else {
            Artifact::CopyVerbatim {
                from: file.abs.clone(),
                mode: file_mode(&file.abs)?,
            }
        };
        out.push((file.rel_posix, artifact));
    }
    Ok(out)
}

/// One marketplace `plugins[]` entry for a bundle.
pub fn marketplace_entry(bundle: &Bundle) -> serde_json::Value {
    json!({
        "name": bundle.name.as_str(),
        "source": format!("./plugins/{}", bundle.name),
        "description": bundle.description,
        "author": { "name": bundle.author },
    })
}

/// Whether a bundle publishes to Claude at all.
pub fn targets_claude(bundle: &Bundle) -> bool {
    bundle.agents.contains(&crate::model::Agent::Claude)
}

/// Helper kept public for tests: the short name of a member skill.
pub fn short_name(full: &SkillFullName, bundle: &Bundle) -> Result<crate::names::SkillShortName> {
    full.require_prefix(&bundle.name)
}
