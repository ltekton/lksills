# lskills

`lskills` manages one local skills workarea assembled from multiple origins and
publishes that collection to the agent targets already supported by the project.

The first useful workflow is:

```text
origin skills-root A ─┐
origin skills-root B ─┼─> one workarea ─> validate ─> publish
local skills-root C ─┘
```

An origin is read-only source content. The workarea is the one local `skills/` plus
`bundles/` tree that the operator owns and edits. An imported bundle keeps a small
provenance record identifying its source and revision.

## Scope

The first slice:

- keeps the existing `Skill` and `Bundle` model;
- manages one workarea selected with `--root` or `LSKILLS_ROOT`;
- imports one existing bundle and its referenced skills from a local or Git origin;
- records import provenance in `.lskills/provenance.toml`;
- rejects collisions instead of merging or replacing content;
- retains the existing validation and native publishing workflow.

The first slice does not implement multiple workareas, upstream synchronization,
three-way merges, generic agent asset kinds, package registries, catalogs, or a new
four-crate architecture.

## Commands in the reduced target

```text
lskills import <origin> --root <workarea> --bundle <name> [--check]
lskills list --root <workarea>
lskills show <name> --root <workarea>
lskills validate --root <workarea>
lskills publish --root <workarea> [--check]
```

`install` and the other existing prototype commands remain available as reference
and regression coverage, but are not expanded by the origin work.

`import` requires an explicit destination root. This prevents a mutating operation
from accidentally writing into the lskills machinery repository.

## Repository layout

```text
crates/
  lskills-core/   current engine and the implementation base
  lskills-cli/    command-line boundary

docs/             active product, architecture, requirements, and plan
```

The current two-crate implementation is the foundation. A new domain/application/
adapters workspace is intentionally deferred until the simple workflow proves it
needs one.

## Development

The Rust toolchain is managed by [mise](https://mise.jdx.dev/):

```sh
mise install
mise run check
```

Do not commit or push without explicit approval. Do not add production skills or
user asset collections to this machinery repository. Tests use synthetic sources
and temporary workareas.

Read the active design in this order:

1. [`AGENTS.md`](AGENTS.md) - contribution and safety rules;
2. [`docs/product.md`](docs/product.md) - product boundary;
3. [`docs/product-requirements.md`](docs/product-requirements.md) - MVP contract;
4. [`docs/architecture.md`](docs/architecture.md) - current two-crate design;
5. [`docs/domain-model.md`](docs/domain-model.md) - small domain model;
6. [`docs/implementation-plan.md`](docs/implementation-plan.md) - execution plan;
7. [`docs/cli-reference.md`](docs/cli-reference.md) - target command behavior;
8. [`docs/decisions/ADR-0009-single-workarea-multi-origin-mvp.md`](docs/decisions/ADR-0009-single-workarea-multi-origin-mvp.md) - scope decision.
