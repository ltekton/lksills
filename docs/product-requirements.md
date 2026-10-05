# lskills Product Requirements

> Status: target product contract

## Vision

### PROD-001 - Complete skill lifecycle (Must)

lskills must support consuming, authoring, modifying, composing, and republishing
Agent Skills through one coherent package lifecycle.

### PROD-002 - Familiar package-manager behavior (Must)

The primary workflow must follow established npm/Cargo/APM expectations:

- a human-authored manifest;
- a generated lockfile;
- install as synchronization;
- update as explicit freshness;
- frozen replay for CI;
- project and global scopes;
- pack and publish as separate operations.

Internal assembly directories must not appear as primary product concepts.

## Manifest and package

### MAN-001 - TOML manifest (Must)

A package is rooted by `lskills.toml`. The manifest declares package metadata,
dependencies, local skill exports, forks, target selection, and publication intent.

### MAN-002 - Concise dependency references (Must)

The common dependency form must be a single string, including Git shorthand,
optional refs, repository subpaths, and local paths. Structured inline tables are
reserved for skill subset selection, aliases, non-default sources, and other
information that cannot be expressed safely in the string form.

### MAN-003 - One manifest for author and consumer roles (Must)

A package may export local skills and consume dependency skills at the same time.
A separate project type or workarea manifest is not required.

### MAN-004 - Skill package shapes (Must)

Support:

- one root `SKILL.md` package;
- a multi-skill `skills/<name>/SKILL.md` package;
- local path packages;
- repository subpath packages.

A publishable package must have stable name and version metadata.

## Resolution and lockfile

### LOCK-001 - TOML lockfile (Must)

`lskills.lock.toml` is generated and records the complete exact dependency graph,
source coordinates, immutable revisions or versions, selected skills, content
hashes, fork bases, and deployment ownership.

### LOCK-002 - Install does not silently update (Must)

A normal `lskills install` replays unchanged lock entries. New or changed manifest
requirements are resolved; unchanged requirements retain their locked resolution.

### LOCK-003 - Explicit update (Must)

`lskills update` checks authoritative upstream state, presents a plan, and changes
only selected or requested lock entries. `lskills outdated` reports freshness
without writing.

### LOCK-004 - Frozen replay (Must)

`lskills install --frozen` fails when the manifest, local source, selected skill
set, or lockfile disagree. It never selects a new version or revision.

### LOCK-005 - Deterministic serialization (Must)

The lockfile contains no volatile timestamps, uses canonical ordering, and remains
byte-stable when semantic state has not changed.

## Skills and composition

### SKILL-001 - Cohesive skill directories (Must)

A skill consists of `SKILL.md` and all regular files beneath its directory.
Relative structure, bytes, and supported executable modes are preserved. The only
content projection permitted is an explicitly declared installed-name alias, which
rewrites the `SKILL.md` name deterministically and records source and projected
hashes.

### SKILL-002 - Explicit identity (Must)

For `skills/<name>/`, `<name>` is the runtime identity. For root `SKILL.md`, the
identity is the final segment of the declared package name. If frontmatter contains
`name`, it must match that identity.

### SKILL-003 - Selectable package exports (Must)

A dependency may export one or many skills. Consumers may select a deterministic
subset; omitted selection means every exported skill. The lockfile records the
resolved subset.

### SKILL-004 - Explicit composition (Must)

A project composes local skills, forked skills, and selected dependency skills.
Name collisions fail unless the manifest supplies an explicit alias or fork
replacement. Declaration order must not silently choose a winner.

## Modification and forks

### FORK-001 - Dependencies are immutable inputs (Must)

Managed dependency materializations and deployed target files are never the
editable source of a durable customization.

### FORK-002 - First-class fork workflow (Must)

`lskills fork` copies one exact locked external skill into local package source,
records its upstream package, skill, revision, and content hash, and explicitly
replaces that external skill in the current project.

### FORK-003 - Forks are normal publishable skills (Must)

After promotion, the fork is ordinary local source. It can be renamed, edited,
validated, composed, packed, and published under the current package identity.
Its upstream relation remains inspectable provenance, not its runtime identity.

### FORK-004 - Upstream comparison is explicit (Should)

The CLI should compare a fork with its locked base and current upstream. Updating
a fork's base must produce a plan and never overwrite local edits silently.

## Scope and configuration

### SCOPE-001 - Project scope by default (Must)

Commands discover the nearest ancestor `lskills.toml`. Project deployment paths
are resolved from that manifest directory.

### SCOPE-002 - Independent XDG global scope (Must)

`--global` operates on
`${XDG_CONFIG_HOME:-$HOME/.config}/lskills/lskills.toml` and its sibling
`lskills.lock.toml`. A set, non-empty `XDG_CONFIG_HOME` must be absolute and must be
obeyed. Global operations never mutate the current project manifest.

### SCOPE-003 - User configuration is separate (Must)

`${XDG_CONFIG_HOME:-$HOME/.config}/lskills/config.toml` stores machine preferences
such as default targets, transport, cache location, and registry aliases. It does
not contain the global dependency set. Secrets are obtained from environment
variables or credential helpers rather than tracked manifests or lockfiles.

### SCOPE-004 - Deterministic precedence (Must)

Effective settings use one documented precedence:

```text
CLI flag > scope manifest > user config > target auto-detection > built-in default
```

Global dependencies never implicitly enter a project lock graph.

## Installation and targets

### INST-001 - Scope-local materialization and native deployment (Must)

Project dependencies materialize under `<project>/lskills_modules/`. Global
dependencies materialize under
`${XDG_DATA_HOME:-$HOME/.local/share}/lskills/modules/`. lskills then deploys
selected skills to the project- or user-scope directory each supported agent reads.
Target adapters control placement only; they do not alter resolution or composed
skill bytes.

### INST-002 - Atomic managed writes (Must)

Deployment stages the complete desired target state before activation. A failed
operation leaves the prior managed state usable.

### INST-003 - Ownership-aware cleanup (Must)

The lockfile records deployed paths, owners, and content hashes. Removal deletes
only content still matching an lskills-owned record. User-modified or unowned
content is retained and reported.

### INST-004 - Preview (Must)

`--dry-run` reports resolution, selection, collision, and deployment changes
without changing manifest, lockfile, cache-visible state, or target files.

### INST-005 - XDG cache isolation (Must)

Reusable downloads, Git objects, and registry archives are stored under
`${XDG_CACHE_HOME:-$HOME/.cache}/lskills/`, independently of project and global
materializations. Deleting the cache cannot delete authored source or invalidate a
lockfile whose exact content can be fetched again.

## Packaging and publication

### PACK-001 - Deterministic bundle (Must)

`lskills pack` emits a target-neutral, self-contained bundle from locked local,
forked, and selected dependency skills. Repeated packing of identical inputs
produces identical bytes.

### PACK-002 - Exhaustive integrity manifest (Must)

A bundle contains an exhaustive sorted file list and SHA-256 hashes. Installation
rejects missing, extra, changed, traversing, symlinked, or unsupported special
entries.

### PACK-003 - Provenance preservation (Must)

The bundle records the source package, exact dependency resolutions, selected
skills, and fork bases needed to explain every included skill.

### PUB-001 - Immutable versions (Must)

Publishing creates an immutable package version. Reusing an existing package
name/version with different bytes fails.

### PUB-002 - Separate discovery from storage (Should)

A registry stores immutable bundle bytes and metadata. Optional catalogs provide
curated discovery and point to immutable versions; catalogs are not trusted as
content stores.

## Validation, audit, and safety

### SAFE-001 - Source content is data (Must)

lskills never executes skill scripts, hooks, binaries, or package lifecycle
commands during resolve, install, audit, pack, or publish.

### SAFE-002 - Filesystem safety (Must)

Reject traversal, absolute archive paths, unsafe symlinks, special files,
non-UTF-8 package-relative paths, and writes outside owned roots.

### SAFE-003 - Pre-deploy content scan (Must)

All files selected for deployment are scanned before target writes for hidden
Unicode and other configured content-policy findings. Scan and copy consume the
same authorized file plan.

### SAFE-004 - Audit and drift (Must)

`lskills audit` verifies manifest-lock consistency, source hashes, selected skills,
bundle integrity, deployment ownership, deployed-file hashes, and reproducibility
through scratch replay.

### CLI-001 - Stable machine output (Must)

Commands support versioned JSON output and stable error codes for usage,
resolution, integrity, collision, policy, safety, and I/O failures.
