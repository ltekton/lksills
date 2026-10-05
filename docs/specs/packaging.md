# Packaging Specification

> Prefix: `PACK`

## Source package

A package is editable source rooted by `lskills.toml`. It exports a root skill or
skills under `skills/`, may declare dependencies, and may contain forks copied into
local source.

## Bundle

`lskills pack` produces a target-neutral, self-contained bundle from the exact
lock-attested composition. It includes:

```text
lskills.toml             # normalized package metadata and declared requirements
lskills.lock.toml        # bundle resolution/provenance projection
skills/<name>/...        # selected skill trees
bundle.toml              # artifact type, schema, exhaustive paths and hashes
```

The physical archive is deterministic: stable path order, normalized metadata,
fixed timestamp policy, no host-dependent owner IDs, and no undeclared files.

## Inclusion

By default, pack includes:

- local exported skills;
- local forked skills;
- dependency skills selected by the manifest;
- selected skills exported by reachable transitive dependency packages.

An explicit export list narrows local package exports. Dependency selection is
already part of the manifest and lock. Dev dependencies never enter the bundle.

Pack reads only verified source inventories and lock-attested dependency content.
It never archives an internal cache directory wholesale.

## Integrity

`bundle.toml` lists every non-metadata payload file and its SHA-256 digest. Bundle
verification rejects missing, extra, changed, traversing, symlinked, non-portable,
or special entries.

The embedded lock projection records provenance for every included skill, including
fork base identity. It excludes machine-local cache paths, credentials, and target
deployment paths.

## Preview

```console
lskills pack --dry-run --verbose
```

prints every included skill, source identity, file path, and output path without
writing an artifact.

## Formats

The first canonical bundle format is the native lskills bundle above. Additional
formats, such as an Agent Plugins bundle or a target-specific plugin, are output
adapters. A format adapter must either represent the complete composition or fail
before writing; it must not silently drop unsupported content.
