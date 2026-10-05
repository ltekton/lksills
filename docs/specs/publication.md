# Publication Specification

> Prefix: `PUB`

## Publish lifecycle

```console
lskills publish --dry-run
lskills publish [--registry <name>]
```

Publish performs:

1. manifest and package validation;
2. frozen resolution validation;
3. security audit;
4. deterministic packing;
5. local bundle verification;
6. immutable registry upload;
7. returned release identity and digest verification.

It does not commit, tag, or push source control.

## Immutable releases

A release is identified by package name and version. A registry rejects a second
artifact for an existing name/version, even when credentials match. Consumers lock
the archive digest as the content trust anchor.

A publishable package requires:

- non-private package metadata;
- stable package name;
- semantic version;
- at least one exported skill;
- no unresolved local-path dependency in the published graph;
- complete license and source metadata required by registry policy;
- a clean frozen lock and successful bundle verification.

## Registry and catalog

A registry stores immutable bundle bytes and version metadata. A catalog is an
optional curated discovery index. Catalog entries resolve to registry releases or
immutable Git revisions and must include the expected digest when available.

Catalogs never override the lockfile and are not trusted as package content.

## Fork publication

A local fork is published under the current package identity, never the upstream
identity. Its bundle provenance records the upstream package, skill, base revision,
and base digest. This communicates lineage without making the release depend on the
upstream package at runtime.

## Git distribution

A Git repository remains a valid package source and a release archive may be
attached to a Git tag. Registry publication is not required for consumption.
`lskills publish` refers specifically to immutable registry upload; ordinary Git
commands remain explicit operator actions.
