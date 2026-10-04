# ADR-0008 - M0 implementation defaults

- Status: Superseded by ADR-0009
- Date: M0 design review
- Deciders: lskills maintainers

## Context

The product and architecture decisions establish external workareas, separate origins,
multiple named workareas, three-state updates, and local-first publication. The
implementation still needs concrete defaults so a new session can build the first
vertical slice without reopening foundational choices.

## Decision

The first implementation uses the following defaults.

### Workarea configuration

Named workareas live in the user's platform configuration directory. The schema is
versioned and has this shape:

```toml
schema = 1

[defaults]
workarea = "personal"

[workareas.personal]
assets_url = "git@github.com:me/personal-agent-assets.git"
local_path = "~/.local/share/lskills/workareas/personal"
ref = "main"
```

`assets_url`, `local_path`, and `ref` are separate values. A local-only workarea is
valid. Registration does not clone, commit, or push. Workarea Git synchronization is
explicit and non-destructive.

### Workarea layout

The initial logical layout is:

```text
<workarea>/
├── assets/
│   └── <managed-asset-id>/
│       ├── asset.toml
│       └── content/
└── .lskills/
    ├── workarea.toml
    ├── baselines/
    ├── ledger/operations.jsonl
    ├── receipts/
    ├── staging/
    ├── packages/
    └── catalogs/
```

The editable tree is under `content/`. Workarea metadata and asset metadata are not
passed to an agent target.

### Persistence formats

Strict, versioned TOML is used for workarea, asset, project, baseline, receipt, and
selection records. JSONL is used for the append-only operation ledger. Canonical JSON
is used for CLI machine output and target catalog formats where appropriate.

Arbitrary metadata maps are not the primary domain model.

### Baselines

Every accepted origin baseline is stored as a complete content-addressed tree under
the workarea's `.lskills/baselines/` directory and is committed with the workarea
when the user commits their changes. lskills does not rely on workarea Git history
or a machine-local cache for baseline recovery.

Source fetch caches and staging are rebuildable and may be ignored by Git. Baselines
are retained until an explicit future pruning operation is implemented.

### Initial source support

The first source adapters support local directories and local/remote Git repositories.
A moving branch is resolved to a concrete commit before import. Archives, packages,
catalogs, and GitHub API integration are deferred.

### Initial format and target

The first end-to-end format is an Agent Skills-compatible directory with a root
`SKILL.md`. The first target is a Claude Code project-scope directory:

```text
<project>/.claude/skills/<asset-name>/
```

Claude user scope, Codex, pi, Claude plugins, and other target layouts are later
adapters.

### Update comparison

The first update engine uses deterministic file-level comparison. Text diagnostics
may show line details, but automatic hunk merging is deferred. Initial decisions
are accept upstream, keep local, file-level selection, hold, ignore, and detach.
Binary files and unsupported structured metadata are whole-file conflicts.

### Project manifest

A project uses `.lskills.toml`:

```toml
schema = 1

[asset_workspace]
name = "personal"

[[assets]]
name = "review-docs"
target = "claude-project"
scope = "project"
```

The first release permits one workarea per project manifest. Cross-workarea project
references are deferred.

### Operation ledger

The workarea stores an append-only `.lskills/ledger/operations.jsonl` record for
user-invoked lifecycle operations, including checks, imports, installations, updates,
package builds, catalog generation, and publication. Records contain IDs, digests,
decisions, timestamps, and outcomes, not full asset content.

lskills writes files atomically but does not automatically commit or push.

### Packaging

The exact first package and native catalog schemas are selected at the start of the
packaging phase, after the first target adapter is working. The invariant is already
fixed: packages are deterministic projections with source and artifact digests, and
catalogs reference concrete package digests. There is no universal package schema in
M1 through M5.

### Existing prototype

The `skills/` plus `bundles/` prototype is not the first adapter. It remains available
as reference material until the new vertical slice is proven. No compatibility
behavior is required.

## Consequences

Positive:

- a new implementation session has concrete storage and scope defaults;
- baselines are portable across workarea clones;
- the first source and target slice is small enough to test end to end;
- advanced merge and publication design cannot block domain and local management;
- user data remains outside the machinery repository.

Costs:

- baseline trees increase workarea repository size, although Git deduplicates blobs;
- the first target is intentionally narrow;
- project aliases are local rather than fully portable;
- package schema work remains a deliberate phase decision.

## Validation

The implementation plan is the executable follow-up to this ADR. Any deviation
from these defaults requires an ADR update before it becomes an implementation
convention.
