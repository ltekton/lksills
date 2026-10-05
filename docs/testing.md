# Testing

> Status: target lifecycle test strategy

Tests must prove reproducibility and safe state transitions through the shipped
binary. Unit coverage supports that claim but does not replace process-level
artifacts and failure injection.

## Repository gate

```sh
mise run check
```

The gate runs formatting, Clippy with warnings denied, and all workspace tests.
Before reporting a documentation or implementation phase complete, also run:

```sh
git diff --check
```

Validate relative Markdown links when documentation changes.

## Test boundaries

- Core unit tests live beside the module that owns the invariant.
- CLI process tests live in `crates/lskills-cli/tests/` and invoke
  `CARGO_BIN_EXE_lskills`.
- source, registry, cache, project, global configuration, global data, global state,
  and target roots are separate temporary directories;
- Git tests use temporary repositories and local transports.
- tests set `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`,
  `XDG_STATE_HOME`, and agent-directory variables explicitly; they never read or
  write the operator's real configuration.
- External content is inert fixture data. Executable-looking files prove that no
  lifecycle step executes package content.

## Contract suites

### Manifest and lockfile

Cover:

- strict schema and unknown-field behavior;
- concise and structured requirement parsing;
- canonical source and package identity;
- root and multi-skill package discovery;
- deterministic semantic-manifest digest;
- canonical TOML ordering and byte-stable rewrites;
- unchanged lock reuse during normal install;
- frozen rejection of missing, stale, or inconsistent state;
- forward schema-version failures with stable error codes.

Golden lockfiles are appropriate only after canonical ordering is specified. Test
semantic round trips as well as bytes so formatting snapshots do not hide model
errors.

### Scope and storage roots

Cover:

- project discovery from nested directories;
- global manifest and lock lookup under `XDG_CONFIG_HOME/lskills`;
- default `~/.config/lskills` behavior when `XDG_CONFIG_HOME` is unset or empty;
- explicit failure for relative XDG directory values;
- project `lskills_modules/` and XDG global data materialization roots;
- XDG cache sharing without scope-graph sharing;
- XDG state isolation for global locking and recovery;
- complete isolation from the operator's real home and agent targets.

### File inventory and integrity

Use one shared conformance corpus for every source adapter, materializer, deployer,
packer, and verifier. It includes:

- nested and hidden regular files;
- binary bytes and executable mode bits;
- relative links inside Markdown;
- absolute and parent traversal;
- file and directory symlinks;
- FIFOs or other supported test special files;
- non-UTF-8 names where the platform permits them;
- missing, extra, and hash-mismatched archive entries;
- hidden Unicode findings.

Assert that scan, digest, copy, pack, and report consume the same authorized path
set. A file excluded from one stage must not reappear in another.

### Resolution graph

Test local and Git graphs with:

- direct and transitive requirements;
- branch, tag, exact commit, full URL, shorthand, and repository subpath forms;
- package identity aliases and duplicate textual references;
- cycles and incompatible immutable requirements;
- selected and all-export dependencies;
- moved branches with normal, frozen, update, offline, and outdated operations;
- cache poisoning and mismatched remote identity;
- ancestry-rich `why` and conflict diagnostics.

Remote tests must be hermetic. Network-dependent tests do not belong in the normal
gate.

### Composition and targets

For every target and scope, cover:

- local, forked, and dependency skill placement;
- explicit selection and aliasing;
- collision refusal independent of declaration order;
- project/global path resolution;
- exact byte and mode preservation;
- ownership rows and deployed-file hashes;
- unchanged, changed, stale, shared, edited, and unowned outputs;
- target-native validation;
- effective project/global inspection without graph contamination.

Target contract tests should consume the same composition fixture so adapter
behavior is comparable.

### Transactions and concurrency

Inject failure after each mutation boundary:

1. lifecycle lock acquisition;
2. manifest rewrite staging;
3. source acquisition;
4. content scan;
5. target staging;
6. target activation;
7. lockfile write;
8. stale-output cleanup;
9. transaction finalization.

After process interruption, the next mutating command must recover to the prior or
next complete state. Read-only commands must report recovery-needed state without
changing it. Run competing process tests to prove that mutations serialize before
their first state read.

### Forks

Cover:

- promotion from the exact locked skill tree;
- local rename and replacement semantics;
- manifest, local source, and lock-base atomicity;
- source/destination collisions;
- deleted cache after promotion;
- local edits plus moved upstream;
- diff against locked base and latest upstream;
- pack and publish provenance for the derivative.

### Bundles and publication

Cover deterministic directory and archive output, exhaustive inventories, schema
compatibility, exact provenance, immutable version conflicts, interrupted upload,
credential redaction, and installation without original sources. Compare hashes
and full bytes across clean temporary runs.

### Security

Test hidden-Unicode classes, path containment, symlink races where practical,
archive bombs or limits once archives exist, credential-bearing URLs, malicious
Git refs beginning with option-like text, and deletion authorization. Security
findings must occur before target writes.

## Process acceptance journeys

Maintain end-to-end scenarios for:

1. initialize, author, lock, install, audit, and pack a local project;
2. install a selected skill from a pinned Git package and replay it frozen;
3. resolve a transitive graph, explain it with `why`, then update one node;
4. install independent project and global scopes and inspect the effective view;
5. fork an external skill, modify it, audit it, and pack a derivative bundle;
6. uninstall a dependency while retaining user-edited deployed content;
7. recover an interrupted install without partial lock or target state;
8. verify and install a packed bundle while offline;
9. publish an immutable version through the hermetic registry fixture when that
   phase exists;
10. prove source scripts, hooks, and binaries were never executed.

Each journey inspects the manifest, lockfile, target files, ownership records, and
exit/output envelope. Exit status alone is insufficient evidence.

## Determinism oracle

For a fixed manifest, lockfile, source set, configuration, and target set:

1. materialize and deploy in a clean temporary environment;
2. record lock bytes, authorized inventories, artifact maps, hashes, and bundles;
3. repeat in a different absolute directory;
4. require identical logical records and artifact bytes;
5. allow absolute machine paths only in diagnostics or explicitly non-portable
   local-source fields, never in portable bundle identity.

`audit --ci` performs the equivalent scratch replay and fails closed when an exact
source cannot be obtained or verified.

## Completion evidence

A phase is complete only when:

- its contract and process tests pass against the actual binary;
- fault paths preserve or recover complete state;
- persisted TOML and deployed files have been inspected;
- tests remain isolated from the machinery repository and operator state;
- the repository and documentation gates pass.
