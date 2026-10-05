# lskills Product

## Product statement

> lskills helps users consume, author, modify, compose, and republish Agent
> Skills in a declarative, deterministic, reproducible workflow.

lskills applies the package-manager model proven by npm, Cargo, and APM to a
narrow domain: directories that implement the Agent Skills `SKILL.md` convention.
It manages the install and integrity plane. Agent runtimes remain responsible for
loading and executing skills.

## Product promises

### Declarative by manifest

A project records desired skills and targets in `lskills.toml`. Dependencies use
concise references for the common case and structured TOML only when source,
selection, or aliasing needs more detail.

```toml
schema = 1
dependencies = [
  "microsoft/apm-sample-package#v1.0.0",
  "github/awesome-copilot/skills/review-and-refactor",
  "./packages/team-skills",
]
targets = ["claude", "codex", "pi"]

[package]
name = "acme/my-skills"
version = "1.0.0"
private = true
```

The same manifest is the unit of authoring and consumption. A package may export
local skills while depending on external packages.

### Reproducible by lockfile

`lskills.lock.toml` records exact Git commits or immutable registry versions,
content hashes, dependency edges, selected skills, fork bases, and deployment
ownership. A normal install replays locked state. `lskills install --frozen`
fails rather than resolving new state.

### Editable by promotion

Installed dependencies are immutable managed inputs, not editing locations.
`lskills fork` promotes one locked external skill into local source, records its
exact upstream base, and makes the local skill the explicit replacement. The user
can modify, test, pack, and publish it as part of a package under a new identity.

### Portable by target

A target maps selected standard skill directories to an agent's native project or
user location. Target adapters control placement and native validation; they do not
change dependency resolution or composed skill content. An explicit alias is
applied earlier as a deterministic composition projection.

### Safe by construction

Resolution and deployment reject path traversal, unsafe links, special files,
content-integrity failures, undeclared archive entries, and ambiguous output
ownership. Source content is never executed by lskills. Security scanning occurs
before deployment and again during audit.

## User lifecycle

```text
init package
  -> author local skills
  -> install local or remote dependencies
  -> lock exact sources and selected skills
  -> deploy to project or global agent targets
  -> fork selected external skills when modification is needed
  -> validate and audit
  -> pack a deterministic bundle
  -> publish an immutable package release
```

Typical project use:

```console
lskills init
lskills install microsoft/apm-sample-package#v1.0.0
lskills install ./packages/my-local-skills
lskills install
lskills audit
```

Typical global use:

```console
lskills install --global acme/everyday-skills#^2.0
lskills update --global
```

Typical producer use:

```console
lskills new skill incident-review
lskills validate
lskills pack
lskills publish
```

Typical modification use:

```console
lskills fork microsoft/team-skills --skill review --as review
# edit skills/review/
lskills install
lskills pack
```

## Project and global scope

Project scope is the default. Commands search upward from the current directory
for `lskills.toml`; its sibling `lskills.lock.toml` is the generated resolution.

Global scope is selected with `--global`. Its authored and locked configuration is
stored under the XDG configuration root:

```text
${XDG_CONFIG_HOME:-$HOME/.config}/lskills/lskills.toml
${XDG_CONFIG_HOME:-$HOME/.config}/lskills/lskills.lock.toml
${XDG_CONFIG_HOME:-$HOME/.config}/lskills/config.toml
```

A set, non-empty `XDG_CONFIG_HOME` must be an absolute path and is obeyed. Global
dependencies are not injected into project resolution. Agent runtimes may naturally
see both project and user skill directories, but a project remains reproducible
without undeclared global state.

## Dependency materialization

Resolved dependency skills are copied into disposable, verified materialization
roots analogous to APM's `apm_modules/`:

```text
<project>/lskills_modules/                                      # project scope
${XDG_DATA_HOME:-$HOME/.local/share}/lskills/modules/           # global scope
```

Both contain only generated dependency packages and can be reconstructed from the
scope lockfile. Local authored skills and forks remain under package source and are
never edited in these directories.

Downloaded archives, bare Git data, and other reusable acquisition objects are
kept separately in:

```text
${XDG_CACHE_HOME:-$HOME/.cache}/lskills/
```

The cache may be shared by project and global operations, but each scope has its
own materialization and lock graph.

## Package shapes

A one-skill package may place `SKILL.md` at its root:

```text
review/
  lskills.toml
  SKILL.md
  references/
```

A multi-skill package uses:

```text
team-skills/
  lskills.toml
  skills/
    review/SKILL.md
    release/SKILL.md
```

All files below a skill directory form one cohesive skill. lskills preserves those
bytes and relative paths; it does not flatten a skill or reinterpret its body.

## Package versus bundle

A package is editable source with a manifest. A bundle is the immutable result of
`lskills pack`. Packing composes local, forked, and selected dependency skills from
the lockfile into one target-neutral artifact with an exhaustive hash manifest.
Publishing stores that artifact under an immutable package version.

## Boundaries

lskills manages skills, not arbitrary agent assets. It does not define or execute:

- package lifecycle scripts;
- agent hooks;
- MCP or LSP servers;
- shell commands;
- model runtimes;
- runtime authorization or sandboxing.

Those exclusions preserve the useful package-manager workflow while keeping the
trust surface appropriate for a skills-focused tool.
