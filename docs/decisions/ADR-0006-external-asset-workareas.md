# ADR-0006 - External asset workareas

- Status: Superseded by ADR-0009
- Date: M0 design review
- Deciders: lskills maintainers

## Context

The lskills source repository is machinery. If it also becomes the default asset root,
user content, baselines, ledgers, generated packages, and test fixtures will become
entangled with the tool's source and release history.

Users also need a durable place to keep editable assets and state, preferably a
repository they can back up, inspect, clone, and synchronize independently of the
lskills binary.

A single generic `root` path does not communicate whether it is an upstream source,
a local copy, an installation target, or a collection repository.

## Decision

Introduce the external **asset workarea** as a first-class domain and application
concept.

A workarea is an operator-owned collection repository containing:

- editable managed asset copies;
- source and origin records;
- immutable baselines;
- update decisions and operation records;
- staging state;
- generated packages and local catalogs where appropriate.

A Git-backed workarea may be configured with a user-facing `assets_url` and a local
checkout path. The remote and local path are separate properties.

An origin is modeled independently. Each managed copy records its own active origin
binding. The workarea remote is not automatically an origin for the assets it
contains.

## Mapping

The primary relationship is through managed copies:

```text
OriginAsset 1 ---- * ManagedAssetCopy * ---- 1 Workarea
```

Therefore:

- one origin asset can be copied into many workareas;
- one workarea can contain assets from many origins;
- each copy has independent local edits and update decisions;
- one copy has at most one active upstream origin or is authored/detached.

## Consequences

Positive:

- machinery and user data are cleanly separated;
- workarea state is portable and versionable;
- personal, client, and experimental collections can be isolated;
- the same origin asset can be customized independently;
- workarea Git and origin Git have separate policies;
- tests can create isolated temporary workareas.

Costs:

- lskills needs workarea registry and selection logic;
- the local checkout path must be managed in addition to `assets_url`;
- workarea Git refresh must protect dirty local edits;
- durable metadata and baseline storage need schemas;
- terminology must avoid calling every repository an origin or every root a
  workspace.

## Rejected alternatives

### Store assets in the lskills repository

Rejected because it contaminates machinery history and makes user data part of the
tool distribution.

### One global assets root

Rejected because it prevents collection isolation and makes context implicit.

### Treat the workarea remote as a universal origin

Rejected because a collection can contain assets from many unrelated origins and can
also contain authored assets.

### Direct origin-to-workarea configuration only

Rejected because it cannot represent one origin asset in multiple workareas or many
origins in one workarea without an explicit managed-copy record.

## Validation

The first workarea implementation must prove:

1. the machinery repository receives no production asset writes;
2. two named workareas remain isolated;
3. one origin asset can be imported into both;
4. one workarea can contain multiple origins;
5. workarea Git refresh does not discard local edits;
6. a cloned workarea recovers durable asset and baseline state.
