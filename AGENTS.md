# AGENTS.md

## Product direction

`lskills` is a skills-focused package manager. The target lifecycle lets users
consume, author, modify, compose, and republish Agent Skills declaratively and
reproducibly.

Use these product primitives in new work:

- `lskills.toml` - human-authored package and dependency intent;
- `lskills.lock.toml` - generated exact resolution and deployment state;
- package, skill, requirement, resolution, selection, composition, fork, target,
  bundle, project scope, and global scope;
- normal install replays unchanged locked resolutions; update changes them
  explicitly; frozen install performs exact replay;
- a fork is exact external skill content promoted into editable local source with
  a recorded upstream base.

[ADR-0010](docs/decisions/ADR-0010-manifest-lockfile-skills-lifecycle.md) is the
active product and target-architecture decision. Read
[the implementation plan](docs/implementation-plan.md) before changing lifecycle
code. Read [the glossary](GLOSSARY.md) when naming domain types or user-facing
concepts.

## Current implementation

The repository currently has two Rust crates:

- `crates/lskills-core/` - prototype domain and filesystem engine;
- `crates/lskills-cli/` - CLI parsing, dispatch, output, and process tests.

Keep this split until a demonstrated dependency problem requires another crate.
The existing workarea/import implementation is migration input, not the target
model. Reuse its frontmatter parsing, validated names, Git argument handling, path
safety, deterministic walking/hashing, staging/recovery, artifact maps, drift
checks, target paths, and test harness where they satisfy the new contracts.
Replace old `Bundle`, `Repo`, `import --root`, and provenance-sidecar assumptions
rather than exposing them through new interfaces.

## Change loop

1. Read the active requirement, domain section, focused spec, and affected callers.
2. Identify the narrow phase and completion evidence in
   `docs/implementation-plan.md`.
3. Add or update tests at the owning boundary.
4. Make the smallest coherent change through the two-crate architecture.
5. Exercise the actual binary in isolated temporary project, global, cache, and
   target roots.
6. Inspect generated manifests, lockfiles, bundles, ownership records, or target
   files rather than inferring success from exit status.
7. Update active docs when behavior or a persisted contract changes.

## Lifecycle invariants

- The manifest contains intent; generated commits, hashes, inventories, and
  deployment receipts belong in the lockfile.
- Project and global scopes use the same engine and independent graphs. Global
  manifests/configuration obey `XDG_CONFIG_HOME`; global materializations, caches,
  and transaction state use their XDG data, cache, and state roots.
- Project dependencies materialize under `lskills_modules/`; global dependencies
  materialize under the XDG data root. Neither is editable source.
- Resolution, composition, deployment, packing, and publication are separate
  stages with typed inputs and outputs.
- One authorized file inventory drives scanning, hashing, copying, deployment,
  packing, and reporting.
- Mutations acquire the scope lifecycle lock before their first state read, build
  a complete plan, stage and verify all output, activate atomically, then record
  lock and ownership state.
- Cleanup removes only paths within an owned target root whose current bytes match
  the recorded managed hash.
- Content is data. Never execute package scripts, hooks, binaries, or helper files.
- Git uses argument APIs, never a shell; user-controlled values must not become
  options.
- Machine output is versioned and errors have stable codes.
- TOML serialization for manifests, lockfiles, and bundle metadata is
  deterministic.

## Repository safety

This is a machinery repository. Use synthetic packages and temporary roots in
tests. Keep production skills, credentials, caches, user configuration, and
persistent workareas outside it.

Ask before destructive or irreversible operations. Do not commit or push without
explicit approval. Do not remove prototype commands or persisted formats until a
migration path and removal are explicitly approved.

## Tests

Cargo is available directly through mise shims. The repository gate is:

```sh
mise run check
```

Run `git diff --check` and validate relative Markdown links for documentation
changes. Process tests belong in `crates/lskills-cli/tests/`; unit tests stay with
the owning core module. Git and registry tests are hermetic and use temporary
servers or repositories. Tests must override home, cache, project, global, and
agent target directories.

Read [the testing strategy](docs/testing.md) when changing manifest/lock codecs,
resolution, transactions, target cleanup, forks, bundles, or publication.

## Documentation map

- Product boundary: `docs/product.md`
- Normative requirements: `docs/product-requirements.md`
- Vocabulary and invariants: `GLOSSARY.md`, `docs/domain-model.md`
- Component and transaction design: `docs/architecture.md`
- Target CLI: `docs/cli-reference.md`
- Focused behavior: `docs/specs/`
- Security controls: `docs/security.md`
- Implementation sequence and reuse map: `docs/implementation-plan.md`
- Accepted decision: `docs/decisions/ADR-0010-manifest-lockfile-skills-lifecycle.md`
- APM evidence and adaptation analysis: `docs/research/microsoft-apm-leverage.md`

`SESSION-LEDGER.md` is a gitignored handoff scratchpad. Update Context, Done,
Verification, Next, and Blockers at phase boundaries.
