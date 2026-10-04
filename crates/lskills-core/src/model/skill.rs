//! A loaded skill directory.

use std::path::PathBuf;

use serde::Serialize;

use crate::names::SkillFullName;

/// A loaded `skills/<full-name>/` directory.
///
/// The `name` is taken from frontmatter (validated into a [`SkillFullName`]);
/// `body` is the markdown after the frontmatter block.
#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    /// The skill's validated full name.
    pub name: SkillFullName,
    /// Absolute path to the `skills/<full-name>/` directory.
    #[serde(skip)]
    pub path: PathBuf,
    /// The frontmatter `description`.
    pub description: String,
    /// The markdown body after the frontmatter block.
    #[serde(skip)]
    pub body: String,
}

impl Skill {
    /// Whether this skill is templated across surfaces (`master.md` present).
    pub fn is_multi_surface(&self) -> bool {
        self.path.join("master.md").exists()
    }
}
