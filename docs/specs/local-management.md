# Authoring, Forks, and Deployment Specification

> Prefixes: `SKILL`, `FORK`, `INST`

## Authoring

A local skill is either:

```text
SKILL.md                 # one-skill package
```

or:

```text
skills/<name>/SKILL.md   # multi-skill package
```

Everything beneath the skill directory belongs to that skill. `scripts/`,
`references/`, `assets/`, and `examples/` are conventional resource directories,
but lskills treats their files as data.

For `skills/<name>/`, the directory name is authoritative. For a root `SKILL.md`,
the final segment of the declared package name is authoritative. If frontmatter
includes `name`, it must match the applicable identity. The package's human
description and the skill's runtime activation description are separate fields and
are never inferred from each other.

## Local authoring loop

```console
lskills new skill review
# edit skills/review/
lskills validate
lskills install --dry-run
lskills install
lskills audit
```

The installed target copy is generated output. Durable edits belong under package
source.

## Forking an external skill

```console
lskills fork acme/team-skills --skill review --as team-review
```

Preconditions:

- the source dependency and skill are present in the lockfile;
- its materialized bytes match the locked hash;
- `skills/team-review/` does not exist;
- the resulting local identity does not collide with another export.

Effects:

1. copy the complete locked skill tree to `skills/team-review/`;
2. add `[forks.team-review]` with the upstream skill identity;
3. make the local fork the explicit replacement for that upstream skill;
4. retain exact base revision and digest in the generated lockfile;
5. leave the dependency cache and deployed targets untouched until install.

A fork is now ordinary local source. It may be modified and published under the
current package identity. Republishing never claims the upstream package identity.

## Comparing upstream

```console
lskills diff team-review --upstream
lskills diff team-review --upstream --latest
```

The first compares against the exact locked base. The second resolves current
upstream read-only and compares against it. Neither changes local files or lock
state.

Automatic merge or rebase is not part of the initial implementation. The persisted
base identity makes a future three-way operation possible without changing fork
semantics.

## Target deployment

Targets map skill directories to native agent paths. The built-in profiles include
project and global locations where supported. Agent-specific global paths use each
agent's documented configuration home.

A target receives a composed skill projection and does not alter it. Without an
alias, all source bytes are unchanged. An explicit installed-name alias changes
only the projected directory name and the `name` field in projected `SKILL.md`;
the lockfile records source and projected hashes. lskills never rewrites source
content implicitly.

Project and global deployment are independent operations. An effective-list command
may detect runtime-visible collisions across scopes, but project install never
imports global packages into its lock graph.
