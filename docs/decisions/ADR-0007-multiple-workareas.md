# ADR-0007 - Multiple named workareas with one active context

- Status: Superseded by ADR-0009
- Date: M0 design review
- Deciders: lskills maintainers

## Context

Users may maintain separate personal, client, project, experimental, or release
collections. Forcing one workarea per lskills installation would mix unrelated
assets and update state. Allowing commands to implicitly combine all workareas would
make same-named assets and destructive operations ambiguous.

## Decision

A user can register multiple named workareas. One is the default for convenience.
Every mutating operation resolves exactly one active workarea through explicit
selection and documented precedence:

1. direct one-shot local or URL override;
2. explicit named workarea;
3. project configuration;
4. user default.

The selected workarea is part of the application context and appears in plans,
results, errors, and durable records.

The first project manifest pins one workarea. Multi-workarea project manifests may
be added later, but would require qualified asset references and collision planning.
Read-only all-workarea inventory may be added without changing mutation semantics.

## Consequences

Positive:

- normal commands stay concise through a default;
- personal and sensitive collections remain isolated;
- same-named assets are naturally qualified;
- a mutation cannot silently cross collection boundaries;
- workarea-specific update and package state remains independent.

Costs:

- configuration needs named entries and local paths;
- users must understand active workarea context;
- future cross-workarea project use needs explicit collision semantics;
- commands must report context to avoid surprising writes.

## Rejected alternatives

### Exactly one workarea per user

Rejected because it mixes unrelated ownership, trust, and lifecycle state and does
not support client or experimental isolation.

### Implicit aggregate of all workareas

Rejected because mutation and same-name resolution become unsafe.

### Multiple active workareas for every command

Rejected for the initial release because it expands planning, conflict, and target
collision semantics before the single-workarea lifecycle is proven.

### Separate lskills installation per collection

Rejected because collection selection is user data/configuration, not a reason to
duplicate the machinery binary.

## Validation

Tests must demonstrate:

- default selection;
- explicit selection overriding the default;
- project selection overriding the default;
- direct URL/local override;
- conflicting selectors rejected;
- mutations isolated between two workareas;
- read-only inventory, if implemented, qualifies every result.
