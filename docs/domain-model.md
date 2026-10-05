# lskills Domain Model

The domain is a skills package lifecycle. Filesystem caches and assembly directories
are implementation details and do not define the product vocabulary.

## Package

A `Package` is the unit of authoring, dependency resolution, versioning, and
publication. Its root contains `lskills.toml` and either one root `SKILL.md` or a
`skills/` directory exporting several skills.

```rust
struct Package {
    identity: PackageName,
    version: Version,
    exports: BTreeMap<SkillName, Skill>,
    dependencies: Vec<Requirement>,
    forks: BTreeMap<SkillName, ForkDeclaration>,
}
```

A project is a package used as the current composition root. `private = true`
prevents publication without changing dependency or installation semantics.

## Skill

A `Skill` is a cohesive directory with a required `SKILL.md` and optional resources.
For `skills/<name>/`, `<name>` is its runtime identity. For a root `SKILL.md`, the
runtime identity is the final segment of the declared package name. If frontmatter
contains `name`, it must match that identity. A globally unambiguous source identity
is:

```text
<package-name>:<skill-name>
```

For example:

```text
acme/review:code-review
```

The installed name may be changed by an explicit alias, but the source identity is
stable. An alias creates a deterministic deployment projection: it changes the
output directory and the `name` field in the projected `SKILL.md`, while retaining
source and projected hashes in the lockfile. It never changes package source.

## Requirement

A `Requirement` is human-authored dependency intent. It identifies a package source
and optional selector but not an exact resolution.

Concise forms follow APM's proven grammar:

```text
owner/repo
owner/repo#v1.2.0
host/owner/repo#main
owner/repo/path/to/package#v1.2.0
https://host/owner/repo.git#v1.2.0
./packages/local-skills
../sibling-package
```

An inline table expresses information that cannot fit unambiguously in a string:

```toml
{ git = "acme/team-skills", ref = "v2.0.0", skills = ["review"] }
```

## Resolution

A `Resolution` maps every direct and transitive requirement to:

- canonical package identity;
- exact source coordinates;
- exact registry version, archive digest, or Git commit;
- package-tree hash;
- exported and selected skills;
- dependency edges.

One package identity resolves to one version in a graph. Incompatible immutable
requirements fail with both dependency paths.

## Manifest

The `Manifest` is the human-authored `lskills.toml`. It describes desired package
and deployment state. It never contains generated commits, content hashes, or
file inventories.

```toml
schema = 1
dependencies = [
  "microsoft/apm-sample-package#v1.0.0",
  { git = "acme/team-skills", ref = "main", skills = ["review"] },
  "./packages/local-skills",
]
targets = ["claude", "codex"]

[package]
name = "acme/my-skills"
version = "1.0.0"
private = true
```

Local skills are discovered from root `SKILL.md` or `skills/<name>/SKILL.md`.
An optional export list narrows publication:

```toml
[exports]
skills = ["review", "release"]
```

## Lockfile

The `Lockfile` is generated `lskills.lock.toml`. It is an executable resolution
record, not a history log. It contains:

- a digest of semantic manifest input;
- exact package nodes and dependency edges;
- selected skill subsets;
- package, skill-tree, and archive hashes;
- exact fork bases;
- target deployment records and file hashes;
- tool and schema versions needed for replay.

A frozen operation reads the lockfile without selecting a different version or
revision.

## Fork

A `Fork` is local package source derived from one exact external skill. The manifest
records the durable relationship:

```toml
[forks.review]
from = "microsoft/team-skills:review"
```

The lockfile records the exact base package, revision, and skill-tree hash used when
the fork was created. The local bytes live at `skills/review/` and are owned by the
current package.

A fork is an explicit replacement in its declaring project. Both the upstream and
fork may exist in the graph, but they cannot both deploy under the same installed
name without an alias.

Forking is not patching a cache. The fork remains editable and publishable even
when the dependency cache is deleted.

## Selection

A `Selection` is the set of skills exported by a dependency that participate in
composition. Omitted selection means all exports. Selection is replacement-based
and declarative: the manifest is the complete desired set, while CLI convenience
commands rewrite it.

## Alias

An `Alias` explicitly gives a composed skill a different installed identity. The
composition stage creates a generated skill projection whose directory and
`SKILL.md` name agree with the alias. All other resource bytes and relative paths
are preserved. Both source and projected hashes are locked and audited.

Aliases are generated deployment and bundle state, not edits to local or dependency
source. A fork is preferable when the skill itself should be renamed and maintained
as authored source.

## Composition

A `Composition` is the collision-free set of local, forked, and dependency skills
chosen for deployment or packing. Composition preserves each skill directory as a
separate unit. lskills never concatenates unrelated `SKILL.md` files.

Resolution answers what packages exist. Composition answers which skills are used.
Deployment answers where those skills are placed.

## Target

A `Target` maps composed skills to a native agent skill directory for one scope.
A target profile defines:

- project and global destination roots;
- whether each scope is supported;
- installed-name rules;
- native validation requirements.

Targets do not change source identity, dependency resolution, or composed skill
bytes. Any explicit alias projection occurs during composition before a target
adapter receives the skill.

## Scope

`Scope` is either `Project` or `Global`.

Project scope discovers the nearest ancestor `lskills.toml`. Global scope uses
`${XDG_CONFIG_HOME:-$HOME/.config}/lskills/lskills.toml`. Each scope owns an
independent lockfile and dependency graph.

Global packages are not transitive undeclared dependencies of a project. An agent
may discover both global and project deployments according to its own precedence,
but lskills reports that effective view separately from project resolution.

## Materialization and deployment

`Materialization` is a verified internal copy of a resolved package. Project
materializations live under `<project>/lskills_modules/`; global materializations
live under `${XDG_DATA_HOME:-$HOME/.local/share}/lskills/modules/`. They may be
removed and recreated from the lockfile. Reusable downloaded source objects are a
separate shared cache under `${XDG_CACHE_HOME:-$HOME/.cache}/lskills/`.

`Deployment` is the native target projection. Each deployed path has one or more
owners, one active owner, and an optional content hash. The ownership record allows
safe removal, handoff, and drift reporting without treating a target directory as
source.

## Bundle

A `Bundle` is a packed, target-neutral artifact containing:

- package metadata;
- the exact composed skill directories;
- an embedded resolved manifest;
- provenance for external and forked skills;
- an exhaustive file-hash list.

A bundle is the artifact installed or published. It is not the editable package
source.

## State transitions

```text
Requirement --resolve--> Resolution --materialize--> Package content
      |                                            |
      +---------------- manifest ------------------+

Package content + local skills + forks
  --compose--> Composition
  --deploy--> native target files
  --pack--> Bundle
  --publish--> immutable release
```

Explicit freshness is separate:

```text
Lockfile --outdated--> report
Lockfile --update--> proposed resolution --accept--> new lockfile
```

## Invariants

- The manifest contains intent; the lockfile contains resolution.
- Normal install does not silently update an unchanged requirement.
- A skill's resource tree remains inside its skill directory.
- Managed dependency content and target output are not editable source.
- A fork has a new local identity and an exact recorded upstream base.
- Target selection cannot change dependency identity or composed bytes; an explicit
  alias changes only the generated runtime identity projection.
- Pack operates only on lock-attested content.
- Global and project dependency graphs remain independent.
- Internal caches and staging directories are disposable.
