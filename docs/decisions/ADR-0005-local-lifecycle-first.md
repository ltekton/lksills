# ADR-0005 - Local lifecycle first

- Status: Superseded by ADR-0009
- Date: M0 design review
- Deciders: lskills maintainers

## Context

The product could grow into a package manager, hosted marketplace, team registry, or
background update service. Building those surfaces before proving safe local
acquisition, editing, installation, and upstream coordination would make the core
lifecycle difficult to validate.

The workarea model provides a durable local boundary and can later support explicit
remote synchronization without requiring a hosted lskills control plane.

## Decision

Prioritize the local lifecycle:

```text
select workarea
  -> acquire and detect
  -> materialize complete copy
  -> install/use locally
  -> check upstream
  -> reconcile explicitly
  -> package
  -> generate local catalog when required
```

Defer hosted publication, universal registries, team workflows, background daemons,
automatic updates, and broad package management.

The user may still use GitHub-backed workareas and Git-backed origins. GitHub is an
origin/workarea transport, not a requirement for a lskills service.

## Consequences

Positive:

- small end-to-end vertical slice;
- deterministic tests with temporary workareas;
- fewer authentication and remote-failure concerns initially;
- clear source-of-truth rules;
- local/offline use remains possible.

Costs:

- no built-in team discovery or hosted distribution;
- remote publication must wait;
- some target-specific integrations remain manual;
- release workflows are not the initial product center.

## Rejected alternatives

### Hosted registry first

Rejected because it would make remote availability and identity policy prerequisites
for local asset management.

### Background synchronization

Rejected because silent local mutation conflicts with explicit update decisions and
would complicate workarea Git behavior.

### Universal package manager

Rejected because agent assets have heterogeneous formats, trust models, and native
projection rules that require adapters before a common registry can be safe.

## Validation

The M1-M6 slice must work with a local-only workarea and with a user-owned Git-backed
workarea without a hosted lskills backend. Remote publication must remain visibly
separate from package and local catalog operations.
