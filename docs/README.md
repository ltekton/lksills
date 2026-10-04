# lskills Design Documents

The active design is intentionally small: one local skills workarea, multiple
local or Git origins, and the existing Skill/Bundle publishing workflow.

## Start here

1. [Product](product.md) - product boundary and workflow;
2. [Product requirements](product-requirements.md) - MVP contract;
3. [Architecture](architecture.md) - current two-crate design;
4. [Domain model](domain-model.md) - origins, workarea, bundles, and provenance;
5. [Implementation plan](implementation-plan.md) - two implementation phases;
6. [CLI reference](cli-reference.md) - target import and existing workflow;
7. [ADR-0009](decisions/ADR-0009-single-workarea-multi-origin-mvp.md) - scope decision.

## Specifications

- [Workarea](specs/workareas.md) - one explicit local workarea;
- [Acquisition](specs/acquisition.md) - importing one bundle from one origin;
- [Local management](specs/local-management.md) - existing validation and publishing;
- [Testing](testing.md) - test strategy;
- [Security](security.md) - safety controls.

The upstream, packaging, and publication specifications are retained as deferred
history. The existing `publish` command is a concrete renderer, not a new generic
package/publication subsystem.

## Superseded design

The earlier multi-workarea, asset-first, baseline, update, package, and publication
architecture was intentionally superseded by ADR-0009. The old ADR files remain for
history, but they are not implementation requirements for this MVP.

## Conventions

- The machinery repository contains no production skills or user workareas.
- Examples are synthetic unless explicitly marked otherwise.
- Imported content is data and is never executed.
- No command commits, pushes, or publishes remotely as a hidden side effect.
