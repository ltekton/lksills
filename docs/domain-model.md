# lskills Domain Model

> Status: reduced MVP baseline

The MVP uses the existing Skill/Bundle model. It does not introduce a generic agent
asset domain.

## Objects

### Workarea

One local skills-root selected for an invocation:

```text
<root>/skills/
<root>/bundles/
<root>/.lskills/provenance.toml
```

The workarea has no configured alias or durable lskills identity. The filesystem
path is the operational identity for the invocation.

### Origin

A read-only source locator:

```rust
struct Origin {
    locator: String,       // local path or Git spec
    reference: Option<String>,
}
```

The existing `RepoSpec` is reused for Git origins. A local origin is identified by
its source path and a content digest captured at import time.

### Bundle and Skill

These are the existing validated types:

```rust
BundleName
SkillFullName
Bundle
Skill
```

A bundle names its member skills. The import unit is one bundle plus all referenced
skill directories.

### Provenance entry

```rust
struct ProvenanceEntry {
    bundle: BundleName,
    source: String,
    selector: String,
    revision: String,
    digest: String,
}
```

The persisted form is a versioned TOML array in `.lskills/provenance.toml`:

```toml
schema = 1

[[imports]]
bundle = "demo"
source = "https://github.com/example/skills.git"
selector = "bundles/demo.toml"
revision = "git:abc123"
digest = "sha256-..."
```

The record is provenance only. It is not a baseline, freshness claim, merge base,
or update subscription.

## Mapping

```text
one origin       -> many bundles
many origins     -> one workarea
one import       -> one destination bundle and its skills
one bundle       -> many referenced skills
```

Two origins may contribute to one workarea as long as their destination bundle and
skill names do not collide.

## Import result

An import result reports:

- destination root;
- origin locator;
- selected bundle;
- referenced skills;
- resolved revision or local digest;
- content digest;
- preview/applied status;
- warnings and collision details.

The MVP does not persist an import plan.

## State rules

- Existing destination names cause failure.
- Import never replaces or merges an existing bundle or skill.
- A failed import leaves no successful provenance entry.
- Editing a copied skill is ordinary local editing.
- The MVP does not compare edited content with its origin.
- `publish` consumes the current workarea content.

## Deferred concepts

These concepts are intentionally absent from the MVP:

- managed asset IDs;
- workarea-qualified names;
- origin lineage and reparenting;
- immutable baseline trees;
- local/upstream/conflict states;
- update decisions;
- package artifacts and catalog entries;
- installation receipts as a new lifecycle model.
