# lskills Architecture

> Status: target architecture, independent of the current implementation

## Architectural thesis

lskills is a skills package manager with one deep lifecycle interface:

```text
manifest + optional prior lock + operation mode
  -> resolve
  -> authorize
  -> materialize
  -> compose
  -> deploy or pack
  -> lock and receipt
```

Callers operate on packages, skills, dependencies, scopes, and targets. Filesystem
staging, source caches, clone locations, and temporary assembly trees remain hidden
inside modules.

## System context

```text
                         registry/catalog
                               ^
                               | publish
                               |
Git / registry / path -> lskills project -> packed bundle
                               |
                               | install
                               v
                    agent-native skill directories
```

The project source consists of `lskills.toml`, optional local skills, and the
generated `lskills.lock.toml`. The lockfile plus project source is sufficient to
reproduce deployment without consulting mutable upstream resolution state.

## Public lifecycle

### Resolve and install

```text
lskills install [requirement]
```

With a requirement, install transactionally edits the manifest and then performs a
normal synchronization. Without one, it reconciles the manifest with the lockfile,
reusing every unchanged locked resolution.

### Refresh

```text
lskills outdated
lskills update [package]
```

`outdated` is read-only. `update` contacts authoritative upstream sources, computes
a plan, and changes exact resolutions only after confirmation or `--yes`.

### Reproduce

```text
lskills install --frozen
lskills audit --ci
```

Frozen install performs no version selection. Audit reconstructs expected target
state in scratch and compares it with source, lock, and deployed files.

### Modify

```text
lskills fork <package> --skill <name> [--as <name>]
```

Fork creates editable local source from exact locked bytes and records the upstream
base. It never modifies cached dependency content.

### Distribute

```text
lskills pack
lskills publish
```

Pack creates a deterministic target-neutral bundle. Publish uploads that immutable
bundle and metadata to a configured registry. A catalog may index releases but is
not the content trust anchor.

## Modules and interfaces

Physical crate layout is an implementation decision. These are logical modules and
seams, not a prescribed number of crates.

### Package module

**Interface**

```text
load_package(root) -> Package
validate_package(package) -> ValidationReport
inventory(package, selection) -> SkillInventory
```

**Hides** manifest parsing, package-shape detection, `SKILL.md` validation,
resource walking, canonical paths, and deterministic hashing.

The returned inventory is the only authorized source-file plan used by scanning,
materialization, deployment, and packing.

### Resolver module

**Interface**

```text
resolve(manifest, prior_lock, mode, sources) -> ResolutionPlan
```

`mode` is one of `sync`, `update(selection)`, or `frozen`.

**Hides** graph traversal, version/ref selection, conflict detection, prior-lock
reuse, canonical identity, subset selection, and source-specific metadata.

The resolver is pure with respect to project and target files. It may query source
adapters and returns a plan before mutation.

### Source module

**Interface**

```text
inspect(requirement) -> CandidateSet
fetch(locked_source) -> VerifiedPackage
```

Adapters initially cover Git, local paths, local bundles, and a registry. An
adapter returns bytes and immutable identity; it does not deploy skills.

Credentials enter at this seam through a credential provider and never appear in
manifest, lockfile, diagnostics, or cache keys.

### Composition module

**Interface**

```text
compose(package, resolution, target) -> CompositionPlan
```

**Hides** local export discovery, dependency skill selection, fork replacement,
alias projection, reachable dependency skills, collision detection, and target
filtering.

An explicit alias produces a generated skill directory whose installed directory
and `SKILL.md` name agree; all other bytes and paths remain unchanged. Source and
projected hashes are retained. The result is a complete ordered set of composed
skill trees with provenance. There is no “first declaration wins” behavior.

### Deployment module

**Interface**

```text
deploy(plan, prior_receipt, mode) -> DeploymentResult
```

**Hides** target-native path mapping, staging, atomic replacement, ownership
handoff, stale cleanup, file hashing, and rollback.

Target adapters are real only where agent-native project/global locations differ.
They cannot alter package resolution or the composed skill projection.

### Fork module

**Interface**

```text
fork(locked_skill, destination, local_name) -> ForkResult
compare(fork, base) -> ForkDiff
```

**Hides** exact base extraction, source copying, manifest mutation, provenance
recording, and replacement selection. Automatic upstream merge is not required by
the first implementation; the model retains enough base identity to add it safely.

### Bundle module

**Interface**

```text
pack(package, resolution, composition) -> Bundle
verify(bundle) -> VerifiedBundle
```

**Hides** canonical archive ordering, normalized metadata, exhaustive file hashes,
provenance projection, and archive safety.

Bundle verification is repeated at install time. A bundle is target-neutral.

### Registry module

**Interface**

```text
publish(bundle, package, version) -> PublishedRelease
versions(package) -> VersionIndex
fetch(package, version) -> Bundle
```

Registry versions are immutable. Catalog/search is a separate optional adapter and
must resolve to an immutable release and digest.

### Configuration module

**Interface**

```text
resolve_context(cwd, global_flag, cli_overrides) -> EffectiveContext
```

It applies the single precedence chain:

```text
CLI > scope manifest > ${XDG_CONFIG_HOME:-$HOME/.config}/lskills/config.toml
    > auto-detection > default
```

Project and global manifests are never merged into one resolution graph.

## Manifest contract

The common case stays short:

```toml
schema = 1
dependencies = [
  "microsoft/apm-sample-package#v1.0.0",
  "github/awesome-copilot/skills/review-and-refactor",
  "./packages/local-skills",
]
targets = ["claude", "codex", "pi"]

[package]
name = "acme/my-skills"
version = "1.0.0"
private = true
```

Structured references are an escape hatch:

```toml
dependencies = [
  { git = "acme/team-skills", ref = "v2.0.0", skills = ["review", "release"] },
  { registry = "corp", package = "acme/security", version = "^3.1" },
]
```

Forks are local source and explicit replacements:

```toml
[forks.review]
from = "acme/team-skills:review"
```

Optional target aliases resolve runtime-name collisions:

```toml
[aliases]
"acme/team-skills:review" = "team-review"
```

## Lockfile contract

The lockfile is generated TOML with canonical ordering and no timestamps. Its
minimum logical shape is:

```toml
lockfile-version = 1
manifest-digest = "sha256:..."
resolver = "lskills@1"

[[packages]]
id = "github.com/microsoft/apm-sample-package"
requested = "microsoft/apm-sample-package#v1.0.0"
version = "1.0.0"
source = { kind = "git", url = "https://github.com/microsoft/apm-sample-package", ref = "v1.0.0", commit = "..." }
content-digest = "sha256:..."
skills = ["review"]
dependencies = []

[[forks]]
local = "acme/my-skills:review"
upstream = "acme/team-skills:review"
base-revision = "..."
base-digest = "sha256:..."
local-digest = "sha256:..."

[[deployments]]
target = "claude"
scope = "project"
files = [
  { path = ".claude/skills/review/SKILL.md", owners = ["acme/my-skills:review"], digest = "sha256:..." },
]
```

The actual serializer may normalize large nested structures into additional TOML
tables. The semantic requirements are exact identity, complete graph, deterministic
ordering, forward versioning, and no user-authored fields.

## Mutation protocol

Every mutating lifecycle command follows one protocol:

1. acquire the scope-specific lifecycle lock before the first state read;
2. load and validate manifest, lockfile, local source, and prior deployment state;
3. build a complete plan;
4. acquire source, produce any declared alias projections, and scan the authorized
   source and output inventories;
5. stage all new package and target state on the destination filesystem;
6. verify hashes and native target contracts;
7. atomically activate target state where supported;
8. atomically write the lockfile and deployment receipt;
9. clean staging after commit;
10. recover or roll back an interrupted transaction on the next mutation.

Read-only commands never repair state implicitly.

## Project and global layout

Project:

```text
project/
  lskills.toml
  lskills.lock.toml
  skills/                 # local editable source
  lskills_modules/        # generated verified dependency materializations
  .lskills/               # ignored transaction state
  .agents/skills/         # target output, example
  .claude/skills/         # target output, example
```

Global configuration, using `${XDG_CONFIG_HOME:-$HOME/.config}`:

```text
$XDG_CONFIG_HOME/lskills/
  config.toml             # machine preferences, no dependency intent
  lskills.toml            # global dependency intent
  lskills.lock.toml       # global exact resolution
```

Global materializations and reusable downloads follow their XDG classes:

```text
${XDG_DATA_HOME:-$HOME/.local/share}/lskills/modules/   # global materializations
${XDG_CACHE_HOME:-$HOME/.cache}/lskills/                # shared acquisition cache
${XDG_STATE_HOME:-$HOME/.local/state}/lskills/          # global locks and staging
```

A non-empty XDG variable must be absolute. Configuration never falls back to the
legacy `~/.lskills` directory. Agent-specific global outputs remain in their native
homes and are not stored under the lskills configuration root.

## Determinism

Deterministic behavior requires:

- canonical requirement and package identities;
- exact immutable source pins in the lockfile;
- sorted package, edge, skill, and file records;
- content hashes over logical paths, bytes, and relevant modes;
- no current timestamps in lock or bundles;
- target adapter versions recorded in deployment state;
- explicit collision and alias behavior;
- a stable archive format and metadata normalization;
- scratch replay as the audit oracle.

## Explicit exclusions

The target architecture remains skills-focused. It does not introduce generic
agent primitives, lifecycle scripts, hooks, MCP/LSP execution, or a generic asset
registry. New source and target adapters may be added behind existing seams when a
real second implementation exists.
