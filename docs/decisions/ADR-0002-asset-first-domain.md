# ADR-0002 - Asset-first domain

- Status: Superseded by ADR-0009
- Date: M0 design review
- Deciders: lskills maintainers

## Context

The previous prototype centered on `Skill`, `Bundle`, and a repository containing
`skills/` and `bundles/`. That model is useful for one source format but cannot
represent Claude plugins, pi extensions and packages, prompt templates, authored
assets, or arbitrary supporting trees without treating one format as universal.

The rewrite also needs to distinguish an external source from a local managed copy.
A skill is not the only kind of asset, and an installed target is not the source of
truth.

## Decision

Use an asset-first domain with explicit format and target adapters.

The generic model includes:

- `AssetKind`;
- `Origin` and `OriginAssetRef`;
- `Workarea` and `WorkareaId`;
- `ManagedAssetId` and `EditableAsset`;
- `AssetTree` and `SourceSnapshot`;
- `BaselineRef`;
- `InstallPlan` and `Installation`;
- `UpdateComparison` and `UpdatePlan`;
- `PackageArtifact` and `CatalogEntry`.

Agent-specific formats implement detection, validation, installation, and export
contracts. A format may provide an asset tree with files that include instructions,
scripts, references, binaries, and manifests. The generic domain does not assume a
single filename or directory shape.

## Identity consequences

A display name is not sufficient identity. The model distinguishes:

```text
origin repository
  -> origin asset selector
  -> managed copy in a selected workarea
  -> target projections
```

The same origin asset may have independent copies in multiple workareas. One
workarea may contain assets from many origins. A managed copy has one active origin
or is explicitly authored/detached.

## Consequences

Positive:

- new asset kinds do not require replacing the core model;
- complete trees are preserved rather than reduced to skill text;
- local authored assets are first-class;
- upstream and installed state remain separate;
- package generation can be target-specific;
- the lskills repository remains machinery-only.

Costs:

- the first adapter must define more explicit contracts;
- generic validation cannot promise format-specific semantics;
- users may need to resolve ambiguous source kinds;
- compatibility with the old skills-root layout is not automatic.

## Rejected alternatives

### Skill as universal domain object

Rejected because it makes plugins, packages, extensions, and supporting asset trees
second-class or forces them into misleading fields.

### Bundle as the universal collection

Rejected because a workarea is a user-owned lifecycle collection, while a bundle is a
format or packaging concept that may not apply to every asset.

### Installed target as source

Rejected because target directories are projections, may be transformed, and are
owned by the agent or user scope rather than by lskills.

## Validation

The first vertical slice must import one supported asset format while keeping the
format-specific rules in an adapter. Domain tests must be able to construct authored,
imported, and detached assets without a filesystem.
