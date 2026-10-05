# Project and Global Scope Specification

> Prefix: `SCOPE`

## Project discovery

Project scope is the default. Starting at the current directory, lskills searches
ancestors for `lskills.toml` and stops at the first match. The manifest directory
is the base for local dependency paths, local skill paths, project targets, and the
sibling `lskills.lock.toml`.

`--project <path>` selects a manifest directory explicitly. It is an advanced path
override, not a registered project or named workarea.

## Global scope

`--global` or `-g` uses the XDG configuration root:

```text
${XDG_CONFIG_HOME:-$HOME/.config}/lskills/lskills.toml
${XDG_CONFIG_HOME:-$HOME/.config}/lskills/lskills.lock.toml
```

When `XDG_CONFIG_HOME` is set and non-empty, lskills must use
`$XDG_CONFIG_HOME/lskills`. XDG directory variables must be absolute; a relative
value is a configuration error rather than an alternate relative root.

The global manifest has the same dependency syntax and lock semantics as a project
manifest. It deploys only to targets with documented user-scope locations.

The global and project graphs are independent. lskills never uses global packages
to satisfy project dependencies. `lskills list --effective` may display what an
agent can see across both scopes, while labeling the source scope of every skill.

## User configuration

Machine preferences live in:

```text
${XDG_CONFIG_HOME:-$HOME/.config}/lskills/config.toml
```

Suitable values include:

- default targets;
- Git transport preference;
- package cache location;
- registry aliases and URLs;
- output and audit preferences.

Dependency intent belongs in a scope manifest. Secrets come from environment
variables or credential helpers.

## Precedence

For settings that admit every layer:

```text
CLI flag
  > selected scope manifest
  > user config
  > target auto-detection
  > built-in default
```

A lower layer cannot override an explicit higher-layer choice. Diagnostic output
must identify the winning source for effective target and registry decisions.

## Materialization and cache roots

Downloaded and resolved package content has two layers:

```text
<project>/lskills_modules/                              # project materializations
${XDG_DATA_HOME:-$HOME/.local/share}/lskills/modules/   # global materializations
${XDG_CACHE_HOME:-$HOME/.cache}/lskills/                # shared downloads/cache
```

`lskills_modules/` is analogous to APM's `apm_modules/`: it contains generated,
verified dependency packages for that project and can be reconstructed from
`lskills.lock.toml`. Global materializations use the XDG data directory because
package content is data, not configuration. The shared cache stores acquisition
objects such as Git data and registry archives; it is not a scope's installed
state.

Project transaction state may live under `<project>/.lskills/`. Global lifecycle
locks and staging use
`${XDG_STATE_HOME:-$HOME/.local/state}/lskills/`. These paths are disposable
implementation state except while an operation or recovery is active. Local
source, manifests, and lockfiles never live in cache or staging directories.
