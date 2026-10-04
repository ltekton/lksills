# ADR-0004 - Separate package generation from publication

- Status: Superseded by ADR-0009
- Date: M0 design review
- Deciders: lskills maintainers

## Context

Agent targets differ in whether they consume a directory, a package, a marketplace
entry, or a catalog. Treating package generation and publication as one operation
would make local use depend on remote infrastructure and would hide which artifact
was actually reviewed.

The editable workarea copy must remain the source. A generated package or catalog
must not become a competing source of truth.

## Decision

Model two explicit stages:

1. `PackageArtifact` - a deterministic native artifact generated from a selected
   managed copy;
2. `CatalogEntry` and `Publication` - metadata and an explicit operation that makes
   the artifact discoverable through a target-native catalog or destination.

The first release supports local filesystem catalogs. Remote publication, hosted
marketplaces, universal registries, and team distribution are later adapters.

A package records:

- workarea-qualified source identity;
- source content digest;
- target and exporter schema;
- artifact content and digest;
- conversion diagnostics.

A catalog entry references a concrete package digest. A local catalog can live in
the same workarea as generated output, but it is not editable source content.

## Consequences

Positive:

- package builds can be reviewed without publication;
- local/offline workflows do not need a hosted service;
- catalog entries can prove which source produced an artifact;
- target-specific publication policy remains at the edge;
- failed publication does not invalidate a built package.

Costs:

- there are more explicit commands and result states;
- artifact and catalog schemas need versioning;
- local output retention and cleanup require policy;
- a remote publication adapter must define destination conflicts and partial failure.

## Rejected alternatives

### Package equals source

Rejected because generated layouts and metadata are projections and may be lossy.

### Build automatically publishes

Rejected because it creates an unexpected external effect and spends remote or CI
resources without a separate decision.

### Hosted marketplace first

Rejected because the local lifecycle, deterministic artifact, and workarea ownership
must be proven before remote infrastructure becomes a dependency.

## Validation

Tests must build identical artifacts from identical workarea content, verify catalog
references by digest, generate local catalogs offline, and prove package generation
does not push a workarea or modify editable content.
