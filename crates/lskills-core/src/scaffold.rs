//! Scaffold and remove skill/bundle source files in a skills-root.
//!
//! `new_*` create a minimal, valid source file and **refuse to overwrite** an
//! existing target; `rm_*` delete and error if the target is absent. Names are
//! validated (they arrive as newtypes) before any filesystem access, so a bad
//! name can never touch disk. These are authoring verbs: they never touch git.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::model::Agent;
use crate::names::{BundleName, SkillFullName};

/// The default author stamped into a new bundle.
const DEFAULT_AUTHOR: &str = "ltekton";
/// The default starting version for a new bundle.
const DEFAULT_VERSION: &str = "0.0.0";

/// Create `skills/<full>/SKILL.md` with a minimal valid stub.
///
/// Errors (usage) if the skill directory already exists, so authored content is
/// never clobbered.
pub fn new_skill(root: &Path, full: &SkillFullName) -> Result<PathBuf> {
    let dir = root.join("skills").join(full.as_str());
    if dir.exists() {
        return Err(Error::Usage(format!(
            "skill {:?} already exists at {dir:?}",
            full.as_str()
        )));
    }
    std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;

    let md = dir.join("SKILL.md");
    let body = format!(
        "---\nname: {name}\ndescription: TODO: describe {name}\n---\n\n# {name}\n\nTODO: write this skill.\n",
        name = full.as_str()
    );
    std::fs::write(&md, body).map_err(|e| Error::io(&md, e))?;
    Ok(md)
}

/// Create `bundles/<name>.toml` with canonical, valid defaults.
///
/// Errors (usage) if the file already exists.
pub fn new_bundle(root: &Path, name: &BundleName) -> Result<PathBuf> {
    let dir = root.join("bundles");
    std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;

    let file = dir.join(format!("{name}.toml"));
    if file.exists() {
        return Err(Error::Usage(format!(
            "bundle {:?} already exists at {file:?}",
            name.as_str()
        )));
    }
    std::fs::write(&file, bundle_toml(name)).map_err(|e| Error::io(&file, e))?;
    Ok(file)
}

/// Remove `skills/<full>/`. Errors (usage) if absent.
pub fn rm_skill(root: &Path, full: &SkillFullName) -> Result<PathBuf> {
    let dir = root.join("skills").join(full.as_str());
    if !dir.exists() {
        return Err(Error::Usage(format!(
            "skill {:?} does not exist at {dir:?}",
            full.as_str()
        )));
    }
    std::fs::remove_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    Ok(dir)
}

/// Remove `bundles/<name>.toml`. Errors (usage) if absent.
pub fn rm_bundle(root: &Path, name: &BundleName) -> Result<PathBuf> {
    let file = root.join("bundles").join(format!("{name}.toml"));
    if !file.exists() {
        return Err(Error::Usage(format!(
            "bundle {:?} does not exist at {file:?}",
            name.as_str()
        )));
    }
    std::fs::remove_file(&file).map_err(|e| Error::io(&file, e))?;
    Ok(file)
}

/// The canonical TOML for a fresh bundle: all agents, empty skill list.
fn bundle_toml(name: &BundleName) -> String {
    let agents = Agent::ALL
        .iter()
        .map(|a| format!("\"{a}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "name = \"{name}\"\n\
         version = \"{DEFAULT_VERSION}\"\n\
         description = \"\"\n\
         author = \"{DEFAULT_AUTHOR}\"\n\
         skills = []\n\
         agents = [{agents}]\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Bundle;

    fn tmp() -> tempfile::TempDir {
        tempfile::TempDir::new().unwrap()
    }

    #[test]
    fn new_skill_creates_loadable_stub_and_refuses_overwrite() {
        let root = tmp();
        let full = SkillFullName::parse("demo-extra").unwrap();
        let md = new_skill(root.path(), &full).unwrap();
        assert!(md.exists());
        // The stub parses as valid frontmatter with the full name.
        let text = std::fs::read_to_string(&md).unwrap();
        let fm = crate::frontmatter::parse(&md, &text).unwrap();
        assert_eq!(fm.fields.get("name").unwrap(), "demo-extra");
        // Second create refuses.
        assert!(matches!(
            new_skill(root.path(), &full),
            Err(Error::Usage(_))
        ));
    }

    #[test]
    fn new_bundle_round_trips_through_parse_and_refuses_overwrite() {
        let root = tmp();
        let name = BundleName::parse("extra").unwrap();
        let file = new_bundle(root.path(), &name).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        let bundle = Bundle::parse(file.clone(), &text).unwrap();
        assert_eq!(bundle.name.as_str(), "extra");
        assert_eq!(bundle.version, DEFAULT_VERSION);
        assert_eq!(bundle.agents, crate::model::all_agents());
        assert!(bundle.skills.is_empty());
        assert!(matches!(
            new_bundle(root.path(), &name),
            Err(Error::Usage(_))
        ));
    }

    #[test]
    fn rm_errors_when_absent_then_removes() {
        let root = tmp();
        let name = BundleName::parse("extra").unwrap();
        assert!(matches!(
            rm_bundle(root.path(), &name),
            Err(Error::Usage(_))
        ));
        new_bundle(root.path(), &name).unwrap();
        rm_bundle(root.path(), &name).unwrap();
        assert!(!root.path().join("bundles/extra.toml").exists());
    }
}
