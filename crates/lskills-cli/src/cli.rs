//! Clap command-line definition.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Author agent skills once and publish them natively to claude, codex, and pi.
#[derive(Debug, Parser)]
#[command(name = "lskills", version, about)]
pub struct Cli {
    /// Skills-root to operate on (default: $LSKILLS_ROOT, else the current dir).
    #[arg(long, global = true)]
    pub root: Option<PathBuf>,

    /// Fetch a remote skills repo instead of a local root: `<spec>[@ref]`
    /// (e.g. `owner/repo@v1`). Read-only; rejected by authoring verbs.
    #[arg(long, global = true, conflicts_with = "root", value_name = "SPEC")]
    pub repo: Option<String>,

    /// Force a clean re-fetch of the `--repo` cache entry.
    #[arg(long, global = true)]
    pub refresh: bool,

    /// Cache directory for `--repo` clones (default: platform cache dir).
    #[arg(long, global = true, env = "LSKILLS_CACHE_DIR", value_name = "DIR")]
    pub cache_dir: Option<PathBuf>,

    /// Emit machine-readable JSON instead of human output.
    #[arg(long, global = true)]
    pub json: bool,

    /// The verb to run.
    #[command(subcommand)]
    pub command: Command,
}

/// The available verbs.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Write every generated artifact, pruning stale files.
    Publish {
        /// Report drift and exit non-zero instead of writing.
        #[arg(long)]
        check: bool,
    },
    /// Import one existing bundle from a local or Git skills-root.
    Import {
        /// Local skills-root path or Git repository spec.
        origin: String,
        /// Bundle to copy from the origin.
        #[arg(long, value_name = "BUNDLE")]
        bundle: String,
        /// Preview the import without writing the workarea.
        #[arg(long)]
        check: bool,
    },
    /// Check bundle membership: unknown/miswired members, empties, orphans.
    Validate,
    /// Estimate always-on and on-invoke token costs per skill and bundle.
    Tokens,
    /// List bundles and skills with inferred dependency edges.
    Catalog,
    /// Scan for em-dashes, secret shapes, and coreutils portability traps.
    Hygiene,
    /// Aggregate drift + validate + hygiene into one health gate.
    Doctor,
    /// Overview of every bundle and skill in the repo.
    List,
    /// Show detail for one bundle or skill.
    Show {
        /// The bundle or skill name to show.
        name: String,
    },
    /// Print the tool's own version.
    Version,
    /// Plan or apply per-bundle version bumps.
    Release {
        /// Whether to plan (show) or apply (write) the bumps.
        #[command(subcommand)]
        action: ReleaseAction,
    },
    /// Install a bundle's skills into each target agent's skills directory.
    Install {
        /// Install to the agent's global config home instead of the project.
        #[arg(short = 'g', long)]
        global: bool,
        /// Restrict to these agents (comma-separated); default = a bundle's own.
        #[arg(long, value_delimiter = ',', value_name = "AGENT")]
        agents: Vec<String>,
        /// Read additional bundle names from a Skillfile.
        #[arg(long, value_name = "FILE")]
        skillfile: Option<PathBuf>,
        /// Project root for project-scope installs (default: current dir).
        #[arg(long, value_name = "DIR")]
        project_dir: Option<PathBuf>,
        /// Bundles to install (merged with any `--skillfile` entries).
        bundles: Vec<String>,
    },
    /// Create a new skill (`skills/<full-name>/SKILL.md`).
    NewSkill {
        /// The skill's full, bundle-prefixed name.
        name: String,
    },
    /// Create a new bundle (`bundles/<name>.toml`).
    NewBundle {
        /// The bundle name.
        name: String,
    },
    /// Remove a skill directory.
    RmSkill {
        /// The skill's full name.
        name: String,
    },
    /// Remove a bundle manifest.
    RmBundle {
        /// The bundle name.
        name: String,
    },
}

/// The `release` sub-actions.
#[derive(Debug, Subcommand)]
pub enum ReleaseAction {
    /// Show the proposed version changes without writing.
    Plan {
        /// A bump, `name=major|minor|patch` (repeatable; SemVer scheme only).
        #[arg(short, long = "bump", value_name = "NAME=LEVEL")]
        bump: Vec<String>,
    },
    /// Write the version changes into the bundle TOMLs.
    Apply {
        /// A bump, `name=major|minor|patch` (repeatable; SemVer scheme only).
        #[arg(short, long = "bump", value_name = "NAME=LEVEL")]
        bump: Vec<String>,
        /// Also `git commit` the rewritten TOMLs (never tags or pushes).
        #[arg(long)]
        commit: bool,
    },
}
