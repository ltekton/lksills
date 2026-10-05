# Dependency Resolution and Installation Specification

> Prefixes: `MAN`, `LOCK`, `INST`

## Manifest dependency forms

The common form is a concise string in `lskills.toml`:

```toml
dependencies = [
  "microsoft/apm-sample-package#v1.0.0",
  "github/awesome-copilot/skills/review-and-refactor",
  "./packages/team-skills",
]
```

Supported source kinds are:

- Git shorthand or full Git URL, with optional ref;
- repository subpath selecting a package or single skill;
- local package path;
- local verified bundle;
- registry package and version constraint.

Structured inline tables handle explicit selection and source metadata:

```toml
dependencies = [
  { git = "acme/team-skills", ref = "v2.0.0", skills = ["review"] },
  { registry = "corp", package = "acme/security", version = "^3.1" },
]
```

Unknown fields fail closed. Local paths are relative to the manifest, not the
process working directory.

## Package loading

A source package contains either:

- root `SKILL.md`, exporting one skill; or
- `skills/<name>/SKILL.md`, exporting one or more skills.

A normal publishable package also contains `lskills.toml`. A manifestless bare
skill may be consumed from a local path or repository subpath using synthesized
private metadata, but it cannot be published until promoted to a named, versioned
package.

## Resolution

Resolution:

1. parses direct requirements;
2. loads package manifests without executing source content;
3. follows transitive package dependencies;
4. canonicalizes package identities;
5. selects one compatible exact version per package identity;
6. validates requested skill subsets;
7. reports cycles and immutable conflicts with complete dependency chains;
8. reuses unchanged prior lock entries in sync mode;
9. produces a complete plan before target mutation.

A requirement's display metadata is not a trust anchor. Git commit, registry
artifact digest, and package content digest establish exact identity.

## Install modes

### Sync

Bare `lskills install`:

- replays unchanged locked requirements;
- resolves new requirements;
- re-resolves requirements changed in the manifest;
- removes unreachable lock nodes and safely reconciles owned deployment output;
- writes a canonical lockfile only when semantic state changes.

It does not refresh an unchanged mutable branch or version range.

### Frozen

`lskills install --frozen` requires an existing compatible lockfile. It fails on:

- manifest digest mismatch;
- missing direct or transitive lock entries;
- requirement/ref/selection drift;
- local source-tree drift;
- missing locked source content that cannot be fetched exactly;
- content or archive hash mismatch;
- target adapter incompatibility.

Frozen means no resolution changes; it does not necessarily mean offline.

### Offline

`--offline` prohibits network access. Combined with `--frozen`, every required
locked artifact must already exist in the verified cache or as local source.

### Dry run

`--dry-run` performs resolution and planning but leaves the manifest, lockfile,
materialization store, staging state, and target files unchanged.

## Source adapters

Git, path, bundle, and registry adapters all return the same verified package
interface. They do not know target locations. Source cache keys include canonical
source identity and exact resolution; credentials and display aliases are excluded.

Git operations use argument-vector execution, disable source hooks, and prevent
option injection. Registry artifacts are accepted only when their digest matches
the immutable release metadata.

## Composition and collision

After resolution, composition combines:

- local package skills;
- forked local skills;
- selected skills from direct and transitive dependencies.

If two effective skills have the same deployed name, installation fails unless the
manifest contains an explicit alias or fork replacement. Declaration order does not
choose a winner.

## Deployment

For each target, lskills creates one authorized file plan, scans it, stages the
complete desired state, verifies it, and activates it. The lockfile records each
managed path, owner, adapter version, and content hash.

Stale files are removed only when still owned and unchanged. Failed or unsafe
cleanup leaves prior ownership in the lockfile for inspection and retry.
