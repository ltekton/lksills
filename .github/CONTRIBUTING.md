# Contributing to lskills

Thank you for contributing. This repository is the lskills machinery repository. It
must not accumulate a production skill collection or user asset workarea as part of
normal development.

## Product boundary

The target product manages external agent assets through named external workareas:

```text
origin -> selected workarea managed copy -> target projection
       -> explicit update coordination -> package/catalog
```

Keep these roles separate in code and tests:

- the lskills repository contains Rust source, tests, and documentation;
- an origin is a read-only source snapshot;
- a workarea contains editable managed copies and durable state;
- installations, packages, and catalogs are projections.

One origin asset may have copies in multiple workareas. One workarea may contain
assets from multiple origins. Mutating operations select one workarea explicitly or
through documented configuration.

## Prerequisites

- **git** - any recent version;
- **mise** - manages the Rust toolchain; install from
  [mise.jdx.dev](https://mise.jdx.dev/).

No other dependencies are required for the current repository gate. mise installs
the toolchain declared in `mise.toml`.

## Development setup

```sh
git clone https://github.com/ltekton/lskills.git
cd lskills
mise install
mise run check
```

`mise run check` is the gate. A fresh clone should pass it before feature work
begins.

## Project layout

Current prototype:

```text
crates/
  lskills-core/   - earlier skills-root engine
  lskills-cli/    - earlier prototype binary
docs/             - active design and transition documentation
```

Target rewrite:

```text
crates/
  lskills-domain/       - pure identities, trees, plans, and invariants
  lskills-application/  - use cases and effect ports
  lskills-adapters/     - workarea, origin, format, target, and publication effects
  lskills-cli/          - parser, composition root, output, and process boundary
```

The target dependency and workarea design are documented in
[`docs/architecture.md`](../docs/architecture.md).

## Design-first workflow

Before implementing a new lifecycle behavior:

1. identify whether it changes the product boundary or a local implementation;
2. update the relevant requirement, domain model, or subsystem specification;
3. record an architectural decision when the choice affects workareas, origins,
   source-of-truth rules, storage, dependencies, or external effects;
4. add the work package and evidence to `docs/implementation-plan.md`;
5. implement the smallest reversible slice;
6. test the actual binary or adapter boundary;
7. run the repository gate.

Do not hide a new storage or source relationship in an adapter just to avoid updating
the design. In particular, do not reintroduce an implicit global `--root` model as
the target workarea abstraction.

## Development workflow

1. Branch from `main`: `git checkout -b <type>/<short-description>`.
2. Make one logical change per reviewable change.
3. Write or update tests for behavior changes.
4. Run `mise run check` and inspect the output.
5. Use [Conventional Commits](https://www.conventionalcommits.org/):

| Prefix | Use |
|---|---|
| `feat:` | new user-visible capability |
| `fix:` | bug fix |
| `test:` | tests only |
| `docs:` | documentation only |
| `refactor:` | internal restructure without behavior change |
| `chore:` | dependencies, CI, tooling, or maintenance |

Do not commit or push without explicit approval.

## Check gate

`mise run check` runs:

```sh
eval "$(mise env -s zsh)"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
INSTA_UPDATE=no LSKILLS_REQUIRE_GIT=1 cargo test --workspace
```

During iteration:

```sh
eval "$(mise env -s zsh)"
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Cargo is provided through mise in this environment.

## Tests

### Domain tests

Place pure invariant tests beside the owning domain module. They must not read a
real workarea, environment, clock, network, or process.

### Application tests

Use fake ports to test selection, acquisition, installation, updates, packaging, and
publication policy. Assert that checks are read-only and that failed applies preserve
old state.

### Adapter tests

Use temporary directories and local bare Git repositories. Test actual filesystem
layout, Git status, path safety, staging, and deterministic artifact bytes.

### Process tests

Process tests that use `CARGO_BIN_EXE_lskills` MUST live in
`crates/lskills-cli/tests/`, because that environment variable is only set for the
crate that defines the binary.

Target workarea process tests must prove:

- machinery repository is not written;
- two workareas are isolated;
- one origin asset can be imported into both;
- one workarea can contain multiple origins;
- workarea selection precedence and conflicts;
- check is read-only;
- package does not publish;
- imported executable payloads are never run.

Current prototype tests use skills-root fixtures. Keep those tests labeled as
prototype coverage and do not let them define the new workarea contract.

### Fixture policy

Use synthetic assets and temporary workareas. Do not add real user assets or a
persistent collection repository under `crates/` or the machinery root. If a stable
fixture is necessary, make its origin/workarea/project role explicit.

### Snapshots

`cargo-insta` is not installed. To accept a deliberate snapshot update, rename
`.snap.new` to `.snap`, remove the `assertion_line:` line, inspect the full diff, and
run `git diff --check`.

## Documentation changes

Keep these documents aligned:

- `docs/product.md` - product boundary and use cases;
- `docs/product-requirements.md` - requirements and acceptance;
- `docs/architecture.md` - layers and ports;
- `docs/domain-model.md` - types and invariants;
- `docs/specs/` - behavior contracts;
- `docs/implementation-plan.md` - milestones and evidence;
- `docs/decisions/` - architectural choices.

If a change affects `assets_url`, workarea selection, origin mapping, baseline
storage, or source-of-truth rules, it needs documentation and likely an ADR.

## Pull requests

Before requesting review:

1. run `mise run check`;
2. run the narrowest changed test separately and inspect it;
3. run `git diff --check`;
4. describe changed files and verification;
5. identify blockers or unrun checks honestly;
6. keep the pull request focused.

## Reporting bugs and features

Use the repository issue templates. Include the selected workarea, origin role,
command, expected effect, actual effect, and whether the issue involves the current
prototype or target rewrite.
