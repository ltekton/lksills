//! Resolve the skills-root, run the requested verb, and choose output + exit.

use std::collections::BTreeMap;
use std::path::PathBuf;

use lskills_core::config::RepoConfig;
use lskills_core::install::target::InstallEnv;
use lskills_core::names::{BundleName, SkillFullName};
use lskills_core::release::{BumpLevel, ReleasePlan};
use lskills_core::{
    Agent, AgentSet, Error, InstallResult, Repo, Scope, catalog, dirs, doctor, git, hygiene,
    import as bundle_import, install, list, provenance, publish, release, remote, scaffold,
    skillfile, tokens, validate,
};

use crate::cli::{Cli, Command, ReleaseAction};
use crate::exit;
use crate::output;

/// The tool version, stamped into the pi manifest at render time.
const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run the CLI, returning a process exit code.
pub fn run(cli: Cli) -> i32 {
    match dispatch(&cli) {
        Ok(code) => code,
        Err(err) => {
            output::emit_error(cli.json, err.code(), &err.to_string());
            exit::code_for(&err)
        }
    }
}

/// Resolve root, load the repo, and run the verb.
fn dispatch(cli: &Cli) -> lskills_core::Result<i32> {
    // `version` reports the tool's own version and needs no skills-root.
    if let Command::Version = cli.command {
        let value = serde_json::json!({ "v": 1, "version": TOOL_VERSION });
        output::emit(cli.json, &value, || println!("{TOOL_VERSION}"));
        return Ok(exit::OK);
    }

    // A `--repo` clone is a read-only cache checkout; verbs that mutate the
    // skills-root itself must not run against it.
    if cli.repo.is_some() && mutates_root(&cli.command) {
        return Err(Error::Usage(format!(
            "--repo is read-only; {} mutates the skills-root",
            verb_name(&cli.command)
        )));
    }

    if let Command::Import {
        origin,
        bundle,
        check,
    } = &cli.command
    {
        return dispatch_import(cli, origin, bundle, *check);
    }

    let root = resolve_source(cli)?;

    // Scaffold verbs write source files into the root directly; they operate on
    // the raw root (which may not yet be a loadable repo), so run before load.
    match &cli.command {
        Command::NewSkill { name } => {
            let full = SkillFullName::parse(name.clone())?;
            let path = scaffold::new_skill(&root, &full)?;
            return emit_path(cli.json, "created", &path);
        }
        Command::NewBundle { name } => {
            let name = BundleName::parse(name.clone())?;
            let path = scaffold::new_bundle(&root, &name)?;
            return emit_path(cli.json, "created", &path);
        }
        Command::RmSkill { name } => {
            let full = SkillFullName::parse(name.clone())?;
            let path = scaffold::rm_skill(&root, &full)?;
            return emit_path(cli.json, "removed", &path);
        }
        Command::RmBundle { name } => {
            let name = BundleName::parse(name.clone())?;
            let path = scaffold::rm_bundle(&root, &name)?;
            return emit_path(cli.json, "removed", &path);
        }
        _ => {}
    }

    let repo = Repo::load(&root)?;

    match &cli.command {
        Command::Version
        | Command::Import { .. }
        | Command::NewSkill { .. }
        | Command::NewBundle { .. }
        | Command::RmSkill { .. }
        | Command::RmBundle { .. } => unreachable!("handled before repo load"),
        Command::Publish { check } => {
            let result = if *check {
                publish::check(&repo, TOOL_VERSION)?
            } else {
                publish::write(&repo, TOOL_VERSION)?
            };
            let ok = result.is_ok();
            output::emit(cli.json, &result, || print_publish(&result));
            Ok(if ok { exit::OK } else { exit::GATE })
        }
        Command::Validate => {
            let _provenance = load_provenance(&repo)?;
            let result = validate::validate(&repo);
            let ok = result.is_ok();
            output::emit(cli.json, &result, || print_validate(&result));
            Ok(if ok { exit::OK } else { exit::GATE })
        }
        Command::Tokens => {
            let result = tokens::report(&repo);
            output::emit(cli.json, &result, || print_tokens(&result));
            Ok(exit::OK)
        }
        Command::Catalog => {
            let result = catalog::catalog(&repo);
            output::emit(cli.json, &result, || print_catalog(&result));
            Ok(exit::OK)
        }
        Command::Hygiene => {
            let result = hygiene::scan(&repo.root)?;
            let ok = result.is_ok();
            output::emit(cli.json, &result, || print_hygiene(&result));
            Ok(if ok { exit::OK } else { exit::GATE })
        }
        Command::Doctor => {
            let result = doctor::doctor(&repo, TOOL_VERSION)?;
            let ok = result.is_ok();
            output::emit(cli.json, &result, || print_doctor(&result));
            Ok(if ok { exit::OK } else { exit::GATE })
        }
        Command::List => {
            let provenance = load_provenance(&repo)?;
            let result = list::list_with_provenance(&repo, &provenance);
            output::emit(cli.json, &result, || print_list(&result));
            Ok(exit::OK)
        }
        Command::Show { name } => {
            let provenance = load_provenance(&repo)?;
            let result = list::show_with_provenance(&repo, name, &provenance)?;
            output::emit(cli.json, &result, || print_show(&result));
            Ok(exit::OK)
        }
        Command::Release { action } => {
            let config = RepoConfig::load(&repo.root)?;
            let now = time::OffsetDateTime::now_utc();
            match action {
                ReleaseAction::Plan { bump } => {
                    let bumps = parse_bumps(bump)?;
                    let plan = release::plan(&repo, config.release.scheme, &bumps, now)?;
                    output::emit(cli.json, &plan, || print_plan(&plan));
                    Ok(exit::OK)
                }
                ReleaseAction::Apply { bump, commit } => {
                    let bumps = parse_bumps(bump)?;
                    let plan = release::plan(&repo, config.release.scheme, &bumps, now)?;
                    let result = release::apply(&repo, &plan, *commit)?;
                    output::emit(cli.json, &result, || print_apply(&result));
                    Ok(exit::OK)
                }
            }
        }
        Command::Install {
            global,
            agents,
            skillfile,
            project_dir,
            bundles,
        } => {
            let mut names = match skillfile {
                Some(path) => skillfile::load(path)?,
                None => Vec::new(),
            };
            for b in bundles {
                names.push(BundleName::parse(b.clone())?);
            }
            if names.is_empty() {
                return Err(Error::Usage(
                    "install needs at least one bundle (positional or via --skillfile)".into(),
                ));
            }

            let filter = parse_agents(agents)?;
            let scope = if *global {
                Scope::Global
            } else {
                Scope::Project
            };
            let env = install_env();
            let proj = project_dir
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

            let result = install::install(&repo, &names, scope, filter.as_ref(), &env, &proj)?;
            output::emit(cli.json, &result, || print_install(&result));
            Ok(exit::OK)
        }
    }
}

/// Resolve and run an import. The destination must be explicit so a source
/// import can never silently write into the machinery repository.
fn dispatch_import(
    cli: &Cli,
    origin: &str,
    bundle: &str,
    check: bool,
) -> lskills_core::Result<i32> {
    let root = cli
        .root
        .clone()
        .ok_or_else(|| Error::Usage("import requires --root <WORKAREA>".into()))?;
    let root = resolve_root(Some(root));
    let bundle = BundleName::parse(bundle.to_string())?;
    let resolved = resolve_import_origin(origin, cli)?;
    let origin_info = bundle_import::OriginInfo {
        locator: origin.to_string(),
        resolved_revision: resolved.revision,
    };
    let plan = bundle_import::plan(&resolved.repo, &root, &bundle, &origin_info)?;
    let result = if check {
        plan.preview()
    } else {
        bundle_import::apply(&plan)?
    };
    output::emit(cli.json, &result, || print_import(&result));
    Ok(exit::OK)
}

struct ResolvedImportOrigin {
    repo: Repo,
    revision: Option<String>,
}

/// Resolve a positional origin as a local path first, then as a Git spec.
fn resolve_import_origin(origin: &str, cli: &Cli) -> lskills_core::Result<ResolvedImportOrigin> {
    if origin.trim().is_empty() {
        return Err(Error::Usage("import origin must not be empty".into()));
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let candidate = PathBuf::from(origin);
    let local = if candidate.is_absolute() {
        candidate
    } else {
        cwd.join(candidate)
    };
    if local.exists() {
        bundle_import::validate_source_tree(&local)?;
        let repo = Repo::load(&local)?;
        let revision = git::head(&local)?.map(|head| format!("git:{head}"));
        return Ok(ResolvedImportOrigin { repo, revision });
    }

    let spec = remote::RepoSpec::parse(origin)?;
    let cache = dirs::cache_dir(cli.cache_dir.as_deref(), |k| std::env::var(k).ok())?;
    let path = remote::resolve(&spec, &cache, cli.refresh)?;
    bundle_import::validate_source_tree(&path)?;
    let repo = Repo::load(&path)?;
    let head = git::head(&path)?.ok_or_else(|| Error::Command {
        command: "git rev-parse".into(),
        reason: "origin cache has no resolved revision".into(),
    })?;
    Ok(ResolvedImportOrigin {
        repo,
        revision: Some(format!("git:{head}")),
    })
}

/// Load and validate workarea provenance for inspection commands.
fn load_provenance(repo: &Repo) -> lskills_core::Result<provenance::ProvenanceFile> {
    let value = provenance::load(&repo.root)?;
    provenance::validate(&value, repo)?;
    Ok(value)
}

/// Parse `--agents` values into an [`AgentSet`] filter, or `None` when empty
/// (meaning: install to each bundle's own target agents). An unknown agent name
/// is a usage error.
fn parse_agents(names: &[String]) -> lskills_core::Result<Option<AgentSet>> {
    if names.is_empty() {
        return Ok(None);
    }
    let mut set = AgentSet::new();
    for name in names {
        let agent =
            Agent::parse(name).ok_or_else(|| Error::Usage(format!("unknown agent {name:?}")))?;
        set.insert(agent);
    }
    Ok(Some(set))
}

/// Build an [`InstallEnv`] from the process environment (the injected seam:
/// core takes these as data; only here do we read `std::env`).
fn install_env() -> InstallEnv {
    let var = |k: &str| std::env::var_os(k).map(PathBuf::from);
    InstallEnv {
        home: var("HOME"),
        claude_config_dir: var("CLAUDE_CONFIG_DIR"),
        codex_home: var("CODEX_HOME"),
        pi_coding_agent_dir: var("PI_CODING_AGENT_DIR"),
    }
}

/// Emit a scaffold result: `{v,action,path}` JSON or `<action> <path>` text.
fn emit_path(json: bool, action: &str, path: &std::path::Path) -> lskills_core::Result<i32> {
    let display = path.display().to_string();
    let value = serde_json::json!({ "v": 1, "action": action, "path": display });
    output::emit(json, &value, || println!("{action} {display}"));
    Ok(exit::OK)
}

/// Parse repeated `name=level` bump flags into a validated map. A malformed
/// flag (missing `=`, bad name, unknown level) is a usage error.
fn parse_bumps(flags: &[String]) -> lskills_core::Result<BTreeMap<BundleName, BumpLevel>> {
    let mut map = BTreeMap::new();
    for flag in flags {
        let (name, level) = flag
            .split_once('=')
            .ok_or_else(|| Error::Usage(format!("--bump {flag:?} must be NAME=LEVEL")))?;
        let name = BundleName::parse(name)?;
        let level = BumpLevel::parse(level).ok_or_else(|| {
            Error::Usage(format!("--bump level {level:?} must be major|minor|patch"))
        })?;
        map.insert(name, level);
    }
    Ok(map)
}

/// Resolve the skills-root: explicit `--root`/`$LSKILLS_ROOT` (clap already
/// merged those), else the current directory. Absolutized against cwd.
fn resolve_root(explicit: Option<PathBuf>) -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    lskills_core::root::resolve(explicit.as_deref(), |key| std::env::var(key).ok(), &cwd)
}

/// Resolve the skills-root source: a `--repo` fetch into the cache, else the
/// local `--root`/`$LSKILLS_ROOT`/cwd. `--repo` and `--root` are mutually
/// exclusive (clap enforces), so at most one branch applies.
fn resolve_source(cli: &Cli) -> lskills_core::Result<PathBuf> {
    match &cli.repo {
        Some(spec) => {
            let spec = remote::RepoSpec::parse(spec)?;
            let cache = dirs::cache_dir(cli.cache_dir.as_deref(), |k| std::env::var(k).ok())?;
            remote::resolve(&spec, &cache, cli.refresh)
        }
        None => Ok(resolve_root(cli.root.clone())),
    }
}

/// Whether a verb mutates the skills-root itself (and so must reject `--repo`).
/// `install` writes into *agent* dirs, not the root, so it is allowed.
fn mutates_root(command: &Command) -> bool {
    matches!(
        command,
        Command::Import { .. }
            | Command::Publish { .. }
            | Command::Release { .. }
            | Command::NewSkill { .. }
            | Command::NewBundle { .. }
            | Command::RmSkill { .. }
            | Command::RmBundle { .. }
    )
}

/// A short verb label for error messages.
fn verb_name(command: &Command) -> &'static str {
    match command {
        Command::Import { .. } => "import",
        Command::Publish { .. } => "publish",
        Command::Release { .. } => "release",
        Command::NewSkill { .. } => "new-skill",
        Command::NewBundle { .. } => "new-bundle",
        Command::RmSkill { .. } => "rm-skill",
        Command::RmBundle { .. } => "rm-bundle",
        _ => "this verb",
    }
}

/// Human summary of an import.
fn print_import(result: &bundle_import::ImportResult) {
    println!(
        "{} bundle {} from {}",
        result.status, result.bundle, result.source
    );
    println!("  root: {}", result.root.display());
    println!("  revision: {}", result.revision);
    println!("  digest: {}", result.digest);
    let skills = result
        .skills
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    println!("  skills: {skills}");
    println!("  files: {}", result.files.len());
}

/// Human summary of a publish run.
fn print_publish(result: &publish::PublishResult) {
    if let Some(drift) = &result.drift {
        if drift.is_clean() {
            println!("clean: no drift");
        } else {
            println!("drift ({} entries):", drift.entries.len());
            for entry in &drift.entries {
                println!("  {entry:?}");
            }
        }
    } else {
        println!(
            "wrote {} files, pruned {}",
            result.written.len(),
            result.pruned.len()
        );
    }
}

/// Human summary of validation.
fn print_validate(result: &validate::ValidateResult) {
    if result.issues.is_empty() {
        println!("ok: no issues");
        return;
    }
    for issue in &result.issues {
        println!(
            "{:?} [{}] {}: {}",
            issue.level, issue.code, issue.subject, issue.message
        );
    }
    println!(
        "{}",
        if result.ok {
            "ok (warnings only)"
        } else {
            "failed"
        }
    );
}

/// Human summary of token costs.
fn print_tokens(result: &tokens::TokensResult) {
    println!(
        "skills (always-on / body), ~{} chars/token:",
        result.chars_per_token
    );
    for s in &result.skills {
        println!("  {:<32} {:>6} / {:>7}", s.name, s.always_on, s.body);
    }
    println!("bundles:");
    for b in &result.bundles {
        println!("  {:<32} {:>6} / {:>7}", b.name, b.always_on, b.body);
    }
    println!(
        "total: always_on={}, body={}",
        result.totals.always_on, result.totals.body
    );
}

/// Human summary of the catalog.
fn print_catalog(result: &catalog::CatalogResult) {
    for b in &result.bundles {
        println!("{} v{} [{}]", b.name, b.version, b.agents.join(", "));
        for skill in &b.skills {
            println!("  {skill}");
        }
    }
    for s in &result.skills {
        if !s.depends_on.is_empty() {
            println!("{} depends on: {}", s.name, s.depends_on.join(", "));
        }
    }
}

/// Human summary of the hygiene scan.
fn print_hygiene(result: &hygiene::HygieneResult) {
    if result.findings.is_empty() {
        println!("clean: no findings");
        return;
    }
    for f in &result.findings {
        println!("{}:{}: [{}] {}", f.file, f.line, f.code, f.message);
    }
    println!("{} findings", result.findings.len());
}

/// Human summary of the doctor gate.
fn print_doctor(result: &doctor::DoctorResult) {
    println!(
        "{} skills, {} bundles",
        result.counts.skills, result.counts.bundles
    );
    println!(
        "drift: {}",
        if result.drift.is_clean() {
            "clean".to_string()
        } else {
            format!("{} entries", result.drift.entries.len())
        }
    );
    println!(
        "validate: {}",
        if result.validate.ok { "ok" } else { "failed" }
    );
    println!(
        "hygiene: {}",
        if result.hygiene.ok {
            "clean".to_string()
        } else {
            format!("{} findings", result.hygiene.findings.len())
        }
    );
    println!(
        "tokens: always_on={}, body={}",
        result.tokens.always_on, result.tokens.body
    );
    println!("{}", if result.ok { "OK" } else { "FAILED" });
}

/// Human overview of the repo.
fn print_list(result: &list::ListResult) {
    println!("bundles:");
    for b in &result.bundles {
        println!(
            "  {} v{} [{}] ({} skills)",
            b.name,
            b.version,
            b.agents.join(", "),
            b.skills.len()
        );
        if let Some(provenance) = &b.provenance {
            println!("    source: {}", provenance.source);
            println!("    revision: {}", provenance.revision);
            println!("    digest: {}", provenance.digest);
        }
    }
    println!("skills:");
    for s in &result.skills {
        let owners = if s.bundles.is_empty() {
            "unbundled".to_string()
        } else {
            s.bundles.join(", ")
        };
        println!("  {} ({})", s.name, owners);
    }
}

/// Human detail for one bundle or skill.
fn print_show(result: &list::ShowResult) {
    match result {
        list::ShowResult::Bundle {
            name,
            version,
            description,
            author,
            agents,
            provenance,
            skills,
            ..
        } => {
            println!("bundle {name} v{version}");
            if !description.is_empty() {
                println!("  {description}");
            }
            println!("  author: {author}");
            println!("  agents: {}", agents.join(", "));
            println!("  skills:");
            for m in skills {
                let mark = if m.present { "" } else { " (missing)" };
                println!("    {}{}", m.name, mark);
            }
            if let Some(provenance) = provenance {
                print_show_provenance(provenance);
            }
        }
        list::ShowResult::Skill {
            name,
            bundles,
            description,
            files,
            provenance,
            ..
        } => {
            println!("skill {name}");
            if !description.is_empty() {
                println!("  {description}");
            }
            let owners = if bundles.is_empty() {
                "unbundled".to_string()
            } else {
                bundles.join(", ")
            };
            println!("  bundles: {owners}");
            println!("  files:");
            for f in files {
                println!("    {f}");
            }
            for provenance in provenance {
                print_show_provenance(provenance);
            }
        }
    }
}

fn print_show_provenance(provenance: &list::ProvenanceSummary) {
    println!("  source: {}", provenance.source);
    println!("  selector: {}", provenance.selector);
    println!("  revision: {}", provenance.revision);
    println!("  digest: {}", provenance.digest);
}

/// Human summary of a release plan.
fn print_plan(plan: &ReleasePlan) {
    println!("scheme: {:?}", plan.scheme);
    if plan.bundles.is_empty() {
        println!("no version changes");
        return;
    }
    for b in &plan.bundles {
        println!("  {}: {} -> {}", b.name, b.from, b.to);
    }
}

/// Human summary of an install run.
fn print_install(result: &InstallResult) {
    if result.installed.is_empty() {
        println!("nothing installed");
        return;
    }
    println!("installed {} skill(s):", result.installed.len());
    for s in &result.installed {
        println!("  [{}] {}/{}", s.agent, s.bundle, s.skill);
    }
}

/// Human summary of an applied release.
fn print_apply(result: &release::ReleaseResult) {
    if result.written.is_empty() {
        println!("no version changes written");
        return;
    }
    println!("wrote {} bundle(s):", result.written.len());
    for f in &result.written {
        println!("  {f}");
    }
    if result.committed {
        println!("committed");
    }
    if !result.tags.is_empty() {
        println!("\nto tag and push:");
        for tag in &result.tags {
            println!("  git tag -a {tag} -m {tag}");
        }
        println!("  git push --follow-tags origin main");
        println!("\n(use annotated tags: git push --follow-tags skips lightweight tags)");
    }
}
