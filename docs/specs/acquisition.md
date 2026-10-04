# Acquisition Specification

> Status: reduced MVP baseline
> Prefixes: `ORG`, `IMP`, `PROV`

## Source contract

An origin is a local directory or Git repository with the existing skills-root
layout:

```text
skills/
bundles/
```

The existing `Repo::load` and `RepoSpec` behavior is reused. Source resolution is
read-only, verifies a reused Git cache against the requested origin and revision,
and does not execute source content. Source and destination roots must be disjoint.

## Import unit

One import selects exactly one existing bundle. The destination receives:

- the bundle manifest;
- every referenced skill directory;
- all nested files and supported modes;
- one provenance entry.

Standalone skills, automatic bundle generation, archives, plugins, packages, and
registries are deferred.

## Preview and apply

`import --check` validates the source, lists the selected bundle and member skills,
shows the source revision or local digest, detects collisions, and writes nothing.

Apply stages the complete copy behind a durable internal transaction marker,
validates it, then places it under the destination root. Interrupted placement is
rolled back before another mutating operation or inspection proceeds. If any
destination bundle or member skill already exists, the import fails without merge,
replacement, or renaming.

## Provenance

The sidecar is:

```text
<root>/.lskills/provenance.toml
```

It is strict, versioned TOML. Each entry records destination bundle, source
locator, source selector, resolved revision or local digest, and selected content
digest. It is provenance only and does not support freshness or update checks.

## Safety

Reject traversal, absolute destination paths, unsafe symlinks, special files,
non-UTF-8 relative filenames, source/destination overlap, and writes outside the
explicit workarea. Source files are data and are never executed.
