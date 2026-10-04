# lskills CLI Reference

> Status: reduced MVP target surface

The MVP has one workarea root and multiple source origins. The current prototype
commands remain available where documented, but only the import and existing
skills-root workflow are in the new scope.

## Root and origin

`--root <PATH>` identifies the local workarea. For import it is required. It may
also be supplied through `LSKILLS_ROOT` for existing commands.

An origin is a local skills-root path or a Git spec. A skills-root has:

```text
skills/
bundles/
```

The existing `--repo` option remains a prototype read-only source selector for
commands that already support it. New import syntax uses the origin as a positional
argument so source and destination cannot be confused.

## Target commands

### Import

```console
lskills import <ORIGIN> \
  --root <WORKAREA> \
  --bundle <BUNDLE> \
  [--check] [--json]
```

The command:

1. resolves the origin;
2. loads and validates the selected bundle;
3. verifies every referenced skill exists and is safe;
4. reports or stages the complete destination copy;
5. records provenance after a successful apply.

`--check` previews the selected bundle, member skills, source revision/digest,
collisions, and planned files without writing anything.

Import never replaces an existing bundle or skill. A collision is an error and the
complete import is abandoned.

### List

```console
lskills list --root <WORKAREA> [--json]
```

Lists bundles and skills. Imported bundles may include their provenance summary.

### Show

```console
lskills show <BUNDLE-OR-SKILL> --root <WORKAREA> [--json]
```

Shows an existing bundle or skill and, when available, its source provenance.

### Validate

```console
lskills validate --root <WORKAREA> [--json]
```

Runs the existing skills-root validation and reports errors and warnings.

### Publish

```console
lskills publish --root <WORKAREA> [--check] [--json]
```

Runs the existing deterministic native renderer. `--check` reports generated-tree
drift without writing. Publishing does not fetch origins, update provenance, commit,
or push.

## Provenance

The workarea sidecar is:

```text
<WORKAREA>/.lskills/provenance.toml
```

It records the imported destination bundle, source locator, source selector, and
resolved revision or local digest. It records provenance, not freshness or update
state.

## Output and errors

JSON results include a version field and operation status. Errors use stable codes
for usage, source resolution, validation, collision, safety, and I/O failures.
Usage errors exit 2. Other failures exit 1.

Import results identify both the source origin and destination root. No output treats
an origin as the workarea or the workarea as the origin of every bundle.

## Existing prototype commands

These remain available for regression compatibility but are not expanded by the
multi-origin MVP:

```text
install
catalog
doctor
hygiene
tokens
release
new-skill
new-bundle
rm-skill
rm-bundle
```

They should not acquire new multi-workarea, package, update, or generic-asset
semantics during this scope.
