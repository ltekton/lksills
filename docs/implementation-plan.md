# lskills Implementation Plan

> Status: MVP complete after hardening pass

This plan implements one simple workflow: import bundles from multiple local or Git
skills-roots into one workarea, then use the existing validation and publishing
commands.

## 1. Scope lock

The MVP is deliberately limited to:

- one workarea selected with an explicit `--root` for import;
- existing `Skill` and `Bundle` types;
- one existing bundle per import;
- local directories and Git skills-roots;
- complete bundle/member-skill copying;
- one provenance sidecar;
- collision refusal;
- existing validation and native publishing.

The following are deferred: multiple workareas, workarea Git management, baselines,
updates, three-way comparison, generic asset types, package/catalog abstractions,
project manifests, additional target scopes, and the four-crate rewrite.

## 2. Current state

The current two-crate prototype already provides:

- validated names and frontmatter;
- skills-root loading;
- bundle validation;
- local/Git source cache resolution;
- deterministic rendering;
- atomic per-skill installation;
- process fixtures and regression tests.

The implementation should extend this code rather than replace its model.

## 3. Phase 0 - Scope reset

**Status:** complete after this documentation pass.

### Required decisions

- One workarea, no workarea registry.
- One-bundle import, no generic asset detection.
- Local and Git skills-root origins only.
- Provenance at import time, no refresh/update claim.
- Existing Skill/Bundle model and two-crate workspace.
- Import collisions fail without merge, replacement, or rename.
- `import` requires an explicit `--root`.
- `publish` remains the native output path; generic packages and catalogs are out.

### Artifacts

- `docs/decisions/ADR-0009-single-workarea-multi-origin-mvp.md`;
- aligned product, requirements, architecture, domain, CLI, testing, and security docs;
- superseded old M0 architecture decisions marked as historical.

## 4. Phase 1 - Import and provenance

**Status:** complete

### Goal

Import one existing bundle and all referenced skills from a local or Git origin into
one explicit workarea without partial writes or source execution.

### Implementation

- Add a provenance record type and strict versioned TOML serialization.
- Add `import <origin> --root <root> --bundle <name> [--check]`.
- Resolve local origins through the existing `Repo::load` path.
- Resolve Git origins through the existing `RepoSpec` and read-only cache.
- Validate the source bundle and all referenced skills before writing.
- Check destination bundle and skill collisions before staging.
- Stage the complete selected bundle and skill directories under the destination.
- Preserve nested files, hidden files, bytes, and supported modes.
- Validate destination paths and reject unsafe symlinks and special files.
- Write or replace the provenance sidecar only after the complete import succeeds.
- Require explicit `--root` for every import mutation.
- Keep `--check` fully read-only.

### Tests

- local origin with one bundle;
- Git origin pinned to a revision;
- two different origins imported into one workarea;
- same bundle collision;
- member skill collision;
- malformed source and missing member;
- nested files, hidden files, binary data, and executable-looking files;
- traversal, symlink, and special-file rejection;
- failed staging leaves no partial bundle or provenance success;
- `--check` leaves source and workarea unchanged;
- no imported content executes;
- explicit root prevents writes to the machinery repository.

### Definition of done

- A local or Git source bundle can be imported into an empty temporary workarea.
- A second origin can add another bundle to the same workarea.
- Reopening the workarea recovers the imported content and provenance.
- Collisions fail before partial writes.
- Existing `validate`, `list`, and `show` can read the resulting workarea.
- `mise run check` passes.

## 5. Phase 2 - Existing workflow integration

**Status:** complete

### Goal

Make provenance visible without changing the existing skills-root publishing model.

### Implementation

- Add provenance summaries to `list` and `show` where useful.
- Validate provenance records for malformed or dangling bundle entries.
- Keep `validate` as the workarea integrity gate.
- Keep `publish [--check]` as the native output operation.
- Document that `install`, scaffolding, release, tokens, hygiene, doctor, and
  catalog remain prototype behavior outside the new import contract.
- Validate Git cache identity and pinned revisions before reuse.
- Reject source/destination overlap and missing source layout directories.
- Validate regular-file and directory kinds, non-UTF-8 filenames, and provenance
  semantics across import, inspection, validation, and publishing.
- Recover interrupted staged imports using an internal transaction marker.
- Preserve a clean machinery repository through explicit-root process coverage.

### Hardening evidence

The final implementation also covers cache identity, root overlap, special-file and
filename safety, provenance enforcement on publishing, interrupted transaction
recovery, and machinery-repository isolation. These are tested in the process suite
and core unit tests.

### Definition of done

- Two origins can be assembled, inspected, validated, and published from one root.
- Provenance is visible and does not alter generated native artifacts.
- Existing regression tests remain active.
- The machinery repository remains unchanged during tests.
- `mise run check` passes.

## 6. Evidence and gates

Run after each implementation phase:

```sh
mise run check
cargo test -p lskills-cli --test import
cargo test -p lskills-cli --test corpus
```

Before declaring the MVP complete, run the actual binary against temporary local
and Git origins and inspect:

- the selected bundle and member skills;
- the provenance sidecar;
- collision behavior;
- generated publish output;
- the machinery repository status.

## 7. Later, only if needed

Possible later work includes explicit refresh, baselines, multiple workareas,
standalone Agent Skills sources, additional formats, user-scope targets, package
artifacts, and remote publication. None is a prerequisite for this MVP.
