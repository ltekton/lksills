# lskills

`lskills` is a skills-focused package manager for consuming, authoring, modifying,
composing, and republishing Agent Skills in a declarative, deterministic,
reproducible workflow.

The target lifecycle uses:

- `lskills.toml` for human-authored package, dependency, selection, fork, and target
  intent;
- `lskills.lock.toml` for exact resolutions, hashes, dependency edges, fork bases,
  and deployment ownership;
- project and global scopes with independent dependency graphs; global manifests
  and configuration under `${XDG_CONFIG_HOME:-$HOME/.config}/lskills`;
- `fork` to promote an exact locked external skill into editable local source;
- deterministic bundles for verification and immutable publication.

```console
lskills init
lskills install microsoft/apm-sample-package#v1.0.0
lskills install --frozen
lskills fork microsoft/apm-sample-package --skill example --as my-example
lskills audit
lskills pack
```

## Status

The target product and architecture are specified, but not yet implemented. The
current Rust prototype still exposes an older `skills/` plus `bundles/` workarea
workflow. It contains reusable validation, Git, filesystem-safety, staging,
rendering, drift, and target-installation code; it must not be mistaken for the
accepted product model.

[ADR-0010](docs/decisions/ADR-0010-manifest-lockfile-skills-lifecycle.md)
supersedes the former single-workarea decision. The migration is sequenced in the
[implementation plan](docs/implementation-plan.md).

## Package shape

A one-skill package can keep `SKILL.md` at its root:

```text
review/
  lskills.toml
  SKILL.md
  references/
```

A package exporting several skills uses:

```text
team-skills/
  lskills.toml
  skills/
    review/SKILL.md
    release/SKILL.md
```

A package may export local skills and depend on local or remote packages at the
same time. Managed dependency materializations and native target directories are
generated state, not editing locations.

Managed dependency packages are generated state. Project dependencies materialize
under `<project>/lskills_modules/`; global dependencies materialize under
`${XDG_DATA_HOME:-$HOME/.local/share}/lskills/modules/`. Reusable downloaded Git or
registry objects live under `${XDG_CACHE_HOME:-$HOME/.cache}/lskills/`.

## Repository layout

```text
crates/
  lskills-core/   lifecycle engine and reusable prototype mechanisms
  lskills-cli/    command-line boundary and process tests

docs/             product, requirements, architecture, specifications, and ADRs
```

This repository contains machinery only. Production skills, user collections,
credentials, caches, and persistent workareas do not belong here.

## Development

The Rust toolchain is managed by [mise](https://mise.jdx.dev/):

```sh
mise install
mise run check
```

Do not commit or push without explicit approval.

## Design map

Read active design documents in this order:

1. [`docs/product.md`](docs/product.md) - product promises and boundaries;
2. [`docs/product-requirements.md`](docs/product-requirements.md) - normative target requirements;
3. [`GLOSSARY.md`](GLOSSARY.md) - canonical vocabulary;
4. [`docs/domain-model.md`](docs/domain-model.md) - entities and invariants;
5. [`docs/architecture.md`](docs/architecture.md) - component and transaction boundaries;
6. [`docs/cli-reference.md`](docs/cli-reference.md) - target command behavior;
7. [`docs/implementation-plan.md`](docs/implementation-plan.md) - phased migration;
8. [`docs/decisions/ADR-0010-manifest-lockfile-skills-lifecycle.md`](docs/decisions/ADR-0010-manifest-lockfile-skills-lifecycle.md) - accepted direction;
9. [`docs/research/microsoft-apm-leverage.md`](docs/research/microsoft-apm-leverage.md) - primary-source APM analysis.
