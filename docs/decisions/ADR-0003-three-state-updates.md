# ADR-0003 - Three-state upstream comparison

- Status: Superseded by ADR-0009
- Date: M0 design review
- Deciders: lskills maintainers

## Context

Comparing the current local copy only with the latest upstream source cannot tell
whether a difference was introduced locally or upstream. Replacing the local copy
from the latest source can destroy operator edits.

The problem is compounded when the same origin asset exists in several workareas.
Each managed copy needs independent update state and decisions.

## Decision

Every imported managed copy retains an immutable accepted baseline. An upstream check
compares three states:

```text
base     - the accepted origin snapshot
local    - the current editable managed copy in one workarea
upstream - the newly resolved origin snapshot
```

The check produces a read-only `UpdateComparison` and an `UpdatePlan`. Applying a
plan is a separate explicit operation.

Supported decisions include:

- accept upstream;
- merge;
- select changes;
- keep local;
- ignore a revision;
- hold the decision;
- detach from upstream.

A managed copy has at most one active upstream. Origin history may retain earlier
bindings, but lskills does not implicitly merge two unrelated live origins.

## Baseline semantics

The baseline is an immutable source snapshot, not a hash of the current local copy.
A successful reconciliation may advance the baseline. A read-only check never does.
If the origin cannot be resolved, status is unavailable rather than current.

The baseline and decisions belong to the workarea-qualified managed asset. Accepting
an update for `personal:review-docs` does not alter `client:review-docs`.

## Conflict granularity

The initial implementation supports deterministic file-level comparison and safe
text hunks where possible. Binary files, executable mode changes, and unsupported
structured metadata may produce whole-file conflicts. The plan records the
comparison granularity.

## Consequences

Positive:

- local edits are not silently overwritten;
- upstream-only and local-only changes are distinguishable;
- checks can be safely automated or run in CI as read-only operations;
- independent workareas can make different decisions;
- decisions are auditable and reproducible.

Costs:

- baselines require durable storage;
- updates require more states and user-visible decisions;
- merge behavior needs a defined text/binary policy;
- stale-plan detection is necessary before apply.

## Rejected alternatives

### Two-way latest-versus-local replacement

Rejected because it cannot preserve provenance or distinguish local from upstream
changes.

### Automatic three-way merge on check

Rejected because a check must be read-only and a merge can change the operator's
editable source.

### One global origin state per repository

Rejected because one origin asset may have independent managed copies in multiple
workareas.

## Validation

Tests must cover upstream-only, local-only, non-conflicting, conflicting, unavailable,
held, ignored, and detached states. Every apply test must prove that a failed
reconciliation leaves the previous managed copy and baseline intact.
