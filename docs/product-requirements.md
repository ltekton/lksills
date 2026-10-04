# lskills Product Requirements

> Status: reduced MVP baseline

These requirements describe the first useful workflow. The existing prototype code
and tests are the implementation base; the requirements below constrain the new
multi-origin import slice.

## ROOT - One workarea

### ROOT-001 - One explicit workarea (Must)

Each invocation operates on one local skills-root. There is no workarea registry or
implicit cross-workarea aggregation.

Acceptance:

- `import` requires `--root <path>`;
- source and destination roots are disjoint;
- the destination contains `skills/`, `bundles/`, and `.lskills/` state;
- an import never writes into the machinery repository or its origin by default.

### ROOT-002 - Ordinary local ownership (Must)

The workarea remains an ordinary editable directory. lskills does not manage its
Git remote, commits, pushes, or refresh policy.

## ORG - Origins

### ORG-001 - Multiple origins in one workarea (Must)

A workarea may contain bundles imported from multiple local or Git skills-roots.
Each import retains its own source provenance.

Acceptance:

- two origins can contribute different bundles to one root;
- the origin of each imported bundle remains inspectable;
- one origin is not treated as the source of all workarea content;
- a reused Git cache entry is verified against the requested origin and pinned
  revision.

### ORG-002 - Existing skills-root source contract (Must)

The first source format is an existing root containing `skills/` and `bundles/`.
An import selects one existing bundle and its referenced skills.

Standalone skill directories, archives, plugins, packages, and registries are
later work.

### ORG-003 - Read-only source resolution (Must)

Local and Git origins are read as source data. Source resolution must not edit the
origin or execute source content.

## IMP - Import

### IMP-001 - Complete bundle import (Must)

Import copies the selected bundle manifest and every referenced skill directory,
including nested files, hidden files, bytes, and supported executable modes.

### IMP-002 - Collision refusal (Must)

If the destination bundle or any referenced skill already exists, the complete
import fails before leaving a partial result. lskills does not merge, replace,
namespace, or automatically rename content.

### IMP-003 - Preview is read-only (Must)

`import --check` validates the source, reports the selected bundle and planned
files, writes nothing to the workarea, and does not recover or remove staged
transaction state.

### IMP-004 - No execution (Must)

Import, validation, and publishing treat scripts, templates, binaries, and other
source files as data. They do not execute imported content or package hooks.

## PROV - Provenance

### PROV-001 - Import provenance is durable (Must)

A successful import records a versioned provenance entry in:

```text
<root>/.lskills/provenance.toml
```

The entry contains the destination bundle, source locator, source selector,
resolved Git revision or local digest, and selected content digest.

### PROV-002 - Provenance is not update state (Must)

Provenance records where content came from at import time. The MVP makes no claim
that an imported bundle is current and does not implement refresh or reconciliation.

## PUB - Existing workflow

### PUB-001 - Existing validation remains usable (Must)

The assembled workarea can be loaded and validated using the existing validation
workflow. A malformed bundle or skill fails closed.

### PUB-002 - Existing publishing remains usable (Must)

The existing deterministic `publish` workflow can render the assembled workarea.
Import does not introduce a separate package, exporter, or catalog model.

## CLI and safety

### CLI-001 - Stable context (Must)

Import results identify the destination root, selected origin, bundle, revision or
digest, and whether the operation was previewed or applied.

### CLI-002 - Stable errors (Must)

Usage errors, source errors, validation errors, collisions, and safety failures have
stable error codes in JSON output and non-zero process status.

### SEC-001 - Safe boundaries (Must)

All source and destination paths are validated. Traversal, unsafe symlinks,
special files, non-UTF-8 relative filenames, source/destination overlap, and writes
outside declared roots are rejected. Git commands use argument APIs and never a
shell.

## Deferred requirements

The MVP does not require:

- multiple named workareas;
- workarea Git synchronization;
- baselines, ledgers, or three-way updates;
- merge, hold, ignore, detach, or refresh decisions;
- generic asset kinds or adapter registries;
- project manifests or additional target scopes;
- package artifacts, native catalogs, or remote publication;
- archives, package sources, hosted registries, or GitHub APIs.
