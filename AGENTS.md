# AGENTS.md

## What this repository is

`lskills` is a local manager for one skills workarea assembled from multiple
read-only origins. The workarea is the operator-owned `skills/` plus `bundles/`
tree. Origins may be local directories or Git skills-roots.

The source repository is machinery only. Do not add production skills, user asset
collections, or persistent workareas here. Tests create synthetic sources and
temporary workareas.

## Current implementation

The current two-crate implementation is the implementation base:

- `crates/lskills-core/` - skills, bundles, source loading, import, validation, rendering,
  installation, and provenance;
- `crates/lskills-cli/` - argument parsing, dispatch, output, and process tests.

The old skills-root model is no longer merely reference material for this reduced
scope. It is the chosen MVP model. Do not create `lskills-domain`,
`lskills-application`, or `lskills-adapters` for the MVP.

## MVP contract

- There is exactly one workarea per invocation, selected with `--root` or
  `LSKILLS_ROOT`.
- `import` requires an explicit `--root` destination.
- An origin is an existing skills-root containing `skills/` and `bundles/`.
- An import selects one existing bundle and copies that bundle plus its referenced
  skills into the workarea.
- Local directories and Git origins are supported; archives and registries are not.
- Import records provenance in `<root>/.lskills/provenance.toml`.
- Provenance records source and resolved revision/digest only. They are not update
  baselines and do not imply refresh support.
- Existing destination names cause the complete import to fail. No merge,
  replacement, namespace, or automatic rename is performed.
- Imported content is copied and inspected as data. It is never executed.
- Existing `validate`, `list`, `show`, and `publish` behavior remains the main
  workarea workflow.

The new target commands are documented in `docs/cli-reference.md` and the
implementation sequence is in `docs/implementation-plan.md`.

## Deliberately deferred

Do not add these to the MVP without an explicit scope decision and design update:

- multiple named workareas or a workarea registry;
- workarea Git synchronization;
- baselines, upstream checks, three-way updates, merge, hold, ignore, or detach;
- generic agent asset kinds or format/target registries;
- package artifacts, native catalog models, or remote publication;
- project manifests and user-scope expansion;
- a four-crate rewrite;
- automatic commits, pushes, or execution of imported content.

## Safety and implementation rules

- Use validated `BundleName`, `SkillFullName`, and safe relative paths.
- `import` requires an explicit destination root.
- Source and destination roots must be disjoint; an import never writes into its
  origin.
- Imports use a durable internal transaction marker so interrupted placement can
  be recovered before another workarea operation proceeds.
- Stage an import and publish provenance only after the complete selected bundle is
  valid. A failed import must not leave a partial bundle in the workarea.
- Reject traversal, unsafe symlinks, and unsupported special files.
- Preserve bytes and supported executable modes.
- Treat source files, scripts, templates, and binaries as data.
- Invoke Git through argument APIs, never `sh -c`, and protect user-controlled
  arguments from option injection.
- Keep origin caches separate from the workarea.
- Keep machine output versioned and errors stable.
- Do not commit, push, or perform destructive commands without explicit approval.

## Toolchain and tests

`cargo` is provided through mise shims on `PATH`, so direct Cargo commands work
without shell-evaluation setup:

```sh
cargo ...
```

The repository gate is:

```sh
mise run check
```

It runs formatting, clippy with warnings denied, and the workspace tests. Process
tests belong in `crates/lskills-cli/tests/`; unit tests stay with their owning core
module. Git-dependent tests must use temporary repositories and must not write into
this machinery repository.

## Documentation source of truth

Keep these aligned when behavior changes:

- `docs/product.md` - product boundary;
- `docs/product-requirements.md` - MVP requirements;
- `docs/architecture.md` - current component boundaries;
- `docs/domain-model.md` - vocabulary and invariants;
- `docs/implementation-plan.md` - phases and evidence;
- `docs/specs/` - focused behavior;
- `docs/decisions/ADR-0009-single-workarea-multi-origin-mvp.md` - current scope decision.

## Session ledger

`SESSION-LEDGER.md` is a gitignored handoff scratchpad. Keep it updated with
Context, Done, Verification, Next, and Blockers at phase boundaries.
