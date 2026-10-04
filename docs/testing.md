# Testing

> Status: reduced MVP strategy

Tests prove the actual two-crate binary and core behavior for one workarea assembled
from multiple origins.

## Gate

```sh
mise run check
```

The gate runs formatting, clippy with warnings denied, and all workspace tests.
Cargo is provided by mise.

## Fixture policy

Use synthetic origins and temporary workareas. A source fixture must be clearly
separate from the destination workarea and the machinery repository.

Fixtures may include executable-looking files and nested data to prove that import
copies them without executing them.

## Core tests

Unit tests should cover:

- provenance serialization and reload;
- local digest and Git revision formatting;
- bundle and member collision detection;
- destination path validation and source/destination disjointness;
- Git cache identity and pinned revision verification;
- deterministic file enumeration;
- all-or-nothing import planning and interrupted-transaction recovery;
- malformed or dangling provenance records.

## Process tests

Process tests live in `crates/lskills-cli/tests/` and run `CARGO_BIN_EXE_lskills`.
Required scenarios:

1. import a local origin bundle into an explicit temporary root;
2. import a pinned Git origin bundle;
3. import bundles from two origins into one root;
4. inspect provenance after reopening the root;
5. reject an existing bundle collision without partial files;
6. reject a referenced skill collision without partial files;
7. run `import --check` and verify no destination writes;
8. reject source/destination overlap and unsafe special-file content;
9. prove source scripts and executable-looking files never run;
10. validate and publish the assembled workarea;
11. prove the machinery repository remains unchanged;
12. recover a partially moved transaction without leaving a destination bundle.

Keep the existing prototype corpus, installation, remote, release, and guard tests
running. They are regression coverage, not reasons to expand the new scope.

## Completion evidence

A phase is complete only when:

- the new process tests pass against the actual binary;
- existing regression tests remain active;
- temporary sources and workareas are isolated;
- provenance and generated output are inspected, not inferred from exit status;
- `mise run check` passes.
