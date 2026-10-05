# lskills handoff

## Current state

The repository is at commit `21536eb fix: harden single-workarea imports`, immediately after `57f82d5 feat: add single-workarea bundle imports`. The working tree was clean after the commit, and nothing was pushed.

The accepted product scope is the reduced two-crate MVP: one explicit local `--root` workarea, imports of one existing `Bundle` plus referenced `Skill` directories from local or Git origins, provenance-only metadata, collision refusal, validation, and existing native publishing. Do not revive the deferred four-crate rewrite or generic asset/update architecture.

The authoritative product/design details are already recorded in:

- `AGENTS.md`
- `docs/product.md`
- `docs/product-requirements.md`
- `docs/architecture.md`
- `docs/domain-model.md`
- `docs/cli-reference.md`
- `docs/security.md`
- `docs/testing.md`
- `docs/specs/`
- `docs/implementation-plan.md`
- `docs/decisions/ADR-0009-single-workarea-multi-origin-mvp.md`

Use those artifacts rather than restating or replacing them.

## Completed in the latest commit

The review-driven hardening included:

- exact-origin/reference Git cache keys and cache remote/pinned-commit verification;
- source/destination overlap checks, including preflight checks before Git cache resolution;
- required `skills/` and `bundles/` source directories;
- rejection of symlinks, special files, and non-UTF-8 filenames across relevant source, workarea, and generated trees;
- staged bundle placement with an internal `.lskills/staging` transaction marker, rollback, and recovery;
- rollback error propagation rather than silent suppression;
- strict provenance revision and complete SHA-256 digest format validation across import, inspection, validation, and publishing;
- renderer protections against unsafe generated roots, symlinked parents, and non-regular destinations;
- process and unit coverage for cache mismatch, overlap, recovery, provenance failures, machinery-repository isolation, and renderer safety;
- aligned architecture, CLI, security, testing, ADR, and implementation-plan documentation.

Relevant implementation areas include `crates/lskills-core/src/import.rs`, `remote.rs`, `provenance.rs`, `repo.rs`, `render.rs`, `drift.rs`, `publish.rs`, and CLI composition in `crates/lskills-cli/src/dispatch.rs`.

## Verification

The following passed before the commit:

- `mise run check` - formatting, Clippy with warnings denied, and all workspace tests;
- `git diff --check`;
- import process suite: 20 tests;
- core library tests: 66 tests;
- guard suite: 15 tests;
- remote suite: 6 tests.

No manual acceptance run was performed after the commit. The ignored `SESSION-LEDGER.md` was updated with the current commit state.

## Recommended continuation

1. Run a manual acceptance pass against two local origins and one pinned Git origin. Inspect `import --check`, `.lskills/provenance.toml`, `validate`, `publish --check`, and generated output.
2. Review remaining operational hardening opportunities:
   - concurrent-import protection with a workarea lock or equivalent;
   - fault-injection tests at staging, move, provenance-write, and cleanup boundaries;
   - directory fsync and platform-specific atomic replacement behavior;
   - explicit branch/tag cache refresh semantics.
3. Improve user-facing examples and structured `import --check --json` output if needed.
4. Only then decide whether a one-bundle refresh/update feature is warranted. If it is, write a new design decision first covering source revisions, local edits, overwrite/collision policy, and whether baselines or merges are required. Keep the currently deferred update/baseline/merge and multi-workarea features out of the MVP until explicitly authorized.
5. Ask before creating any additional commit or pushing.

## Suggested skills

For the next session, call the Skill tool for:

- `code-review` - review changes after `21536eb` or assess the remaining safety gaps;
- `tdd` - add fault-injection, recovery, and concurrency regression tests;
- `codebase-design` - design an import lock or strengthen transaction boundaries;
- `domain-modeling` - only if refresh/update state or provenance semantics are reconsidered;
- `diagnosing-bugs` - if any crash-recovery, cache, or filesystem race is reproduced.

## Constraints

- Keep only `crates/lskills-core` and `crates/lskills-cli`.
- Use direct `cargo` commands through mise shims; do not prefix with `eval "$(mise env -s zsh)"`.
- Keep production assets and user collections out of this machinery repository.
- Imported content is data and must never be executed.
- Do not commit or push without explicit approval for the specific action.
- Redact credentials and personal data from any future notes or artifacts.
