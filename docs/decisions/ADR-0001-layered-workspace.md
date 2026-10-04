# ADR-0001 - Layered Cargo workspace

- Status: Superseded by ADR-0009
- Date: 2025-??-??
- Deciders: lskills maintainers

## Context

lskills combines domain policy, filesystem and Git effects, agent-specific format
rules, and CLI/process behavior. Keeping those concerns in one crate makes it easy
to accidentally read the environment from core code, execute process effects during
validation, or make a target layout part of generic asset identity.

The machinery repository is also separate from user asset workareas. The binary
must open external workareas without making the domain depend on a particular
checkout implementation.

## Decision

Use a Cargo workspace with four primary crates:

```text
lskills-cli
  -> lskills-adapters
  -> lskills-application
  -> lskills-domain
```

### Domain

`lskills-domain` owns pure identities, validated paths, asset kinds, origins,
workareas, managed copies, snapshots, trees, baselines, plans, conflicts, packages,
catalog entries, and matchable errors.

### Application

`lskills-application` owns use cases and ports. It receives a selected workarea
context and coordinates acquisition, local management, upstream checks, packaging,
and publication without knowing concrete filesystem, Git, network, or agent-home
implementations.

### Adapters

`lskills-adapters` implements ports for workareas, origins, storage, Git,
filesystem, formats, targets, packages, and local catalogs.

### CLI

`lskills-cli` is the composition root. It parses arguments, reads actual environment
and process state, constructs adapters, dispatches use cases, renders output, and
maps errors to exit codes.

## Dependency rules

- domain has no external-effect policy;
- application depends on domain and defines ports;
- adapters depend on application/domain and implement ports;
- CLI constructs the graph and owns process boundaries;
- format and target adapters do not add branches to generic domain policy;
- no crate may hide effects through process-global state.

## Consequences

Positive:

- compiler-visible dependency direction;
- deterministic domain tests;
- testable application use cases with fakes;
- workarea and origin effects remain explicit;
- target additions remain adapters;
- the machinery repository can be tested without production assets.

Costs:

- more crates and types before the first end-to-end command;
- adapter contracts must be designed carefully;
- some simple file operations require injected ports;
- CLI DTOs and domain values need explicit mapping.

## Rejected alternatives

### One core crate

Rejected because it permits effects and target policy to spread through the domain and
makes the machinery/workarea boundary implicit.

### Python orchestration around a Rust core

Deferred. A single Rust implementation is preferred initially. A future external
adapter can be introduced only when a format has a compelling runtime requirement.

### Generic plugin loading inside the domain

Rejected. Format-specific behavior belongs behind explicit adapter contracts, and
imported plugin code must never execute as part of lifecycle management.

## Validation

M1 must demonstrate:

- Cargo dependency direction;
- domain tests with no filesystem or environment reads;
- application tests with fake workarea and origin ports;
- adapter tests against temporary workareas;
- CLI process tests using the actual binary.
