# lskills Design Documents

The active design treats lskills as a skills-focused package manager. It is guided
by one product statement:

> lskills helps users consume, author, modify, compose, and republish Agent
> Skills in a declarative, deterministic, reproducible workflow.

## Start here

1. [Product](product.md) - product goal, promises, and lifecycle;
2. [Product requirements](product-requirements.md) - normative target behavior;
3. [Domain model](domain-model.md) - package, skill, requirement, lock, fork, and bundle;
4. [Architecture](architecture.md) - deep modules and lifecycle data flow;
5. [CLI reference](cli-reference.md) - npm/APM-shaped command surface;
6. [Security](security.md) - trust boundaries and required controls;
7. [Implementation plan](implementation-plan.md) - migration from the current prototype;
8. [ADR-0010](decisions/ADR-0010-manifest-lockfile-skills-lifecycle.md) - reframe decision.

The canonical vocabulary is in [`../GLOSSARY.md`](../GLOSSARY.md).

## Focused specifications

- [Project and global scope](specs/workareas.md);
- [Dependency resolution and installation](specs/acquisition.md);
- [Authoring, forks, and deployment](specs/local-management.md);
- [Update and upstream coordination](specs/upstream-coordination.md);
- [Packing](specs/packaging.md);
- [Publication](specs/publication.md);
- [Testing](testing.md).

## Research basis

- [Microsoft APM analysis and lskills adaptations](research/microsoft-apm-leverage.md).

The APM research is pinned to a specific upstream revision. APM provides the
manifest/lock/install/update/audit/package-manager baseline; lskills narrows the
domain to skills and adds first-class fork-to-local-source behavior.

## Historical decisions

ADRs 0001-0009 describe earlier designs and implementation scope reductions. They
remain historical context. ADR-0010 supersedes them as the active product and
target-architecture decision.

## Conventions

- `lskills.toml` is human-authored intent.
- `lskills.lock.toml` is generated exact resolution and deployment state.
- Project scope is the default; `--global` selects independent user state.
- Internal cache, staging, and assembly paths are not product concepts.
- Skill resources are data and are never executed by lskills.
- No command commits or pushes source control as a hidden side effect.
