# lskills CLI Reference

> Status: target command surface

The CLI follows npm/APM conventions. Project scope is the default; `--global` or
`-g` selects the independent user scope under
`${XDG_CONFIG_HOME:-$HOME/.config}/lskills`. Commands discover the nearest
`lskills.toml` unless an advanced `--project <path>` override is supplied.

## Core lifecycle

### `lskills init`

```console
lskills init
lskills init --global
```

Creates a minimal TOML manifest. Project initialization derives a default private
package name from the directory and does not create target output.

### `lskills install`

```console
lskills install
lskills install <REFERENCE>
lskills install <REFERENCE> --skill <NAME>...
lskills install --frozen
lskills install --dry-run
lskills install --global <REFERENCE>
```

With no reference, synchronizes manifest, lockfile, materialized packages, and
selected targets. Unchanged requirements replay their locked resolutions.

With a reference, transactionally adds the requirement to `lskills.toml` and then
runs synchronization. If resolution, validation, scanning, or deployment fails,
the manifest change is rolled back.

`--skill` records an explicit replacement selection for that dependency. Omitted
selection means all package exports.

`--frozen` requires an exact manifest/lock/source match and never resolves a new
version. `--dry-run` reports the complete plan without mutation.

Supported concise references:

```text
owner/repo
owner/repo#v1.2.0
host/owner/repo#main
owner/repo/path/to/package#v1.2.0
https://host/owner/repo.git#v1.2.0
./packages/local-skills
../sibling-package
```

### `lskills uninstall`

```console
lskills uninstall <PACKAGE>
lskills uninstall --global <PACKAGE>
```

Removes a direct requirement, recomputes the reachable graph, and removes only
unchanged target files owned exclusively by unreachable skills. Edited files are
retained and reported.

### `lskills update`

```console
lskills update
lskills update <PACKAGE>...
lskills update --dry-run
lskills update --global
```

Checks authoritative upstream sources, computes an added/updated/removed/unchanged
plan, and asks for confirmation before changing the lockfile or deployment.
`--yes` makes confirmation non-interactive.

### `lskills outdated`

```console
lskills outdated
lskills outdated --global
```

Reports newer commits or versions matching the manifest without writing state.
Local path dependencies are reported as local rather than queried remotely.

### `lskills lock`

```console
lskills lock
lskills lock --update
lskills lock --check
```

Resolves and writes `lskills.lock.toml` without changing target deployments.
`--check` fails if regeneration would change the lockfile.

## Authoring and modification

### `lskills new skill`

```console
lskills new skill <NAME>
```

Creates `skills/<name>/SKILL.md` in the current package and updates an explicit
export list when one exists.

### `lskills fork`

```console
lskills fork <PACKAGE> --skill <NAME> [--as <LOCAL-NAME>]
```

Requires the source package and skill to be locked. It:

1. copies the exact locked skill tree to `skills/<local-name>/`;
2. records `[forks.<local-name>]` in the manifest;
3. records the exact upstream base in the next lockfile;
4. makes the local fork the explicit deployment replacement;
5. never edits dependency materialization or target output.

If the destination exists, the command fails without merging.

### `lskills diff`

```console
lskills diff <LOCAL-SKILL> --upstream
lskills diff <LOCAL-SKILL> --upstream --latest
```

Compares a fork to its locked base or, with `--latest`, to current upstream. It is
read-only. Automated rebase is deferred until conflict semantics are specified.

### `lskills validate`

```console
lskills validate
lskills validate --global
```

Validates the manifest, package shapes, `SKILL.md` frontmatter, skill identities,
resource paths, dependencies, fork declarations, selections, aliases, and target
compatibility without deploying.

## Inspection and audit

### `lskills list`

```console
lskills list
lskills list --global
lskills list --effective
```

Lists local and dependency skills with package identity, installed name, selection,
and target reach. `--effective` reports the runtime-visible project/global union
without treating global entries as project dependencies.

### `lskills view`

```console
lskills view <PACKAGE-or-SKILL>
lskills view <PACKAGE> --versions
```

Shows manifest and lock metadata, source, exact resolution, selected skills,
content hashes, fork provenance, and target deployments.

### `lskills why`

```console
lskills why <PACKAGE-or-SKILL>
```

Explains the direct or transitive path that caused an item to be resolved and the
selection or fork rule that caused it to be deployed.

### `lskills audit`

```console
lskills audit
lskills audit --ci
lskills audit --format json
```

Checks manifest-lock consistency, source and package hashes, local skill hashes,
fork bases, skill selection, deployment ownership, deployed-file integrity,
content security, and scratch replay drift. `--ci` fails closed when exact replay
cannot be established.

## Distribution

### `lskills pack`

```console
lskills pack
lskills pack --archive
lskills pack --dry-run --verbose
```

Builds a target-neutral bundle from lock-attested local, forked, and selected
dependency skills. The bundle includes package metadata, provenance, and an
exhaustive hash manifest. `--dry-run` shows every included file and source.

### `lskills verify`

```console
lskills verify <BUNDLE>
```

Checks bundle schema, exhaustive file inventory, content hashes, paths, file kinds,
and provenance without installing it.

### `lskills publish`

```console
lskills publish
lskills publish --registry <NAME>
lskills publish --dry-run
```

Validates and packs before uploading an immutable package version. Publishing a
name/version that already exists fails. Private packages cannot be published.

## Configuration

### `lskills config`

```console
lskills config list
lskills config get <KEY>
lskills config set <KEY> <VALUE>
lskills config unset <KEY>
```

Manages `${XDG_CONFIG_HOME:-$HOME/.config}/lskills/config.toml`. Suitable keys
include default targets, transport, cache location, registry aliases, and output
preferences. Dependency intent belongs in the project or global manifest, not user
configuration.

Effective precedence:

```text
CLI > scope manifest > user config > target auto-detection > built-in default
```

## Storage locations

| State | Project scope | Global/shared scope |
|---|---|---|
| Manifest | `<project>/lskills.toml` | `${XDG_CONFIG_HOME:-$HOME/.config}/lskills/lskills.toml` |
| Lockfile | `<project>/lskills.lock.toml` | `${XDG_CONFIG_HOME:-$HOME/.config}/lskills/lskills.lock.toml` |
| Dependency materializations | `<project>/lskills_modules/` | `${XDG_DATA_HOME:-$HOME/.local/share}/lskills/modules/` |
| Download/cache objects | shared XDG cache | `${XDG_CACHE_HOME:-$HOME/.cache}/lskills/` |
| Lifecycle state | `<project>/.lskills/` | `${XDG_STATE_HOME:-$HOME/.local/state}/lskills/` |

`lskills_modules/` is the project equivalent of APM's `apm_modules/`. It contains
verified generated package trees, not authored skills. The shared cache contains
reusable transport objects and is not a materialized scope. Deleting either is
safe when exact locked content remains fetchable.

## Common options

- `--global`, `-g` - use the global manifest and lockfile under the XDG
  configuration root and materialize dependencies under the XDG data root;
- `--project <path>` - use a specific project manifest directory;
- `--target <name>` - narrow deployment for this invocation;
- `--dry-run` - produce a plan without writes;
- `--frozen` - require exact lockfile replay;
- `--offline` - prohibit network access;
- `--json` - emit versioned machine output;
- `--verbose` - show source, resolution, and deployment detail.

`--project` is an advanced location override. It does not create a distinct product
concept or alternate manifest format.

## Exit behavior

- `0` - operation completed or read-only check passed;
- `1` - resolution, validation, integrity, safety, policy, or I/O failure;
- `2` - command usage error.

JSON errors contain a stable schema version, code, message, and structured context.
