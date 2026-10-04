//! Assemble the full [`RenderMap`] for a repo from the three generators.

pub mod marketplace;
pub mod pi;
pub mod plugin;
pub mod walk;

use crate::error::Result;
use crate::render::RenderMap;
use crate::repo::Repo;

/// Render every generated artifact for `repo`.
///
/// `tool_version` is injected (rather than read from the environment deep in the
/// code) so callers control it and tests stay deterministic; it becomes the pi
/// manifest `version`.
pub fn render(repo: &Repo, tool_version: &str) -> Result<RenderMap> {
    let mut map = RenderMap::default();

    for bundle in &repo.bundles {
        if plugin::targets_claude(bundle) {
            plugin::add_plugin_tree(&mut map, repo, bundle)?;
        }
    }

    marketplace::add_marketplace(&mut map, repo);
    pi::add_pi_manifest(&mut map, repo, tool_version);

    Ok(map)
}
