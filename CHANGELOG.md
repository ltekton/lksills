# Changelog

All notable changes to `lskills` are documented here.

## [Unreleased]

### Design

- Reframed lskills as a skills-focused package manager for consuming, authoring,
  modifying, composing, and republishing Agent Skills.
- Chose `lskills.toml` for authored intent and `lskills.lock.toml` for exact
  resolution, integrity, fork-base, and deployment state.
- Defined normal locked install, explicit update, frozen replay, independent
  project/global scopes, deterministic bundles, immutable publication, and a
  first-class fork-to-local-source workflow.
- Added ADR-0010, which supersedes the single-workarea/import MVP as the active
  product and target-architecture decision.
- Added pinned Microsoft APM research and a phased migration/reuse assessment.

### Current prototype implementation

- Supports local and pinned Git-origin bundle import into an explicit workarea,
  with staged copying, provenance summaries, collision refusal, and read-only
  preview mode.
- Retains validation, rendering, native target installation, and related process
  coverage from the skills-root prototype.
- This behavior remains available during migration but is not the target product
  model.

## Prototype history

### [0.1.0] - 2026-09-23

Fresh idiomatic Rust reimplementation of the original Go tool. No byte-parity with
Go output was required.

Features included:

- `publish [--check]` for generated plugin artifacts;
- `validate`, `hygiene`, `tokens`, `catalog`, and `doctor` checks;
- `list` and `show` for bundles and skills;
- per-bundle SemVer/CalVer release planning and optional local commits;
- project and global installation for the prototype targets;
- skills-root scaffolding and removal commands;
- read-only Git-backed skills-root fetching through `--repo`;
- deterministic maps and stable JSON errors;
- validated names, safe joins, symlink rejection, atomic installation, and safe Git
  argument handling.

[0.1.0]: https://github.com/ltekton/lskills/releases/tag/v0.1.0
