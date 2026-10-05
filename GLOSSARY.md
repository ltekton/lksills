# lskills Glossary

lskills manages the source, resolution, installation, modification, composition,
and publication of Agent Skills.

## Authoring and distribution

**Skill**:
A cohesive directory containing `SKILL.md` and optional relative resources. The
skill directory name is its runtime identity.
_Avoid_: Asset, primitive, module

**Package**:
A named, versioned source unit that exports one or more skills and may declare
package dependencies. A directory containing `lskills.toml` is a package.
_Avoid_: Workarea, collection

**Bundle**:
A deterministic, self-contained artifact produced from a resolved package by
`lskills pack`. A bundle contains selected skills, provenance, and an exhaustive
file-hash manifest.
_Avoid_: Package, source repository

**Fork**:
A local skill copied from an exact locked external skill for modification under a
new package identity while retaining its upstream base as provenance.
_Avoid_: Override, patch, imported skill

**Target**:
A named agent integration that maps selected skills into the project- or
user-scope directory read by that agent.
_Avoid_: Agent, runtime

## Dependency management

**Manifest**:
The human-authored `lskills.toml` file declaring package identity, dependencies,
local skills, forks, and targets.
_Avoid_: Configuration file, lockfile

**Lockfile**:
The generated `lskills.lock.toml` file recording the exact resolved dependency
graph, selected skills, source revisions, content hashes, fork bases, and deployed
file ownership.
_Avoid_: Manifest, provenance sidecar

**Dependency**:
A package requirement declared in the manifest using a Git, registry, bundle, or
local-path reference.
_Avoid_: Origin, import

**Resolution**:
The deterministic mapping from dependency requirements to exact package sources,
versions or revisions, selected skills, and content hashes.
_Avoid_: Installation

**Selection**:
The exact exported skills chosen from a package dependency. Omitted selection
means all exported skills.
_Avoid_: Bundle membership

**Alias**:
An explicit generated installed identity for a selected skill. Aliasing leaves
source untouched and changes only the composed directory and `SKILL.md` name, with
source and projected hashes recorded.
_Avoid_: Implicit rename

## Scope and state

**Project scope**:
The manifest, lockfile, local skills, and target deployments associated with the
nearest `lskills.toml` found from the current directory.
_Avoid_: Workarea

**Global scope**:
The independent manifest and lockfile under
`${XDG_CONFIG_HOME:-$HOME/.config}/lskills/`, used to manage user-level skill
installation.
_Avoid_: Default workarea, user workarea

**Materialization**:
A disposable verified copy of a resolved package: project copies live in
`lskills_modules/`, while global copies live under the XDG data root.
_Avoid_: Workarea, installation, cache

**Deployment**:
The projection of selected skills into a target's native skill directory, together
with ownership and content hashes recorded in the lockfile.
_Avoid_: Publish, materialization

**Drift**:
A difference between the manifest, lockfile, source content, materialized packages,
or deployed target files that prevents exact replay.
_Avoid_: Update
