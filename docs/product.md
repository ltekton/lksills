# lskills Product

> Status: reduced MVP baseline

`lskills` assembles one local skills workarea from multiple origins and publishes
the resulting collection to the agent targets already supported by the project.

## Product thesis

> **Bring skills from several trusted source repositories into one editable local
> collection, then publish that collection consistently.**

The workarea is an ordinary local skills-root. lskills does not own a hosted
registry, operate an agent runtime, or manage a network of synchronized copies.

## Core model

### Origin

A read-only local directory or Git repository containing the existing skills-root
layout:

```text
origin/
  skills/
  bundles/
```

An origin may provide many bundles. A single import selects one bundle and its
referenced skills.

### Workarea

The one local `skills/` plus `bundles/` tree operated on by lskills:

```text
workarea/
  skills/
  bundles/
  .lskills/provenance.toml
```

The workarea is selected explicitly with `--root` for imports. There is no registry
of named workareas.

### Imported bundle

An imported bundle is a normal local bundle and its referenced skill directories,
plus a provenance entry. The local bundle and skills are the editable source for
validation and publishing.

## Workflow

```text
local/Git origin
  -> select one bundle
  -> validate
  -> stage and copy bundle plus member skills
  -> record source provenance
  -> edit the one workarea normally
  -> validate
  -> publish
```

Import is provenance capture, not synchronization. The recorded revision or digest
explains where content came from; it does not promise update checks.

## MVP scope

In scope:

- one workarea;
- multiple local or Git origins;
- existing Skill and Bundle formats;
- one-bundle-at-a-time import;
- provenance sidecar;
- collision refusal;
- complete file and mode copying;
- validation and existing native publishing;
- stable human and JSON output;
- safe, non-executing source handling.

Deferred:

- multiple workareas and workarea configuration;
- upstream refresh and three-way reconciliation;
- baselines and operation ledgers;
- generic agent asset kinds;
- package and catalog abstractions;
- project manifests and new target adapters;
- remote publication, commits, and pushes;
- archives, registries, and hosted services.

## Success criteria

The first useful release is successful when an operator can:

1. choose one explicit workarea;
2. import a bundle from origin A;
3. import a different bundle from origin B;
4. see both origins in the workarea provenance record;
5. reject a colliding import without partial writes;
6. validate the assembled workarea; and
7. publish the assembled collection using the existing renderer.

## Vocabulary

**Origin** - a read-only local or Git skills-root.

**Workarea** - the one local `skills/` plus `bundles/` collection.

**Imported bundle** - one bundle and its referenced skills copied from an origin.

**Provenance** - source, selector, revision, and digest recorded at import time.

**Publish** - the existing deterministic generation of native agent artifacts from
the workarea.
