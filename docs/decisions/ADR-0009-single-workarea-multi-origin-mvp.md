# ADR-0009 - Single-workarea, multi-origin MVP

- Status: Accepted
- Date: 2026-10-04
- Deciders: lskills maintainers

## Context

The previous M0 design expanded lskills into a general agent-asset lifecycle
manager with multiple named workareas, generic asset kinds, durable baselines,
three-way updates, packages, catalogs, and publication adapters.

That architecture is disproportionate to the immediate use case: assemble one
local skills collection from several origins and publish it using the existing
skills-root workflow.

The repository already contains a working two-crate prototype with validated Skill
and Bundle models, local/Git source caching, deterministic rendering, installation,
and process fixtures.

## Decision

The first implementation will use the existing skills-root model and two-crate
workspace.

### Workarea

There is one ordinary local workarea per invocation. It is selected with `--root`.
Imports require an explicit root. lskills does not register, clone, refresh, commit,
or push the workarea.

### Origins

Origins are local directories or Git repositories with this layout:

```text
skills/
bundles/
```

An import selects one existing bundle and copies that bundle plus every referenced
skill into the workarea. Multiple origins may contribute different bundles to the
same workarea.

### Provenance

A successful import appends a versioned entry to:

```text
<root>/.lskills/provenance.toml
```

The entry records source locator, selector, resolved revision or local digest, and
content digest. This is provenance only. It is not a baseline, freshness claim, or
update subscription.

### Collision policy

If the destination bundle or any referenced skill already exists, the import fails
without partial writes. There is no merge, replacement, namespace, or automatic
rename.

### Command surface

The new acceptance surface is:

```text
import --check
list
show
validate
publish --check
```

The existing prototype commands remain available as regression behavior but are not
expanded by this scope.

### Deferred behavior

Multiple workareas, workarea Git management, upstream updates, three-way merge,
generic agent assets, package/catalog models, project manifests, new target scopes,
remote publication, archives, registries, and the four-crate rewrite are deferred.

## Consequences

Positive:

- the product matches the immediate use case;
- existing code and tests become the implementation base;
- source provenance is retained without an update database;
- the first new feature is a small, testable import boundary;
- workarea and origin identity cannot be accidentally conflated.

Costs:

- the MVP is limited to the existing skills-root and bundle formats;
- imported content cannot be refreshed through lskills yet;
- destination names must be unique;
- existing prototype commands remain in the repository until a later cleanup.

## Supersession

This ADR supersedes earlier M0 decisions wherever they introduce multiple
workareas, a generic asset-first domain, baselines, upstream coordination, package
artifacts, catalogs, publication adapters, or a four-crate rewrite:

- ADR-0001;
- ADR-0002;
- ADR-0003;
- ADR-0004;
- ADR-0005;
- ADR-0006;
- ADR-0007;
- ADR-0008.

Those files remain historical context and are not active implementation contracts.
