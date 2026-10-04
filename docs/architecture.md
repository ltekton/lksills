# lskills Architecture

> Status: reduced MVP baseline

The MVP deliberately uses the existing two-crate implementation. It is a local
skills-root tool, not a general asset lifecycle platform.

## System boundary

```text
CLI
  -> one explicit workarea root
  -> one local or Git origin
  -> existing Skill/Bundle loader
  -> staged bundle import + provenance
  -> existing validation and publishing
```

The machinery repository is separate from the selected workarea. The source origin
is read-only. The workarea is the only editable collection.

## Crates

```text
lskills-cli
  -> lskills-core
```

### `lskills-core`

Owns the existing skills-root model and behavior:

- validated `BundleName`, `SkillFullName`, and agent identifiers;
- loading `skills/` and `bundles/`;
- frontmatter and bundle validation;
- local/Git source resolution and cache reuse;
- import planning and staged materialization;
- provenance parsing and writing;
- deterministic rendering and existing installation behavior.

The core may use the existing injected seams and filesystem helpers. Do not split it
into new domain, application, and adapter crates for this MVP.

### `lskills-cli`

Owns:

- process arguments and `--root` resolution;
- the `import` command and `--check` mode;
- human and JSON output;
- process exit codes;
- composition of the core operations.

## Components

### Workarea root

A `PathBuf` selected for the invocation. Import requires the caller to provide it
explicitly. The root has this logical shape:

```text
<root>/
  skills/
  bundles/
  .lskills/provenance.toml
```

The user may version this directory with ordinary Git. lskills does not inspect or
synchronize its Git history for the MVP.

### Origin loader

The existing loader resolves either:

- a local skills-root path; or
- a Git `RepoSpec` into the existing read-only cache.

It returns the current validated `Repo`. It does not return an editable workarea or
write destination state.

### Importer

The importer:

1. selects one existing bundle;
2. resolves its referenced skills;
3. checks all destination names and paths;
4. builds a complete staged copy;
5. writes the staged bundle and skills into the workarea;
6. appends one provenance entry only after the copy succeeds.

A failed import must not leave a partial bundle or a successful provenance record.

### Provenance store

The store reads and writes one versioned TOML sidecar. It is a record of import
origin, not an update database. It has no history, baseline tree, or operation
ledger beyond the import entries needed for inspection.

### Existing renderer

`publish` remains the concrete native renderer already implemented in the core. It
is not generalized into package, catalog, exporter, or publication abstractions.

## Data flow

```text
origin path/spec
  -> RepoSpec / local path
  -> validated source Repo
  -> selected Bundle + member Skills
  -> ImportPlan or --check result
  -> staged destination
  -> workarea + provenance
  -> existing validate/list/show/publish
```

Plans are useful as internal values, but the MVP does not persist plans or expose a
general plan/application framework.

## Invariants

- one invocation has one workarea root;
- an origin is never mutated by import;
- a destination bundle and its member skills are all-or-nothing;
- names and relative paths are validated before joins;
- imported files are never executed;
- provenance is written only after successful materialization;
- source provenance is per imported bundle;
- publishing reads the workarea, not the origin cache;
- no operation commits, pushes, or refreshes a remote workarea.

## Explicitly outside the architecture

There is no MVP model for:

- `WorkareaId`, workarea aliases, or repository identities;
- multiple workarea contexts;
- `AssetKind`, generic managed assets, or format registries;
- baseline snapshots, lineage, or update states;
- target/package/catalog/publication ports;
- project manifests or hosted services.
