//! Load the source-of-truth tree (`skills/` + `bundles/`) into a [`Repo`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::frontmatter;
use crate::model::{Bundle, Skill};
use crate::names::SkillFullName;

/// A loaded skills-root: its skills keyed by full name, and its bundles.
#[derive(Debug, Clone)]
pub struct Repo {
    /// Absolute root directory.
    pub root: PathBuf,
    /// Loaded skills, keyed by full name (sorted by the map).
    pub skills: BTreeMap<SkillFullName, Skill>,
    /// Loaded bundles, sorted by file path.
    pub bundles: Vec<Bundle>,
}

impl Repo {
    /// `<root>/skills`.
    pub fn skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }

    /// `<root>/bundles`.
    pub fn bundles_dir(&self) -> PathBuf {
        self.root.join("bundles")
    }

    /// Load and validate the tree at `root`.
    ///
    /// Skills are read from `skills/<full-name>/SKILL.md`; the frontmatter
    /// `name` is validated into a [`SkillFullName`]. Bundles are read from
    /// `bundles/*.toml`. A malformed skill or bundle fails the whole load — the
    /// engine never operates on a partially valid tree.
    pub fn load(root: &Path) -> Result<Repo> {
        if !root.is_dir() {
            return Err(Error::NotARoot {
                path: root.to_path_buf(),
            });
        }

        let skills = load_skills(&root.join("skills"))?;
        let bundles = load_bundles(&root.join("bundles"))?;

        Ok(Repo {
            root: root.to_path_buf(),
            skills,
            bundles,
        })
    }
}

/// Load every `skills/<name>/` directory, sorted by directory name.
fn load_skills(dir: &Path) -> Result<BTreeMap<SkillFullName, Skill>> {
    let mut out = BTreeMap::new();
    if !dir.is_dir() {
        return Ok(out);
    }

    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| Error::io(dir, e))?
        .map(|e| e.map(|e| e.path()).map_err(|e| Error::io(dir, e)))
        .collect::<Result<Vec<_>>>()?;
    entries.sort();

    for path in entries {
        if !path.is_dir() {
            continue;
        }
        let skill = load_skill(&path)?;
        out.insert(skill.name.clone(), skill);
    }
    Ok(out)
}

/// Load one `skills/<name>/` directory.
fn load_skill(path: &Path) -> Result<Skill> {
    let md = path.join("SKILL.md");
    let text = std::fs::read_to_string(&md).map_err(|e| Error::io(&md, e))?;
    let fm = frontmatter::parse(&md, &text)?;

    let name = fm.fields.get("name").ok_or_else(|| Error::Frontmatter {
        path: md.clone(),
        reason: "missing required `name` field".into(),
    })?;
    let name = SkillFullName::parse(name.clone())?;

    // The directory name is the skill's identity on disk; the frontmatter `name`
    // must match it, so a skill cannot claim to be some other skill (which would
    // let it forge bundle membership or shadow a real skill).
    if let Some(dir) = path.file_name().and_then(|s| s.to_str()) {
        if dir != name.as_str() {
            return Err(Error::Frontmatter {
                path: md.clone(),
                reason: format!(
                    "frontmatter name {:?} does not match directory name {dir:?}",
                    name.as_str()
                ),
            });
        }
    }

    let description = fm
        .fields
        .get("description")
        .cloned()
        .ok_or_else(|| Error::Frontmatter {
            path: md.clone(),
            reason: "missing required `description` field".into(),
        })?;

    if description.is_empty() {
        return Err(Error::Frontmatter {
            path: md.clone(),
            reason: "description is empty".into(),
        });
    }
    if description.len() > 1024 {
        return Err(Error::Frontmatter {
            path: md.clone(),
            reason: format!(
                "description is {} chars; maximum is 1024",
                description.len()
            ),
        });
    }

    Ok(Skill {
        name,
        path: path.to_path_buf(),
        description,
        body: fm.body,
    })
}

/// Load every `bundles/*.toml`, sorted by path.
fn load_bundles(dir: &Path) -> Result<Vec<Bundle>> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }

    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| Error::io(dir, e))?
        .map(|e| e.map(|e| e.path()).map_err(|e| Error::io(dir, e)))
        .collect::<Result<Vec<_>>>()?;
    paths.retain(|p| p.extension().is_some_and(|ext| ext == "toml"));
    paths.sort();

    for path in paths {
        let contents = std::fs::read_to_string(&path).map_err(|e| Error::io(&path, e))?;
        let bundle = Bundle::parse(path.clone(), &contents)?;
        // The filename stem is the bundle's identity on disk; the `name` field
        // must match it, so `bundles/demo.toml` cannot declare `name = "other"`
        // (which would misroute prefix checks and confuse every downstream verb).
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if stem != bundle.name.as_str() {
                return Err(Error::Bundle {
                    path: path.clone(),
                    reason: format!(
                        "bundle name {:?} does not match filename stem {stem:?}",
                        bundle.name.as_str()
                    ),
                });
            }
        }
        out.push(bundle);
    }
    Ok(out)
}
