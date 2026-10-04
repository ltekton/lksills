# Local Management Specification

> Status: reduced MVP baseline
> Prefix: `PUB`

The imported workarea remains the editable source for the existing skills-root
commands:

```text
list -> show -> validate -> publish
```

Imported provenance is displayed where useful but is not interpreted as update
state. `publish` reads the current workarea, not a source cache, and uses the
existing deterministic renderer.

The current `install` command and its tests remain prototype behavior. The MVP does
not add a new projection model, receipt system, target registry, user scope, or
project manifest.

A source edit is an ordinary workarea edit. lskills does not automatically refresh
or reinstall it.
