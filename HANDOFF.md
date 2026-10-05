# lskills handoff

## Current state

The target product has been reframed as a skills-focused package manager:

> lskills helps users consume, author, modify, compose, and republish Agent Skills
> in a declarative, deterministic, reproducible workflow.

ADR-0010 is the active decision. `lskills.toml` is authored intent;
`lskills.lock.toml` is generated exact resolution and deployment state. Project
and global scopes are independent. Acquired skills remain immutable until `fork`
promotes an exact locked skill into editable local source with an upstream base.
Packing produces a deterministic target-neutral bundle; publishing stores immutable
versions.

The current two-crate Rust implementation still implements the older
single-workarea/import prototype. It has not yet been migrated. Keep both crates,
reuse proven mechanisms, and avoid exposing old `Repo`, bundle-membership,
`import --root`, or provenance-sidecar assumptions through new lifecycle APIs.

## Completed in the design reframe

Active or newly aligned documents include:

- `README.md`, `AGENTS.md`, `GLOSSARY.md`, and `CHANGELOG.md`;
- `docs/product.md` and `docs/product-requirements.md`;
- `docs/domain-model.md` and `docs/architecture.md`;
- `docs/cli-reference.md`, `docs/security.md`, and `docs/testing.md`;
- `docs/specs/`;
- `docs/implementation-plan.md`;
- `docs/decisions/ADR-0010-manifest-lockfile-skills-lifecycle.md`;
- `docs/research/microsoft-apm-leverage.md`.

The APM analysis is pinned to commit
`18c4c43c924ceae890fe0f2038806690e5b2d6c8`. It covers official consumer and
producer documentation plus resolver, lockfile, source, staging, security,
deployment-ledger, target, and integrity implementation.

Post-design inspection identified reusable prototype code for frontmatter,
validated names, safe paths and walks, Git argument handling and caching,
deterministic hashing, transaction recovery, artifact maps, drift checks, native
target paths, stable errors, and process tests. The detailed disposition is in
`docs/implementation-plan.md`.

## Recommended continuation

Start Phase 1 of `docs/implementation-plan.md`:

1. specify strict versioned Rust codecs for `lskills.toml` and
   `lskills.lock.toml`;
2. add project discovery and global `ScopeContext`;
3. support root and multi-skill local package shapes;
4. consolidate one authorized portable file inventory and digest contract;
5. resolve local path dependencies and write deterministic lockfiles;
6. add frozen local replay and actual-binary process tests.

Use TDD for persisted schemas and transaction behavior. Do not start Git updates,
target deployment, forks, bundles, or registries before the preceding phase's
completion evidence exists.

## Constraints

- Keep `crates/lskills-core` and `crates/lskills-cli` until a concrete dependency
  rule proves another crate is needed.
- Use direct Cargo commands through mise shims.
- Keep production skills, credentials, caches, global state, and user collections
  out of this machinery repository.
- Treat source content as data and never execute it.
- Do not silently reinterpret old provenance as a replayable lockfile.
- Do not remove prototype commands or formats without an approved migration.
- Do not commit or push without explicit approval.
