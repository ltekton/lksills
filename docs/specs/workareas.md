# Workarea Specification

> Status: reduced MVP baseline
> Prefixes: `ROOT`, `PROV`

## One workarea

The MVP operates on one ordinary local skills-root per invocation:

```text
<root>/
  skills/
  bundles/
  .lskills/provenance.toml
```

`--root` is the destination selector. There is no named registry, default
workarea, project workarea, workarea identity, workarea clone, or workarea refresh
operation.

The operator may use normal Git commands to version the workarea. lskills does not
commit, push, pull, or reset it.

## Multiple origins

The workarea can contain bundles from several origins. Each imported bundle has an
independent provenance entry. The workarea itself is not an origin for its contents.

```text
one workarea
  bundle-a <- origin-a
  bundle-b <- origin-b
```

## Import safety

An import must:

1. require an explicit destination root;
2. resolve one local or Git source;
3. validate one bundle and all referenced skills;
4. check every destination collision before writing;
5. stage the complete copy;
6. install the bundle and member skills together;
7. record provenance after success.

A failed import must not leave a partial selected bundle or successful provenance
entry.

## Deferred

Multiple workareas, workarea Git synchronization, baselines, update state, and
operation ledgers are not part of this specification.
