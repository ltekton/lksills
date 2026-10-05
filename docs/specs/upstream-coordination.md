# Update and Upstream Coordination Specification

> Prefixes: `LOCK`, `FORK`

## Install versus update

Install synchronizes desired state to the existing lock. Update asks authoritative
sources whether a different exact resolution satisfies the manifest.

```console
lskills outdated              # read-only freshness report
lskills update --dry-run       # proposed resolution changes
lskills update                # confirm and apply
lskills update <PACKAGE>       # selected refresh
```

A normal install never silently accepts a newer branch commit, tag, or registry
version for an unchanged requirement.

## Update plan

An update plan classifies each package as:

- added;
- updated;
- removed;
- unchanged;
- unavailable;
- conflicted.

For each changed package it reports requested constraint, previous exact identity,
new exact identity, affected selected skills, fork relationships, and target
changes. Declining the plan leaves manifest, lockfile, cache-visible state, and
targets unchanged.

## Local paths

Local path dependencies have no remote freshness operation. Their current tree
hash is compared with the lock. An ordinary non-frozen install accepts intentional
local source changes and regenerates affected lock and deployment hashes. Frozen
install reports drift.

## Forks

A fork retains:

- upstream package and skill identity;
- exact base revision/version;
- base skill-tree digest;
- current local skill-tree digest.

`outdated` may report that a fork's upstream has moved, but normal package update
does not modify the fork or advance its base. `diff --upstream --latest` previews
the difference.

A future automated fork update must be a separate, explicit operation with a
three-way base/local/upstream model. It must stage changes, surface conflicts in
local source, and update the recorded base only after acceptance.

## Removal and cleanup

Removing a requirement recalculates the reachable graph. A package retained by
another dependency remains locked. Deployed content remains when another skill
owns it. Unchanged content with no surviving owner is removed; edited content is
retained and reported.
