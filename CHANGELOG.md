# Changelog

All notable changes to `lskills` are documented here.

## [Unreleased]

### Design

- Reduced the first implementation to one local skills workarea assembled from
  multiple local or Git origins.
- Chosen the existing `Skill`/`Bundle` model and two-crate implementation as the
  MVP foundation.
- Added one-bundle import with strict provenance and collision refusal.
- Deferred multiple workareas, update coordination, generic assets, package/catalog
  abstractions, remote publication, and the four-crate rewrite.

### Implementation

- Implemented local and pinned Git-origin bundle import with staged copying,
  provenance summaries, collision refusal, and read-only preview mode.
- Existing skills-root behavior remains covered by the prototype test suite.

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
