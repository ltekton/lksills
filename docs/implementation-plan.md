# lskills Implementation Plan

> Status: target implementation sequence

This plan starts from the accepted product model in
[ADR-0010](decisions/ADR-0010-manifest-lockfile-skills-lifecycle.md). The current
workarea/import prototype is assessed for reusable code only; its persisted model
and command surface do not set the target architecture.

## Delivery principles

- Ship vertical lifecycle slices, not an up-front crate rewrite.
- Keep the existing `lskills-core` and `lskills-cli` boundary until a concrete
  dependency rule requires another crate.
- Build every mutating command around plan, authorize, stage, verify, activate,
  record, and recover.
- Define one authorized file inventory per operation. Scanning, hashing, copying,
  packing, and reporting consume that inventory.
- Make lock serialization deterministic before adding remote resolution.
- Treat project and global scopes as the same engine with different roots.
- Preserve existing commands until replacement behavior and migration are tested.
  Compatibility code is temporary and must not leak old workarea concepts into new
  domain interfaces.

## Existing-code assessment

The prototype contains useful mechanisms, but most types are coupled to
`skills/` plus `bundles/*.toml`. Reuse behavior, not accidental vocabulary.

| Existing area | Disposition | Target use |
|---|---|---|
| `frontmatter.rs`, `skillfile.rs` | Adapt | Parse and validate standard `SKILL.md`; expand only where the target skill contract requires it. |
| `names.rs` | Adapt | Retain validated newtypes and stable diagnostics; replace bundle-prefix identity assumptions with package and skill identities. |
| `path.rs`, `generate/walk.rs`, tree checks in `repo.rs` and `import.rs` | Consolidate | Form one portable-path and authorized-inventory module covering traversal, symlinks, special files, UTF-8 paths, bytes, and modes. |
| `git.rs`, `remote.rs` | Adapt | Keep argument-based Git invocation, credential-helper compatibility, exact-commit checkout, and cache isolation. Replace `spec@ref` parsing with the documented requirement grammar and lock-aware cache policy. |
| `import.rs` transaction marker, staging, copy, and digest routines | Extract | Reuse recovery, durable staging, mode-preserving copy, and deterministic hashing in the lifecycle transaction and materializer. Retire bundle-import orchestration. |
| `provenance.rs` | Replace | Its source/revision/digest fields inform lock nodes, but `.lskills/provenance.toml` is not replay state and is not extended into the new lockfile. |
| `repo.rs`, `model/bundle.rs`, `model/skill.rs` | Replace around retained parsers | Introduce `Package`, `Manifest`, `Requirement`, `Resolution`, `Composition`, and packed `Bundle`; do not preserve the old meaning of bundle. |
| `validate.rs` | Adapt | Keep structured issues and relative-link checks; validate package shapes, manifest/lock agreement, exports, selections, forks, aliases, and targets. |
| `render.rs`, `drift.rs`, `publish.rs` | Adapt and separate | Keep deterministic artifact maps and drift comparison. Split target deployment from deterministic bundle packing and registry publication. |
| `install/target.rs`, `install/mod.rs` | Adapt | Retain native path knowledge and staged per-skill writes; add global/project symmetry, complete-operation activation, ownership receipts, and safe cleanup. |
| `generate/*` | Selectively reuse | Preserve target-specific generation only where native agents require it. Standard skill directories should otherwise retain bytes and layout. |
| `config.rs`, `root.rs`, `dirs.rs` | Replace/adapt | Implement upward `lskills.toml` discovery, XDG configuration/data/cache/state roots for global scope, separate user config, and documented precedence. Keep platform directory discovery as a low-level helper. |
| `release.rs` | Adapt later | SemVer and plan/apply structure can support package versioning; automatic commits remain outside the lifecycle. |
| CLI parsing, JSON output, process-test harness | Adapt | Keep the core/CLI error boundary and process tests; version every machine envelope and add structured context. |

The old `BundleName` prefix rule, `Repo` aggregate, `import --root` flow,
`.lskills/provenance.toml`, and generated-root publication contract are migration
inputs, not target abstractions.

## Phase 0 - Design baseline

**Status:** complete when this documentation reframe is accepted.

Deliverables:

- product statement and requirements;
- domain vocabulary and target architecture;
- TOML manifest and lockfile direction;
- project/global command surface;
- modification, bundle, publication, and security contracts;
- ADR-0010 and the APM research record.

Gate:

- documentation has no active pointer that presents ADR-0009 as the target;
- all relative Markdown links resolve;
- unresolved format questions are identified rather than silently encoded.

## Phase 1 - Local package and deterministic lock kernel

### Goal

Make one local package discoverable, validatable, lockable, and reproducible
without network access or target deployment.

### Implement

1. Define strict versioned codecs for `lskills.toml` and `lskills.lock.toml`.
2. Add project discovery and global scope roots using one `ScopeContext`: XDG
   configuration for manifests, XDG data for materializations, XDG cache for
   reusable downloads, and XDG state for lifecycle locks and staging.
3. Materialize project dependencies under `<project>/lskills_modules/` and global
   dependencies under the XDG data root.
4. Support root-skill and `skills/<name>/` package shapes.
5. Build the authorized skill-file inventory and canonical SHA-256 contract.
6. Resolve local path package dependencies, including transitive manifests and
   cycle/conflict diagnostics.
7. Implement deterministic lock generation, semantic-manifest digesting, atomic
   lock writes, `lock --check`, and frozen local replay.
8. Implement `init`, `new skill`, and target-independent `validate` against the
   new package model.
9. Keep old commands behind their existing paths until replacement coverage is
   present; do not make new models parse `.lskills/provenance.toml` implicitly.

### Evidence

- equivalent manifests serialize to one canonical lockfile;
- unchanged `install`/`lock` planning leaves lock bytes untouched;
- local dependency edits are detected in frozen mode;
- root and multi-skill fixtures resolve identically from project and global
  contexts;
- cycles, identity conflicts, selection errors, path escapes, links, special
  files, and non-UTF-8 paths fail with stable codes.

## Phase 2 - Git acquisition and graph resolution

### Goal

Resolve concise Git references and transitive package dependencies to exact,
replayable content.

### Implement

1. Parse and canonicalize shorthand, full URL, ref, and repository-subpath
   requirements without conflating `@` in transport syntax with version syntax.
2. Separate source adapters from graph resolution and package loading.
3. Resolve symbolic refs to exact commits; lock canonical source coordinates,
   requested refs, package subpaths, and package/skill hashes.
4. Materialize into content-addressed or exact-resolution cache entries outside
   project source.
5. Reuse unchanged locked nodes during normal install; change them only for a
   changed requirement or explicit update.
6. Add offline and frozen behavior, parent-linked conflict explanations, and
   `why` data.
7. Scan the authorized file plan before any deployment-capable state is accepted.

### Evidence

- hermetic temporary Git repositories cover branches, tags, SHAs, subpaths,
  transitive edges, cycles, and conflicting immutable requirements;
- frozen replay works after moving the project and with network access disabled
  when exact content is cached;
- cache identity cannot substitute content from another origin;
- credentials and credential-bearing URLs never enter diagnostics or lockfiles.

## Phase 3 - Composition and transactional deployment

### Goal

Synchronize a collision-free composition to native project and global targets
without losing prior valid state.

### Implement

1. Compose local and selected dependency skills with explicit aliases and
   deterministic collision failures.
2. Turn current target path logic into small adapters for Claude, Codex, and Pi.
3. Compute a complete deployment plan before writes.
4. Add a scope lifecycle lock, destination-filesystem staging, transaction
   recovery, and complete-operation activation.
5. Record canonical ownership rows and deployed-file hashes in the lockfile.
6. Reconcile stale output only when ownership and current hashes authorize
   deletion; retain and report edited or unowned content.
7. Implement `install`, `uninstall`, `list`, `view`, `why`, `audit`, and dry-run
   behavior for both scopes.

### Evidence

- project and global graphs remain independent;
- a fault at each mutation boundary leaves either prior or next complete managed
  state and is recoverable on the next mutation;
- concurrent mutations serialize before their first state read;
- dry-run writes no manifest, lock, cache-visible, staging, or target state;
- uninstall never deletes edited, shared, malformed, or unowned output;
- scratch replay produces the same target artifact map.

This is the first complete consumer milestone.

## Phase 4 - Update and freshness

### Goal

Make upstream change explicit and auditable without changing normal install
semantics.

### Implement

- `outdated` as a read-only authoritative-source query;
- selected and whole-graph `update` planning;
- confirmation and `--yes` behavior;
- added, changed, removed, and unchanged node/deployment reporting;
- atomic lock and deployment transition using the Phase 3 protocol.

### Evidence

- normal install remains stable after an upstream branch moves;
- update changes only requested nodes plus necessary transitive consequences;
- an update failure preserves the prior lock and deployment;
- offline and frozen modes perform no freshness queries.

## Phase 5 - Fork and local modification

### Goal

Promote an exact external skill into durable editable source with explainable
upstream provenance.

### Implement

1. Resolve the requested source skill from the current lock.
2. Plan copy, local identity, export, and replacement changes together.
3. Copy the exact locked tree into `skills/<local-name>/` transactionally.
4. Write the manifest fork declaration and lock base record atomically.
5. Make the local fork an explicit composition replacement.
6. Implement diff against the locked base and current upstream. Defer automatic
   rebase until conflict semantics are separately accepted.

### Evidence

- deleting all caches does not affect editable fork source;
- the base package, commit/version, source skill, and tree hash remain inspectable;
- fork creation never mutates dependency materialization or target output;
- destination collisions fail without partial manifest or source changes;
- upstream updates never overwrite local edits.

## Phase 6 - Deterministic bundles

### Goal

Pack and verify a self-contained, target-neutral artifact from one locked
composition.

### Implement

- specify the bundle directory and normalized archive formats;
- embed package metadata, exact resolved graph, skill provenance, fork bases, and
  exhaustive sorted file hashes;
- normalize archive order, timestamps, ownership metadata, path separators, and
  supported modes;
- reject missing, extra, changed, linked, special, absolute, or traversing entries;
- implement `pack`, `verify`, and archive install through the same inventory and
  resolution interfaces.

### Evidence

- repeated pack operations over identical input are byte-identical;
- verification fails for every inventory and path mutation class;
- a bundle installs without contacting original Git sources;
- unpacked content reproduces the same skill hashes and provenance.

## Phase 7 - Immutable publication

### Goal

Publish verified bundles under immutable package versions.

Before implementation, accept a registry protocol decision covering namespace,
authentication, upload finalization, immutability, metadata, and yanking. Then:

- implement one registry adapter, not a speculative registry framework;
- validate and pack before upload;
- refuse private packages and version-byte replacement;
- keep credentials in environment or credential-helper boundaries;
- treat catalogs as optional discovery metadata, not package storage.

Evidence includes a hermetic registry fixture, interrupted-upload behavior,
immutable-version conflicts, credential redaction, and install of the published
artifact by digest.

## Migration from the prototype

Do not silently reinterpret old state. Before removing prototype commands:

1. inventory the old `skills/`, `bundles/*.toml`, `.lskills.toml`, and
   `.lskills/provenance.toml` shapes encountered in fixtures;
2. specify a one-way, previewable migration into package manifest, local exports,
   dependencies or forks, and lock records;
3. state which provenance can be preserved exactly and which cannot establish a
   replayable lock;
4. run old and new corpus fixtures through the migration;
5. remove old command paths only after explicit approval and release notes.

ADR-0009 remains a historical description of the implemented prototype. It does
not constrain new interfaces.

## Continuous gates

Run the narrowest relevant tests during each phase and the repository gate before
claiming completion:

```sh
mise run check
git diff --check
```

Also run Markdown link validation whenever documentation changes. A phase is done
only when the actual binary has been exercised against isolated temporary
projects and its persisted artifacts have been inspected.
